#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = nano_installer_native::run_runtime() {
        nano_installer_native::show_runtime_error(&error);
    }
}
