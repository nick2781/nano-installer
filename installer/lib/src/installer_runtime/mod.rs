/// 安装器运行时模块
///
/// 提供统一的安装器和卸载器入口点
use anyhow::{Context, Result};
use std::sync::Arc;

use parking_lot::RwLock;

pub mod mode;

pub use mode::InstallerMode;

use crate::common::close_targets::prepare_install_close_targets;
use crate::config::InstallerConfig;
use crate::resources::RuntimeResources;
use crate::ui::InstallerApp;
use crate::ui::WizardMode;

/// 运行安装器
///
/// 注意：调用者需要先初始化日志和运行时资源
pub fn run_installer(mode: InstallerMode) -> Result<()> {
    // 1. 加载配置
    let config = RuntimeResources::get_config().context("Failed to load installer config")?;
    let effective_mode = match mode {
        InstallerMode::Install if InstallerMode::detect_update(&config) => InstallerMode::Update,
        other => other,
    };

    tracing::info!(
        "Starting {} v{} in {:?} mode",
        config.project.name,
        config.project.version,
        effective_mode
    );

    // 2. 根据模式运行
    match effective_mode {
        InstallerMode::Install => run_install_mode(config, WizardMode::Install),
        InstallerMode::Update => {
            if !config.advanced.update_mode_support {
                Err(anyhow::anyhow!(
                    "Update mode is disabled by installer configuration"
                ))
            } else {
                run_install_mode(config, WizardMode::Update)
            }
        }
        InstallerMode::Silent => {
            if !config.advanced.silent_mode_support {
                Err(anyhow::anyhow!(
                    "Silent mode is disabled by installer configuration"
                ))
            } else {
                run_silent_mode(config)
            }
        }
        InstallerMode::Uninstall => {
            if !config.advanced.uninstall_mode_support {
                Err(anyhow::anyhow!(
                    "Uninstall mode is disabled by installer configuration"
                ))
            } else {
                run_uninstall_mode(config)
            }
        }
        InstallerMode::SilentUninstall => {
            if !config.advanced.uninstall_mode_support {
                Err(anyhow::anyhow!(
                    "Uninstall mode is disabled by installer configuration"
                ))
            } else if !config.advanced.silent_mode_support {
                Err(anyhow::anyhow!(
                    "Silent mode is disabled by installer configuration"
                ))
            } else {
                run_silent_uninstall(config)
            }
        }
    }
}

/// 安装/更新模式
fn run_install_mode(config: InstallerConfig, wizard_mode: WizardMode) -> Result<()> {
    if wizard_mode == WizardMode::Update && resolve_existing_install_path(&config).is_none() {
        return Err(anyhow::anyhow!(
            "Update mode requires an existing installation. InstallPath could not be resolved."
        ));
    }

    // 先检查管理员权限 (必须在 mutex 之前, 否则提权后的新进程无法获取 mutex)
    #[cfg(windows)]
    if config.install.require_admin {
        if !crate::common::platform::is_elevated()
            .map_err(|e| anyhow::anyhow!("Failed to check elevation: {}", e))?
        {
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
            .map_err(|e| anyhow::anyhow!("Failed to check elevation: {}", e))?
        {
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
            .map_err(|e| anyhow::anyhow!("Failed to check elevation: {}", e))?
        {
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

    let close_report = prepare_install_close_targets(&config.install.effective_close_targets())
        .map_err(|e| {
            anyhow::anyhow!("Failed to prepare running targets for installation: {}", e)
        })?;
    if let Some(message) = close_report.blocking_message() {
        return Err(anyhow::anyhow!(message));
    }
    if !close_report.closed_targets.is_empty() {
        tracing::info!(
            "Closed install targets before silent install: {}",
            close_report.closed_targets.join(", ")
        );
    }

    // 创建安装目录
    std::fs::create_dir_all(&install_path)
        .map_err(|e| anyhow::anyhow!("Cannot create directory {}: {}", install_path, e))?;

    // 创建安装状态
    let state = InstallState::new(install_path.clone());
    state.set_create_desktop_shortcut(config.shortcuts.desktop_default);
    state.set_create_start_menu_shortcut(config.shortcuts.start_menu);
    state.set_autostart_enabled(config.autostart.enabled && config.autostart.default);

    // Script mode or config mode
    if let Ok(script_source) =
        crate::resources::RuntimeResources::get_script("scripts/install.rhai")
    {
        tracing::info!("Running install script (silent mode)");
        let script_ctx =
            crate::scripting::ScriptContext::for_install(state.clone(), config.clone());
        script_ctx.checkbox_values.write().insert(
            "desktop_shortcut".to_string(),
            state.create_desktop_shortcut(),
        );
        script_ctx
            .checkbox_values
            .write()
            .insert("autorun".to_string(), state.autostart_enabled());
        let mut engine = crate::scripting::ScriptEngine::new(script_ctx);
        if let Err(e) = engine.run_script(&script_source) {
            let _ = engine.context().rollback_install();
            return Err(anyhow::anyhow!("Script error: {}", e));
        }
        engine
            .context()
            .finalize_install()
            .map_err(|e| anyhow::anyhow!("Failed to finalize script installation: {}", e))?;
    } else {
        let mut runner = TaskRunner::new(&config);
        runner
            .execute(&state, &config)
            .map_err(|e| anyhow::anyhow!("Silent installation failed: {}", e))?;
    }

    tracing::info!("Silent installation completed successfully");

    // 安装完成后启动应用
    if config.advanced.launch_app_after_install {
        let exe_path = format!("{}\\{}", install_path, config.install.exe_name);
        tracing::info!("Launching app: {}", exe_path);
        let _ = std::process::Command::new(&exe_path).spawn();
    }

    Ok(())
}

/// 静默卸载模式 (无 GUI)
fn run_silent_uninstall(config: InstallerConfig) -> Result<()> {
    tracing::info!("Running silent uninstallation");

    #[cfg(windows)]
    if config.install.require_admin {
        if !crate::common::platform::is_elevated()
            .map_err(|e| anyhow::anyhow!("Failed to check elevation: {}", e))?
        {
            let args: Vec<String> = std::env::args().collect();
            crate::common::platform::request_elevation(&args)
                .map_err(|e| anyhow::anyhow!("Failed to request elevation: {}", e))?;
            return Ok(());
        }
    }

    if !config.install.mutex_name.is_empty() {
        let mutex_name = format!("{}_Uninstall", config.install.mutex_name);
        crate::common::mutex::init_global_mutex(&mutex_name)
            .map_err(|e| anyhow::anyhow!("Failed to init uninstall mutex: {}", e))?;
    }

    let install_path = resolve_silent_uninstall_path(&config)?;
    tracing::info!("Silent uninstall path: {}", install_path);

    let progress = Arc::new(RwLock::new(0.0f32));
    let status = Arc::new(RwLock::new(String::new()));
    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));

    if let Ok(script_source) =
        crate::resources::RuntimeResources::get_script("scripts/uninstall.rhai")
    {
        tracing::info!("Running uninstall script (silent mode)");
        let script_ctx = crate::scripting::ScriptContext::for_uninstall(
            config.clone(),
            install_path.clone(),
            progress,
            status,
            finished,
            false,
        );
        let mut engine = crate::scripting::ScriptEngine::new(script_ctx);
        engine
            .run_script(&script_source)
            .map_err(|e| anyhow::anyhow!("Uninstall script error: {}", e))?;
        if !engine.context().tracked_uninstall_invoked() {
            let start_pct = (*engine.context().uninstall_progress.read() * 100.0).clamp(10.0, 90.0);
            engine
                .context()
                .run_manifest_uninstall(start_pct, 95.0)
                .map_err(|e| anyhow::anyhow!("Implicit tracked uninstall failed: {}", e))?;
        }
    } else {
        let install_dir = std::path::Path::new(&install_path);
        let mut engine = crate::uninstaller::UninstallEngine::from_install_path(install_dir, false)
            .map_err(|e| anyhow::anyhow!("Failed to load uninstall manifest: {}", e))?;
        engine
            .uninstall_with_progress(|pct, step| {
                tracing::info!(
                    "Silent uninstall progress: {:>3.0}% ({:?})",
                    pct * 100.0,
                    step
                );
            })
            .map_err(|e| anyhow::anyhow!("Silent uninstall failed: {}", e))?;
    }

    schedule_self_delete_if_needed(std::path::Path::new(&install_path), &config.project.name)?;
    tracing::info!("Silent uninstallation completed successfully");
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
            .with_transparent(false)
            .with_active(true)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_icon(icon.unwrap_or_default()),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        &format!("{} {}", config.project.name, title_prefix),
        native_options,
        Box::new(move |cc| {
            let dpi_config = create_dpi_config(&config);
            setup_chinese_font(&cc.egui_ctx);

            let app = InstallerApp::new_with_mode(config.clone(), dpi_config, wizard_mode);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;

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

    let icon_data =
        RuntimeResources::get_asset(icon_path).context("Failed to load installer icon")?;

    load_icon_from_ico(&icon_data)
}

/// 从 ICO 字节加载图标
fn load_icon_from_ico(ico_bytes: &[u8]) -> Result<Option<egui::IconData>> {
    use image::GenericImageView;

    let img = image::load_from_memory(ico_bytes).context("Failed to decode icon image")?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    Ok(Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }))
}

fn resolve_silent_uninstall_path(config: &InstallerConfig) -> Result<String> {
    if let Some(path) = InstallerMode::get_cli_install_path() {
        return Ok(path);
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(file_name) = exe_path.file_name() {
            let name = file_name.to_string_lossy().to_lowercase();
            if (name.contains("uninst") || name.contains("uninstall"))
                && exe_path.parent().is_some()
            {
                return Ok(exe_path.parent().unwrap().to_string_lossy().to_string());
            }
        }
    }

    #[cfg(windows)]
    if let Some(path) = crate::installer::windows::registry::get_string_value(
        &config.registry.install_path_key,
        "InstallPath",
    )
    .map_err(|e| anyhow::anyhow!("Failed to read install path from registry: {}", e))?
    {
        return Ok(path);
    }

    Err(anyhow::anyhow!(
        "Unable to determine install path for silent uninstall. Provide --path or run uninst.exe directly."
    ))
}

pub(crate) fn resolve_existing_install_path(config: &InstallerConfig) -> Option<String> {
    #[cfg(windows)]
    {
        if let Ok(Some(path)) = crate::installer::windows::registry::get_string_value(
            &config.registry.install_path_key,
            "InstallPath",
        ) {
            return Some(path);
        }

        if let Ok(Some(path)) = crate::installer::windows::registry::get_string_value(
            &config.registry.uninstall_key,
            "InstallLocation",
        ) {
            return Some(path);
        }
    }

    None
}

#[cfg(windows)]
fn schedule_self_delete_if_needed(install_dir: &std::path::Path, product_name: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let current_exe = std::env::current_exe()
        .context("Failed to resolve current executable during uninstall cleanup")?;
    if !current_exe.starts_with(install_dir) {
        return Ok(());
    }

    let batch_path = write_self_delete_script(&current_exe, install_dir, product_name)?;
    std::process::Command::new("cmd")
        .args(["/C", &batch_path.to_string_lossy().to_string()])
        .current_dir(std::env::temp_dir())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("Failed to launch uninstall cleanup script")?;
    Ok(())
}

#[cfg(not(windows))]
fn schedule_self_delete_if_needed(
    _install_dir: &std::path::Path,
    _product_name: &str,
) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
fn write_self_delete_script(
    exe_path: &std::path::Path,
    install_dir: &std::path::Path,
    product_name: &str,
) -> Result<std::path::PathBuf> {
    let batch_content = format!(
        "@echo off\r\n\
        cd /d \"%TEMP%\"\r\n\
        :retry\r\n\
        timeout /t 2 /nobreak >nul\r\n\
        del /F /Q \"{exe}\"\r\n\
        if exist \"{exe}\" goto retry\r\n\
        del /F /Q \"{dir}\\*.*\"\r\n\
        rmdir /S /Q \"{dir}\"\r\n\
        del /F /Q \"%~f0\"\r\n",
        exe = exe_path.display(),
        dir = install_dir.display()
    );

    let batch_path = std::env::temp_dir().join(format!("{}_uninstall_cleanup.bat", product_name));
    std::fs::write(&batch_path, batch_content)
        .with_context(|| format!("Failed to write cleanup script {}", batch_path.display()))?;
    Ok(batch_path)
}

/// 设置中文字体支持
fn setup_chinese_font(ctx: &egui::Context) {
    use egui::FontDefinitions;
    use egui::FontFamily;
    use std::sync::Arc;

    let mut fonts = FontDefinitions::default();

    // Load multiple system fonts for multi-language support
    // Each font covers different Unicode ranges
    let font_configs: &[(&str, &[&str])] = &[
        // CJK Chinese + Japanese (primary)
        (
            "cjk",
            &[
                "C:\\Windows\\Fonts\\msyh.ttc",   // 微软雅黑 (Chinese + Japanese)
                "C:\\Windows\\Fonts\\simhei.ttf", // 黑体
            ],
        ),
        // Korean
        (
            "korean",
            &[
                "C:\\Windows\\Fonts\\malgun.ttf",   // Malgun Gothic
                "C:\\Windows\\Fonts\\malgunsl.ttf", // Malgun Gothic Semilight
                "C:\\Windows\\Fonts\\gulim.ttc",    // Gulim
            ],
        ),
        // Thai
        (
            "thai",
            &[
                "C:\\Windows\\Fonts\\leelawad.ttf", // Leelawadee
                "C:\\Windows\\Fonts\\LeelUIsl.ttf", // Leelawadee UI Semilight
                "C:\\Windows\\Fonts\\cordia.ttc",   // Cordia New
            ],
        ),
        // Vietnamese / Latin Extended (covers Portuguese, Spanish, etc.)
        (
            "latin_ext",
            &[
                "C:\\Windows\\Fonts\\segoeui.ttf", // Segoe UI (broad Latin coverage)
                "C:\\Windows\\Fonts\\arial.ttf",   // Arial
            ],
        ),
    ];

    let mut loaded_count = 0;
    for (name, paths) in font_configs {
        for path in *paths {
            if let Ok(data) = std::fs::read(path) {
                tracing::info!("Loaded font '{}': {}", name, path);
                fonts
                    .font_data
                    .insert(name.to_string(), Arc::new(egui::FontData::from_owned(data)));

                // Insert into font families (CJK first, then others as fallback)
                let priority = if *name == "cjk" { 0 } else { loaded_count + 1 };
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(priority, name.to_string());
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .insert(priority, name.to_string());

                loaded_count += 1;
                break; // Only need one font per category
            }
        }
    }

    if loaded_count == 0 {
        tracing::warn!("No system fonts loaded, text may not display correctly");
    } else {
        tracing::info!(
            "Loaded {} font families for multi-language support",
            loaded_count
        );
    }

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::resolve_existing_install_path;
    use crate::config::InstallerConfig;

    #[cfg(windows)]
    #[test]
    fn resolve_existing_install_path_prefers_install_path_registry_key() {
        let mut config = InstallerConfig::default();
        config.registry.install_path_key =
            "HKCU\\Software\\NanoInstallerTests\\ResolvePath".to_string();
        config.registry.uninstall_key =
            "HKCU\\Software\\NanoInstallerTests\\ResolvePath\\Uninstall".to_string();

        let write_result = crate::installer::windows::registry::set_string_value(
            &config.registry.install_path_key,
            "InstallPath",
            "C:\\ResolvedPath",
        );
        if let Err(err) = write_result {
            let message = err.to_string();
            if message.contains("拒绝访问")
                || message.to_ascii_lowercase().contains("access is denied")
            {
                return;
            }
            panic!("failed to set install path registry value: {}", message);
        }

        let resolved = resolve_existing_install_path(&config);
        assert_eq!(resolved.as_deref(), Some("C:\\ResolvedPath"));

        let _ = crate::installer::windows::registry::delete_key(
            "HKCU\\Software\\NanoInstallerTests\\ResolvePath",
        );
    }
}
