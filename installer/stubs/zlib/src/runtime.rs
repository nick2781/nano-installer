use anyhow::Result;

pub fn run() -> Result<()> {
    #[cfg(debug_assertions)]
    eprintln!("=== nano-installer ZLIB DEBUG MODE ===");

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()));
    let _ = nano_installer::logger::init(exe_dir.as_deref(), "installer", true);

    nano_installer::resources::RuntimeResources::init(None)?;
    let mode = nano_installer::installer_runtime::InstallerMode::detect();
    tracing::info!("Starting ZIP/Deflate runtime in {:?} mode", mode);
    nano_installer::installer_runtime::run_installer(mode)
}
