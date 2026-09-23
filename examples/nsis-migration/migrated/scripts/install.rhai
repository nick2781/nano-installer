// The install section of legacy.nsi, as a script. A project that ships
// scripts/install.rhai owns the steps: nothing below happens unless this file
// asks for it, which is why the payload, the uninstaller, the shortcuts and the
// settings are all named here in the order the NSIS sections ran them.

let app_name = get_config_value("project.name");
let exe_name = get_config_value("install.exe_name");
let install_path = get_install_path();
let exe_path = path_join(install_path, exe_name);
let uninst_path = path_join(install_path, get_config_value("output.uninstaller_name"));

// legacy.nsi: Function .onInit closed a running copy before touching the
// directory. install.detect_running_process in the configuration does the same
// thing before this script starts; this is the explicit form.
set_status_key("status.checking_processes");
set_progress(2.0);
if is_process_running(exe_name) {
    kill_process(exe_name);
    sleep_ms(500);
}

if is_cancelled() {
    return;
}

// legacy.nsi: the required "Core files" section unpacked app\*.* with File /r,
// and "Documentation" / "Sample projects" were optional sections. Here the
// project declares core as required and the other two as components, so one
// call unpacks the payload and whichever components this run selected.
set_status_key("status.extracting");
if !extract_payload_with_progress(5.0, 45.0) {
    show_error(app_name, "The application files could not be unpacked.");
    return;
}
copy_uninstaller();
create_dir(path_join(install_path, "logs"));

// legacy.nsi: ExecWait '"$INSTDIR\vc_redist.x64.exe" /install /quiet
// /norestart'. The machine-wide runtime lives in dependencies.items now — the
// project says how to recognise it and which installer to run, the script says
// when. The exit code is checked the same way: a required dependency that
// cannot be installed stops the run.
set_status_key("status.dependencies");
install_dependency("vcredist_x64");

if is_cancelled() {
    return;
}

// legacy.nsi: nsExec::ExecToLog '"$INSTDIR\tools\migrate.exe" --quiet', and
// Abort when it answered with a non-zero code. run_command_output hands back
// the exit code and both streams, so the same decision is written the same way.
set_status_key("status.migrating");
set_progress(55.0);
let migration = run_command_output(path_join(install_path, "tools\\migrate.exe"), ["--quiet"]);
if migration.code != 0 {
    log_error("the bundled migration tool failed with code " + migration.code);
    show_error(app_name, "The settings of the previous version could not be moved.");
    return;
}

// legacy.nsi: CreateShortCut three times, with the desktop one behind the
// checkbox the components page carried.
set_status_key("status.shortcuts");
set_progress(70.0);
if get_checkbox_value("chkShotcut") {
    create_desktop_shortcut(app_name, exe_path);
}
let menu_folder = get_config_value("shortcuts.start_menu_folder");
create_start_menu_shortcut(app_name, exe_path, menu_folder);
create_uninstall_shortcut("Uninstall " + app_name, uninst_path, menu_folder);

// legacy.nsi's uninstall read and deleted HKCU ...\Run\LegacyApp. The entry it
// was cleaning up is written here, behind the page's checkbox, and the runtime
// records the value this installation put in that shared key: the uninstall
// takes this one value back and leaves every other product's entry alone. The
// project configures the same entry through `autostart`, which a project
// without an install script gets for free -- this one owns its steps.
if get_checkbox_value("chkAutoRun") {
    reg_write_string(
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        app_name,
        exe_path
    );
}

// legacy.nsi: WriteRegExpandStr HKLM "Software\LegacyApp" "DataDir"
// "$APPDATA\LegacyApp". The reference is stored as written and expanded by
// whoever reads it back, exactly like REG_EXPAND_SZ.
set_status_key("status.registry");
reg_write_expand_string("HKLM\\Software\\" + app_name, "DataDir", "%APPDATA%\\" + app_name);

// What the registry's uninstall group used to take a dozen lines of NSIS to
// write — DisplayName, DisplayVersion, Publisher, UninstallString,
// QuietUninstallString, EstimatedSize, NoModify, NoRepair — is written by the
// runtime from registry.uninstall_key and the project's own metadata. A script
// only adds the fields of its own on top.

shell_notify();
set_status_key("status.install_complete");
set_progress(100.0);
