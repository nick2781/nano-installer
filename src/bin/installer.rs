// 安装器主程序

use nano_installer::common::{cli::InstallerArgs, config::InstallerConfig, platform};
use nano_installer::installer::{InstallEngine, InstallState};
use nano_installer::{i18n, logger};
use std::path::Path;

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let args = match InstallerArgs::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse arguments: {}", e);
            std::process::exit(1);
        }
    };
    
    let config = InstallerConfig::default();
    
    // 显示帮助或版本信息
    if args.help {
        InstallerArgs::print_help(&config.app_name, &config.app_version);
        return;
    }
    
    if args.version {
        println!("{} v{}", config.app_name, config.app_version);
        return;
    }
    
    // 初始化日志系统
    let log_path = match logger::init(None, &config.app_name, true) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Failed to initialize logger: {}", e);
            std::process::exit(1);
        }
    };
    
    tracing::info!("Installer started");
    tracing::info!("Arguments: silent={}, install_dir={:?}, locale={:?}", 
        args.silent, args.install_dir, args.locale);
    
    // 检测或使用指定的语言
    let locale = args.locale.as_deref().unwrap_or_else(|| {
        let detected = platform::detect_system_locale();
        if i18n::is_locale_supported(&detected) {
            detected.as_str()
        } else {
            "en-US"
        }
    });
    
    // 初始化多语言系统
    if let Err(e) = i18n::init(locale) {
        tracing::error!("Failed to initialize i18n: {}", e);
        eprintln!("Failed to initialize language system: {}", e);
        std::process::exit(1);
    }
    
    tracing::info!("Language set to: {}", locale);
    
    // 确定安装路径
    let install_path = args.install_dir.unwrap_or_else(|| config.default_install_path.clone());
    
    // 检查是否需要管理员权限
    if config.require_admin || needs_admin(&install_path) {
        match platform::is_elevated() {
            Ok(true) => {
                tracing::info!("Running with elevated privileges");
            }
            Ok(false) => {
                tracing::warn!("Elevated privileges required");
                
                if args.silent {
                    eprintln!("Error: Administrator privileges required for installation to {}", install_path);
                    std::process::exit(1);
                } else {
                    // GUI 模式：请求提升权限
                    println!("Requesting administrator privileges...");
                    if let Err(e) = platform::request_elevation(&std::env::args().collect::<Vec<_>>()) {
                        eprintln!("Failed to request elevation: {}", e);
                        std::process::exit(1);
                    }
                    return;
                }
            }
            Err(e) => {
                tracing::error!("Failed to check elevation status: {}", e);
            }
        }
    }
    
    // 静默安装模式
    if args.silent {
        match run_silent_install(&install_path).await {
            Ok(_) => {
                println!("Installation completed successfully");
                tracing::info!("Installation completed successfully");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Installation failed: {}", e);
                tracing::error!("Installation failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // GUI 安装模式
        println!("GUI mode requires GPUI implementation.");
        println!("Current status: UI framework ready, rendering code pending.");
        println!("\nTo complete:");
        println!("1. Implement Render trait in src/ui/app.rs");
        println!("2. Reference: src/ui/gpui_impl.rs (example code)");
        println!("3. Follow: docs/IMPLEMENTATION_STEPS.md");
        println!("\nFor now, use silent mode:");
        println!("  installer.exe /S /D={}", install_path);
        
        // TODO: 实现 GPUI 渲染后取消注释
        // match run_gui_install(config).await {
        //     Ok(_) => std::process::exit(0),
        //     Err(e) => {
        //         eprintln!("GUI installation failed: {}", e);
        //         std::process::exit(1);
        //     }
        // }
    }
}

/// 检查是否需要管理员权限
fn needs_admin(install_path: &str) -> bool {
    #[cfg(windows)]
    {
        nano_installer::installer::windows::elevation::needs_elevation(install_path)
    }
    
    #[cfg(not(windows))]
    {
        false
    }
}

/// 运行静默安装
async fn run_silent_install(install_path: &str) -> nano_installer::common::Result<()> {
    use nano_installer::installer::tasks::*;
    use nano_installer::resources::PayloadExtractor;
    
    tracing::info!("Starting silent installation");
    
    // 提取嵌入的 payload
    let payload_data = PayloadExtractor::extract_embedded_payload()?;
    tracing::info!("Payload extracted: {} bytes", payload_data.len());
    
    // 创建安装引擎
    let mut engine = InstallEngine::new(install_path.to_string());
    
    // 添加任务
    engine.add_task(Box::new(ExtractFilesTask { payload_data }));
    
    // TODO: 添加更多任务
    // engine.add_task(Box::new(CreateShortcutsTask { ... }));
    // engine.add_task(Box::new(WriteRegistryTask { ... }));
    
    // 执行安装
    let _manifest = engine.install().await?;
    
    Ok(())
}

