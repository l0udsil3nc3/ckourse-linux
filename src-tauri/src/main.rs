// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn configure_stream_playback() {
    use std::path::PathBuf;

    let mut schemes = std::env::var("WEBKIT_GST_ALLOWED_URI_PROTOCOLS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if !schemes.iter().any(|s| s == "stream") {
        schemes.push("stream".to_string());
    }

    let app_name = env!("CARGO_PKG_NAME");
    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(debug_assertions)]
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/gstreamer"));

    if let Some(appdir) = std::env::var_os("APPDIR") {
        let base = PathBuf::from(appdir).join("usr/lib").join(app_name);
        candidates.push(base.join("gstreamer"));
        candidates.push(base.join("resources/gstreamer"));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(prefix) = exe.parent().and_then(|p| p.parent()) {
            let base = prefix.join("lib").join(app_name);
            candidates.push(base.join("gstreamer"));
            candidates.push(base.join("resources/gstreamer"));
        }
    }

    let plugin_dir = candidates
        .into_iter()
        .find(|p| p.join("libgsttauriasset.so").is_file());

    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(path) = plugin_dir {
        paths.push(path);
    } else {
        eprintln!("[ckourse] Bundled GStreamer plugin not found in expected paths.");
    }

    if let Some(existing) = std::env::var_os("GST_PLUGIN_PATH") {
        for path in std::env::split_paths(&existing) {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }

    // AppImage's GStreamer hook sets GST_PLUGIN_PATH_1_0 separately.
    // Preserve its standard plugin directory and add our custom stream plugin.
    let mut versioned_paths = std::env::var_os("GST_PLUGIN_PATH_1_0")
        .map(|existing| std::env::split_paths(&existing).collect::<Vec<_>>())
        .unwrap_or_default();

    for path in &paths {
        if !versioned_paths.contains(path) {
            versioned_paths.push(path.clone());
        }
    }

    unsafe {
        std::env::set_var("WEBKIT_GST_ALLOWED_URI_PROTOCOLS", schemes.join(","));

        if !paths.is_empty() {
            if let Ok(joined) = std::env::join_paths(&paths) {
                std::env::set_var("GST_PLUGIN_PATH", joined);
            }
        }

        if !versioned_paths.is_empty() {
            if let Ok(joined) = std::env::join_paths(&versioned_paths) {
                std::env::set_var("GST_PLUGIN_PATH_1_0", joined);
            }
        }
    }

    eprintln!(
        "[ckourse] WEBKIT_GST_ALLOWED_URI_PROTOCOLS={}",
        schemes.join(",")
    );
    eprintln!("[ckourse] GStreamer plugin paths: {:?}", paths);
}

fn main() {
    #[cfg(target_os = "linux")]
    configure_stream_playback();
    ckourse_lib::run()
}
