#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = nano_installer_core::run_uninstaller_runtime() {
        // A windowless run reports to whoever started it; the notice is a no-op
        // when there is no window either way.
        if nano_installer_core::silent_mode() {
            eprintln!("error: {error:#}");
        } else {
            nano_installer_core::show_runtime_error(&error);
        }
        std::process::exit(1);
    }
}
