//! S3-compatible object storage — MinIO, Backblaze B2, Cloudflare R2, Wasabi, AWS.
//!
//! Object stores have no directories: keys are flat strings that merely look
//! like paths. `ListObjectsV2` with `delimiter=/` fakes one level of hierarchy —
//! matching keys come back as objects, and everything below a shared prefix
//! collapses into a "common prefix" we present as a folder. That's enough for
//! the parser, which only ever asks for one level at a time.
//!
//! Requests are signed with `rusty-s3` (presigned URLs) and then issued by
//! `reqwest`, so no AWS SDK is pulled in.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header;
use reqwest::Client;
use rusty_s3::actions::{ListObjectsV2, S3Action};
use rusty_s3::{Bucket, Credentials, UrlStyle};
use url::Url;

use super::{RemoteBackend, RemoteNode, ServerConfig, ServerSecrets};

/// Presigned URLs are used immediately; a short window is plenty and limits
/// how long a leaked URL stays valid.
const SIGN_TTL: Duration = Duration::from_secs(120);

pub struct S3Backend {
    client: Client,
    bucket: Bucket,
    credentials: Credentials,
}

pub async fn connect(
    config: &ServerConfig,
    secrets: &ServerSecrets,
) -> Result<Arc<dyn RemoteBackend>, String> {
    let endpoint = config.host.trim();
    if endpoint.is_empty() {
        return Err("This server has no endpoint URL set.".to_string());
    }
    if config.bucket.trim().is_empty() {
        return Err("This server has no bucket name set.".to_string());
    }
    if config.username.trim().is_empty() || secrets.password.is_empty() {
        return Err("This server is missing its access key or secret key.".to_string());
    }

    let url = Url::parse(endpoint)
        .map_err(|e| format!("That endpoint isn't a valid URL ({e}). Include https://"))?;

    // Path style (`endpoint/bucket/key`) is what self-hosted gateways serve;
    // AWS and R2 want virtual-host style (`bucket.endpoint/key`).
    let style = if config.path_style {
        UrlStyle::Path
    } else {
        UrlStyle::VirtualHost
    };
    let region = if config.region.trim().is_empty() {
        "us-east-1".to_string()
    } else {
        config.region.trim().to_string()
    };

    let bucket = Bucket::new(url, style, config.bucket.trim().to_string(), region)
        .map_err(|e| format!("Couldn't configure that bucket: {e}"))?;

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Couldn't create an HTTP client: {e}"))?;

    Ok(Arc::new(S3Backend {
        client,
        bucket,
        credentials: Credentials::new(config.username.trim(), secrets.password.clone()),
    }))
}

/// Our paths are `/a/b`; S3 keys are `a/b`. Directory prefixes carry a trailing
/// slash so `delimiter` groups on them.
fn key_of(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}

fn prefix_of(path: &str) -> String {
    let key = key_of(path);
    if key.is_empty() || key.ends_with('/') {
        key
    } else {
        format!("{key}/")
    }
}

#[async_trait]
impl RemoteBackend for S3Backend {
    async fn list_dir(&self, path: &str) -> Result<Vec<RemoteNode>, String> {
        let prefix = prefix_of(path);
        let mut nodes = Vec::new();
        let mut continuation: Option<String> = None;

        loop {
            let mut action: ListObjectsV2 = self.bucket.list_objects_v2(Some(&self.credentials));
            if !prefix.is_empty() {
                action.with_prefix(prefix.clone());
            }
            action.with_delimiter("/");
            if let Some(token) = &continuation {
                action.with_continuation_token(token.clone());
            }
            let url = action.sign(SIGN_TTL);

            let resp = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("Couldn't reach the storage endpoint: {e}"))?;
            let status = resp.status();
            let body = resp
                .text()
                .await
                .map_err(|e| format!("Couldn't read the listing: {e}"))?;
            if !status.is_success() {
                return Err(s3_error(status, &body));
            }

            let parsed = ListObjectsV2::parse_response(&body)
                .map_err(|e| format!("Couldn't parse the listing: {e}"))?;

            // Common prefixes are the "folders" one level down.
            for cp in &parsed.common_prefixes {
                let trimmed = cp.prefix.trim_end_matches('/');
                let name = trimmed.rsplit('/').next().unwrap_or_default().to_string();
                if name.is_empty() || name.starts_with('.') {
                    continue;
                }
                nodes.push(RemoteNode {
                    name,
                    path: format!("/{trimmed}"),
                    is_dir: true,
                    size: 0,
                });
            }

            for object in &parsed.contents {
                // The prefix itself comes back as a zero-byte object when the
                // bucket has an explicit folder marker — not a file.
                if object.key == prefix {
                    continue;
                }
                let name = object
                    .key
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_string();
                if name.is_empty() || name.starts_with('.') {
                    continue;
                }
                nodes.push(RemoteNode {
                    name,
                    path: format!("/{}", object.key),
                    is_dir: false,
                    size: object.size,
                });
            }

            continuation = parsed.next_continuation_token;
            if continuation.is_none() {
                break;
            }
        }

        Ok(nodes)
    }

    async fn read_range(&self, path: &str, start: u64, end: u64) -> Result<Vec<u8>, String> {
        if end < start {
            return Ok(Vec::new());
        }
        let key = key_of(path);
        let url = self
            .bucket
            .get_object(Some(&self.credentials), &key)
            .sign(SIGN_TTL);

        let resp = self
            .client
            .get(url)
            .header(header::RANGE, format!("bytes={start}-{end}"))
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the storage endpoint: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(s3_error(status, &body));
        }
        let full = status == reqwest::StatusCode::OK;
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Couldn't read {key}: {e}"))?;

        if full {
            let from = (start as usize).min(bytes.len());
            let to = ((end + 1) as usize).min(bytes.len());
            return Ok(bytes[from..to].to_vec());
        }
        Ok(bytes.to_vec())
    }

    async fn size_of(&self, path: &str) -> Result<u64, String> {
        let key = key_of(path);
        let url = self
            .bucket
            .head_object(Some(&self.credentials), &key)
            .sign(SIGN_TTL);

        let resp = self
            .client
            .head(url)
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the storage endpoint: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "The endpoint returned {} for {key}.",
                resp.status()
            ));
        }
        resp.headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("The endpoint didn't report a size for {key}."))
    }

    async fn read_all(&self, path: &str) -> Result<Vec<u8>, String> {
        let key = key_of(path);
        let url = self
            .bucket
            .get_object(Some(&self.credentials), &key)
            .sign(SIGN_TTL);

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the storage endpoint: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(s3_error(status, &body));
        }
        Ok(resp
            .bytes()
            .await
            .map_err(|e| format!("Couldn't read {key}: {e}"))?
            .to_vec())
    }
}

/// S3 reports failures as an XML document; surface its `<Message>` rather than
/// a bare status code, since "SignatureDoesNotMatch" tells the user far more.
fn s3_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = body
        .split_once("<Message>")
        .and_then(|(_, rest)| rest.split_once("</Message>"))
        .map(|(msg, _)| msg.trim().to_string());
    match message {
        Some(m) if !m.is_empty() => format!("Storage error {status}: {m}"),
        _ => format!("Storage error {status}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_get_a_trailing_slash() {
        assert_eq!(prefix_of("/Courses/Rust"), "Courses/Rust/");
        assert_eq!(prefix_of("/Courses/Rust/"), "Courses/Rust/");
        assert_eq!(prefix_of("/"), "");
    }

    #[test]
    fn keys_drop_the_leading_slash() {
        assert_eq!(key_of("/Courses/01 Intro.mp4"), "Courses/01 Intro.mp4");
    }

    #[test]
    fn error_body_message_is_surfaced() {
        let body = "<Error><Code>SignatureDoesNotMatch</Code><Message>The request signature we calculated does not match</Message></Error>";
        let msg = s3_error(reqwest::StatusCode::FORBIDDEN, body);
        assert!(msg.contains("does not match"));
    }
}
