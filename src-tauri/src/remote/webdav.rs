//! WebDAV backend — Nextcloud, ownCloud, `rclone serve webdav`, Apache mod_dav.
//!
//! Listing is a `PROPFIND` with `Depth: 1`; streaming is a plain ranged `GET`.
//! Both are ordinary HTTP, so this backend is just `reqwest` plus a small
//! response parser.

use std::sync::Arc;

use async_trait::async_trait;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::header;
use reqwest::{Client, Method};

use super::{RemoteBackend, RemoteNode, ServerConfig, ServerSecrets};

/// Everything outside RFC 3986's unreserved set gets escaped. Escaping `.` and
/// `-` too would work, but leaves unreadable URLs in error messages.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

pub struct WebdavBackend {
    client: Client,
    /// Base URL with no trailing slash, e.g. `https://cloud.example.com/remote.php/dav/files/me`.
    base: String,
    username: String,
    password: String,
}

pub async fn connect(
    config: &ServerConfig,
    secrets: &ServerSecrets,
) -> Result<Arc<dyn RemoteBackend>, String> {
    let base = config.host.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("This server has no URL set.".to_string());
    }
    if !base.starts_with("http://") && !base.starts_with("https://") {
        return Err("The WebDAV URL must start with http:// or https://".to_string());
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Couldn't create an HTTP client: {e}"))?;

    Ok(Arc::new(WebdavBackend {
        client,
        base,
        username: config.username.clone(),
        password: secrets.password.clone(),
    }))
}

impl WebdavBackend {
    /// Absolute URL for a server path, encoding each segment but keeping the
    /// separators — spaces and accents in course folders are the norm.
    fn url_for(&self, path: &str) -> String {
        let encoded = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|seg| utf8_percent_encode(seg, SEGMENT).to_string())
            .collect::<Vec<_>>()
            .join("/");
        if encoded.is_empty() {
            self.base.clone()
        } else {
            format!("{}/{}", self.base, encoded)
        }
    }

    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.username.is_empty() && self.password.is_empty() {
            req
        } else {
            req.basic_auth(&self.username, Some(&self.password))
        }
    }

    /// Map an href from a PROPFIND response back to a server path. Servers
    /// return hrefs as absolute paths including the DAV mount point, so strip
    /// the base URL's own path prefix.
    fn path_from_href(&self, href: &str) -> Option<String> {
        let decoded = percent_decode_str(href).decode_utf8().ok()?.into_owned();
        let path_part = match decoded.find("://") {
            Some(i) => {
                let after = &decoded[i + 3..];
                let slash = after.find('/')?;
                after[slash..].to_string()
            }
            None => decoded,
        };

        let base_path = self
            .base
            .find("://")
            .and_then(|i| {
                self.base[i + 3..]
                    .find('/')
                    .map(|s| &self.base[i + 3 + s..])
            })
            .unwrap_or("");

        let rel = path_part.strip_prefix(base_path).unwrap_or(&path_part);
        let rel = rel.trim_end_matches('/');
        Some(if rel.is_empty() {
            "/".to_string()
        } else if rel.starts_with('/') {
            rel.to_string()
        } else {
            format!("/{rel}")
        })
    }
}

#[async_trait]
impl RemoteBackend for WebdavBackend {
    async fn list_dir(&self, path: &str) -> Result<Vec<RemoteNode>, String> {
        let method = Method::from_bytes(b"PROPFIND").map_err(|e| e.to_string())?;
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop><d:resourcetype/><d:getcontentlength/></d:prop>
</d:propfind>"#;

        let resp = self
            .authed(self.client.request(method, self.url_for(path)))
            .header("Depth", "1")
            .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the server: {e}"))?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err("The server rejected those WebDAV credentials.".to_string());
        }
        if !status.is_success() {
            return Err(format!("The server returned {status} listing {path}."));
        }

        let xml = resp
            .text()
            .await
            .map_err(|e| format!("Couldn't read the server's reply: {e}"))?;

        let self_path = self.path_from_href(&self.url_for(path)).unwrap_or_default();
        let mut nodes = Vec::new();
        for entry in parse_propfind(&xml)? {
            let Some(node_path) = self.path_from_href(&entry.href) else {
                continue;
            };
            // Depth:1 includes the directory itself — skip it.
            if node_path == self_path {
                continue;
            }
            let name = node_path.rsplit('/').next().unwrap_or_default().to_string();
            if name.is_empty() || name.starts_with('.') {
                continue;
            }
            nodes.push(RemoteNode {
                name,
                path: node_path,
                is_dir: entry.is_dir,
                size: entry.size,
            });
        }
        Ok(nodes)
    }

    async fn read_range(&self, path: &str, start: u64, end: u64) -> Result<Vec<u8>, String> {
        if end < start {
            return Ok(Vec::new());
        }
        let resp = self
            .authed(self.client.get(self.url_for(path)))
            .header(header::RANGE, format!("bytes={start}-{end}"))
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the server: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("The server returned {status} reading {path}."));
        }
        // A server that ignores Range answers 200 with the whole file; slice it
        // ourselves rather than handing the player the wrong bytes.
        let full = status == reqwest::StatusCode::OK;
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Couldn't read {path}: {e}"))?;

        if full {
            let from = (start as usize).min(bytes.len());
            let to = ((end + 1) as usize).min(bytes.len());
            return Ok(bytes[from..to].to_vec());
        }
        Ok(bytes.to_vec())
    }

    async fn size_of(&self, path: &str) -> Result<u64, String> {
        let resp = self
            .authed(self.client.head(self.url_for(path)))
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the server: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("The server returned {} for {path}.", resp.status()));
        }
        resp.headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("The server didn't report a size for {path}."))
    }

    async fn read_all(&self, path: &str) -> Result<Vec<u8>, String> {
        let resp = self
            .authed(self.client.get(self.url_for(path)))
            .send()
            .await
            .map_err(|e| format!("Couldn't reach the server: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("The server returned {} for {path}.", resp.status()));
        }
        Ok(resp
            .bytes()
            .await
            .map_err(|e| format!("Couldn't read {path}: {e}"))?
            .to_vec())
    }
}

struct PropfindEntry {
    href: String,
    is_dir: bool,
    size: u64,
}

/// Pull `href`, `resourcetype` and `getcontentlength` out of a multistatus
/// response. Namespace prefixes vary by server (`d:`, `D:`, none), so match on
/// each element's local name and ignore the prefix.
fn parse_propfind(xml: &str) -> Result<Vec<PropfindEntry>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut entries: Vec<PropfindEntry> = Vec::new();
    let mut current: Option<PropfindEntry> = None;
    let mut text_target: Option<&'static str> = None;

    loop {
        match reader.read_event() {
            Err(e) => return Err(format!("Couldn't parse the server's reply: {e}")),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                "response" => {
                    current = Some(PropfindEntry {
                        href: String::new(),
                        is_dir: false,
                        size: 0,
                    });
                }
                "href" => text_target = Some("href"),
                "getcontentlength" => text_target = Some("size"),
                "collection" => {
                    if let Some(c) = current.as_mut() {
                        c.is_dir = true;
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => {
                // `<d:collection/>` is usually self-closing.
                if e.local_name().as_ref() == "collection" {
                    if let Some(c) = current.as_mut() {
                        c.is_dir = true;
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if let (Some(target), Some(c)) = (text_target, current.as_mut()) {
                    let value = quick_xml::escape::unescape(&t.into_inner())
                        .map_err(|e| format!("Couldn't parse the server's reply: {e}"))?
                        .into_owned();
                    match target {
                        "href" => c.href = value,
                        "size" => c.size = value.trim().parse().unwrap_or(0),
                        _ => {}
                    }
                }
                text_target = None;
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == "response" {
                    if let Some(c) = current.take() {
                        if !c.href.is_empty() {
                            entries.push(c);
                        }
                    }
                }
                text_target = None;
            }
            _ => {}
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEXTCLOUD: &str = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/remote.php/dav/files/me/Courses/</d:href>
    <d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat>
  </d:response>
  <d:response>
    <d:href>/remote.php/dav/files/me/Courses/Rust/</d:href>
    <d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat>
  </d:response>
  <d:response>
    <d:href>/remote.php/dav/files/me/Courses/01%20Intro.mp4</d:href>
    <d:propstat><d:prop><d:resourcetype/><d:getcontentlength>1048576</d:getcontentlength></d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;

    fn backend() -> WebdavBackend {
        WebdavBackend {
            client: Client::new(),
            base: "https://cloud.example.com/remote.php/dav/files/me".to_string(),
            username: "me".to_string(),
            password: "pw".to_string(),
        }
    }

    #[test]
    fn parses_collections_and_files() {
        let entries = parse_propfind(NEXTCLOUD).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[1].is_dir);
        assert!(!entries[2].is_dir);
        assert_eq!(entries[2].size, 1_048_576);
    }

    #[test]
    fn href_maps_back_to_server_path() {
        let b = backend();
        let entries = parse_propfind(NEXTCLOUD).unwrap();
        assert_eq!(
            b.path_from_href(&entries[2].href).unwrap(),
            "/Courses/01 Intro.mp4"
        );
        assert_eq!(b.path_from_href(&entries[1].href).unwrap(), "/Courses/Rust");
    }

    #[test]
    fn href_handles_absolute_urls() {
        let b = backend();
        assert_eq!(
            b.path_from_href("https://cloud.example.com/remote.php/dav/files/me/Courses/Rust/")
                .unwrap(),
            "/Courses/Rust"
        );
    }

    #[test]
    fn url_encodes_each_segment() {
        let b = backend();
        assert_eq!(
            b.url_for("/Courses/Rust 101/01 Intro.mp4"),
            "https://cloud.example.com/remote.php/dav/files/me/Courses/Rust%20101/01%20Intro.mp4"
        );
    }

    #[test]
    fn ignores_namespace_prefix_variations() {
        let xml = r#"<multistatus xmlns="DAV:"><response><href>/base/a.mp4</href>
          <propstat><prop><resourcetype/><getcontentlength>42</getcontentlength></prop></propstat>
        </response></multistatus>"#;
        let entries = parse_propfind(xml).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, 42);
    }
}
