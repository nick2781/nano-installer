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
use crate::ui::WizardMode;

/// 运行安装器
///
/// 注意：调用者需要先初始化日志和运行时资源
pub fn run_installer(mode: InstallerMode) -> Result<()> {
    // 1. 加载配置
    let config = RuntimeResources::get_config()
        .context("Failed to load installer config")?;

    // 自动检测更新模式（仅当 CLI 未明确指定模式时）
    // 注意：自动检测需要用户显式通过 --mode update 或在 config 中配置
    // 不会自动跳过配置页，避免用户困惑

    tracing::info!("Starting {} v{} in {:?} mode",
        config.project.name, config.project.version, mode);

    // 2. 根据模式运行
    match mode {
        InstallerMode::Install => run_install_mode(config, WizardMode::Install),
        InstallerMode::Update => run_install_mode(config, WizardMode::Update),
        InstallerMode::Silent => run_silent_mode(config),
        InstallerMode::Uninstall => run_uninstall_mode(config),
    }
}

/// 安装/更新模式
fn run_install_mode(config: InstallerConfig, wizard_mode: WizardMode) -> Result<()> {
    // 先检查管理员权限 (必须在 mutex 之前, 否则提权后的新进程无法获取 mutex)
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

    // 已提权, 再获取互斥锁
    if !config.install.mutex_name.is_empty() {
        crate::common::mutex::init_global_mutex(&config.install.mutex_name)
            .map_err(|e| anyhow::anyhow!("Failed to init mutex: {}", e))?;
    }

    run_gui(config, wizard_mode)
}

/// 卸载模式
fn run_uninstall_mode(config: InstallerConfig) -> Result<()> {
    // 先检查管理员权限
    #[cfg(windows)]
    if config.install.require_admin {
        if !crate::common::platform::is_elevated()
            .map_err(|e| anyhow::anyhow!("Failed to check elevation: {}", e))? {
            tracing::warn!("Uninstaller requires admin privileges, attempting elevation...");
            let args: Vec<String> = std::env::args().collect();
            crate::common::platform::request_elevation(&args)
                .map_err(|e| anyhow::anyhow!("Failed to request elevation: {}", e))?;
            return Ok(());
        }
    }

    // 已提权, 再获取互斥锁
    if !config.install.mutex_name.is_empty() {
        let mutex_name = format!("{}_Uninstall", config.install.mutex_name);
        crate::common::mutex::init_global_mutex(&mutex_name)
            .map_err(|e| anyhow::anyhow!("Failed to init uninstall mutex: {}", e))?;
    }

    run_gui(config, WizardMode::Uninstall)
}

/// 静默安装模式 (无 GUI)
fn run_silent_mode(config: InstallerConfig) -> Result<()> {
    use crate::installer::state::InstallState;
    use crate::installer::task_runner::TaskRunner;

    tracing::info!("Running silent installation");

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
            let args: Vec<String> = std::env::args().collect();
            crate::common::platform::request_elevation(&args)
                .map_err(|e| anyhow::anyhow!("Failed to request elevation: {}", e))?;
            return Ok(());
        }
    }

    // 确定安装路径
    let install_path = InstallerMode::get_cli_install_path()
        .unwrap_or_else(|| config.install.default_path.clone());

    tracing::info!("Silent install path: {}", install_path);

    // 创建安装目录
    std::fs::create_dir_all(&install_path)
        .map_err(|e| anyhow::anyhow!("Cannot create directory {}: {}", install_path, e))?;

    // 创建安装状态
    let state = InstallState::new(install_path.clone());
    state.set_create_desktop_shortcut(config.shortcuts.desktop_default);
    state.set_create_start_menu_shortcut(config.shortcuts.start_menu);

    // 使用 TaskRunner 执行任务流水线
    let mut runner = TaskRunner::new(&config);
    runner.execute(&state, &config)
        .map_err(|e| anyhow::anyhow!("Silent installation failed: {}", e))?;

    tracing::info!("Silent installation completed successfully");

    // 安装完成后启动应用
    if config.advanced.launch_app_after_install {
        let exe_path = format!("{}\\{}", install_path, config.install.exe_name);
        tracing::info!("Launching app: {}", exe_path);
        let _ = std::process::Command::new(&exe_path).spawn();
    }

    Ok(())
}

/// 运行 GUI（安装/更新/卸载模式通用）
fn run_gui(config: InstallerConfig, wizard_mode: WizardMode) -> Result<()> {
    let icon = load_icon_from_config(&config)?;

    let win_w = config.ui.window_width as f32;
    let win_h = config.ui.window_height as f32;

    let title_prefix = match wizard_mode {
        WizardMode::Install => "Setup",
        WizardMode::Update => "Update",
        WizardMode::Uninstall => "Uninstall",
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("{} {}", config.project.name, title_prefix))
            .with_inner_size([win_w, win_h])
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(icon.unwrap_or_default()),
        centered: true,
        ..Default::default()
    };

    let config_base_path = PathBuf::from(".");

    eframe::run_native(
        &format!("{} {}", config.project.name, title_prefix),
        native_options,
        Box::new(move |cc| {
            let dpi_config = create_dpi_config(&config);
            setup_chinese_font(&cc.egui_ctx);

            let app = InstallerApp::new_with_mode(
                config.clone(),
                config_base_path.clone(),
                dpi_config,
                wizard_mode,
            );
            Ok(Box::new(app))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;

    Ok(())
}

/// 从 InstallerConfig 创建 DpiConfig，窗口尺寸从配置读取
fn create_dpi_config(config: &InstallerConfig) -> crate::ui::dpi_handler::DpiConfig {
    #[cfg(target_os = "windows")]
    let window_dpi = unsafe {
        use windows::Win32::UI::HiDpi::GetDpiForSystem;
        GetDpiForSystem() as u32
    };

    #[cfg(not(target_os = "windows"))]
    let window_dpi = 96u32;

    crate::ui::dpi_handler::DpiConfig {
        scale_factor: window_dpi as f32 / 96.0,
        use_2x: window_dpi >= config.ui.dpi_threshold,
        window_width: config.ui.window_width as f32,
        window_height: config.ui.window_height as f32,
        expanded_height: config.ui.expanded_height as f32,
    }
}

/// 从配置加载图标
fn load_icon_from_config(config: &InstallerConfig) -> Result<Option<egui::IconData>> {
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

    let font_data = load_system_chinese_font();

    if let Some(data) = font_data {
        fonts.font_data.insert(
            "chinese".to_owned(),
            Arc::new(egui::FontData::from_owned(data)),
        );

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
    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "C:\\Windows\\Fonts\\simkai.ttf",
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
    None
}
