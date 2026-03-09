// TapTap Install Script
// This script controls the entire installation flow.
// If this file doesn't exist, the installer uses config.install_tasks instead.

let app_name = get_config_value("project.name");
let exe_name = get_config_value("install.exe_name");
let install_path = get_install_path();

// Step 1: Kill running process
set_status("Checking for running processes...");
set_progress(2.0);
if is_process_running(exe_name) {
    log_info("Killing running process: " + exe_name);
    kill_process(exe_name);
    sleep_ms(500);
}

if is_cancelled() { return; }

// Step 2: Extract payload
set_status("Extracting files...");
set_progress(5.0);
if !extract_payload() {
    log_error("Failed to extract payload");
    // TaskRunner would throw; script can handle gracefully
}
set_progress(50.0);

if is_cancelled() { return; }

// Step 3: Copy uninstaller
set_status("Installing uninstaller...");
set_progress(55.0);
copy_uninstaller();

// Step 4: Write channel.conf
let channel = get_config_value("channel.default_channel");
if channel != () {
    write_file(path_join(install_path, "channel.conf"), channel);
    log_info("Wrote channel.conf: " + channel);
}
set_progress(60.0);

if is_cancelled() { return; }

// Step 5: Create shortcuts
set_status("Creating shortcuts...");
let exe_path = install_path + "\\" + exe_name;
if get_checkbox_value("desktop_shortcut") {
    create_desktop_shortcut(app_name, exe_path);
}
create_start_menu_shortcut(app_name, exe_path, app_name);
// Uninstall shortcut in Start Menu (NSIS parity)
let uninst_path = install_path + "\\" + get_config_value("output.uninstaller_name");
create_uninstall_shortcut("Uninstall " + app_name, uninst_path, app_name);
set_progress(70.0);

if is_cancelled() { return; }

// Step 6: Write registry
set_status("Writing registry...");
let reg_base = "HKLM\\Software\\" + app_name;
reg_write_string(reg_base, "InstallPath", install_path);

let uninst_key = "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\" + app_name;
reg_write_string(uninst_key, "DisplayName", app_name);
reg_write_string(uninst_key, "DisplayVersion", get_config_value("project.version"));
reg_write_string(uninst_key, "Publisher", get_config_value("project.publisher"));
reg_write_string(uninst_key, "InstallLocation", install_path);
reg_write_string(uninst_key, "UninstallString", uninst_path);
reg_write_string(uninst_key, "DisplayIcon", exe_path);
reg_write_dword(uninst_key, "NoModify", 1);
reg_write_dword(uninst_key, "NoRepair", 1);
set_progress(85.0);

// Step 7: Register URI scheme (NSIS parity)
// register_uri_scheme("taptap", exe_path);

// Step 8: Shell notification
shell_notify();
set_progress(95.0);

set_status("Installation complete");
set_progress(100.0);
