//! Acquisition primitives: fetching what the setup does not carry.
//!
//! A product whose payload is too large to ship inside itself, or which needs
//! something from a server it maintains, asks for it while it installs. What
//! arrives is written where the script says and, when the script names a
//! SHA-256, checked against it: a file that does not match is taken away again
//! rather than left for the next step to run.

use rhai::Engine;
use std::path::{Path, PathBuf};

use super::context::{log, ScriptContext};
use crate::net;

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn("download_file", move |url: &str, path: &str| -> bool {
        download(&c, url, path, None)
    });

    let c = context.clone();
    engine.register_fn(
        "download_file_with_hash",
        move |url: &str, path: &str, sha256: &str| -> bool {
            download(&c, url, path, Some(sha256))
        },
    );

    engine.register_fn("sha256_of_file", |path: &str| -> String {
        match net::sha256_file(Path::new(path)) {
            Ok(digest) => digest,
            Err(error) => {
                log("error", &format!("sha256_of_file {path} failed: {error:#}"));
                String::new()
            }
        }
    });
}

/// Fetches `url` into `path`, recovering nothing about where the file came from.
///
/// The target is journaled before the first byte is written, so a download into
/// the installation directory is undone with everything else when the run
/// fails.
fn download(context: &ScriptContext, url: &str, path: &str, sha256: Option<&str>) -> bool {
    let target = PathBuf::from(path);
    if target.as_os_str().is_empty() {
        log("error", "download_file needs the path to write");
        return false;
    }
    if let Err(error) = context.record_write(&target) {
        log("error", &format!("cannot journal {path}: {error:#}"));
        return false;
    }
    match net::download(url, &target, sha256, &mut |_, _| {}, context.cancellation()) {
        Ok(()) => true,
        Err(error) => {
            log("error", &format!("download_file {url} failed: {error:#}"));
            false
        }
    }
}
