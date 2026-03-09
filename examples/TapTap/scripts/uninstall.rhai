// TapTap Uninstall Script — 1:1 NSIS parity
// Translates ui_TapTap_setup.nsh::un.onUninstall + un.DeleteDataInBackground

let app_name = get_config_value("project.name");
let exe_name = get_config_value("install.exe_name");
let install_path = get_install_path();
let keep_data = get_checkbox_value("keep_data");

// ═══════════════════════════════════════════
// Step 1: Kill running process
// (NSIS: nsProcess::_FindProcess + _KillProcess)
// ═══════════════════════════════════════════
set_status("Closing application...");
set_progress(5.0);
if is_process_running(exe_name) {
    kill_process(exe_name);
    sleep_ms(500);
}

// ═══════════════════════════════════════════
// Step 2: Delete user data (if not reserving)
// (NSIS: un.DeleteUserData → RMDir /r $APPDATA\TapTap)
// ═══════════════════════════════════════════
if !keep_data {
    set_status("Removing user data...");
    set_progress(10.0);

    let appdata = get_env("APPDATA");
    let localappdata = get_env("LOCALAPPDATA");
    delete_dir(path_join(appdata, app_name));
    delete_dir(path_join(localappdata, app_name));

    // ═══════════════════════════════════════════
    // Step 3: Multi-drive game directory cleanup
    // (NSIS: un.DeleteSupportDirs + un.DeleteTapTapDirs)
    // Iterate all HDD drives, delete $DRIVE\TapTap\Support and $DRIVE\TapTap
    // ═══════════════════════════════════════════
    set_status("Cleaning game data...");
    set_progress(20.0);
    let drives = get_drives();
    for drive in drives {
        // Support directories
        let support_dir = path_join(drive + app_name, "Support");
        if file_exists(support_dir) {
            delete_dir(support_dir);
            log_info("Cleaned support dir: " + support_dir);
        }
        // Game root
        let game_dir = drive + app_name;
        if file_exists(game_dir) {
            delete_dir(game_dir);
            log_info("Cleaned game dir: " + game_dir);
        }
    }
    set_progress(50.0);
} else {
    // Reserve data: skip game deletion, animate progress
    set_progress(50.0);
}

// ═══════════════════════════════════════════
// Step 4: Delete desktop shortcuts
// (NSIS: un.DeleteDesktopShortcuts — scans for .url files with taptap://)
// ═══════════════════════════════════════════
set_status("Removing shortcuts...");
set_progress(55.0);
delete_desktop_shortcut(app_name);

// ═══════════════════════════════════════════
// Step 5: Delete start menu
// (NSIS: un.DeleteStartMenuShortcuts + un.DeleteShortcutAndInstallInfo)
// ═══════════════════════════════════════════
delete_start_menu_folder(app_name);
set_progress(60.0);

// ═══════════════════════════════════════════
// Step 6: Clean registry
// (NSIS: un.DeleteShortcutAndInstallInfo)
// ═══════════════════════════════════════════
set_status("Cleaning registry...");
set_progress(65.0);

// Uninstall registry key (always delete)
reg_delete_key("HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\" + app_name);
reg_delete_key("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\" + app_name);

// App registry key
if !keep_data {
    // Delete entire key
    reg_delete_key("HKLM\\Software\\" + app_name);
    reg_delete_key("HKCU\\Software\\" + app_name);
} else {
    // Only remove InstallPath, keep other data
    reg_delete_value("HKLM\\Software\\" + app_name, "InstallPath");
    reg_delete_value("HKCU\\Software\\" + app_name, "InstallPath");
}

// AutoStart registry (NSIS: DeleteRegValue HKCU Run)
reg_delete_value("HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run", app_name);
reg_delete_value("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run", app_name);

// URI scheme (NSIS: DeleteRegKey HKCR taptap)
// delete_uri_scheme("taptap");

set_progress(75.0);

// ═══════════════════════════════════════════
// Step 7: Delete installation files
// (NSIS: un.RemoveFiles → ${Locate} $INSTDIR)
// Skip uninst.exe (it's running) and log files
// ═══════════════════════════════════════════
set_status("Removing files...");
set_progress(80.0);

let entries = list_dir(install_path);
for entry in entries {
    let name = path_filename(entry);
    // Skip the running uninstaller
    if name == "uninst.exe" { continue; }
    // Delete files and subdirectories
    if is_dir(entry) {
        delete_dir(entry);
    } else {
        delete_file(entry);
    }
}
set_progress(92.0);

// ═══════════════════════════════════════════
// Step 8: Shell notification (NSIS: SHChangeNotify)
// ═══════════════════════════════════════════
shell_notify();

set_status("Uninstall complete");
set_progress(100.0);
