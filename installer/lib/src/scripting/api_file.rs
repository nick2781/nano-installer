//! File operation API — extract, copy, delete, write, lock detection

use super::context::ScriptContext;
use rhai::Engine;
use std::path::Path;
use std::time::Instant;

pub fn register(engine: &mut Engine, ctx: ScriptContext) {
    // Extract embedded payload to install path
    let c = ctx.clone();
    engine.register_fn("extract_payload", move || -> bool {
        let install_path = c.get_install_path();
        tracing::info!("[script] extract_payload to {}", install_path);
        match crate::resources::RuntimeResources::get_payload() {
            Some(data) => {
                match crate::resources::PayloadExt::extract_7z_to_dir(
                    &data,
                    Path::new(&install_path),
                ) {
                    Ok(()) => {
                        if let Err(e) = c.record_install_tree_delta() {
                            tracing::warn!("[script] failed to record extracted files: {}", e);
                        }
                        true
                    }
                    Err(e) => {
                        tracing::error!("[script] extract_payload failed: {}", e);
                        false
                    }
                }
            }
            None => {
                tracing::error!("[script] No payload found in resources");
                false
            }
        }
    });

    // Copy uninstaller to install directory
    let c = ctx.clone();
    engine.register_fn("copy_uninstaller", move || -> bool {
        let install_path = c.get_install_path();
        let uninst_name = &c.config.output.uninstaller_name;
        match crate::resources::RuntimeResources::get_uninstaller() {
            Some(data) => {
                let dest = Path::new(&install_path).join(uninst_name);
                match std::fs::write(&dest, &data) {
                    Ok(()) => {
                        c.record_file(dest.clone());
                        tracing::info!("[script] Copied uninstaller to {}", dest.display());
                        true
                    }
                    Err(e) => {
                        tracing::error!("[script] copy_uninstaller failed: {}", e);
                        false
                    }
                }
            }
            None => {
                tracing::warn!("[script] No uninstaller found in resources");
                false
            }
        }
    });

    // Write text file
    let c = ctx.clone();
    engine.register_fn("write_file", move |path: &str, content: &str| -> bool {
        match std::fs::write(path, content) {
            Ok(()) => {
                let path_buf = Path::new(path).to_path_buf();
                c.record_file(path_buf.clone());
                if let Some(parent) = path_buf.parent() {
                    if c.should_track_install_path(parent) {
                        c.record_directory(parent.to_path_buf());
                    }
                }
                true
            }
            Err(e) => {
                tracing::error!("[script] write_file {} failed: {}", path, e);
                false
            }
        }
    });

    // Delete file
    engine.register_fn("delete_file", |path: &str| -> bool {
        match std::fs::remove_file(path) {
            Ok(()) => true,
            Err(_) => false,
        }
    });

    // Delete directory recursively
    engine.register_fn("delete_dir", |path: &str| -> bool {
        if !Path::new(path).exists() {
            return true;
        }
        match std::fs::remove_dir_all(path) {
            Ok(()) => {
                tracing::info!("[script] Deleted directory: {}", path);
                true
            }
            Err(e) => {
                tracing::warn!("[script] delete_dir {} failed: {}", path, e);
                false
            }
        }
    });

    // Check if file exists
    engine.register_fn("file_exists", |path: &str| -> bool {
        Path::new(path).exists()
    });

    // Check if file is locked (DLL-in-use detection)
    engine.register_fn("is_file_locked", |path: &str| -> bool {
        #[cfg(windows)]
        {
            use std::fs::OpenOptions;
            // Try to open file with exclusive write access
            match OpenOptions::new().write(true).open(path) {
                Ok(_) => false, // Not locked
                Err(_) => true, // Locked
            }
        }
        #[cfg(not(windows))]
        {
            false
        }
    });

    // Read text file
    engine.register_fn("read_text_file", |path: &str| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    });

    // List directory contents
    engine.register_fn("list_dir", |path: &str| -> rhai::Array {
        let mut result = rhai::Array::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                result.push(rhai::Dynamic::from(
                    entry.path().to_string_lossy().to_string(),
                ));
            }
        }
        result
    });

    // Extract payload with smooth progress animation
    // extract_payload_with_progress(start_pct, end_pct) — animates progress during extraction
    let c = ctx.clone();
    engine.register_fn(
        "extract_payload_with_progress",
        move |start_pct: f64, end_pct: f64| -> bool {
            let install_path = c.get_install_path();
            tracing::info!(
                "[script] extract_payload_with_progress {}% → {}%",
                start_pct,
                end_pct
            );

            let payload_started = Instant::now();
            let payload = match crate::resources::RuntimeResources::get_payload() {
                Some(data) => data,
                None => {
                    tracing::error!("[script] No payload found");
                    return false;
                }
            };
            let payload_elapsed = payload_started.elapsed();
            let range = end_pct - start_pct;
            c.set_progress(start_pct as f32);
            let extract_started = Instant::now();
            if let Err(e) = crate::resources::PayloadExt::extract_7z_to_dir_with_progress(
                &payload,
                std::path::Path::new(&install_path),
                |progress| {
                    let mapped = start_pct + range * progress as f64;
                    c.set_progress(mapped as f32);
                },
            ) {
                tracing::error!("[script] extract failed: {}", e);
                return false;
            }
            let extract_elapsed = extract_started.elapsed();

            c.set_progress(end_pct as f32);
            let delta_started = Instant::now();
            if let Err(e) = c.record_install_tree_delta() {
                tracing::warn!("[script] failed to record extracted files: {}", e);
            }
            let delta_elapsed = delta_started.elapsed();
            tracing::info!(
                "[script] extract timings: payload={:.3}s extract={:.3}s install_tree_delta={:.3}s total={:.3}s",
                payload_elapsed.as_secs_f64(),
                extract_elapsed.as_secs_f64(),
                delta_elapsed.as_secs_f64(),
                (payload_elapsed + extract_elapsed + delta_elapsed).as_secs_f64()
            );
            true
        },
    );

    // Copy file
    let c = ctx.clone();
    engine.register_fn("copy_file", move |src: &str, dst: &str| -> bool {
        match std::fs::copy(src, dst) {
            Ok(_) => {
                let dst_path = Path::new(dst).to_path_buf();
                c.record_file(dst_path.clone());
                if let Some(parent) = dst_path.parent() {
                    if c.should_track_install_path(parent) {
                        c.record_directory(parent.to_path_buf());
                    }
                }
                true
            }
            Err(e) => {
                tracing::error!("[script] copy_file {} -> {} failed: {}", src, dst, e);
                false
            }
        }
    });

    // Rename / move file or directory
    let c = ctx.clone();
    engine.register_fn("rename", move |src: &str, dst: &str| -> bool {
        match std::fs::rename(src, dst) {
            Ok(()) => {
                let dst_path = Path::new(dst);
                if dst_path.is_dir() {
                    c.record_directory(dst_path.to_path_buf());
                } else {
                    c.record_file(dst_path.to_path_buf());
                }
                true
            }
            Err(e) => {
                tracing::error!("[script] rename {} -> {} failed: {}", src, dst, e);
                false
            }
        }
    });

    // Create directory (recursive)
    let c = ctx.clone();
    engine.register_fn("create_dir", move |path: &str| -> bool {
        match std::fs::create_dir_all(path) {
            Ok(()) => {
                let dir_path = Path::new(path);
                if c.should_track_install_path(dir_path) {
                    c.record_directory(dir_path.to_path_buf());
                }
                true
            }
            Err(e) => {
                tracing::error!("[script] create_dir {} failed: {}", path, e);
                false
            }
        }
    });

    // Is directory
    engine.register_fn("is_dir", |path: &str| -> bool { Path::new(path).is_dir() });

    // Get file size in bytes
    engine.register_fn("get_file_size", |path: &str| -> i64 {
        std::fs::metadata(path)
            .map(|m| m.len() as i64)
            .unwrap_or(-1)
    });

    // Get temp directory path
    engine.register_fn("get_temp_path", || -> String {
        std::env::temp_dir().to_string_lossy().to_string()
    });

    // Join path segments
    engine.register_fn("path_join", |base: &str, child: &str| -> String {
        Path::new(base).join(child).to_string_lossy().to_string()
    });

    // Get parent directory
    engine.register_fn("path_parent", |path: &str| -> String {
        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    // Get file name from path
    engine.register_fn("path_filename", |path: &str| -> String {
        Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    // Sleep
    engine.register_fn("sleep_ms", |ms: i64| {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    });
}
