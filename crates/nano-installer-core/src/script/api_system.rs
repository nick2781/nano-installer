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
        message_box(
            title,
            message,
            windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
        );
    });

    engine.register_fn("show_error", |title: &str, message: &str| {
        message_box(
            title,
            message,
            windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    });

    engine.register_fn("ask_yes_no", |title: &str, message: &str| -> bool {
        let result = unsafe {
            windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                None,
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

fn message_box(
    title: &str,
    message: &str,
    icon: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE,
) {
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            None,
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
