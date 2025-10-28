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
            "en-US" // 使用静态字符串而不是引用局部变量
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
        if let Err(e) = run_gui_install(config).await {
            eprintln!("GUI安装失败: {}", e);
            std::process::exit(1);
        }
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

/// 运行简单安装（文本界面）
async fn run_simple_install(install_path: &str) -> nano_installer::common::Result<()> {
    use nano_installer::installer::tasks::*;
    use nano_installer::resources::PayloadExtractor;
    
    println!("Starting installation to: {}", install_path);
    
    // 创建安装目录
    std::fs::create_dir_all(install_path)?;
    println!("✓ Created installation directory");
    
    // 模拟安装过程
    println!("Installing files...");
    for i in 1..=10 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        println!("  Progress: {}%", i * 10);
    }
    
    println!("✓ Installation completed!");
    println!("Application installed to: {}", install_path);
    
    Ok(())
}

/// 运行GUI安装（使用egui）
async fn run_gui_install(config: InstallerConfig) -> nano_installer::common::Result<()> {
    use nano_installer::ui::InstallerApp;
    
    // 配置窗口选项 - 使用NSIS的尺寸 574x358
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([574.0, 358.0])
            .with_min_inner_size([574.0, 358.0])
            .with_max_inner_size([574.0, 518.0])  // 允许展开到518px（NSIS展开高度）
            .with_resizable(false)
            .with_decorations(false)  // 无边框
            .with_transparent(false)   // 不透明，使用背景图
            .with_title(format!("{} 安装程序", config.app_name.clone())),
        ..Default::default()
    };
    
    // 启动egui应用
    eframe::run_native(
        &format!("{} 安装程序", config.app_name),
        options,
        Box::new(|cc| {
            // 加载中文字体
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(InstallerApp::new(config)))
        }),
    ).map_err(|e| nano_installer::common::Error::Unknown(format!("GUI启动失败: {}", e)))?;
    
    Ok(())
}

/// 设置自定义字体（支持中文）
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    
    // 添加中文字体（使用 Windows 系统字体）
    #[cfg(target_os = "windows")]
    {
        // 尝试加载 Windows 系统中文字体
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert(
                "msyh".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            
            // 将中文字体设置为最高优先级
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "msyh".to_owned());
                
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "msyh".to_owned());
        }
        
        // 尝试加载 Segoe UI 字体（支持越南语等更多语言）
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
            fonts.font_data.insert(
                "segoe_ui".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            
            // 将 Segoe UI 字体设置为第二优先级
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(1, "segoe_ui".to_owned());
        }
    }
    
    ctx.set_fonts(fonts);
}

