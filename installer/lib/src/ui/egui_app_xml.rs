//! 基于 XML 布局的安装程序界面
//!
//! 所有 UI 元素（背景、按钮、文本等）完全由 XML 布局文件定义
//! 按钮行为由 XML action 属性声明，代码通过 dispatch_action 通用分发

use eframe::egui;
use crate::config::InstallerConfig;
use crate::installer::state::{InstallState, InstallProgress};
use crate::installer::task_runner::TaskRunner;
use crate::ui::wizard::{Wizard, WizardMode};
use crate::ui::message_box::{MessageBoxManager, MessageBoxResult as MsgBoxResult};
use crate::ui::layout_renderer::LayoutRenderer;
use crate::ui::style_engine::StyleEngine;
use crate::ui::dpi_handler::DpiConfig;
use crate::layout::{LayoutTree, LayoutElement, XmlParser};
use crate::common::path_validation::PathValidator;
use crate::common::process::ProcessDetector;
use crate::resources::RuntimeResources;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 安装程序应用
pub struct InstallerApp {
    /// 向导状态（配置驱动，String page ID）
    wizard: Wizard,
    /// 配置
    config: InstallerConfig,
    /// 配置文件基础路径
    config_base_path: PathBuf,
    /// 安装状态 (shared with background install thread via internal Arc<RwLock>)
    install_state: Option<InstallState>,
    /// 安装路径
    install_path: String,
    /// 是否创建桌面快捷方式
    create_desktop_shortcut: bool,
    /// 是否创建开始菜单快捷方式
    create_start_menu_shortcut: bool,
    /// 是否同意条款
    agree_to_terms: bool,
    /// 当前语言
    current_language: String,

    // 消息框管理器
    message_box_manager: MessageBoxManager,

    // 待处理的关闭确认对话框 ID
    pending_close_confirm_id: Option<String>,

    // 消息框 XML 布局缓存
    msgbox_layout: Option<LayoutTree>,

    // XML 布局系统
    layout_renderer: Option<LayoutRenderer>,
    layout_cache: HashMap<String, LayoutTree>,
    layout_parser: XmlParser,

    // 国际化字符串
    i18n_strings: HashMap<String, String>,

    // DPI 配置
    dpi_config: DpiConfig,

    // 路径校验是否已初始化
    path_validation_initialized: bool,

    // ── Installation engine state for background install ──
    install_thread: Option<std::thread::JoinHandle<Result<(), String>>>,
    install_error: Option<String>,
    install_completed: bool,

    // 自启动偏好
    autorun_preference: bool,
    // 保留数据偏好 (卸载时)
    reserve_data_preference: bool,

    // 卸载状态
    uninstall_progress: Arc<RwLock<f32>>,
    uninstall_status: Arc<RwLock<String>>,
    uninstall_finished: Arc<AtomicBool>,
    uninstall_error: Arc<RwLock<Option<String>>>,

    // 待处理的窗口大小调整
    pending_resize: Option<egui::Vec2>,
}

impl InstallerApp {
    /// 创建 InstallerApp（接受已检测的 DpiConfig，避免重复检测导致不一致）
    pub fn new_with_dpi(config: InstallerConfig, config_base_path: PathBuf, dpi_config: DpiConfig) -> Self {
        Self::new_with_mode(config, config_base_path, dpi_config, WizardMode::Install)
    }

    /// 创建 InstallerApp（指定模式：安装或卸载）
    pub fn new_with_mode(config: InstallerConfig, config_base_path: PathBuf, dpi_config: DpiConfig, mode: WizardMode) -> Self {
        // 卸载模式: 安装路径 = uninst.exe 所在目录 (而非 config 中的 default_path)
        let install_path = if mode == WizardMode::Uninstall {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_string_lossy().to_string()))
                .unwrap_or_else(|| config.install.default_path.clone())
        } else {
            config.install.default_path.clone()
        };
        tracing::info!("Creating InstallerApp with DPI: use_2x={}, window: {}x{}, mode: {:?}",
            dpi_config.use_2x, dpi_config.window_width, dpi_config.window_height, mode);

        // 加载国际化字符串
        let i18n_strings = Self::load_i18n_strings(&config, &config_base_path);
        tracing::debug!("Loaded {} i18n strings", i18n_strings.len());

        // 创建布局渲染器
        let style_engine = StyleEngine::new();
        let layout_renderer = LayoutRenderer::with_style_engine(
            dpi_config.clone(),
            style_engine,
            i18n_strings.clone(),
        );
        tracing::debug!("LayoutRenderer created");

        // 创建配置驱动的向导
        let wizard = Wizard::new(mode, &config.wizard.pages, &config.wizard.update_pages, &config.wizard.uninstall_pages);

        let create_desktop_shortcut = config.shortcuts.desktop_default;
        let autorun_preference = config.autostart.default;
        let reserve_data_preference = config.uninstall.keep_data_default;

        // 预加载 msgBox.xml 布局
        let mut parser = XmlParser::new();
        let msgbox_layout = RuntimeResources::get_layout("layouts/msgBox.xml")
            .ok()
            .and_then(|content| parser.parse_string(&content).ok());
        if msgbox_layout.is_some() {
            tracing::debug!("msgBox.xml layout loaded");
        }

        tracing::info!("InstallerApp created successfully");

        let mut message_box_manager = MessageBoxManager::new();
        // TODO: XML dialog rendering has positioning issues, use code fallback for now
        // if let Some(ref template) = msgbox_layout {
        //     message_box_manager.set_template(template.clone());
        // }

        Self {
            wizard,
            config,
            config_base_path,
            install_state: None,
            install_path,
            create_desktop_shortcut,
            create_start_menu_shortcut: true,
            agree_to_terms: false,
            current_language: "zh-CN".to_string(),

            message_box_manager,
            pending_close_confirm_id: None,
            msgbox_layout,
            layout_renderer: Some(layout_renderer),
            layout_cache: HashMap::new(),
            layout_parser: parser,
            i18n_strings,
            dpi_config,
            path_validation_initialized: false,

            install_thread: None,
            install_error: None,
            install_completed: false,

            autorun_preference,
            reserve_data_preference,

            uninstall_progress: Arc::new(RwLock::new(0.0)),
            uninstall_status: Arc::new(RwLock::new(String::new())),
            uninstall_finished: Arc::new(AtomicBool::new(false)),
            uninstall_error: Arc::new(RwLock::new(None)),
            pending_resize: None,
        }
    }

    /// 加载国际化字符串
    fn load_i18n_strings(config: &InstallerConfig, _base_path: &PathBuf) -> HashMap<String, String> {
        let mut strings = HashMap::new();

        // 从配置加载基本字符串
        strings.insert("app_name".to_string(), config.project.name.clone());
        strings.insert("version".to_string(), config.project.version.clone());

        let locale = &config.localization.default_locale;

        // 从嵌入的 .pak 语言包加载 (Locales 段)
        match RuntimeResources::get_locale(locale) {
            Ok(pak_data) => {
                match crate::i18n::langpack::LanguagePack::from_bytes(&pak_data) {
                    Ok(pack) => {
                        tracing::info!("Loaded language pack: {} ({} keys)", locale, pack.translations.len());
                        for (key, value) in &pack.translations {
                            strings.insert(key.clone(), value.clone());
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse language pack {}: {}", locale, e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load locale {}: {}", locale, e);
                // 回退: 尝试从 UI Resources 段加载 JSON 格式
                let locale_file = format!("locales/{}.json", locale);
                if let Ok(json_content) = RuntimeResources::get_layout(&locale_file) {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&json_content) {
                        if let Some(obj) = json_value.as_object() {
                            for (key, value) in obj {
                                if let Some(text) = value.as_str() {
                                    strings.insert(key.clone(), text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        strings
    }

    /// 加载或获取页面布局（按 page ID）
    fn get_page_layout(&mut self, page_id: &str) -> Option<&LayoutTree> {
        // 如果已经缓存，直接返回
        if self.layout_cache.contains_key(page_id) {
            return self.layout_cache.get(page_id);
        }

        // 从配置中查找布局文件
        let layout_file = self.find_layout_file_for_page(page_id)?;

        // 新格式 XML 使用 1x 逻辑像素，DPI 缩放由框架自动处理，不需要 @2x 文件
        let selected_file = if self.dpi_config.use_2x {
            let path = std::path::Path::new(&layout_file);
            let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");
            let stem = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_else(|| path.to_str().unwrap_or(&layout_file));
            let ext = path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("xml");

            if parent.is_empty() {
                format!("{}@2x.{}", stem, ext)
            } else {
                format!("{}/{}@2x.{}", parent, stem, ext)
            }
        } else {
            eprintln!("[layout] Using 1x layout: {}", layout_file);
            layout_file.clone()
        };

        // 加载布局文件: 先尝试直接加载 1x 文件 (新格式不需要 2x)
        let layout_content = RuntimeResources::get_layout(&layout_file)
            .or_else(|_| {
                if self.dpi_config.use_2x {
                    RuntimeResources::get_layout(&selected_file)
                } else {
                    Err(anyhow::anyhow!("Layout not found"))
                }
            })
            .ok()?;

        // 解析布局
        match self.layout_parser.parse_string(&layout_content) {
            Ok(layout_tree) => {
                let layout_width = layout_tree.root.attributes.width.unwrap_or(0.0);
                let layout_height = layout_tree.root.attributes.height.unwrap_or(0.0);

                eprintln!("[layout] Loaded: {} -> {} ({}x{}, window: {}x{})",
                    page_id, selected_file,
                    layout_width, layout_height,
                    self.dpi_config.window_width, self.dpi_config.window_height);

                // 检查尺寸是否匹配
                if layout_width > 0.0 && layout_height > 0.0 {
                    let width_match = (layout_width - self.dpi_config.window_width).abs() < 1.0;
                    let height_match = (layout_height - self.dpi_config.window_height).abs() < 1.0;
                    if !width_match || !height_match {
                        eprintln!("[layout] Warning: layout size mismatch! layout: {}x{}, window: {}x{}",
                            layout_width, layout_height,
                            self.dpi_config.window_width, self.dpi_config.window_height);
                    }
                }

                self.layout_cache.insert(page_id.to_string(), layout_tree);
                self.layout_cache.get(page_id)
            }
            Err(e) => {
                eprintln!("[layout] Parse failed: {} - {}", selected_file, e);
                None
            }
        }
    }

    /// 查找页面对应的布局文件（从 config.wizard 读取）
    fn find_layout_file_for_page(&self, page_id: &str) -> Option<String> {
        for page_config in &self.config.wizard.pages {
            if page_config.id == page_id {
                return Some(page_config.layout.clone());
            }
        }
        for page_config in &self.config.wizard.uninstall_pages {
            if page_config.id == page_id {
                return Some(page_config.layout.clone());
            }
        }
        None
    }

    /// 处理布局渲染结果
    fn handle_layout_result(&mut self, ctx: &egui::Context, result: crate::ui::layout_renderer::RenderResult) {
        // 处理按钮点击 — 优先使用 action 属性
        for (button_id, clicked) in &result.button_clicks {
            if *clicked {
                if let Some(action) = result.button_actions.get(button_id) {
                    self.dispatch_action(action, ctx);
                } else {
                    // 向后兼容: 没有 action 时按 button_id 分发
                    self.dispatch_action(button_id, ctx);
                }
            }
        }

        // 处理复选框变化
        for (checkbox_id, checked) in &result.checkbox_changes {
            self.handle_checkbox_change(checkbox_id, *checked);
        }

        // 处理文本输入
        for (input_id, text) in &result.text_input_changes {
            self.handle_text_input(input_id, text);
        }

        // 处理链接点击（来自 checkbox 内联链接）
        for (link_id, clicked) in &result.link_clicks {
            if *clicked {
                // 尝试从 config.links 查找
                if let Some(url) = self.config.links.get(link_id) {
                    self.open_url_in_browser(url);
                } else {
                    // 兼容旧的 link_id 映射
                    match link_id.as_str() {
                        "agreement" => {
                            if let Some(url) = self.config.links.get("terms_of_service") {
                                self.open_url_in_browser(url);
                            }
                        }
                        "policy" => {
                            if let Some(url) = self.config.links.get("privacy_policy") {
                                self.open_url_in_browser(url);
                            }
                        }
                        _ => {
                            tracing::warn!("Unknown link: {}", link_id);
                        }
                    }
                }
            }
        }
    }

    // =========================================================================
    //  Action Dispatcher — 替代 handle_button_click 的通用分发器
    // =========================================================================

    fn dispatch_action(&mut self, action: &str, ctx: &egui::Context) {
        let (cmd, arg) = action.split_once(':').unwrap_or((action, ""));
        tracing::info!("dispatch_action: cmd={}, arg={}", cmd, arg);

        match cmd {
            // ── 导航 ──
            "next_page" => {
                self.wizard.next();
            }
            "prev_page" => {
                self.wizard.previous();
            }

            // ── 安装 ──
            "install" => {
                self.start_installation(ctx);
            }

            // ── 卸载 ──
            "uninstall" => {
                self.start_uninstall_task(ctx);
            }

            // ── 启动已安装应用 ──
            "launch_app" => {
                let exe_path = format!("{}\\{}", self.install_path, self.config.install.exe_name);
                tracing::info!("Launching installed app: {}", exe_path);
                let _ = Command::new(&exe_path).spawn();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // ── 关闭 ──
            "close" => {
                // 卸载模式退出时启动自删除批处理
                if self.wizard.mode() == WizardMode::Uninstall {
                    self.launch_self_delete_script();
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "close_confirm" => {
                self.show_close_confirmation();
            }

            // ── 打开 URL (action="open_url:KEY") ──
            "open_url" => {
                if let Some(url) = self.config.links.get(arg) {
                    self.open_url_in_browser(url);
                } else {
                    tracing::warn!("Unknown link key: {}", arg);
                }
            }

            // ── 切换面板 (action="toggle_panel:ID:show/hide") ──
            "toggle_panel" => {
                let parts: Vec<&str> = arg.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let show = parts[1] == "show";
                    self.toggle_more_config(show);
                }
            }

            // ── 浏览文件夹 ──
            "browse_folder" => {
                self.browse_for_folder();
            }

            // ── 切换语言 (action="switch_language:zh-CN") ──
            "switch_language" => {
                self.switch_language(arg, ctx);
            }

            // ── 取消安装 ──
            "cancel" => {
                tracing::info!("User cancelled installation");
                if let Some(ref state) = self.install_state {
                    state.cancel();
                }
            }

            // ── 向后兼容: 旧 button ID 直接作为 action ──
            "btnInstall" => self.start_installation(ctx),
            "btnRun" => {
                let exe_path = format!("{}\\{}", self.install_path, self.config.install.exe_name);
                let _ = Command::new(&exe_path).spawn();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "btnShowMore" => self.toggle_more_config(true),
            "btnHideMore" => self.toggle_more_config(false),
            "btnSelectDir" => self.browse_for_folder(),
            "btnAgreement" => {
                if let Some(url) = self.config.links.get("terms_of_service") {
                    self.open_url_in_browser(url);
                }
            }
            "btnPolicy" => {
                if let Some(url) = self.config.links.get("privacy_policy") {
                    self.open_url_in_browser(url);
                }
            }
            "btnUnInstall" => self.start_uninstall_task(ctx),
            "btnNotNow" | "btnUninstalled" | "uninstall_finish" | "finish" => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "next" => self.wizard.next(),
            "back" => self.wizard.previous(),

            _ => {
                tracing::warn!("Unknown action: {}", action);
            }
        }
    }

    // =========================================================================
    //  Start installation in background thread
    // =========================================================================

    fn start_installation(&mut self, ctx: &egui::Context) {
        // a) Check and kill running target processes
        if self.config.install.detect_running_process {
            let target = vec![self.config.install.exe_name.clone()];
            let detector = ProcessDetector::new(target);
            if let Ok(true) = detector.is_target_running() {
                tracing::warn!("Target process is running");
                if self.config.install.kill_process_on_install {
                    tracing::info!("Attempting to terminate target process");
                    if let Err(e) = detector.terminate_target_processes() {
                        tracing::error!("Failed to terminate process: {}", e);
                        self.install_error = Some(format!("Failed to terminate running process: {}", e));
                        self.show_install_error_dialog();
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                } else {
                    self.install_error = Some(
                        "Target application is running. Please close it first.".to_string()
                    );
                    self.show_install_error_dialog();
                    return;
                }
            }
        }

        // b) Validate install path
        let install_path = self.install_path.clone();
        let required_space_bytes = (self.config.install.required_space_mb as u64) * 1024 * 1024;
        let validator = PathValidator::new(required_space_bytes);

        match validator.validate_path(Path::new(&install_path)) {
            Ok(result) => {
                if !result.is_valid {
                    let error_msg = result.errors.join("; ");
                    tracing::error!("Path validation failed: {}", error_msg);
                    self.install_error = Some(format!("Invalid install path: {}", error_msg));
                    self.show_install_error_dialog();
                    return;
                }
                for warning in &result.warnings {
                    tracing::warn!("Path warning: {}", warning);
                }
            }
            Err(e) => {
                // Non-fatal: the directory may not exist yet, we will create it
                tracing::warn!("Path validation error (non-fatal): {}", e);
            }
        }

        // c) Create install directory
        if let Err(e) = std::fs::create_dir_all(&install_path) {
            tracing::error!("Failed to create install directory: {}", e);
            self.install_error = Some(format!("Cannot create directory: {}", e));
            self.show_install_error_dialog();
            return;
        }

        // d) Create InstallState and configure preferences
        let state = InstallState::new(install_path.clone());
        state.set_create_desktop_shortcut(self.create_desktop_shortcut);
        state.set_create_start_menu_shortcut(self.create_start_menu_shortcut);
        self.install_state = Some(state.clone());

        // e) Run installation — script mode or config mode
        let config_clone = self.config.clone();
        let ctx_clone = ctx.clone();
        let create_desktop_shortcut = self.create_desktop_shortcut;
        let autorun_pref = self.autorun_preference;

        let handle = std::thread::spawn(move || {
            // Check if scripts/install.rhai exists in embedded resources
            if let Ok(script_source) = crate::resources::RuntimeResources::get_script("scripts/install.rhai") {
                tracing::info!("Running install script (scripts/install.rhai)");
                let mut script_ctx = crate::scripting::ScriptContext::for_install(state, config_clone);
                // Populate checkbox values from UI state
                script_ctx.checkbox_values.write().insert("desktop_shortcut".to_string(), create_desktop_shortcut);
                script_ctx.checkbox_values.write().insert("autorun".to_string(), autorun_pref);
                let mut engine = crate::scripting::ScriptEngine::new(script_ctx);
                engine.run_script(&script_source)?;
            } else {
                // Config mode: existing TaskRunner behavior
                let mut runner = TaskRunner::new(&config_clone);
                runner.execute(&state, &config_clone)
                    .map_err(|e| format!("{}", e))?;
            }

            ctx_clone.request_repaint();
            Ok(())
        });

        self.install_thread = Some(handle);
        self.install_completed = false;
        self.install_error = None;

        // Mark wizard as installing (disables navigation)
        let preparing = self.i18n("status.preparing");
        self.wizard.start_installation(&preparing);

        // g) Navigate to Installing page
        self.wizard.next();
    }

    // =========================================================================
    //  Start uninstall in background thread
    // =========================================================================

    fn start_uninstall_task(&mut self, ctx: &egui::Context) {
        let progress = self.uninstall_progress.clone();
        let status = self.uninstall_status.clone();
        let finished = self.uninstall_finished.clone();
        let _error = self.uninstall_error.clone();
        let install_path = self.install_path.clone();
        let config = self.config.clone();
        let reserve_data = self.reserve_data_preference;
        let ctx_clone = ctx.clone();

        // Clone locale strings for background thread
        let status_closing = self.i18n("uninstall.status.closing_app");
        let status_shortcuts = self.i18n("uninstall.status.removing_shortcuts");
        let status_registry = self.i18n("uninstall.status.cleaning_registry");
        let status_user_data = self.i18n("uninstall.status.removing_user_data");
        let status_files = self.i18n("uninstall.status.removing_files");
        let status_finishing = self.i18n("uninstall.status.finishing");
        let status_complete = self.i18n("uninstall.status.complete");

        // 标记向导为安装中状态
        let preparing = self.i18n("status.preparing");
        self.wizard.start_installation(&preparing);
        self.wizard.next();

        std::thread::spawn(move || {
            tracing::info!("Uninstall background thread started");

            // Check if scripts/uninstall.rhai exists
            if let Ok(script_source) = crate::resources::RuntimeResources::get_script("scripts/uninstall.rhai") {
                tracing::info!("Running uninstall script (scripts/uninstall.rhai)");
                let script_ctx = crate::scripting::ScriptContext::for_uninstall(
                    config.clone(), install_path, progress, status, finished, reserve_data,
                );
                let mut engine = crate::scripting::ScriptEngine::new(script_ctx);
                if let Err(e) = engine.run_script(&script_source) {
                    tracing::error!("Uninstall script failed: {}", e);
                }
                ctx_clone.request_repaint();
                return;
            }

            // Step 1: Kill running process
            *status.write() = status_closing;
            *progress.write() = 0.05;
            if config.install.kill_process_on_uninstall {
                let exe_name = &config.install.exe_name;
                tracing::info!("Attempting to kill process: {}", exe_name);
                let detector = ProcessDetector::new(vec![exe_name.clone()]);
                if let Ok(true) = detector.is_target_running() {
                    let _ = detector.terminate_target_processes();
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            // Step 2: Remove shortcuts
            *status.write() = status_shortcuts;
            *progress.write() = 0.15;
            #[cfg(windows)]
            {
                if let Ok(userprofile) = std::env::var("USERPROFILE") {
                    let desktop = PathBuf::from(userprofile).join("Desktop");
                    let shortcut_path = desktop.join(format!("{}.lnk", config.project.name));
                    if shortcut_path.exists() {
                        let _ = std::fs::remove_file(&shortcut_path);
                        tracing::info!("Removed desktop shortcut: {:?}", shortcut_path);
                    }
                }
                if let Ok(appdata) = std::env::var("APPDATA") {
                    let start_menu_folder = config.shortcuts.start_menu_folder.as_str();
                    let start_menu_path = PathBuf::from(appdata)
                        .join("Microsoft\\Windows\\Start Menu\\Programs")
                        .join(start_menu_folder);
                    if start_menu_path.exists() {
                        let _ = std::fs::remove_dir_all(&start_menu_path);
                        tracing::info!("Removed start menu folder: {:?}", start_menu_path);
                    }
                }
            }

            // Step 3: Clean registry
            *status.write() = status_registry;
            *progress.write() = 0.30;
            #[cfg(windows)]
            {
                use winreg::enums::*;
                use winreg::RegKey;

                // Clean both HKLM and HKCU (config may use either)
                let hives = [
                    RegKey::predef(HKEY_LOCAL_MACHINE),
                    RegKey::predef(HKEY_CURRENT_USER),
                ];
                let uninstall_key = format!(
                    "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
                    config.project.name
                );
                let app_key = format!("Software\\{}", config.project.name);

                for hive in &hives {
                    if let Ok(key) = hive.open_subkey(&uninstall_key) {
                        drop(key);
                        let _ = hive.delete_subkey_all(&uninstall_key);
                        tracing::info!("Removed uninstall registry key: {}", uninstall_key);
                    }
                    if let Ok(key) = hive.open_subkey(&app_key) {
                        drop(key);
                        let _ = hive.delete_subkey_all(&app_key);
                        tracing::info!("Removed app registry key: {}", app_key);
                    }
                }

                if config.autostart.enabled {
                    for hive in &hives {
                        if let Ok(run_key) = hive.open_subkey_with_flags(
                            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                            winreg::enums::KEY_WRITE
                        ) {
                            let _ = run_key.delete_value(&config.autostart.registry_value_name);
                        }
                    }
                }

                if config.uninstall.cleanup_game_registry {
                    let game_key = &config.uninstall.game_registry_path;
                    for hive in &hives {
                        if let Ok(key) = hive.open_subkey(game_key) {
                            drop(key);
                            let _ = hive.delete_subkey_all(game_key);
                        }
                    }
                }
            }

            // Step 4: Remove user data (if not reserved)
            *status.write() = status_user_data;
            *progress.write() = 0.45;
            if !reserve_data {
                for data_path_template in &config.uninstall.data_paths {
                    let expanded = Self::expand_env_vars(data_path_template);
                    let data_path = PathBuf::from(&expanded);
                    if data_path.exists() {
                        tracing::info!("Removing user data: {:?}", data_path);
                        let _ = std::fs::remove_dir_all(&data_path);
                    }
                }
            }

            // Step 5: Remove install files
            *status.write() = status_files;
            *progress.write() = 0.60;
            let install_dir = PathBuf::from(&install_path);
            if install_dir.exists() {
                let exe_path = std::env::current_exe().unwrap_or_default();
                if let Ok(entries) = std::fs::read_dir(&install_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path == exe_path {
                            continue;
                        }
                        if path.is_dir() {
                            let _ = std::fs::remove_dir_all(&path);
                        } else {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
                tracing::info!("Removed installation files from: {:?}", install_dir);
            }
            *progress.write() = 0.85;

            // Step 6: Prepare self-deletion script (will be launched on process exit)
            *status.write() = status_finishing;
            *progress.write() = 0.95;
            #[cfg(windows)]
            {
                if let Ok(exe_path) = std::env::current_exe() {
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
                    let temp_dir = std::env::temp_dir();
                    let batch_path = temp_dir.join(format!("{}_uninstall_cleanup.bat", config.project.name));
                    if let Ok(()) = std::fs::write(&batch_path, &batch_content) {
                        // 不在这里启动，而是保存路径，在进程退出时启动
                        tracing::info!("Self-deletion script prepared: {:?}", batch_path);
                    }
                }
            }

            // Done
            *progress.write() = 1.0;
            *status.write() = status_complete;
            finished.store(true, Ordering::SeqCst);
            ctx_clone.request_repaint();
            tracing::info!("Uninstall background thread completed successfully");
        });
    }

    // =========================================================================
    //  Toggle expanded config panel
    // =========================================================================

    fn toggle_more_config(&mut self, show: bool) {
        let current_page_id = self.wizard.current_page_id().to_string();
        if let Some(layout) = self.layout_cache.get_mut(&current_page_id) {
            Self::update_element_visible_recursive(&mut layout.root, "moreconfiginfo", show);
            Self::update_element_visible_recursive(&mut layout.root, "btnShowMore", !show);
            Self::update_element_visible_recursive(&mut layout.root, "btnHideMore", show);
        }

        // 调整窗口高度
        let new_height = if show {
            self.dpi_config.expanded_height
        } else {
            self.dpi_config.window_height
        };
        self.pending_resize = Some(egui::vec2(self.dpi_config.window_width, new_height));

        if show {
            if let Some(ref mut renderer) = self.layout_renderer {
                renderer.set_text_input_value("editDir", self.install_path.clone());
            }
            let path = self.install_path.clone();
            self.validate_and_update_path_info(&path);
        }
    }

    // =========================================================================
    //  Browse for folder
    // =========================================================================

    fn browse_for_folder(&mut self) {
        let dialog = rfd::FileDialog::new()
            .set_title("Select Install Directory")
            .set_directory(&self.install_path);

        if let Some(folder) = dialog.pick_folder() {
            let mut new_path = folder.to_string_lossy().to_string();

            let append = &self.config.install.append_to_path;
            if !append.is_empty() {
                if !new_path.ends_with('\\') && !new_path.ends_with('/') {
                    new_path.push('\\');
                }
                new_path.push_str(append);
            }

            self.install_path = new_path.clone();

            if let Some(ref mut renderer) = self.layout_renderer {
                renderer.set_text_input_value("editDir", new_path.clone());
            }

            self.validate_and_update_path_info(&new_path);
        }
    }

    // =========================================================================
    //  Poll install progress (called every frame while installing)
    // =========================================================================

    fn poll_install_progress(&mut self, ctx: &egui::Context) {
        let current_page_id = self.wizard.current_page_id().to_string();

        // Poll install thread progress
        if let Some(ref state) = self.install_state {
            if self.install_thread.is_some() {
                let progress = state.progress();

                // Update the progress bar and label in the layout
                if let Some(layout) = self.layout_cache.get_mut(&current_page_id) {
                    Self::update_progress_bar_recursive(
                        &mut layout.root,
                        "slrProgress",
                        progress.percentage / 100.0,
                    );

                    let label_text = if progress.percentage >= 100.0 {
                        progress.current_step.clone()
                    } else if progress.current_step.is_empty() {
                        format!("{:.0}%", progress.percentage)
                    } else {
                        format!("{} ({:.0}%)", progress.current_step, progress.percentage)
                    };
                    Self::update_label_text(&mut layout.root, "progress_pos", &label_text);
                }

                self.wizard.update_install_progress(
                    progress.percentage / 100.0,
                    progress.current_step.clone(),
                );

                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        // Check if the install thread has finished
        let thread_finished = self.install_thread.as_ref().map_or(false, |h| h.is_finished());
        if thread_finished {
            if let Some(handle) = self.install_thread.take() {
                match handle.join() {
                    Ok(Ok(())) => {
                        tracing::info!("Installation thread completed successfully");
                        self.install_completed = true;
                        let complete = self.i18n("status.install_complete");
                        self.wizard.finish_installation(&complete);
                    }
                    Ok(Err(err_msg)) => {
                        tracing::error!("Installation thread failed: {}", err_msg);
                        self.install_error = Some(err_msg);
                        let complete = self.i18n("status.install_complete");
                        self.wizard.finish_installation(&complete);
                        self.show_install_error_dialog();
                    }
                    Err(_) => {
                        tracing::error!("Installation thread panicked");
                        self.install_error = Some("Installation thread panicked unexpectedly".to_string());
                        let complete = self.i18n("status.install_complete");
                        self.wizard.finish_installation(&complete);
                        self.show_install_error_dialog();
                    }
                }
            }
        }

        // Poll uninstall progress
        if self.wizard.is_installing() {
            let is_uninstall_page = self.config.wizard.uninstall_pages.iter()
                .any(|p| p.id == current_page_id);
            if is_uninstall_page {
                let progress_val = *self.uninstall_progress.read();
                let status_val = self.uninstall_status.read().clone();

                self.wizard.update_install_progress(progress_val, status_val.clone());

                if let Some(layout) = self.layout_cache.get_mut(&current_page_id) {
                    Self::update_progress_bar_recursive(
                        &mut layout.root,
                        "slrUnInstProgress",
                        progress_val,
                    );

                    let label_text = if progress_val >= 1.0 {
                        status_val
                    } else if status_val.is_empty() {
                        format!("{:.0}%", progress_val * 100.0)
                    } else {
                        format!("{} ({:.0}%)", status_val, progress_val * 100.0)
                    };
                    Self::update_label_text(&mut layout.root, "un_progress_pos", &label_text);
                }

                if self.uninstall_finished.load(Ordering::SeqCst) {
                    let complete = self.i18n("uninstall.status.complete");
                    self.wizard.finish_installation(&complete);
                }

                ctx.request_repaint();
            }
        }
    }

    // =========================================================================
    //  Show install error dialog
    // =========================================================================

    fn show_install_error_dialog(&mut self) {
        let error_msg = self.install_error.clone().unwrap_or_else(|| "Unknown error".to_string());
        let dialog_width = if self.dpi_config.use_2x { 800.0 } else { 400.0 };
        let dialog_height = if self.dpi_config.use_2x { 460.0 } else { 230.0 };

        let _dialog_id = self.message_box_manager.show(
            crate::ui::message_box::MessageBoxConfig::new(
                "Installation Error".to_string(),
                error_msg,
            )
            .with_type(crate::ui::message_box::MessageBoxType::Error)
            .with_buttons(crate::ui::message_box::MessageBoxButton::Ok)
            .with_size(dialog_width, dialog_height)
            .with_dpi(self.dpi_config.use_2x)
        );
    }

    /// 处理复选框变化
    fn handle_checkbox_change(&mut self, checkbox_id: &str, checked: bool) {
        match checkbox_id {
            "chkAgree" | "agree_terms" | "agree_license" => {
                self.agree_to_terms = checked;
                tracing::info!("User agreement checkbox: {}", checked);
            }
            "chkShotcut" | "desktop_shortcut" => {
                self.create_desktop_shortcut = checked;
                tracing::info!("Desktop shortcut checkbox: {}", checked);
            }
            "chkAutoRun" => {
                self.autorun_preference = checked;
                tracing::info!("Autorun checkbox: {}", checked);
            }
            "chkReserveData" => {
                self.reserve_data_preference = checked;
                tracing::info!("Reserve data checkbox: {}", checked);
            }
            "start_menu_shortcut" => {
                self.create_start_menu_shortcut = checked;
            }
            _ => {
                tracing::warn!("Unhandled checkbox: {}", checkbox_id);
            }
        }
    }

    /// 处理文本输入
    fn handle_text_input(&mut self, input_id: &str, text: &str) {
        match input_id {
            "install_path" | "editDir" => {
                self.install_path = text.to_string();
                self.validate_and_update_path_info(text);
            }
            _ => {
                tracing::warn!("Unhandled text input: {}", input_id);
            }
        }
    }

    /// 格式化字节数为人类可读的大小
    fn format_size_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;
        const TB: u64 = 1024 * GB;

        if bytes >= TB {
            format!("{:.1} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{} MB", bytes / MB)
        } else if bytes >= KB {
            format!("{} KB", bytes / KB)
        } else {
            format!("{} B", bytes)
        }
    }

    /// 校验安装路径并更新布局中的磁盘空间标签
    fn validate_and_update_path_info(&mut self, path_str: &str) {
        let required_space_bytes = self.config.install.required_space_mb as u64 * 1024 * 1024;
        let validator = PathValidator::new(required_space_bytes);
        let path = Path::new(path_str);

        let required_label = self.i18n("required_space");
        let available_label = self.i18n("available_space");
        let required_text = format!("{}{}", required_label, Self::format_size_bytes(required_space_bytes));

        let current_page_id = self.wizard.current_page_id().to_string();

        match validator.validate_path(path) {
            Ok(result) => {
                let available_text = format!("{}{}", available_label, Self::format_size_bytes(result.free_space_bytes));

                if let Some(layout) = self.layout_cache.get_mut(&current_page_id) {
                    Self::update_label_text(&mut layout.root, "lblRequiredSpace", &required_text);
                    Self::update_label_text(&mut layout.root, "local_space", &available_text);

                    if result.free_space_bytes < result.required_space_bytes {
                        Self::update_label_color(&mut layout.root, "local_space", "#FFFF4444");
                    } else {
                        Self::update_label_color(&mut layout.root, "local_space", "#FF96A1A9");
                    }
                    if !result.errors.is_empty() {
                        tracing::warn!("Path validation errors: {:?}", result.errors);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Path validation failed: {}", e);
                let available_text = format!("{}--", available_label);
                if let Some(layout) = self.layout_cache.get_mut(&current_page_id) {
                    Self::update_label_text(&mut layout.root, "lblRequiredSpace", &required_text);
                    Self::update_label_text(&mut layout.root, "local_space", &available_text);
                    Self::update_label_color(&mut layout.root, "local_space", "#FFFF4444");
                }
            }
        }
    }

    /// 递归更新标签文本
    fn update_label_text(element: &mut LayoutElement, label_id: &str, new_text: &str) {
        if let Some(id) = &element.attributes.id {
            if id == label_id {
                element.attributes.text = Some(new_text.to_string());
                return;
            }
        }
        for child in &mut element.children {
            Self::update_label_text(child, label_id, new_text);
        }
    }

    /// 递归更新标签颜色
    fn update_label_color(element: &mut LayoutElement, label_id: &str, new_color: &str) {
        if let Some(id) = &element.attributes.id {
            if id == label_id {
                element.attributes.color = Some(new_color.to_string());
                return;
            }
        }
        for child in &mut element.children {
            Self::update_label_color(child, label_id, new_color);
        }
    }

    /// 递归更新按钮的 enabled 状态
    fn update_button_enabled_recursive(element: &mut LayoutElement, button_id: &str, enabled: bool) {
        if let Some(id) = &element.attributes.id {
            if id == button_id {
                element.attributes.enabled = Some(enabled);
                if let Some(vs) = &mut element.visual_style {
                    vs.enabled = enabled;
                }
                return;
            }
        }
        for child in &mut element.children {
            Self::update_button_enabled_recursive(child, button_id, enabled);
        }
    }

    /// 递归更新元素的 visible 状态
    fn update_element_visible_recursive(element: &mut LayoutElement, target_id: &str, visible: bool) {
        if let Some(id) = &element.attributes.id {
            if id == target_id {
                element.attributes.visible = Some(visible);
                if let Some(vs) = &mut element.visual_style {
                    vs.visible = visible;
                }
                return;
            }
        }
        for child in &mut element.children {
            Self::update_element_visible_recursive(child, target_id, visible);
        }
    }

    /// 递归更新进度条的 progress 值
    fn update_progress_bar_recursive(element: &mut LayoutElement, target_id: &str, progress: f32) {
        if let Some(id) = &element.attributes.id {
            if id == target_id {
                element.attributes.progress = Some(progress.clamp(0.0, 1.0));
                return;
            }
        }
        for child in &mut element.children {
            Self::update_progress_bar_recursive(child, target_id, progress);
        }
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        tracing::trace!("update() called");

        // Poll installation / uninstall progress before rendering
        self.poll_install_progress(ctx);

        // 处理待执行的窗口大小调整
        if let Some(size) = self.pending_resize.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }

        // 设置暗色主题
        ctx.set_visuals(egui::Visuals::dark());

        // 无边框窗口拖动区域（整个窗口上半部分可拖动）
        let title_bar_height = 200.0;
        let title_bar_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(ctx.viewport_rect().width(), title_bar_height),
        );

        let is_dragging = ctx.input(|i| {
            i.pointer.primary_down() && i.pointer.hover_pos().map_or(false, |pos| title_bar_rect.contains(pos))
        });

        if is_dragging {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // 渲染消息框并处理结果
        let layout_renderer = self.layout_renderer.as_mut().unwrap();
        let message_results = self.message_box_manager.render(
            ctx,
            &self.dpi_config,
            layout_renderer.get_resource_cache_mut()
        );
        for (id, result) in message_results {
            if id.starts_with("message_box_") {
                if let Some(close_confirm_id) = self.pending_close_confirm_id.as_ref() {
                    if id == *close_confirm_id {
                        match result {
                            MsgBoxResult::Ok => {
                                if let Some(ref state) = self.install_state {
                                    state.cancel();
                                }
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            _ => {}
                        }
                        self.pending_close_confirm_id = None;
                    }
                }
            }
        }

        // 主面板 - 完全由 XML 布局控制
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let current_page_id = self.wizard.current_page_id().to_string();

                // 确保布局已加载
                self.get_page_layout(&current_page_id);

                // 动态更新安装按钮的 enabled 状态（根据协议复选框）
                let is_config_page = current_page_id == "config" || current_page_id == "welcome";
                if is_config_page {
                    if let Some(layout) = self.layout_cache.get_mut(&current_page_id) {
                        Self::update_button_enabled_recursive(&mut layout.root, "install", self.agree_to_terms);
                        Self::update_button_enabled_recursive(&mut layout.root, "btnInstall", self.agree_to_terms);
                    }
                }

                // 首次加载配置页时，用默认安装路径初始化磁盘空间信息
                if !self.path_validation_initialized && self.layout_cache.contains_key(&current_page_id) {
                    if current_page_id == "config" {
                        let default_path = self.install_path.clone();
                        self.validate_and_update_path_info(&default_path);
                        self.path_validation_initialized = true;
                    }
                }

                let layout_opt = self.layout_cache.get(&current_page_id);
                if let Some(layout) = layout_opt {
                    if let Some(ref mut renderer) = self.layout_renderer {
                        let render_result = renderer.render(ui, layout);
                        self.handle_layout_result(ctx, render_result);
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Failed to load layout: {}", current_page_id))
                                .size(16.0)
                                .color(egui::Color32::RED)
                        );
                    });
                }
            });
    }
}

impl InstallerApp {
    /// 从 i18n 获取文本，回退到 key 本身
    fn i18n(&self, key: &str) -> String {
        self.i18n_strings.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    fn show_close_confirmation(&mut self) {
        let msg = self.i18n("close_confirm_message");
        let ok = self.i18n("ok");
        let cancel = self.i18n("cancel");
        let dialog_id = self.message_box_manager.show(
            crate::ui::message_box::MessageBoxConfig::new("".to_string(), msg)
                .with_type(crate::ui::message_box::MessageBoxType::Question)
                .with_buttons(crate::ui::message_box::MessageBoxButton::OkCancel)
                .with_size(self.config.ui.dialog_width as f32, self.config.ui.dialog_height as f32)
                .with_dpi(false)
                .with_button_texts(&ok, &cancel)
        );
        self.pending_close_confirm_id = Some(dialog_id);
    }

    /// 切换运行时语言
    fn switch_language(&mut self, locale: &str, ctx: &egui::Context) {
        if locale == self.current_language {
            return;
        }
        tracing::info!("Switching language to: {}", locale);

        // 1. 重新加载 i18n 字符串
        let mut config_clone = self.config.clone();
        config_clone.localization.default_locale = locale.to_string();
        let new_strings = Self::load_i18n_strings(&config_clone, &self.config_base_path);

        if new_strings.len() <= 2 {
            tracing::warn!("Failed to load locale {}, only got {} keys", locale, new_strings.len());
            return;
        }

        self.i18n_strings = new_strings;
        self.current_language = locale.to_string();

        // 2. 清除布局缓存（强制重加载 XML 以应用新 i18n）
        self.layout_cache.clear();

        // 3. 重建 LayoutRenderer（使用新的 i18n 字符串）
        let style_engine = StyleEngine::new();
        self.layout_renderer = Some(LayoutRenderer::with_style_engine(
            self.dpi_config.clone(),
            style_engine,
            self.i18n_strings.clone(),
        ));

        // 4. 重新加载 msgBox 模板
        let mut parser = XmlParser::new();
        if let Ok(content) = RuntimeResources::get_layout("layouts/msgBox.xml") {
            if let Ok(layout) = parser.parse_string(&content) {
                self.message_box_manager.set_template(layout);
            }
        }

        // 5. 重置路径校验标志（重新用新语言填充标签）
        self.path_validation_initialized = false;

        ctx.request_repaint();
        tracing::info!("Language switched to: {}", locale);
    }

    /// 在默认浏览器中打开 URL
    fn open_url_in_browser(&self, url: &str) {
        #[cfg(windows)]
        {
            let _ = Command::new("cmd")
                .args(["/C", "start", "", url])
                .spawn();
        }

        #[cfg(not(windows))]
        {
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("xdg-open").arg(url).spawn();
            }

            #[cfg(target_os = "macos")]
            {
                let _ = Command::new("open").arg(url).spawn();
            }
        }

        tracing::info!("Opening URL: {}", url);
    }

    /// 启动自删除批处理脚本（在进程退出后删除 uninst.exe 和安装目录）
    fn launch_self_delete_script(&self) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let batch_path = std::env::temp_dir()
                .join(format!("{}_uninstall_cleanup.bat", self.config.project.name));
            if batch_path.exists() {
                let _ = Command::new("cmd")
                    .args(["/C", &batch_path.to_string_lossy().to_string()])
                    .current_dir(std::env::temp_dir())
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
                tracing::info!("Launched self-deletion script: {:?}", batch_path);
            }
        }
    }

    /// 展开 Windows 风格的环境变量 (%VAR% -> value)
    fn expand_env_vars(input: &str) -> String {
        let mut result = input.to_string();
        while let Some(start) = result.find('%') {
            if let Some(end) = result[start + 1..].find('%') {
                let var_name = &result[start + 1..start + 1 + end];
                if let Ok(value) = std::env::var(var_name) {
                    result = format!("{}{}{}", &result[..start], value, &result[start + 2 + end..]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        result
    }
}
