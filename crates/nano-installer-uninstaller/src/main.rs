#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = nano_installer_core::run_uninstaller_runtime() {
        nano_installer_core::show_runtime_error(&error);
        std::process::exit(1);
    }
}
