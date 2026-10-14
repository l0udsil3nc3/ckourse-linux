//! Commands for saved servers: CRUD, connection testing, browsing and import.

use crate::db::{self, DbState};
use crate::remote::{self, RemoteNode, ServerConfig, ServerKind, ServerSecrets};
use serde::{Deserialize, Serialize};

/// What the Settings form sends. Secrets are optional on edit: an empty
/// `password`/`privateKey` means "keep what's in the keychain", so the UI never
/// has to read a stored secret back out to re-save the rest of the form.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInput {
    /// Absent when adding, present when editing.
    pub id: Option<String>,
    pub name: String,
    pub kind: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub base_path: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub path_style: Option<bool>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseResult {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<RemoteNode>,
}

fn build_config(
    input: &ServerInput,
    id: String,
    existing: Option<&ServerConfig>,
) -> Result<ServerConfig, String> {
    let kind = ServerKind::parse(&input.kind)?;
    if input.name.trim().is_empty() {
        return Err("Give this server a name.".to_string());
    }
    Ok(ServerConfig {
        id,
        name: input.name.trim().to_string(),
        kind,
        host: input.host.trim().to_string(),
        port: input.port.unwrap_or(22),
        username: input.username.trim().to_string(),
        base_path: {
            let p = input.base_path.clone().unwrap_or_default();
            let p = p.trim();
            if p.is_empty() {
                "/".to_string()
            } else {
                p.to_string()
            }
        },
        bucket: input.bucket.clone().unwrap_or_default().trim().to_string(),
        region: input.region.clone().unwrap_or_default().trim().to_string(),
        path_style: input.path_style.unwrap_or(true),
        // Changing the host means the pinned key no longer applies.
        host_fingerprint: existing
            .filter(|e| e.host == input.host.trim() && e.port == input.port.unwrap_or(22))
            .and_then(|e| e.host_fingerprint.clone()),
    })
}

/// Merge submitted secrets over what's already saved, so blank fields on an
/// edit keep the stored value rather than wiping it.
fn merge_secrets(input: &ServerInput, existing: ServerSecrets) -> ServerSecrets {
    let take = |submitted: &Option<String>, current: String| match submitted {
        Some(v) if !v.is_empty() => v.clone(),
        _ => current,
    };
    ServerSecrets {
        password: take(&input.password, existing.password),
        private_key: take(&input.private_key, existing.private_key),
        // A passphrase only means anything alongside a key, and users do clear
        // it deliberately, so an explicit empty string wins here.
        passphrase: input.passphrase.clone().unwrap_or(existing.passphrase),
    }
}

#[tauri::command]
pub fn get_servers(state: tauri::State<'_, DbState>) -> Result<Vec<ServerConfig>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_servers(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_server(
    state: tauri::State<'_, DbState>,
    input: ServerInput,
) -> Result<ServerConfig, String> {
    let is_new = input.id.is_none();
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let existing = if is_new {
        None
    } else {
        remote::get_config(&id).ok()
    };
    let config = build_config(&input, id.clone(), existing.as_ref())?;

    let secrets = merge_secrets(&input, remote::load_secrets(&id).unwrap_or_default());
    if is_new && secrets.is_empty() {
        return Err(match config.kind {
            ServerKind::Sftp => "Add a password or a private key for this server.".to_string(),
            ServerKind::Webdav => "Add the WebDAV password for this server.".to_string(),
            ServerKind::S3 => "Add the secret access key for this bucket.".to_string(),
        });
    }
    remote::store_secrets(&id, &secrets)?;

    {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        db::upsert_server(&conn, &config).map_err(|e| e.to_string())?;
    }

    // Settings changed — drop any live connection so the next use reconnects.
    remote::cache_server(config.clone());
    remote::evict_backend(&id).await;

    Ok(config)
}

#[tauri::command]
pub async fn delete_server(state: tauri::State<'_, DbState>, id: String) -> Result<(), String> {
    {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        db::delete_server(&conn, &id).map_err(|e| e.to_string())?;
    }
    remote::uncache_server(&id);
    remote::evict_backend(&id).await;
    let _ = remote::delete_secrets(&id);
    Ok(())
}

/// Courses that would stop playing if this server were removed.
#[tauri::command]
pub fn count_server_courses(state: tauri::State<'_, DbState>, id: String) -> Result<i64, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::count_courses_for_server(&conn, &id).map_err(|e| e.to_string())
}

/// Connect with the given settings without saving them, so the user can fix a
/// typo before committing. Returns the number of entries at the base path.
#[tauri::command]
pub async fn test_server(input: ServerInput) -> Result<String, String> {
    let id = input.id.clone().unwrap_or_else(|| "test".to_string());
    let existing = input.id.as_ref().and_then(|i| remote::get_config(i).ok());
    let config = build_config(&input, id.clone(), existing.as_ref())?;
    let secrets = merge_secrets(&input, remote::load_secrets(&id).unwrap_or_default());

    let backend = remote::connect(&config, &secrets).await?;
    let entries = backend.list_dir(&config.base_path).await?;
    let folders = entries.iter().filter(|e| e.is_dir).count();
    Ok(format!(
        "Connected. {} at {} — {} folder{}, {} file{}.",
        config.name,
        config.base_path,
        folders,
        if folders == 1 { "" } else { "s" },
        entries.len() - folders,
        if entries.len() - folders == 1 {
            ""
        } else {
            "s"
        },
    ))
}

/// One level of the remote filesystem, for the folder browser.
#[tauri::command]
pub async fn browse_server(
    server_id: String,
    path: Option<String>,
) -> Result<BrowseResult, String> {
    let config = remote::get_config(&server_id)?;
    let path = path.unwrap_or_else(|| config.base_path.clone());
    let entries = remote::browse(&server_id, &path).await?;

    // Don't offer to navigate above the configured base path.
    let parent = if path.trim_end_matches('/') == config.base_path.trim_end_matches('/') {
        None
    } else {
        let trimmed = path.trim_end_matches('/');
        match trimmed.rfind('/') {
            Some(0) | None => Some("/".to_string()),
            Some(i) => Some(trimmed[..i].to_string()),
        }
    };

    Ok(BrowseResult {
        path,
        parent,
        entries,
    })
}

/// Walk a remote folder and build the same ParsedCourse the local parser produces.
#[tauri::command]
pub async fn parse_server_folder(
    server_id: String,
    path: String,
    name: String,
) -> Result<crate::parser::ParsedCourse, String> {
    remote::parse_remote_folder(&server_id, &path, &name).await
}
