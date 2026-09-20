//! File and payload primitives.
//!
//! Deployment reuses `install::extract_payload`, so a script deploys through
//! exactly the same stubbed extraction, conflict checks, and staging the
//! built-in flow uses. Every write is journaled first, so a script that fails
//! half way through still rolls back cleanly.

use rhai::Engine;
use std::path::{Path, PathBuf};

use super::context::{checked_delete_target, log, ScriptContext};
use crate::install;

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn("extract_payload", move || -> bool {
        extract(&c, 0.0, 100.0)
    });

    let c = context.clone();
    engine.register_fn(
        "extract_payload_with_progress",
        move |start: f64, end: f64| -> bool { extract(&c, start, end) },
    );

    let c = context.clone();
    engine.register_fn("copy_uninstaller", move || -> bool {
        let name = c.config()["output"]["uninstaller_name"]
            .as_str()
            .unwrap_or("uninst.exe")
            .to_string();
        let target = c.install_path().join(&name);
        let contents = match c.bundle().read_file(&format!("runtime/{name}")) {
            Ok(contents) => contents,
            Err(error) => {
                log("error", &format!("cannot read the uninstaller: {error:#}"));
                return false;
            }
        };
        match write(&c, &target, &contents) {
            Ok(()) => true,
            Err(error) => {
                log("error", &format!("copy_uninstaller failed: {error:#}"));
                false
            }
        }
    });

    let c = context.clone();
    engine.register_fn("write_file", move |path: &str, contents: &str| -> bool {
        match write(&c, Path::new(path), contents.as_bytes()) {
            Ok(()) => true,
            Err(error) => {
                log("error", &format!("write_file {path} failed: {error:#}"));
                false
            }
        }
    });

    let c = context.clone();
    engine.register_fn("create_dir", move |path: &str| -> bool {
        let target = PathBuf::from(path);
        if let Err(error) = c.record_write(&target) {
            log("error", &format!("cannot journal {path}: {error:#}"));
            return false;
        }
        match std::fs::create_dir_all(&target) {
            Ok(()) => true,
            Err(error) => {
                log("error", &format!("create_dir {path} failed: {error}"));
                false
            }
        }
    });

    engine.register_fn("delete_dir", |path: &str| -> bool {
        let target = match checked_delete_target(path) {
            Ok(target) => target,
            Err(error) => {
                log("error", &format!("delete_dir refused: {error:#}"));
                return false;
            }
        };
        // A script cleaning up its own leftovers must never fail the
        // uninstall, so a missing directory is already the wanted outcome.
        match std::fs::remove_dir_all(&target) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(error) => {
                log("warn", &format!("delete_dir {path} failed: {error}"));
                false
            }
        }
    });

    let c = context.clone();
    engine.register_fn("delete_file", move |path: &str| -> bool {
        let target = PathBuf::from(path);
        if let Err(error) = c.record_write(&target) {
            log("error", &format!("cannot journal {path}: {error:#}"));
            return false;
        }
        match std::fs::remove_file(&target) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(error) => {
                log("warn", &format!("delete_file {path} failed: {error}"));
                false
            }
        }
    });

    let c = context.clone();
    engine.register_fn("copy_file", move |source: &str, target: &str| -> bool {
        let target_path = PathBuf::from(target);
        if let Err(error) = c.record_write(&target_path) {
            log("error", &format!("cannot journal {target}: {error:#}"));
            return false;
        }
        match std::fs::copy(source, &target_path) {
            Ok(_) => true,
            Err(error) => {
                log(
                    "error",
                    &format!("copy_file {source} -> {target} failed: {error}"),
                );
                false
            }
        }
    });

    engine.register_fn("file_exists", |path: &str| -> bool {
        Path::new(path).exists()
    });

    engine.register_fn("is_dir", |path: &str| -> bool { Path::new(path).is_dir() });

    engine.register_fn("get_file_size", |path: &str| -> i64 {
        std::fs::metadata(path)
            .map(|metadata| metadata.len() as i64)
            .unwrap_or(-1)
    });

    engine.register_fn("read_text_file", |path: &str| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    });

    engine.register_fn("list_dir", |path: &str| -> rhai::Array {
        let mut entries = rhai::Array::new();
        let Ok(reader) = std::fs::read_dir(path) else {
            return entries;
        };
        for entry in reader.flatten() {
            entries.push(rhai::Dynamic::from(
                entry.file_name().to_string_lossy().to_string(),
            ));
        }
        entries
    });

    engine.register_fn("path_join", |base: &str, child: &str| -> String {
        Path::new(base).join(child).to_string_lossy().to_string()
    });

    engine.register_fn("path_parent", |path: &str| -> String {
        Path::new(path)
            .parent()
            .map(|parent| parent.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    engine.register_fn("path_filename", |path: &str| -> String {
        Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    engine.register_fn("get_temp_path", || -> String {
        std::env::temp_dir().to_string_lossy().to_string()
    });

    engine.register_fn("sleep_ms", |milliseconds: i64| {
        std::thread::sleep(std::time::Duration::from_millis(milliseconds.max(0) as u64));
    });
}

/// Expands the payload into the installation directory.
fn extract(context: &ScriptContext, start: f64, end: f64) -> bool {
    let extracted = context.stage().join("files");
    let _ = std::fs::remove_dir_all(&extracted);
    let result = install::extract_payload(
        context.setup(),
        context.bundle(),
        context.config(),
        context.stage(),
        &extracted,
        context.cancellation(),
    );
    let files = match result {
        Ok(files) => files,
        Err(error) => {
            log("error", &format!("payload extraction failed: {error:#}"));
            return false;
        }
    };
    if let Err(error) = deploy(context, &extracted, &files) {
        log("error", &format!("payload deployment failed: {error:#}"));
        return false;
    }
    context.progress(end.max(start));
    true
}

/// Copies the expanded payload into the destination, journaling each target.
fn deploy(context: &ScriptContext, extracted: &Path, files: &[PathBuf]) -> anyhow::Result<()> {
    let destination = context.install_path();
    for relative in files {
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        context.record_write(&target)?;
        std::fs::copy(extracted.join(relative), &target)?;
    }
    Ok(())
}

/// Writes `contents` to `target`, recording the target first.
fn write(context: &ScriptContext, target: &Path, contents: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    context.record_write(target)?;
    std::fs::write(target, contents)?;
    Ok(())
}
