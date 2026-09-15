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
set_status_key("uninstall.status.closing_app");
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
    set_status_key("uninstall.status.removing_user_data");
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
    set_status_key("uninstall.status.cleaning_game_data");
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
// Step 4: Manifest-driven core uninstall
// Shortcuts + registry + installed files now follow tracked install artifacts
// ═══════════════════════════════════════════
set_status_key("uninstall.status.removing_files");
if !run_tracked_uninstall(55.0, 95.0) {
    show_error("Uninstall Error", "Failed to remove tracked installation artifacts");
    return;
}

// ═══════════════════════════════════════════
// Step 5: Shell notification (NSIS: SHChangeNotify)
// ═══════════════════════════════════════════
shell_notify();

set_status_key("uninstall.status.complete");
set_progress(100.0);
