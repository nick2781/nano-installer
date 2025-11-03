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
pub fn run_installer(mode: InstallerMode) -> Result<()> {
    // 1. 初始化日志
    crate::logger::init();
    
    // 2. 初始化运行时资源
    RuntimeResources::init()
        .context("Failed to initialize runtime resources")?;
    
    // 3. 加载配置
    let config = RuntimeResources::get_config()
        .context("Failed to load installer config")?;
    
    tracing::info!("Starting {} v{} in {:?} mode", 
        config.project.name, config.project.version, mode);
    
    // 4. 根据模式运行
    match mode {
        InstallerMode::Install => run_install_mode(config),
        InstallerMode::Uninstall => run_uninstall_mode(config),
    }
}

/// 安装模式
fn run_install_mode(config: InstallerConfig) -> Result<()> {
    // 检查互斥锁
    if let Some(mutex_name) = &config.install.mutex_name {
        crate::common::mutex::init_global_mutex(mutex_name)
            .context("Another installer instance is running")?;
    }
    
    // 检查管理员权限
    #[cfg(windows)]
    if config.install.require_admin {
        if !crate::installer::windows::elevation::is_elevated()? {
            tracing::warn!("Installer requires admin privileges, attempting elevation...");
            crate::installer::windows::elevation::request_elevation()?;
            return Ok(());
        }
    }
    
    // 启动 GUI
    run_gui_install(config)
}

/// 卸载模式
fn run_uninstall_mode(config: InstallerConfig) -> Result<()> {
    // TODO: 实现卸载 GUI
    tracing::info!("Uninstall mode not yet implemented");
    println!("Uninstaller is not yet implemented");
    Ok(())
}

/// 运行 GUI 安装
fn run_gui_install(config: InstallerConfig) -> Result<()> {
    // 加载图标
    let icon = load_icon_from_config(&config)?;
    
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("{} Setup", config.project.name))
            .with_inner_size([800.0, 600.0])
            .with_resizable(false)
            .with_icon(icon.unwrap_or_default()),
        ..Default::default()
    };
    
    // 创建一个假的 config_base_path（因为资源都嵌入了）
    let config_base_path = PathBuf::from(".");
    
    eframe::run_native(
        &format!("{} Setup", config.project.name),
        native_options,
        Box::new(move |cc| {
            // 禁用 egui 的自动 DPI 缩放
            cc.egui_ctx.set_pixels_per_point(1.0);
            
            Ok(Box::new(InstallerApp::new(config.clone(), config_base_path.clone())))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;
    
    Ok(())
}

/// 从配置加载图标
fn load_icon_from_config(config: &InstallerConfig) -> Result<Option<egui::IconData>> {
    let icon_name = config.resources.installer_icon
        .as_ref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("logo.ico");
    
    let icon_data = RuntimeResources::get_asset(icon_name)
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

