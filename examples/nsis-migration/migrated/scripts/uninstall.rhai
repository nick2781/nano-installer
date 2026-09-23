// The uninstall section of legacy.nsi, as a script. What the installation wrote
// through the primitives is recorded in the machine's manifest, so this file
// says when that goes rather than what goes: run_tracked_uninstall removes the
// shortcuts, the registry values and the files the manifest lists. The library
// runs the same pass after the script when the script does not ask for it.

let install_path = get_install_path();

// legacy.nsi: the uninstall section deleted the running copy's files by hand.
// What the install recorded is enough: the same question goes to the manifest.

// The box the runtime reads before it deletes what uninstall.data_paths
// declares is the layout's `chkReserveData`; the answer reaches an uninstall
// script under `keep_data`, which is the only value an uninstall page hands
// over. Asking for the layout id here would read a control this run never
// fills, and an unread checkbox answers false -- which deletes exactly the data
// the user asked to keep.
let keep_data = get_checkbox_value("keep_data");
log_info("keeping the data the user asked to keep: " + keep_data.to_string());

set_status_key("status.uninstalling");
if !run_tracked_uninstall(5.0, 70.0) {
    show_error("Uninstall", "The installed files could not be removed.");
    return;
}

// legacy.nsi: Delete "$INSTDIR\*.log" and RMDir /r "$INSTDIR\docs" — files the
// product wrote into its own folder after it was installed, which no manifest
// knows about. The user's own data under uninstall.data_paths is the library's
// business, and it follows the same answer this script just read.
set_status_key("status.cleaning");
delete_dir(path_join(install_path, "logs"));
delete_file(path_join(install_path, "LegacyApp.log"));

shell_notify();
set_status_key("status.uninstall_complete");
set_progress(100.0);
