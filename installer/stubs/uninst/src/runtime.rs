use anyhow::Result;

pub fn run() -> Result<()> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()));
    let _ = nano_installer::logger::init(exe_dir.as_deref(), "uninstaller", false);

    nano_installer::resources::RuntimeResources::init(None)?;
    let mode = nano_installer::installer_runtime::InstallerMode::detect();
    tracing::info!("Starting uninstaller runtime in {:?} mode", mode);
    nano_installer::installer_runtime::run_installer(mode)
}
