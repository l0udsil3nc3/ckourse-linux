//! SFTP backend — the "any box you can ssh into" source.
//!
//! No server-side setup: if `sftp` works from a terminal, it works here. Ranged
//! reads are native (seek + read on an open remote handle), so seeking a video
//! costs one round-trip rather than a fresh request per byte range.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use russh::client::{self, Handle};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg};
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::{join_path, RemoteBackend, RemoteNode, ServerConfig, ServerSecrets};

/// SSH negotiation shouldn't hang the UI if the host is unreachable.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

pub struct SftpBackend {
    session: SftpSession,
    /// Keeping the SSH handle alive keeps the channel — and so the SFTP
    /// subsystem — open for the life of the backend.
    _ssh: Handle<ClientHandler>,
}

/// Verifies the server's host key against a pinned fingerprint, and records
/// whatever it saw so a first connection can pin it.
struct ClientHandler {
    expected: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let fingerprint = match server_public_key {
            russh::keys::PublicKeyOrCertificate::PublicKey { key, .. } => {
                key.fingerprint(HashAlg::Sha256).to_string()
            }
            russh::keys::PublicKeyOrCertificate::Certificate(cert) => {
                cert.public_key().fingerprint(HashAlg::Sha256).to_string()
            }
        };

        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some(fingerprint.clone());
        }

        // Trust on first use: accept anything the first time, then require the
        // same key. A changed key fails the connection rather than warning.
        Ok(match &self.expected {
            None => true,
            Some(pinned) => pinned == &fingerprint,
        })
    }
}

/// Connect, authenticate and open the SFTP subsystem. Returns the backend and
/// the host key fingerprint the caller should pin.
pub async fn connect(
    config: &ServerConfig,
    secrets: &ServerSecrets,
) -> Result<(Arc<dyn RemoteBackend>, String), String> {
    if config.host.trim().is_empty() {
        return Err("This server has no hostname set.".to_string());
    }
    if secrets.is_empty() {
        return Err("This server has no password or private key saved.".to_string());
    }

    let seen = Arc::new(Mutex::new(None));
    let handler = ClientHandler {
        expected: config.host_fingerprint.clone(),
        seen: seen.clone(),
    };

    let ssh_config = Arc::new(client::Config {
        inactivity_timeout: None,
        ..Default::default()
    });

    let port = if config.port == 0 { 22 } else { config.port };
    let connecting = client::connect(ssh_config, (config.host.as_str(), port), handler);
    let mut ssh = match tokio::time::timeout(CONNECT_TIMEOUT, connecting).await {
        Err(_) => return Err(format!("Timed out connecting to {}:{port}.", config.host)),
        Ok(Err(e)) => return Err(host_key_error(&seen, config, e)),
        Ok(Ok(handle)) => handle,
    };

    let fingerprint = seen
        .lock()
        .ok()
        .and_then(|s| s.clone())
        .ok_or_else(|| "The server didn't present a host key.".to_string())?;

    authenticate(&mut ssh, config, secrets).await?;

    let channel = ssh
        .channel_open_session()
        .await
        .map_err(|e| format!("Couldn't open an SSH channel: {e}"))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("Couldn't start SFTP on this server: {e}"))?;

    let session = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("SFTP handshake failed: {e}"))?;

    Ok((
        Arc::new(SftpBackend { session, _ssh: ssh }) as Arc<dyn RemoteBackend>,
        fingerprint,
    ))
}

async fn authenticate(
    ssh: &mut Handle<ClientHandler>,
    config: &ServerConfig,
    secrets: &ServerSecrets,
) -> Result<(), String> {
    let user = config.username.trim();
    if user.is_empty() {
        return Err("This server has no username set.".to_string());
    }

    if !secrets.private_key.trim().is_empty() {
        let passphrase = (!secrets.passphrase.is_empty()).then_some(secrets.passphrase.as_str());
        let key = russh::keys::decode_secret_key(&secrets.private_key, passphrase).map_err(|e| {
            if secrets.passphrase.is_empty() {
                format!("Couldn't read that private key ({e}). If it's passphrase-protected, add the passphrase.")
            } else {
                format!("Couldn't read that private key: {e}")
            }
        })?;

        // RSA keys need a modern signature hash; anything else ignores this.
        let key = PrivateKeyWithHashAlg::new(Arc::new(key), Some(HashAlg::Sha256));
        let result = ssh
            .authenticate_publickey(user, key)
            .await
            .map_err(|e| format!("Key authentication failed: {e}"))?;
        if result.success() {
            return Ok(());
        }
        return Err(format!(
            "The server rejected that key for user \"{user}\". Check the key is in ~/.ssh/authorized_keys on the server."
        ));
    }

    let result = ssh
        .authenticate_password(user, secrets.password.clone())
        .await
        .map_err(|e| format!("Password authentication failed: {e}"))?;
    if result.success() {
        return Ok(());
    }
    Err(format!(
        "The server rejected the password for user \"{user}\". Many servers also disable password login entirely — try a private key."
    ))
}

/// A rejected host key surfaces as a generic negotiation failure, so translate
/// it into the one message that tells the user what actually happened.
fn host_key_error(
    seen: &Arc<Mutex<Option<String>>>,
    config: &ServerConfig,
    err: russh::Error,
) -> String {
    let saw = seen.lock().ok().and_then(|s| s.clone());
    match (&config.host_fingerprint, saw) {
        (Some(pinned), Some(actual)) if pinned != &actual => format!(
            "The host key for {} changed.\n\nPinned: {pinned}\nNow:    {actual}\n\nIf you rebuilt the server this is expected — remove and re-add it in Settings. Otherwise stop and investigate.",
            config.host
        ),
        _ => format!("Couldn't connect to {}: {err}", config.host),
    }
}

impl SftpBackend {
    /// Read up to `len` bytes at `start` through a fresh handle, stopping early
    /// only at EOF.
    async fn read_chunk(&self, path: &str, start: u64, len: u64) -> Result<Vec<u8>, String> {
        let mut file = self
            .session
            .open(path)
            .await
            .map_err(|e| format!("Couldn't open {path}: {e}"))?;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|e| format!("Couldn't seek in {path}: {e}"))?;

        let want = len as usize;
        let mut buf = vec![0u8; want];
        let mut filled = 0usize;
        while filled < want {
            let n = file
                .read(&mut buf[filled..])
                .await
                .map_err(|e| format!("Couldn't read {path}: {e}"))?;
            if n == 0 {
                break; // EOF
            }
            filled += n;
        }
        buf.truncate(filled);
        Ok(buf)
    }
}

#[async_trait]
impl RemoteBackend for SftpBackend {
    async fn list_dir(&self, path: &str) -> Result<Vec<RemoteNode>, String> {
        let dir = if path.is_empty() { "/" } else { path };
        let entries = self
            .session
            .read_dir(dir)
            .await
            .map_err(|e| format!("Couldn't list {dir}: {e}"))?;

        let mut nodes = Vec::new();
        for entry in entries {
            let name = entry.file_name();
            if name.starts_with('.') {
                continue;
            }
            let meta = entry.metadata();
            // Symlinks report their own type, not the target's; treat one with
            // no size as a directory so linked course folders stay browsable.
            let is_dir = entry.file_type().is_dir()
                || (entry.file_type().is_symlink() && meta.size.unwrap_or(0) == 0);
            nodes.push(RemoteNode {
                path: join_path(dir, &name),
                name,
                is_dir,
                size: meta.size.unwrap_or(0),
            });
        }
        Ok(nodes)
    }

    async fn read_range(&self, path: &str, start: u64, end: u64) -> Result<Vec<u8>, String> {
        if end < start {
            return Ok(Vec::new());
        }

        // SFTP caps each reply at the negotiated packet size (often 32 KB), so a
        // single handle reads a 2 MB block as dozens of sequential round-trips —
        // seconds of dead time on a distant server. Split the range into chunks
        // read concurrently over their own handles instead; the requests
        // multiplex over the one SSH connection.
        const CHUNK: u64 = 256 * 1024;
        let mut chunks = Vec::new();
        let mut off = start;
        while off <= end {
            let len = CHUNK.min(end - off + 1);
            chunks.push((off, len));
            off += len;
        }

        let parts = futures::future::try_join_all(
            chunks
                .iter()
                .map(|&(chunk_start, len)| self.read_chunk(path, chunk_start, len)),
        )
        .await?;

        let mut buf = Vec::with_capacity((end - start + 1) as usize);
        for (part, &(_, len)) in parts.iter().zip(&chunks) {
            let short = (part.len() as u64) < len;
            buf.extend_from_slice(part);
            if short {
                break; // EOF — a short final block is expected
            }
        }
        Ok(buf)
    }

    async fn size_of(&self, path: &str) -> Result<u64, String> {
        let meta = self
            .session
            .metadata(path)
            .await
            .map_err(|e| format!("Couldn't stat {path}: {e}"))?;
        meta.size
            .ok_or_else(|| format!("The server didn't report a size for {path}."))
    }

    async fn read_all(&self, path: &str) -> Result<Vec<u8>, String> {
        self.session
            .read(path)
            .await
            .map_err(|e| format!("Couldn't read {path}: {e}"))
    }
}
