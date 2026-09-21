//! System primitives: environment, drives, dialogs, elevation, and the
//! manifest-driven uninstall a script can call mid-step.

use rhai::{Dynamic, Engine};
use std::path::PathBuf;

use super::context::{json_to_dynamic, log, ScriptContext};
use crate::install;

pub(super) fn register(engine: &mut Engine, context: ScriptContext) {
    engine.register_fn("get_env", |name: &str| -> String {
        std::env::var(name).unwrap_or_default()
    });

    let c = context.clone();
    engine.register_fn("set_env", move |name: &str, value: &str| -> bool {
        set_user_environment(&c, name, value)
    });

    let c = context.clone();
    engine.register_fn("remove_env", move |name: &str| -> bool {
        clear_user_environment(&c, name)
    });

    engine.register_fn("get_drives", || -> rhai::Array {
        let mut drives = rhai::Array::new();
        let mask = unsafe { windows::Win32::Storage::FileSystem::GetLogicalDrives() };
        for index in 0..26u32 {
            if mask & (1 << index) == 0 {
                continue;
            }
            let root = format!("{}:\\", (b'A' + index as u8) as char);
            let wide = crate::install::wide(&root);
            let kind = unsafe {
                windows::Win32::Storage::FileSystem::GetDriveTypeW(windows::core::PCWSTR(
                    wide.as_ptr(),
                ))
            };
            // Fixed disks only: removable media and network shares are not
            // where a product keeps its game data.
            if kind == windows::Win32::System::WindowsProgramming::DRIVE_FIXED {
                drives.push(Dynamic::from(root));
            }
        }
        drives
    });

    engine.register_fn("get_drive_space", |drive: &str| -> rhai::Array {
        let wide = crate::install::wide(drive);
        let mut free = 0u64;
        let mut total = 0u64;
        let status = unsafe {
            windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                windows::core::PCWSTR(wide.as_ptr()),
                Some(&mut free),
                Some(&mut total),
                None,
            )
        };
        match status {
            Ok(()) => vec![Dynamic::from(free as i64), Dynamic::from(total as i64)],
            Err(error) => {
                log("warn", &format!("cannot query {drive}: {error}"));
                Vec::new()
            }
        }
    });

    engine.register_fn("shell_notify", crate::shell::notify_shell);

    let c = context.clone();
    engine.register_fn("get_config_value", move |path: &str| -> Dynamic {
        c.config_value(path)
            .map(json_to_dynamic)
            .unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("get_current_exe", || -> String {
        std::env::current_exe()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    engine.register_fn("get_exe_dir", || -> String {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(PathBuf::from))
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let c = context.clone();
    engine.register_fn("is_elevated", move || -> bool {
        let _ = &c;
        is_elevated()
    });

    engine.register_fn("show_message", |title: &str, message: &str| {
        // The product's own dialog carries its skin and stays with the wizard,
        // which is what a person is looking at; a run with nothing to draw in
        // keeps the system box it used before.
        if crate::show_script_message(title, message) {
            return;
        }
        message_box(
            title,
            message,
            windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
        );
    });

    engine.register_fn("show_error", |title: &str, message: &str| {
        if crate::show_script_message(title, message) {
            return;
        }
        message_box(
            title,
            message,
            windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    });

    engine.register_fn("ask_yes_no", |title: &str, message: &str| -> bool {
        // A silent run cannot wait for an answer, and the safe answer to "may I
        // do this" is no, so the question is recorded and declined.
        if crate::silent_mode() {
            log(
                "warn",
                &format!("{title}: {message} (no answer during a windowless run; treated as No)"),
            );
            return false;
        }
        if let Some(answer) = crate::ask_script_question(title, message) {
            return answer;
        }
        // A project that ships no dialog layout keeps the system question,
        // because the alternative is a question nothing can draw.
        let result = unsafe {
            windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                crate::runtime_window().unwrap_or_default(),
                &windows::core::HSTRING::from(message),
                &windows::core::HSTRING::from(title),
                windows::Win32::UI::WindowsAndMessaging::MB_YESNO
                    | windows::Win32::UI::WindowsAndMessaging::MB_ICONQUESTION,
            )
        };
        result == windows::Win32::UI::WindowsAndMessaging::IDYES
    });

    let c = context.clone();
    engine.register_fn(
        "run_tracked_uninstall",
        move |start: f64, end: f64| -> bool { tracked_uninstall(&c, start, end) },
    );
}

/// The key Windows keeps a user's environment variables in.
const USER_ENVIRONMENT_KEY: &str = r"HKCU\Environment";

/// Writes one of the user's own environment variables.
///
/// Later processes read it from there, which is why the write goes to the
/// registry rather than to this process: a variable a setup exports into its own
/// environment dies with the setup. The key belongs to Windows and every other
/// product on the machine, so the manifest records the value alone.
fn set_user_environment(context: &ScriptContext, name: &str, value: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        log("error", "set_env needs the name of a variable");
        return false;
    }
    let Some((root, path)) = super::api_registry::split(USER_ENVIRONMENT_KEY) else {
        return false;
    };
    if let Err(error) = install::write_registry_string(root, &path, name, value) {
        log("error", &format!("set_env {name} failed: {error:#}"));
        return false;
    }
    context.record_registry_write(USER_ENVIRONMENT_KEY, name);
    // A process started from Explorer inherits the environment block Explorer
    // cached when it started, so Explorer is told the block changed.
    crate::shell::notify_shell();
    true
}

/// Removes one of the user's own environment variables.
///
/// The value is forgotten as well, so an uninstall does not replay a removal of
/// something the script already took away.
fn clear_user_environment(context: &ScriptContext, name: &str) -> bool {
    let name = name.trim();
    let Some((root, path)) = super::api_registry::split(USER_ENVIRONMENT_KEY) else {
        return false;
    };
    if let Err(error) = install::delete_registry_value(root, &path, name) {
        log("error", &format!("remove_env {name} failed: {error:#}"));
        return false;
    }
    let mut state = context.state();
    state.registry_values.retain(|(recorded, recorded_name)| {
        !(recorded == USER_ENVIRONMENT_KEY && recorded_name == name)
    });
    crate::shell::notify_shell();
    true
}

/// Replays the manifest the way an uninstall does, between two progress marks.
///
/// The script calls this instead of the library's own removal, so it stays in
/// charge of ordering; the driver skips its fallback once this has run.
fn tracked_uninstall(context: &ScriptContext, start: f64, end: f64) -> bool {
    if context.mode() != super::Mode::Uninstall {
        log(
            "error",
            "run_tracked_uninstall is only available while uninstalling",
        );
        return false;
    }
    let manifest = context.manifest().clone();
    let destination = context.install_path();
    let config = context.config().clone();
    let span = (end - start).max(0.0);
    context.progress(start);
    install::remove_recorded_artifacts(&manifest);
    context.progress(start + span * 0.34);
    if !context.keep_data() {
        if let Err(error) = install::remove_user_data(&config) {
            log("error", &format!("cannot remove user data: {error:#}"));
            return false;
        }
    }
    context.progress(start + span * 0.67);
    if let Err(error) = install::remove_installed_files(&destination, &manifest) {
        log(
            "error",
            &format!("cannot remove installed files: {error:#}"),
        );
        return false;
    }
    context.state().tracked_uninstall = true;
    context.progress(end);
    true
}

/// Shows a box on behalf of a project script.
///
/// This is the fallback for a run with nothing to draw the product's own dialog
/// in: a silent run, a script that runs before the window exists, or a project
/// that ships no dialog layout. The box is owned by the installer window, so a
/// message a script raises cannot sink behind the wizard that raised it.
fn message_box(
    title: &str,
    message: &str,
    icon: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE,
) {
    // A windowless run has no one to click OK, so the message goes to the log
    // the run already keeps instead of to a box nothing can dismiss.
    if crate::silent_mode() {
        log("info", &format!("{title}: {message}"));
        return;
    }
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            crate::runtime_window().unwrap_or_default(),
            &windows::core::HSTRING::from(message),
            &windows::core::HSTRING::from(title),
            windows::Win32::UI::WindowsAndMessaging::MB_OK | icon,
        );
    }
}

/// Reports whether the current process runs with an elevated token.
fn is_elevated() -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned = 0u32;
    let status = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    };
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(token);
    }
    status.is_ok() && elevation.TokenIsElevated != 0
}
