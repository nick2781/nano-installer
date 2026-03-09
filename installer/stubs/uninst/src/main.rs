// 卸载器 stub — 与安装器共用同一引擎
//
// 和 lzma stub 一样的架构：从 exe 自身读取嵌入的资源 bundle，
// 通过 exe 文件名 "uninst" 自动检测为 Uninstall 模式，
// 启动 egui UI 显示自定义卸载界面。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

fn main() -> Result<()> {
    // 初始化日志
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let log_dir = exe_dir.as_deref();

    let _ = nano_installer::logger::init(log_dir, "uninstaller", false);

    tracing::info!("Uninstaller starting...");

    // 初始化运行时资源 (从 exe 自身读取嵌入的 bundle)
    if let Err(e) = nano_installer::resources::RuntimeResources::init(None) {
        tracing::error!("Failed to init resources: {:#}", e);
        eprintln!("ERROR: Failed to init resources: {:#}", e);
        std::process::exit(1);
    }

    // 使用统一的模式检测 (exe 名含 "uninst" → Uninstall 模式)
    let mode = nano_installer::installer_runtime::InstallerMode::detect();
    tracing::info!("Running in {:?} mode", mode);

    // 运行引擎（Uninstall 模式走 egui 卸载 UI）
    nano_installer::installer_runtime::run_installer(mode)?;

    Ok(())
}
