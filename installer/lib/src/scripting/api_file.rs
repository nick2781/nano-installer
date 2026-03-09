//! File operation API — extract, copy, delete, write, lock detection

use rhai::Engine;
use super::context::ScriptContext;
use std::path::Path;

pub fn register(engine: &mut Engine, ctx: ScriptContext) {
    // Extract embedded payload to install path
    let c = ctx.clone();
    engine.register_fn("extract_payload", move || -> bool {
        let install_path = c.get_install_path();
        tracing::info!("[script] extract_payload to {}", install_path);
        match crate::resources::RuntimeResources::get_payload() {
            Some(data) => {
                match crate::resources::PayloadExt::extract_7z_to_dir(&data, Path::new(&install_path)) {
                    Ok(()) => true,
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
    engine.register_fn("write_file", |path: &str, content: &str| -> bool {
        match std::fs::write(path, content) {
            Ok(()) => true,
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
        { false }
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
                result.push(rhai::Dynamic::from(entry.path().to_string_lossy().to_string()));
            }
        }
        result
    });

    // Extract payload with smooth progress animation
    // extract_payload_with_progress(start_pct, end_pct) — animates progress during extraction
    let c = ctx.clone();
    engine.register_fn("extract_payload_with_progress", move |start_pct: f64, end_pct: f64| -> bool {
        let install_path = c.get_install_path();
        tracing::info!("[script] extract_payload_with_progress {}% → {}%", start_pct, end_pct);

        let payload = match crate::resources::RuntimeResources::get_payload() {
            Some(data) => data,
            None => {
                tracing::error!("[script] No payload found");
                return false;
            }
        };

        // Run extraction in background thread
        let install_path_clone = install_path.clone();
        let payload_clone = payload;
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done2 = done.clone();
        let failed2 = failed.clone();

        std::thread::spawn(move || {
            match crate::resources::PayloadExt::extract_7z_to_dir(
                &payload_clone, std::path::Path::new(&install_path_clone)
            ) {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!("[script] extract failed: {}", e);
                    failed2.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            done2.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        // Animate progress while waiting
        let range = end_pct - start_pct;
        let mut elapsed_ms: u64 = 0;
        let poll_interval = 100u64; // ms

        while !done.load(std::sync::atomic::Ordering::SeqCst) {
            elapsed_ms += poll_interval;
            // Non-linear progress: fast start, slow finish (asymptotic to 95% of range)
            let t = (elapsed_ms as f64 / 1000.0).min(120.0); // cap at 120s
            let ratio = 1.0 - (-t / 15.0f64).exp(); // ~95% after 45s
            let pct = start_pct + range * ratio * 0.95; // never reach end_pct until done
            c.set_progress(pct as f32);
            std::thread::sleep(std::time::Duration::from_millis(poll_interval));
        }

        if failed.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }

        c.set_progress(end_pct as f32);
        true
    });

    // Copy file
    engine.register_fn("copy_file", |src: &str, dst: &str| -> bool {
        match std::fs::copy(src, dst) {
            Ok(_) => true,
            Err(e) => {
                tracing::error!("[script] copy_file {} -> {} failed: {}", src, dst, e);
                false
            }
        }
    });

    // Rename / move file or directory
    engine.register_fn("rename", |src: &str, dst: &str| -> bool {
        match std::fs::rename(src, dst) {
            Ok(()) => true,
            Err(e) => {
                tracing::error!("[script] rename {} -> {} failed: {}", src, dst, e);
                false
            }
        }
    });

    // Create directory (recursive)
    engine.register_fn("create_dir", |path: &str| -> bool {
        match std::fs::create_dir_all(path) {
            Ok(()) => true,
            Err(e) => {
                tracing::error!("[script] create_dir {} failed: {}", path, e);
                false
            }
        }
    });

    // Is directory
    engine.register_fn("is_dir", |path: &str| -> bool {
        Path::new(path).is_dir()
    });

    // Get file size in bytes
    engine.register_fn("get_file_size", |path: &str| -> i64 {
        std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(-1)
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
        Path::new(path).parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    // Get file name from path
    engine.register_fn("path_filename", |path: &str| -> String {
        Path::new(path).file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    // Sleep
    engine.register_fn("sleep_ms", |ms: i64| {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    });
}
