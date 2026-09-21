//! File associations: telling Windows that this product opens a file type.
//!
//! A user's own associations live under `HKCU\Software\Classes`, so a setup
//! that runs without elevation can claim a file type for the person who ran it.
//! The manifest records what was written, which is what lets the uninstall take
//! it back.

use rhai::Engine;

use super::context::{log, ScriptContext};
use crate::install::{delete_registry_key, registry_path, write_registry_string};

/// Where Windows keeps the file associations a user has of their own.
const CLASSES: &str = r"HKCU\Software\Classes";

/// The name of the value Windows reads a key's own text from.
const DEFAULT_VALUE: &str = "";

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn(
        "register_file_association",
        move |extension: &str,
              prog_id: &str,
              description: &str,
              command: &str,
              icon: &str|
              -> bool { associate(&c, extension, prog_id, description, command, icon) },
    );

    let c = context.clone();
    engine.register_fn(
        "unregister_file_association",
        move |extension: &str, prog_id: &str| -> bool { dissociate(&c, extension, prog_id) },
    );
}

/// Splits a key into a hive handle and a subkey path.
fn split(key: &str) -> Option<(windows::Win32::System::Registry::HKEY, String)> {
    match registry_path(key) {
        Ok(parts) => Some(parts),
        Err(error) => {
            log("error", &format!("invalid registry key {key}: {error:#}"));
            None
        }
    }
}

/// Claims a file type for the current user.
///
/// Windows reads an association from two keys: the extension names the program
/// id, and the program id carries the words Explorer shows, the icon, and the
/// command that opens the file.
fn associate(
    context: &ScriptContext,
    extension: &str,
    prog_id: &str,
    description: &str,
    command: &str,
    icon: &str,
) -> bool {
    let Some((extension_key, prog_id_key)) = association_keys(extension, prog_id) else {
        return false;
    };
    if command.trim().is_empty() {
        log(
            "error",
            "register_file_association needs the command that opens the file",
        );
        return false;
    }
    // The command goes in first and the extension last: until the extension
    // names the program id nothing reaches it, and a file opened in between
    // should find a working command rather than a file type that names nothing.
    let mut written = write_default(
        context,
        &format!("{prog_id_key}\\shell\\open\\command"),
        command,
    );
    if !description.trim().is_empty() {
        written &= write_default(context, &prog_id_key, description);
    }
    if !icon.trim().is_empty() {
        written &= write_default(context, &format!("{prog_id_key}\\DefaultIcon"), icon);
    }
    written &= write_default(context, &extension_key, prog_id);
    if written {
        crate::shell::notify_shell();
    }
    written
}

/// Gives the file type back.
///
/// Both keys go: the extension that named the program id, and the program id
/// that carried the words, the icon and the command. The pair is one file type,
/// and half of it left behind is a file type that opens nothing.
fn dissociate(context: &ScriptContext, extension: &str, prog_id: &str) -> bool {
    let Some((extension_key, prog_id_key)) = association_keys(extension, prog_id) else {
        return false;
    };
    // Both are attempted, so a failure on the first does not leave the second
    // behind for no reason.
    let removed = delete_key(context, &extension_key) & delete_key(context, &prog_id_key);
    crate::shell::notify_shell();
    removed
}

/// The two keys an association lives in.
///
/// A name carrying a separator or a hive would write outside the classes tree a
/// user's associations belong in, so it is refused rather than cleaned up after
/// the fact.
fn association_keys(extension: &str, prog_id: &str) -> Option<(String, String)> {
    let extension = extension.trim().trim_start_matches('.');
    let prog_id = prog_id.trim();
    if !is_plain_name(extension) || !is_plain_name(prog_id) {
        log(
            "error",
            "register_file_association needs a file name extension and a program id, \
             neither of them carrying a path separator",
        );
        return None;
    }
    Some((
        format!("{CLASSES}\\.{extension}"),
        format!("{CLASSES}\\{prog_id}"),
    ))
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['\\', '/', ':'])
}

/// Writes the one value a key carries about itself.
fn write_default(context: &ScriptContext, key: &str, value: &str) -> bool {
    let Some((root, path)) = split(key) else {
        return false;
    };
    match write_registry_string(root, &path, DEFAULT_VALUE, value) {
        Ok(()) => {
            context.record_registry_write(key, DEFAULT_VALUE);
            true
        }
        Err(error) => {
            log("error", &format!("cannot write {key}: {error:#}"));
            false
        }
    }
}

/// Removes a key this product owns and drops it from what the uninstall replays.
fn delete_key(context: &ScriptContext, key: &str) -> bool {
    let Some((root, path)) = split(key) else {
        return false;
    };
    if let Err(error) = delete_registry_key(root, &path) {
        log("error", &format!("cannot remove {key}: {error:#}"));
        return false;
    }
    context.forget_registry_key(key);
    true
}
