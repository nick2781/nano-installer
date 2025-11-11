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
    
    // XML 布局系统
    layout_renderer: Option<LayoutRenderer>,
    layout_cache: HashMap<WizardPage, LayoutTree>,
    layout_parser: XmlParser,
    
    // DPI 配置
    dpi_config: DpiConfig,
}

impl InstallerApp {
    pub fn new(config: InstallerConfig, config_base_path: PathBuf) -> Self {
        let install_path = config.install.default_path.clone();
        let dpi_config = DpiConfig::new(&config);
        
        // 加载国际化字符串
        let i18n_strings = Self::load_i18n_strings(&config, &config_base_path);
        
        // 创建布局渲染器
        let style_engine = StyleEngine::new();
        let layout_renderer = LayoutRenderer::with_style_engine(
            dpi_config.clone(),
            style_engine,
            i18n_strings,
        );
        
        Self {
            wizard: Wizard::new(WizardMode::Install),
            config,
            config_base_path,
            install_state: Some(Arc::new(InstallState::new(install_path.clone()))),
            install_path,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            agree_to_terms: false,
            current_language: "zh-CN".to_string(),
            
            message_box_manager: MessageBoxManager::new(),
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
        
        // TODO: 从语言包文件加载更多字符串
        
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
        tracing::debug!("Loading layout: {}", layout_file);
        
        // 从嵌入资源加载
        use crate::resources::RuntimeResources;
        
        // 尝试加载（可能需要 @2x 版本）
        let dpi = crate::layout::xml_parser::XmlParser::detect_system_dpi();
        tracing::info!("System DPI detected: {}", dpi);
        let selected_file = if dpi >= 144.0 {
            let stem = std::path::Path::new(&layout_file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&layout_file);
            let ext = std::path::Path::new(&layout_file)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("xml");
            let file_2x = format!("{}@2x.{}", stem, ext);
            tracing::info!("High DPI detected, trying @2x version: {}", file_2x);
            file_2x
        } else {
            tracing::info!("Normal DPI, using standard version: {}", layout_file);
            layout_file.clone()
        };
        
        // 文件名已经包含完整路径，直接使用
        tracing::debug!("Trying to load: {} or {}", selected_file, layout_file);
        
        let layout_content = RuntimeResources::get_layout(&selected_file)
            .or_else(|e1| {
                tracing::debug!("Failed to load {}: {}", selected_file, e1);
                RuntimeResources::get_layout(&layout_file)
                    .map_err(|e2| {
                        tracing::error!("Failed to load {}: {}", layout_file, e2);
                        e2
                    })
            })
            .ok()?;
        
        tracing::debug!("Layout content loaded, length: {} bytes", layout_content.len());
        
        // 调试：输出前 100 字符和字节
        let preview = if layout_content.len() > 100 {
            &layout_content[..100]
        } else {
            &layout_content[..]
        };
        tracing::debug!("Layout content preview (first 100 chars): {}", preview);
        
        #[cfg(debug_assertions)]
        {
            eprintln!("Layout content (first 100 bytes as hex):");
            let bytes = layout_content.as_bytes();
            let hex_preview = &bytes[..bytes.len().min(100)];
            for (i, byte) in hex_preview.iter().enumerate() {
                if i % 16 == 0 {
                    eprint!("\n  {:04x}: ", i);
                }
                eprint!("{:02x} ", byte);
            }
            eprintln!("\n");
        }
        
        // 解析布局
        match self.layout_parser.parse_string(&layout_content) {
            Ok(layout_tree) => {
                tracing::info!("Layout parsed successfully: {:?}", page);
                #[cfg(debug_assertions)]
                eprintln!("✓ Layout parsed successfully: {:?}", page);
                self.layout_cache.insert(page, layout_tree);
                self.layout_cache.get(&page)
            }
            Err(e) => {
                tracing::error!("Failed to parse layout: {}", e);
                #[cfg(debug_assertions)]
                {
                    eprintln!("✗ XML Parse Error for {:?}:", page);
                    eprintln!("  Error: {}", e);
                    eprintln!("  Content length: {} bytes", layout_content.len());
                    eprintln!("  Bytes 75-95 (around position 84):");
                    if layout_content.len() >= 95 {
                        let bytes = layout_content.as_bytes();
                        eprintln!("    Hex: {:02x?}", &bytes[75..95]);
                        eprintln!("    Text: {:?}", &layout_content[75..95]);
                    }
                    eprintln!("  Content preview (first 300 chars):");
                    let preview_len = layout_content.len().min(300);
                    eprintln!("  {}", &layout_content[..preview_len]);
                }
                None
            }
        }
    }
    
    /// 查找页面对应的布局文件
    fn find_layout_file_for_page(&self, page: WizardPage) -> Option<String> {
        tracing::debug!("Looking for layout for page: {:?}", page);
        tracing::debug!("Config has {} pages", self.config.wizard.pages.len());
        
        // 从配置中查找页面配置
        for page_config in &self.config.wizard.pages {
            let page_id = &page_config.id;
            tracing::debug!("Checking page_id: {}", page_id);
            
            let expected_page = match page_id.as_str() {
                "config" | "welcome" => WizardPage::Welcome,  // config 对应 NSIS 的 configpage
                "license" => WizardPage::License,
                "install_path" => WizardPage::InstallPath,
                "installing" => WizardPage::Installing,
                "finish" => WizardPage::Finish,
                "language" => WizardPage::Language,
                _ => {
                    tracing::debug!("Unknown page_id: {}", page_id);
                    continue;
                }
            };
            
            if expected_page == page {
                tracing::debug!("Found layout: {}", page_config.layout);
                return Some(page_config.layout.clone());
            }
        }
        
        tracing::error!("No layout found for page: {:?}", page);
        None
    }
    
    /// 处理布局渲染结果
    fn handle_layout_result(&mut self, result: crate::ui::layout_renderer::RenderResult) {
        // 处理按钮点击
        for (button_id, clicked) in &result.button_clicks {
            if *clicked {
                self.handle_button_click(button_id);
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
            "cancel" | "close" => {
                // 处理取消/关闭
                tracing::info!("User cancelled installation");
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
        // 禁用 egui 的自动 DPI 缩放，我们手动处理
        ctx.set_pixels_per_point(1.0);

        // 根据系统 DPI 决定使用哪套资源
        let system_dpi = ctx.native_pixels_per_point().unwrap_or(1.0);
        self.dpi_config.scale_factor = system_dpi;
        self.dpi_config.use_2x = system_dpi >= 1.5;

        tracing::debug!("System DPI: {}, use_2x: {}", system_dpi, self.dpi_config.use_2x);

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

        // 渲染消息框
        let _message_results = self.message_box_manager.render(ctx);

        // 主面板 - 完全由 XML 布局控制
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let current_page = self.wizard.current_page();
                
                // 确保布局已加载
                self.get_page_layout(current_page);
                
                // 渲染 XML 布局
                // 动态更新安装按钮的 enabled 状态（根据协议复选框）
                if current_page == WizardPage::Welcome {
                    if let Some(layout) = self.layout_cache.get_mut(&current_page) {
                        Self::update_button_enabled_recursive(&mut layout.root, "install", self.agree_to_terms);
                    }
                }

                let layout_opt = self.layout_cache.get(&current_page);
                if let Some(layout) = layout_opt {
                    if let Some(ref mut renderer) = self.layout_renderer {
                        let render_result = renderer.render(ui, layout);
                        self.handle_layout_result(render_result);
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
