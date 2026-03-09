// Runtime Stub - 纯粹的安装/卸载运行时
// 
// 这是一个轻量级的 stub 程序，类似 NSIS 的 stub
// 不包含任何 CLI 命令，只包含安装/卸载逻辑
//
// 工作流程：
// 1. 从 exe 中提取嵌入的资源
// 2. 根据配置判断是安装还是卸载模式
// 3. 启动对应的 UI 和逻辑

// Release 模式：隐藏控制台窗口（纯 GUI 应用）
// Debug 模式：保留控制台窗口（方便调试和查看日志）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

fn main() -> Result<()> {
    // Debug 模式下输出到控制台
    #[cfg(debug_assertions)]
    eprintln!("=== nano-installer DEBUG MODE ===");
    #[cfg(debug_assertions)]
    eprintln!("Console output enabled for debugging");
    
    // 初始化日志：优先写到当前 exe 所在目录，便于调试时查找
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let log_dir = exe_dir.as_deref();

    let _ = nano_installer::logger::init(log_dir, "installer", true);
    
    #[cfg(debug_assertions)]
    eprintln!("Logger initialized");
    
    tracing::info!("Initializing runtime resources...");
    
    // 初始化运行时资源（从 exe 中提取）
    if let Err(e) = nano_installer::resources::RuntimeResources::init(None) {
        tracing::error!("Failed to initialize runtime resources: {:#}", e);
        eprintln!("ERROR: Failed to initialize runtime resources");
        eprintln!("Cause: {:#}", e);
        std::process::exit(1);
    }
    
    tracing::info!("Loading configuration...");
    
    // 加载配置
    let config = match nano_installer::resources::RuntimeResources::get_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("Failed to load installer config: {:#}", e);
            eprintln!("ERROR: Failed to load installer config");
            eprintln!("Cause: {:#}", e);
            std::process::exit(1);
        }
    };
    
    // 使用统一的模式检测
    let mode = nano_installer::installer_runtime::mode::InstallerMode::detect();

    tracing::info!("Starting in {:?} mode", mode);

    // 运行安装器（内部处理 install/update/silent/uninstall）
    nano_installer::installer_runtime::run_installer(mode)?;

    Ok(())
}

