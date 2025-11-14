/// 安装器运行时模块
/// 
/// 提供统一的安装器和卸载器入口点

use anyhow::{Context, Result};
use std::path::PathBuf;

pub mod mode;

pub use mode::InstallerMode;

use crate::config::InstallerConfig;
use crate::resources::RuntimeResources;
use crate::ui::InstallerApp;

/// 运行安装器
/// 
/// 注意：调用者需要先初始化日志和运行时资源
pub fn run_installer(mode: InstallerMode) -> Result<()> {
    // 1. 加载配置
    let config = RuntimeResources::get_config()
        .context("Failed to load installer config")?;
    
    tracing::info!("Starting {} v{} in {:?} mode", 
        config.project.name, config.project.version, mode);
    
    // 2. 根据模式运行
    match mode {
        InstallerMode::Install => run_install_mode(config),
        InstallerMode::Uninstall => run_uninstall_mode(config),
    }
}

/// 安装模式
fn run_install_mode(config: InstallerConfig) -> Result<()> {
    // 检查互斥锁
    if !config.install.mutex_name.is_empty() {
        crate::common::mutex::init_global_mutex(&config.install.mutex_name)
            .map_err(|e| anyhow::anyhow!("Failed to init mutex: {}", e))?;
    }
    
    // 检查管理员权限
    #[cfg(windows)]
    if config.install.require_admin {
        if !crate::common::platform::is_elevated()
            .map_err(|e| anyhow::anyhow!("Failed to check elevation: {}", e))? {
            tracing::warn!("Installer requires admin privileges, attempting elevation...");
            let args: Vec<String> = std::env::args().collect();
            crate::common::platform::request_elevation(&args)
                .map_err(|e| anyhow::anyhow!("Failed to request elevation: {}", e))?;
            return Ok(());
        }
    }
    
    // 启动 GUI
    run_gui_install(config)
}

/// 卸载模式
fn run_uninstall_mode(_config: InstallerConfig) -> Result<()> {
    // TODO: 实现卸载 GUI
    tracing::info!("Uninstall mode not yet implemented");
    println!("Uninstaller is not yet implemented");
    Ok(())
}

/// 运行 GUI 安装
fn run_gui_install(config: InstallerConfig) -> Result<()> {
    // 加载图标
    let icon = load_icon_from_config(&config)?;
    
    // 注意：在窗口创建前，我们无法准确检测到真实的系统 DPI（因为 DPI 感知模式）
    // 所以先使用一个合理的默认值，然后在窗口创建回调中根据 egui 检测到的真实 DPI 调整
    // 默认使用 2x 大小（1148x716），因为大多数高 DPI 显示器都是 192 DPI
    // 如果实际是 96 DPI，会在回调中调整为 1x（574x358）
    eprintln!("[窗口] 开始创建窗口 - 调用位置: run_gui_install");
    eprintln!("[窗口] 初始窗口大小: 1148x716 (2x，将在窗口创建后根据实际 DPI 调整)");
    
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("{} Setup", config.project.name))
            .with_inner_size([1148.0, 716.0])  // 默认使用 2x 大小
            .with_resizable(false)
            .with_decorations(false)  // 无边框窗口
            .with_transparent(true)  // 启用透明背景以支持圆角
            .with_icon(icon.unwrap_or_default()),
        centered: true,  // 自动居中显示
        ..Default::default()
    };
    
    // 创建一个假的 config_base_path（因为资源都嵌入了）
    let config_base_path = PathBuf::from(".");
    
    eframe::run_native(
        &format!("{} Setup", config.project.name),
        native_options,
        Box::new(move |cc| {
            // 注意：窗口圆角通过透明背景和背景图片的圆角边缘来实现视觉效果
            // Windows 11+ 的系统级圆角需要在窗口完全创建后通过 DwmSetWindowAttribute 设置
            // 这里先使用透明背景，圆角效果由背景图片提供
            
            // 在窗口创建后，此时线程应该已经是 AWARE 模式了
            // 使用 GetDpiForSystem() 再次检测真实的系统 DPI
            #[cfg(target_os = "windows")]
            let window_dpi = {
                use windows::Win32::UI::HiDpi::GetDpiForSystem;
                
                unsafe {
                    let dpi = GetDpiForSystem() as u32;
                    eprintln!("[窗口创建] 窗口创建后 GetDpiForSystem() = {} (此时应该是 AWARE 模式)", dpi);
                    dpi
                }
            };
            
            #[cfg(not(target_os = "windows"))]
            let window_dpi = 96;
            
            // 根据窗口 DPI 判断使用 1x 还是 2x（与 NSIS 一致：>= 144 使用 2x）
            let actual_use_2x = window_dpi >= config.ui.dpi_threshold;
            let (actual_window_width, actual_window_height) = if actual_use_2x {
                (1148.0, 716.0)
            } else {
                (574.0, 358.0)
            };
            
            let scale_factor = window_dpi as f32 / 96.0;
            
            eprintln!("[窗口创建] 根据窗口 DPI 计算: DPI={}, use_2x={}, 窗口大小: {}x{}", 
                window_dpi, actual_use_2x, actual_window_width, actual_window_height);
            
            // 如果窗口大小需要调整，立即调整
            let current_size = cc.egui_ctx.viewport_rect().size();
            if (current_size.x - actual_window_width).abs() > 1.0 ||
               (current_size.y - actual_window_height).abs() > 1.0 {
                eprintln!("[窗口创建] 调整窗口大小: {}x{} -> {}x{}", 
                    current_size.x, current_size.y, actual_window_width, actual_window_height);
                cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    actual_window_width,
                    actual_window_height
                )));
                // 注意：窗口居中由 NativeOptions 的 centered: true 处理
            }
            
            // 创建新的 DpiConfig，使用窗口的实际 DPI
            use crate::ui::dpi_handler::DpiConfig;
            let actual_dpi_config = DpiConfig {
                scale_factor,
                use_2x: actual_use_2x,
                window_width: actual_window_width,
                window_height: actual_window_height,
                expanded_height: if actual_use_2x { 1036.0 } else { 518.0 },
            };
            
            eprintln!("[窗口创建] 使用实际 DpiConfig: use_2x={}, 窗口: {}x{}", 
                actual_dpi_config.use_2x, actual_dpi_config.window_width, actual_dpi_config.window_height);
            
            // 强制设置 pixels_per_point = 1.0，我们手动处理缩放
            cc.egui_ctx.set_pixels_per_point(1.0);
            eprintln!("[窗口创建] 强制设置 pixels_per_point = 1.0");

            // 设置中文字体
            setup_chinese_font(&cc.egui_ctx);

            // 使用实际检测到的 dpi_config
            Ok(Box::new(InstallerApp::new_with_dpi(config.clone(), config_base_path.clone(), actual_dpi_config)))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;
    
    Ok(())
}

/// 从配置加载图标
fn load_icon_from_config(config: &InstallerConfig) -> Result<Option<egui::IconData>> {
    // 使用配置中的完整路径（如 assets/logo.ico）
    let icon_path = &config.resources.installer_icon;
    
    tracing::debug!("Loading installer icon: {}", icon_path);
    
    let icon_data = RuntimeResources::get_asset(icon_path)
        .context("Failed to load installer icon")?;
    
    load_icon_from_ico(&icon_data)
}

/// 从 ICO 字节加载图标
fn load_icon_from_ico(ico_bytes: &[u8]) -> Result<Option<egui::IconData>> {
    use image::GenericImageView;

    let img = image::load_from_memory(ico_bytes)
        .context("Failed to decode icon image")?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    Ok(Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }))
}

/// 设置中文字体支持
fn setup_chinese_font(ctx: &egui::Context) {
    use egui::FontDefinitions;
    use egui::FontFamily;
    use std::sync::Arc;

    let mut fonts = FontDefinitions::default();

    // 尝试从 Windows 系统加载中文字体
    let font_data = load_system_chinese_font();

    if let Some(data) = font_data {
        // 添加中文字体
        fonts.font_data.insert(
            "chinese".to_owned(),
            Arc::new(egui::FontData::from_owned(data)),
        );

        // 将中文字体设置为最高优先级
        fonts.families.entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "chinese".to_owned());

        fonts.families.entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "chinese".to_owned());

        tracing::info!("Chinese font loaded successfully");
    } else {
        tracing::warn!("Failed to load Chinese font, text may not display correctly");
    }

    ctx.set_fonts(fonts);
}

/// 从 Windows 系统加载中文字体
#[cfg(windows)]
fn load_system_chinese_font() -> Option<Vec<u8>> {
    // 按优先级尝试不同的中文字体
    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",      // 微软雅黑
        "C:\\Windows\\Fonts\\msyhbd.ttc",    // 微软雅黑 Bold
        "C:\\Windows\\Fonts\\simhei.ttf",    // 黑体
        "C:\\Windows\\Fonts\\simsun.ttc",    // 宋体
        "C:\\Windows\\Fonts\\simkai.ttf",    // 楷体
    ];

    for path in &font_paths {
        tracing::debug!("Trying to load font: {}", path);
        if let Ok(data) = std::fs::read(path) {
            tracing::info!("Successfully loaded font: {}", path);
            return Some(data);
        }
    }

    tracing::error!("Could not load any Chinese font from system");
    None
}

#[cfg(not(windows))]
fn load_system_chinese_font() -> Option<Vec<u8>> {
    // 非 Windows 平台暂不支持
    None
}

