//! Shortcut primitives.
//!
//! Links go through the same `IShellLinkW` writer the built-in flow uses, and
//! each one is journaled before it is written so a failed install restores any
//! link it replaced.

use rhai::Engine;
use std::path::{Path, PathBuf};

use super::context::{log, ScriptContext};
use crate::shell;

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    let c = context.clone();
    engine.register_fn(
        "create_desktop_shortcut",
        move |name: &str, target: &str| -> bool {
            let directory = match shell::desktop_directory() {
                Ok(directory) => directory,
                Err(error) => {
                    log("error", &format!("cannot resolve the desktop: {error:#}"));
                    return false;
                }
            };
            create(&c, &directory, name, target, false)
        },
    );

    let c = context.clone();
    engine.register_fn(
        "create_start_menu_shortcut",
        move |name: &str, target: &str, folder: &str| -> bool {
            start_menu(&c, name, target, folder)
        },
    );

    let c = context.clone();
    engine.register_fn(
        "create_uninstall_shortcut",
        move |name: &str, target: &str, folder: &str| -> bool {
            start_menu(&c, name, target, folder)
        },
    );

    engine.register_fn("delete_desktop_shortcut", |name: &str| -> bool {
        let Ok(directory) = shell::desktop_directory() else {
            return false;
        };
        remove(&directory.join(format!("{name}.lnk")))
    });

    engine.register_fn("delete_start_menu_folder", |folder: &str| -> bool {
        let Ok(programs) = shell::programs_directory() else {
            return false;
        };
        let folder = programs.join(folder);
        match std::fs::remove_dir_all(&folder) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(error) => {
                log(
                    "warn",
                    &format!("cannot remove {}: {error}", folder.display()),
                );
                false
            }
        }
    });
}

/// Writes a link into the Start Menu folder `folder` names.
fn start_menu(context: &ScriptContext, name: &str, target: &str, folder: &str) -> bool {
    let directory = match shell::programs_directory() {
        Ok(directory) => directory.join(folder),
        Err(error) => {
            log(
                "error",
                &format!("cannot resolve the Start Menu: {error:#}"),
            );
            return false;
        }
    };
    let created = create(context, &directory, name, target, true);
    if created {
        context.record_shortcut_dir(&directory);
    }
    created
}

fn create(
    context: &ScriptContext,
    directory: &Path,
    name: &str,
    target: &str,
    track_directory: bool,
) -> bool {
    let link = directory.join(format!("{name}.lnk"));
    let target = PathBuf::from(target);
    let working_dir = target.parent().unwrap_or(directory);
    if let Err(error) = context.record_write(&link) {
        log(
            "error",
            &format!("cannot journal {}: {error:#}", link.display()),
        );
        return false;
    }
    if track_directory {
        if let Err(error) = std::fs::create_dir_all(directory) {
            log(
                "error",
                &format!("cannot create {}: {error}", directory.display()),
            );
            return false;
        }
    }
    match shell::create_shortcut(&link, &target, working_dir) {
        Ok(()) => {
            context.record_shortcut(&link);
            true
        }
        Err(error) => {
            log(
                "error",
                &format!("cannot create {}: {error:#}", link.display()),
            );
            false
        }
    }
}

fn remove(link: &Path) -> bool {
    match std::fs::remove_file(link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => {
            log(
                "warn",
                &format!("cannot remove {}: {error}", link.display()),
            );
            false
        }
    }
}
