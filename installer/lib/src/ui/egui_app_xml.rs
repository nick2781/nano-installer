//! 基于 XML 布局的安装程序界面
//! 
//! 所有 UI 元素（背景、按钮、文本等）完全由 XML 布局文件定义
//! 代码只负责渲染 XML 和处理交互事件

use eframe::egui;
use crate::config::InstallerConfig;
use crate::installer::state::InstallState;
use crate::ui::wizard::{Wizard, WizardPage, WizardMode};
use crate::ui::message_box::MessageBoxManager;
use crate::ui::layout_renderer::LayoutRenderer;
use crate::ui::style_engine::StyleEngine;
use crate::ui::dpi_handler::DpiConfig;
use crate::layout::{LayoutTree, LayoutElement, XmlParser};
use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// 安装程序应用
pub struct InstallerApp {
    /// 向导状态
    wizard: Wizard,
    /// 配置
    config: InstallerConfig,
    /// 配置文件基础路径
    config_base_path: PathBuf,
    /// 安装状态
    install_state: Option<Arc<InstallState>>,
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
    
    // XML 布局系统
    layout_renderer: Option<LayoutRenderer>,
    layout_cache: HashMap<WizardPage, LayoutTree>,
    layout_parser: XmlParser,
    
    // DPI 配置
    dpi_config: DpiConfig,
}

impl InstallerApp {
    /// 创建 InstallerApp（接受已检测的 DpiConfig，避免重复检测导致不一致）
    pub fn new_with_dpi(config: InstallerConfig, config_base_path: PathBuf, dpi_config: DpiConfig) -> Self {
        let install_path = config.install.default_path.clone();
        eprintln!("[App] 开始创建 InstallerApp - 使用已检测的 DpiConfig");
        eprintln!("[App] DpiConfig: use_2x={}, 窗口: {}x{}", 
            dpi_config.use_2x, dpi_config.window_width, dpi_config.window_height);
        
        // 加载国际化字符串
        let i18n_strings = Self::load_i18n_strings(&config, &config_base_path);
        
        // 创建布局渲染器
        let style_engine = StyleEngine::new();
        let layout_renderer = LayoutRenderer::with_style_engine(
            dpi_config.clone(),
            style_engine,
            i18n_strings,
        );
        
        // 从配置读取初始页面（第一个页面），直接使用配置中的页面 ID
        let initial_page = config.wizard.pages.first()
            .and_then(|page_config| {
                match page_config.id.as_str() {
                    "config" => Some(WizardPage::Config),
                    "welcome" => Some(WizardPage::Welcome),
                    "language" => Some(WizardPage::Language),
                    "license" => Some(WizardPage::License),
                    "install_path" => Some(WizardPage::InstallPath),
                    "installing" => Some(WizardPage::Installing),
                    "finish" => Some(WizardPage::Finish),
                    _ => None,
                }
            })
            .unwrap_or(WizardPage::Config); // 默认使用 Config
        
        // 创建向导并设置初始页面
        let mut wizard = Wizard::new(WizardMode::Install);
        wizard.set_page(initial_page);
        
        Self {
            wizard,
            config,
            config_base_path,
            install_state: Some(Arc::new(InstallState::new(install_path.clone()))),
            install_path,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            agree_to_terms: false,
            current_language: "zh-CN".to_string(),
            
            message_box_manager: MessageBoxManager::new(),
            pending_close_confirm_id: None,
            layout_renderer: Some(layout_renderer),
            layout_cache: HashMap::new(),
            layout_parser: XmlParser::new(),
            dpi_config,
        }
    }
    
    /// 加载国际化字符串
    fn load_i18n_strings(config: &InstallerConfig, _base_path: &PathBuf) -> HashMap<String, String> {
        let mut strings = HashMap::new();
        
        // 从配置加载基本字符串
        strings.insert("app_name".to_string(), config.project.name.clone());
        strings.insert("version".to_string(), config.project.version.clone());
        
        // 从语言包文件加载字符串
        // 默认使用 zh-CN，可以从 config 中读取当前语言
        let locale = "zh-CN"; // TODO: 从 config 读取当前语言
        let locale_file = format!("locales/{}.json", locale);
        
        // 从嵌入资源加载语言包
        // 语言包被打包在 UIResources 段中（和布局文件一起）
        use crate::resources::RuntimeResources;
        
        // 使用与 get_layout 类似的逻辑，从 UIResources 段加载语言包 JSON 文件
        // 注意：这里不能直接调用 get_layout，因为语言包不是布局文件
        // 需要手动解压 UIResources 段并查找语言包文件
        if let Ok(locale_content) = RuntimeResources::get_layout(&locale_file) {
            // get_layout 会尝试从 UIResources 段加载，如果语言包也在那里，应该能加载到
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&locale_content) {
                if let Some(obj) = json_value.as_object() {
                    for (key, value) in obj {
                        if let Some(text) = value.as_str() {
                            strings.insert(key.clone(), text.to_string());
                        }
                    }
                }
            }
        }
        
        strings
    }
    
    /// 加载或获取页面布局
    fn get_page_layout(&mut self, page: WizardPage) -> Option<&LayoutTree> {
        // 如果已经缓存，直接返回
        if self.layout_cache.contains_key(&page) {
            return self.layout_cache.get(&page);
        }
        
        // 查找页面对应的布局文件
        let layout_file = self.find_layout_file_for_page(page)?;
        
        // 从嵌入资源加载
        use crate::resources::RuntimeResources;
        
        // 使用已创建的 DpiConfig 来判断是否使用 2x 布局（避免重复检测导致不一致）
        let selected_file = if self.dpi_config.use_2x {
            // 保留目录路径，只修改文件名
            let path = std::path::Path::new(&layout_file);
            let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");
            let stem = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_else(|| {
                    // 如果没有文件名，使用整个路径作为 stem
                    path.to_str().unwrap_or(&layout_file)
                });
            let ext = path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("xml");
            
            // 构建 2x 文件路径，保留目录结构
            let file_2x = if parent.is_empty() {
                format!("{}@2x.{}", stem, ext)
            } else {
                format!("{}/{}@2x.{}", parent, stem, ext)
            };
            eprintln!("[布局] 使用 2x 布局: {} -> {}", layout_file, file_2x);
            file_2x
        } else {
            eprintln!("[布局] 使用 1x 布局: {}", layout_file);
            layout_file.clone()
        };
        
        // 文件名已经包含完整路径，直接使用
        let layout_content = RuntimeResources::get_layout(&selected_file)
            .or_else(|_e1| {
                eprintln!("[布局] 2x 布局加载失败，回退到 1x: {} -> {}", selected_file, layout_file);
                RuntimeResources::get_layout(&layout_file)
                    .map_err(|e2| {
                        tracing::error!("Failed to load layout {}: {}", layout_file, e2);
                        e2
                    })
            })
            .ok()?;
        
        // 解析布局
        match self.layout_parser.parse_string(&layout_content) {
            Ok(layout_tree) => {
                // 检查布局中的页面尺寸
                let layout_width = layout_tree.root.attributes.width.unwrap_or(0.0);
                let layout_height = layout_tree.root.attributes.height.unwrap_or(0.0);
                
                // 只在首次加载时输出一次清晰的日志
                let page_name = match page {
                    WizardPage::Config => "Config (配置页)",
                    WizardPage::Welcome => "Welcome (欢迎页)",
                    WizardPage::Installing => "Installing (安装中)",
                    WizardPage::Finish => "Finish (完成页)",
                    _ => "其他页面",
                };
                eprintln!("[布局] ✓ 加载成功: {} -> {}", page_name, selected_file);
                eprintln!("[布局]   布局尺寸: {}x{} (窗口: {}x{})", 
                    layout_width, layout_height, 
                    self.dpi_config.window_width, self.dpi_config.window_height);
                
                // 检查尺寸是否匹配
                if layout_width > 0.0 && layout_height > 0.0 {
                    let width_match = (layout_width - self.dpi_config.window_width).abs() < 1.0;
                    let height_match = (layout_height - self.dpi_config.window_height).abs() < 1.0;
                    if !width_match || !height_match {
                        eprintln!("[布局] ⚠️  警告: 布局尺寸与窗口大小不匹配！");
                        eprintln!("[布局]   布局: {}x{}, 窗口: {}x{}, 差异: {}x{}",
                            layout_width, layout_height,
                            self.dpi_config.window_width, self.dpi_config.window_height,
                            (layout_width - self.dpi_config.window_width).abs(),
                            (layout_height - self.dpi_config.window_height).abs());
                    } else {
                        eprintln!("[布局] ✓ 布局尺寸与窗口大小匹配");
                    }
                }
                
                self.layout_cache.insert(page, layout_tree);
                self.layout_cache.get(&page)
            }
            Err(e) => {
                eprintln!("[布局] ✗ 解析失败: {} - {}", selected_file, e);
                None
            }
        }
    }
    
    /// 查找页面对应的布局文件
    fn find_layout_file_for_page(&self, page: WizardPage) -> Option<String> {
        // 直接根据页面枚举查找对应的配置，不使用字符串映射
        let page_id = match page {
            WizardPage::Config => "config",
            WizardPage::Welcome => "welcome",
            WizardPage::Language => "language",
            WizardPage::License => "license",
            WizardPage::InstallPath => "install_path",
            WizardPage::Installing => "installing",
            WizardPage::Finish => "finish",
            WizardPage::UninstallConfirm => "uninstall_confirm",
            WizardPage::UninstallProgress => "uninstall_progress",
            WizardPage::UninstallFinish => "uninstall_finish",
        };
        
        // 从配置中查找页面配置
        for page_config in &self.config.wizard.pages {
            if page_config.id == page_id {
                return Some(page_config.layout.clone());
            }
        }
        
        None
    }
    
    /// 处理布局渲染结果
    fn handle_layout_result(&mut self, ctx: &egui::Context, result: crate::ui::layout_renderer::RenderResult) {
        // 处理按钮点击
        for (button_id, clicked) in &result.button_clicks {
            if *clicked {
                // 特殊处理关闭按钮：显示确认对话框
                if button_id == "close" {
                    self.show_close_confirmation();
                } else {
                    self.handle_button_click(button_id);
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
                match link_id.as_str() {
                    "agreement" => {
                        tracing::info!("用户协议链接被点击");
                        self.open_url_in_browser(&self.config.links.terms_of_service);
                    }
                    "policy" => {
                        tracing::info!("隐私政策链接被点击");
                        self.open_url_in_browser(&self.config.links.privacy_policy);
                    }
                    _ => {
                        tracing::warn!("未知链接被点击: {}", link_id);
                    }
                }
            }
        }
    }
    
    /// 处理按钮点击
    fn handle_button_click(&mut self, button_id: &str) {
        tracing::info!("Button clicked: {}", button_id);
        
        match button_id {
            "next" | "install" => {
                self.wizard.next();
            }
            "back" => {
                self.wizard.previous();
            }
            "cancel" => {
                // 处理取消
                tracing::info!("User cancelled installation");
            }
            "close" => {
                // 关闭按钮已在 handle_layout_result 中处理
                tracing::info!("Close button clicked");
            }
            "finish" => {
                tracing::info!("Installation finished");
            }
            _ => {
                tracing::warn!("Unhandled button: {}", button_id);
            }
        }
    }
    
    /// 处理复选框变化
    fn handle_checkbox_change(&mut self, checkbox_id: &str, checked: bool) {
        match checkbox_id {
            "desktop_shortcut" => {
                self.create_desktop_shortcut = checked;
            }
            "start_menu_shortcut" => {
                self.create_start_menu_shortcut = checked;
            }
            "agree_terms" | "agree_license" => {
                self.agree_to_terms = checked;
                tracing::info!("User agreement checkbox: {}", checked);
            }
            _ => {
                tracing::warn!("Unhandled checkbox: {}", checkbox_id);
            }
        }
    }
    
    /// 处理文本输入
    fn handle_text_input(&mut self, input_id: &str, text: &str) {
        match input_id {
            "install_path" => {
                self.install_path = text.to_string();
            }
            _ => {
                tracing::warn!("Unhandled text input: {}", input_id);
            }
        }
    }

    /// 递归更新按钮的 enabled 状态
    fn update_button_enabled_recursive(element: &mut LayoutElement, button_id: &str, enabled: bool) {
        // 如果当前元素是目标按钮，更新 enabled 状态
        if let Some(id) = &element.attributes.id {
            if id == button_id {
                element.attributes.enabled = Some(enabled);
                return;
            }
        }

        // 递归检查子元素
        for child in &mut element.children {
            Self::update_button_enabled_recursive(child, button_id, enabled);
        }
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 注意：窗口圆角通过透明背景和背景图片的圆角边缘实现视觉效果
        // 系统级圆角需要在窗口创建时通过 DwmSetWindowAttribute 设置，这里暂时跳过
        
        // 禁用 egui 的自动 DPI 缩放，我们手动处理
        // 
        // 为什么设置为 1.0？
        // 1. 我们的布局系统使用绝对像素坐标（如 position="1052,32"），这些坐标是设计时的物理像素
        // 2. 我们通过加载不同的布局文件（1x 或 2x）来处理 DPI 缩放，而不是依赖 egui 的自动缩放
        // 3. 如果 pixels_per_point != 1.0，egui 会将所有坐标乘以这个值，导致布局错位
        // 4. 例如：如果系统 DPI 是 192（2x），egui 可能设置 pixels_per_point=2.0，这会导致：
        //    - 我们的 1148x716 窗口会被渲染为 2296x1432（物理像素），但逻辑尺寸变成 1144x358
        //    - 这会导致布局元素位置错误，因为 XML 中的坐标是基于逻辑像素的
        // 5. 通过设置为 1.0，我们确保 1 逻辑像素 = 1 物理像素，布局坐标直接对应屏幕坐标
        ctx.set_pixels_per_point(1.0);
        
        // 追踪 pixels_per_point 和窗口大小（只在首次或变化时输出）
        static LAST_PIXELS_PER_POINT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        static LAST_WINDOW_SIZE: std::sync::Mutex<Option<(f32, f32)>> = std::sync::Mutex::new(None);
        static FIRST_FRAME: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
        static SKIP_SIZE_CHECK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
        
        let current_ppp = ctx.pixels_per_point();
        let ppp_bits = (current_ppp.to_bits() as u64);
        let last_ppp_bits = LAST_PIXELS_PER_POINT.swap(ppp_bits, std::sync::atomic::Ordering::SeqCst);
        
        let viewport_rect = ctx.viewport_rect();
        let window_size = (viewport_rect.width(), viewport_rect.height());
        
        let mut last_size = LAST_WINDOW_SIZE.lock().unwrap();
        let size_changed = last_size.map_or(true, |(w, h)| (w - window_size.0).abs() > 0.1 || (h - window_size.1).abs() > 0.1);
        let is_first_frame = FIRST_FRAME.swap(false, std::sync::atomic::Ordering::SeqCst);
        
        // 跳过第一帧的窗口大小检查，因为 pixels_per_point 刚被设置为 1.0，viewport_rect 可能还没有更新
        let skip_check = SKIP_SIZE_CHECK.swap(false, std::sync::atomic::Ordering::SeqCst);
        
        // 当 pixels_per_point = 1.0 时，强制检查并重置窗口大小
        // 注意：当 pixels_per_point = 1.0 时，viewport_rect 返回的应该是逻辑像素大小
        // 我们的 dpi_config.window_width 也是逻辑像素大小（1148x716 或 574x358）
        if (current_ppp - 1.0).abs() < 0.01 {
            let size_diff = (
                (window_size.0 - self.dpi_config.window_width).abs(),
                (window_size.1 - self.dpi_config.window_height).abs()
            );
            
            // 如果窗口大小与期望大小不一致，强制重置
            // 注意：当 pixels_per_point 从 2.0 变为 1.0 时，窗口大小可能还是 2x（2294x1430）
            // 我们需要强制重置为逻辑像素大小（1148x716）
            if size_diff.0 > 1.0 || size_diff.1 > 1.0 {
                // 检查是否是 2x 的差异（可能是 pixels_per_point 调整导致的）
                let is_2x_diff = (window_size.0 / self.dpi_config.window_width - 2.0).abs() < 0.1 &&
                                 (window_size.1 / self.dpi_config.window_height - 2.0).abs() < 0.1;
                
                if is_2x_diff {
                    // 这是 pixels_per_point 调整导致的，强制重置
                    eprintln!("[渲染] ⚠️  窗口大小是期望的 2 倍: {}x{} -> 期望: {}x{}，强制重置", 
                        window_size.0, window_size.1,
                        self.dpi_config.window_width, self.dpi_config.window_height);
                } else if !skip_check {
                    // 其他情况下的差异，只在非第一帧时重置
                    eprintln!("[渲染] ⚠️  窗口大小被改变: {}x{} -> 期望: {}x{}，强制重置", 
                        window_size.0, window_size.1,
                        self.dpi_config.window_width, self.dpi_config.window_height);
                }
                
                // 强制设置窗口大小为期望大小
                if is_2x_diff || !skip_check {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        self.dpi_config.window_width,
                        self.dpi_config.window_height
                    )));
                }
            }
        }
        
        // 只在首次或真正变化时输出日志（减少不必要的日志）
        let ppp_changed = ppp_bits != last_ppp_bits && (current_ppp - 1.0).abs() > 0.01;
        let size_changed_significantly = size_changed && 
            ((window_size.0 - self.dpi_config.window_width).abs() > 1.0 || 
             (window_size.1 - self.dpi_config.window_height).abs() > 1.0);
        
        if is_first_frame || ppp_changed || size_changed_significantly {
            if is_first_frame {
                eprintln!("[渲染] 第一帧: pixels_per_point={}, 窗口: {}x{} (期望: {}x{})", 
                    current_ppp, window_size.0, window_size.1,
                    self.dpi_config.window_width, self.dpi_config.window_height);
            }
            
            if ppp_changed {
                eprintln!("[渲染] ⚠️  pixels_per_point 被改变: {} -> 1.0 (强制覆盖)", current_ppp);
            }
            
            if size_changed_significantly {
                eprintln!("[渲染] ⚠️  窗口大小被改变: {}x{} -> {}x{} (已重置)", 
                    window_size.0, window_size.1,
                    self.dpi_config.window_width, self.dpi_config.window_height);
            }
            
            *last_size = Some(window_size);
        }

        // 设置暗色主题
        ctx.set_visuals(egui::Visuals::dark());

        // 无边框窗口拖动区域（整个窗口上半部分可拖动）
        let title_bar_height = 200.0; // 可拖动区域高度
        let title_bar_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(ctx.viewport_rect().width(), title_bar_height),
        );

        let is_dragging = ctx.input(|i| {
            i.pointer.primary_down() && i.pointer.hover_pos().map_or(false, |pos| title_bar_rect.contains(pos))
        });

        if is_dragging {
            // 使用 egui 的拖动API
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // 渲染消息框并处理结果
        // 需要传递 DpiConfig 和 ResourceCache 给 MessageBoxManager
        let layout_renderer = self.layout_renderer.as_mut().unwrap();
        let message_results = self.message_box_manager.render(
            ctx, 
            &self.dpi_config,
            layout_renderer.get_resource_cache_mut()
        );
        for (id, result) in message_results {
            // 处理关闭确认对话框的结果
            if id.starts_with("message_box_") {
                if let Some(close_confirm_id) = self.pending_close_confirm_id.as_ref() {
                    if id == *close_confirm_id {
                        match result {
                            crate::ui::message_box::MessageBoxResult::Ok => {
                                // 用户确认关闭
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            _ => {
                                // 用户取消，不做任何操作
                            }
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
                let current_page = self.wizard.current_page();
                
                // 确保布局已加载
                self.get_page_layout(current_page);
                
                // 渲染 XML 布局
                // 动态更新安装按钮的 enabled 状态（根据协议复选框）
                if current_page == WizardPage::Config || current_page == WizardPage::Welcome {
                    if let Some(layout) = self.layout_cache.get_mut(&current_page) {
                        Self::update_button_enabled_recursive(&mut layout.root, "install", self.agree_to_terms);
                    }
                }

                let layout_opt = self.layout_cache.get(&current_page);
                if let Some(layout) = layout_opt {
                    if let Some(ref mut renderer) = self.layout_renderer {
                        let render_result = renderer.render(ui, layout);
                        self.handle_layout_result(ctx, render_result);
                    }
                } else {
                    // 布局加载失败，显示错误
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Failed to load layout: {:?}", current_page))
                                .size(16.0)
                                .color(egui::Color32::RED)
                        );
                    });
                }
            });
    }
}

impl InstallerApp {
    /// 显示关闭确认对话框
    fn show_close_confirmation(&mut self) {
        // 根据 DPI 配置动态调整对话框尺寸（匹配 NSIS msgbox 布局）
        // 1x: 400x230, 2x: 800x460（匹配 NSIS msgbox.xml 和 msgbox2x.xml）
        let dialog_width = if self.dpi_config.use_2x { 800.0 } else { 400.0 };
        let dialog_height = if self.dpi_config.use_2x { 460.0 } else { 230.0 };
        
        let dialog_id = self.message_box_manager.show(
            crate::ui::message_box::MessageBoxConfig::new(
                "".to_string(),  // 无标题
                "安装尚未完成,您确定要退出安装吗?".to_string()
            )
            .with_type(crate::ui::message_box::MessageBoxType::Question)
            // 注意：按钮类型是硬编码的，因为这是关闭确认对话框的标准行为
            // 如果需要支持其他按钮组合，可以通过参数传入
            .with_buttons(crate::ui::message_box::MessageBoxButton::OkCancel)
            .with_size(dialog_width, dialog_height)
            .with_dpi(self.dpi_config.use_2x)  // 传递 DPI 配置
        );
        self.pending_close_confirm_id = Some(dialog_id);
    }
    
    /// 在默认浏览器中打开 URL
    fn open_url_in_browser(&self, url: &str) {
        #[cfg(windows)]
        {
            // Windows: 使用 start 命令打开默认浏览器
            let _ = Command::new("cmd")
                .args(["/C", "start", "", url])
                .spawn();
        }
        
        #[cfg(not(windows))]
        {
            // Linux/macOS: 使用 xdg-open 或 open 命令
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("xdg-open").arg(url).spawn();
            }
            
            #[cfg(target_os = "macos")]
            {
                let _ = Command::new("open").arg(url).spawn();
            }
        }
        
        tracing::info!("尝试在默认浏览器中打开: {}", url);
    }
}
