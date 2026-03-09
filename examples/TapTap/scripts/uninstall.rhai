// TapTap Uninstall Script
// Controls the entire uninstallation flow.
// If this file doesn't exist, the built-in uninstall logic is used.

let app_name = get_config_value("project.name");
let exe_name = get_config_value("install.exe_name");
let install_path = get_install_path();
let keep_data = get_checkbox_value("keep_data");

// Step 1: Kill running process
set_status("Closing application...");
set_progress(5.0);
if is_process_running(exe_name) {
    kill_process(exe_name);
    sleep_ms(500);
}

// Step 2: Remove shortcuts
set_status("Removing shortcuts...");
set_progress(15.0);
delete_desktop_shortcut(app_name);
delete_start_menu_folder(app_name);

// Step 3: Clean registry (HKLM + HKCU)
set_status("Cleaning registry...");
set_progress(30.0);
let uninst_key_lm = "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\" + app_name;
let uninst_key_cu = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\" + app_name;
reg_delete_key(uninst_key_lm);
reg_delete_key(uninst_key_cu);
reg_delete_key("HKLM\\Software\\" + app_name);
reg_delete_key("HKCU\\Software\\" + app_name);

// Remove autostart
reg_delete_value("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", app_name);
reg_delete_value("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", app_name);

// URI scheme cleanup
// delete_uri_scheme("taptap");
set_progress(40.0);

// Step 4: Remove user data (unless keep_data)
if !keep_data {
    set_status("Removing user data...");
    let appdata = get_env("APPDATA");
    let localappdata = get_env("LOCALAPPDATA");
    delete_dir(appdata + "\\TapTapTest");
    delete_dir(localappdata + "\\TapTapTest");

    // Multi-drive game directory cleanup (NSIS parity)
    // let drives = get_drives();
    // for drive in drives {
    //     delete_dir(drive + "TapTapGames");
    // }
}
set_progress(60.0);

// Step 5: Remove installation files (skip uninst.exe itself)
set_status("Removing files...");
set_progress(70.0);

// List and delete all files except uninst.exe
let entries = list_dir(install_path);
for entry in entries {
    // Skip the uninstaller itself (it's running)
    if entry.ends_with("uninst.exe") { continue; }
    if entry.ends_with(".log") { continue; }

    if file_exists(entry) {
        delete_file(entry);
    }
}

// Try to remove subdirectories
let entries2 = list_dir(install_path);
for entry in entries2 {
    if entry.ends_with("uninst.exe") { continue; }
    delete_dir(entry);
}
set_progress(90.0);

// Step 6: Shell notification
shell_notify();

set_status("Uninstall complete");
set_progress(100.0);
