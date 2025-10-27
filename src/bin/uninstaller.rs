// 卸载器主程序

use nano_installer::common::{cli::UninstallerArgs, config::UninstallerConfig};
use nano_installer::uninstaller::UninstallEngine;
use nano_installer::{i18n, logger};
use std::path::Path;

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let args = match UninstallerArgs::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse arguments: {}", e);
            std::process::exit(1);
        }
    };
    
    // 显示帮助
    if args.help {
        UninstallerArgs::print_help("MyApp", "1.0.0");
        return;
    }
    
    // 初始化日志系统
    let _log_path = match logger::init(None, "MyApp", false) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Failed to initialize logger: {}", e);
            std::process::exit(1);
        }
    };
    
    tracing::info!("Uninstaller started");
    tracing::info!("Arguments: silent={}, keep_data={}", args.silent, args.keep_data);
    
    // 确定安装路径（从注册表或参数读取）
    let install_path = match get_install_path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Failed to determine installation path: {}", e);
            tracing::error!("Failed to determine installation path: {}", e);
            std::process::exit(1);
        }
    };
    
    // 从清单加载语言设置
    let manifest_path = Path::new(&install_path).join("install_manifest.json");
    if let Ok(manifest) = nano_installer::resources::UninstallManifest::load(&manifest_path) {
        let _ = i18n::init(&manifest.locale);
    } else {
        let _ = i18n::init("en-US");
    }
    
    // 静默卸载模式
    if args.silent {
        match run_silent_uninstall(&install_path, args.keep_data).await {
            Ok(_) => {
                println!("Uninstallation completed successfully");
                tracing::info!("Uninstallation completed successfully");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Uninstallation failed: {}", e);
                tracing::error!("Uninstallation failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // GUI 卸载模式
        println!("GUI mode is not yet implemented. Please use silent mode with /S flag.");
        println!("Example: uninstaller.exe /S");
        
        // TODO: 启动 GPUI 界面
        // run_gui_uninstall(&install_path, args.keep_data).await;
    }
}

/// 获取安装路径
fn get_install_path() -> nano_installer::common::Result<String> {
    // 从当前 exe 所在目录推断
    let exe_path = std::env::current_exe()?;
    let install_dir = exe_path
        .parent()
        .ok_or_else(|| nano_installer::common::Error::Path("Cannot determine install path".to_string()))?;
    
    Ok(install_dir.to_string_lossy().to_string())
}

/// 运行静默卸载
async fn run_silent_uninstall(install_path: &str, keep_data: bool) -> nano_installer::common::Result<()> {
    tracing::info!("Starting silent uninstallation");
    
    // 创建卸载引擎
    let engine = UninstallEngine::from_install_path(Path::new(install_path), keep_data)?;
    
    // 执行卸载
    engine.uninstall().await?;
    
    Ok(())
}

