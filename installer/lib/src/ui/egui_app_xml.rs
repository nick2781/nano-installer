//! 基于 XML 布局的安装程序界面

use eframe::egui;
use crate::config::InstallerConfig;
use crate::installer::state::InstallState;
use crate::ui::wizard::{Wizard, WizardPage, WizardMode};
use crate::ui::message_box::MessageBoxManager;
use crate::ui::layout_renderer::{LayoutRenderer, InteractionState};
use crate::ui::style_engine::StyleEngine;
use crate::ui::dpi_handler::DpiConfig;
use crate::layout::{LayoutTree, XmlParser};
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
    
    // 背景纹理
    bg_main_texture: Option<egui::TextureHandle>,
    bg_installing_texture: Option<egui::TextureHandle>,
    
    // 按钮纹理（用于标题栏）
    btn_close_texture: Option<egui::TextureHandle>,
    
    // 消息框管理器
    message_box_manager: MessageBoxManager,
    
    // XML 布局系统
    layout_renderer: Option<LayoutRenderer>,
    layout_cache: HashMap<WizardPage, LayoutTree>,
    layout_parser: XmlParser,
    
    // DPI 配置
    dpi_config: DpiConfig,
    
    // 交互状态
    interaction_state: InteractionState,
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
            
            bg_main_texture: None,
            bg_installing_texture: None,
            btn_close_texture: None,
            
            message_box_manager: MessageBoxManager::new(),
            layout_renderer: Some(layout_renderer),
            layout_cache: HashMap::new(),
            layout_parser: XmlParser::new(),
            dpi_config,
            interaction_state: InteractionState::default(),
        }
    }
    
    /// 加载国际化字符串
    fn load_i18n_strings(config: &InstallerConfig, _base_path: &PathBuf) -> HashMap<String, String> {
        let mut strings = HashMap::new();
        
        // 从配置加载基本信息
        strings.insert("app_name".to_string(), config.project.name.clone());
        strings.insert("version".to_string(), config.project.version.clone());
        strings.insert("publisher".to_string(), config.project.publisher.clone());
        
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
        let selected_file = if dpi >= 144.0 {
            let stem = std::path::Path::new(&layout_file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&layout_file);
            let ext = std::path::Path::new(&layout_file)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("xml");
            format!("{}@2x.{}", stem, ext)
        } else {
            layout_file.clone()
        };
        
        // 移除路径前缀 "layouts/"（如果有）
        let clean_selected = selected_file.strip_prefix("layouts/").unwrap_or(&selected_file);
        let clean_layout = layout_file.strip_prefix("layouts/").unwrap_or(&layout_file);
        
        let layout_content = RuntimeResources::get_layout(clean_selected)
            .or_else(|_| RuntimeResources::get_layout(clean_layout))
            .ok()?;
        
        // 解析布局
        match self.layout_parser.parse_string(&layout_content) {
            Ok(layout_tree) => {
                tracing::info!("Layout parsed successfully: {:?}", page);
                self.layout_cache.insert(page, layout_tree);
                self.layout_cache.get(&page)
            }
            Err(e) => {
                tracing::error!("Failed to parse layout: {}", e);
                None
            }
        }
    }
    
    /// 查找页面对应的布局文件
    fn find_layout_file_for_page(&self, page: WizardPage) -> Option<String> {
        // 从配置中查找页面配置
        for page_config in &self.config.wizard.pages {
            let page_id = &page_config.id;
            let expected_page = match page_id.as_str() {
                "welcome" => WizardPage::Welcome,
                "language" => WizardPage::Language,
                "license" => WizardPage::License,
                "install_path" | "config" => WizardPage::InstallPath, // 支持 config 别名
                "installing" => WizardPage::Installing,
                "finish" => WizardPage::Finish,
                _ => continue,
            };
            
            if expected_page == page {
                return Some(page_config.layout.clone());
            }
        }
        
        None
    }
    
    /// 加载背景纹理
    fn load_background_textures(&mut self, ctx: &egui::Context) {
        let assets_dir = &self.config.resources.assets_dir;
        
        if self.bg_main_texture.is_none() {
            let bg_path = format!("{}/bg_main.png", assets_dir);
            self.bg_main_texture = self.load_texture(ctx, &bg_path, "bg_main");
        }
        
        if self.bg_installing_texture.is_none() {
            let bg_path = format!("{}/bg_installing.png", assets_dir);
            self.bg_installing_texture = self.load_texture(ctx, &bg_path, "bg_installing");
        }
        
        if self.btn_close_texture.is_none() {
            let btn_path = format!("{}/btn_close.png", assets_dir);
            self.btn_close_texture = self.load_texture(ctx, &btn_path, "btn_close");
        }
    }
    
    /// 加载单个纹理
    fn load_texture(&self, ctx: &egui::Context, relative_path: &str, name: &str) -> Option<egui::TextureHandle> {
        use crate::resources::RuntimeResources;
        
        // 从嵌入资源加载
        // 提取文件名
        let file_name = std::path::Path::new(relative_path)
            .file_name()?
            .to_str()?;
        
        // 检查是否需要 2x 版本
        let asset_name = if self.dpi_config.scale_factor >= 1.5 {
            let stem = std::path::Path::new(file_name).file_stem()?.to_str()?;
            let ext = std::path::Path::new(file_name).extension()?.to_str()?;
            format!("{}@2x.{}", stem, ext)
        } else {
            file_name.to_string()
        };
        
        // 尝试加载
        let image_data = RuntimeResources::get_asset(&asset_name)
            .or_else(|_| RuntimeResources::get_asset(file_name))
            .ok()?;
        
        self.load_texture_from_bytes(ctx, &image_data, name)
    }
    
    /// 从字节数据加载纹理
    fn load_texture_from_bytes(&self, ctx: &egui::Context, data: &[u8], name: &str) -> Option<egui::TextureHandle> {
        match image::load_from_memory(data) {
            Ok(img) => self.load_texture_from_image(ctx, img, name),
            Err(e) => {
                tracing::warn!("Failed to load texture from bytes: {}", e);
                None
            }
        }
    }
    
    /// 从图片对象加载纹理
    fn load_texture_from_image(&self, ctx: &egui::Context, img: image::DynamicImage, name: &str) -> Option<egui::TextureHandle> {
        let rgba_image = img.to_rgba8();
        let size = [rgba_image.width() as usize, rgba_image.height() as usize];
        let pixels = rgba_image.into_raw();
        
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
        Some(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
    }
    
    /// 渲染自定义标题栏
    fn render_custom_titlebar(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let titlebar_height = 48.0;
        let close_btn_size = 32.0;
        let close_btn_x = 574.0 - 16.0 - close_btn_size;
        
        // 可拖动区域（除了关闭按钮）
        let drag_area = egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(close_btn_x, titlebar_height),
        );
        
        let drag_response = ui.interact(drag_area, ui.id().with("titlebar_drag"), egui::Sense::drag());
        if drag_response.dragged() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        
        // 关闭按钮
        let close_btn_rect = egui::Rect::from_min_size(
            egui::pos2(close_btn_x, 16.0),
            egui::vec2(close_btn_size, close_btn_size),
        );
        
        let close_response = ui.interact(close_btn_rect, ui.id().with("close_btn"), egui::Sense::click());
        
        if let Some(texture) = &self.btn_close_texture {
            ui.painter().image(
                texture.id(),
                close_btn_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        
        if close_response.clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        
        // 语言选择器（如果启用）
        if self.config.localization.show_language_selector {
            self.render_language_selector(ui);
        }
    }
    
    /// 渲染语言选择器
    fn render_language_selector(&mut self, ui: &mut egui::Ui) {
        let lang_x = 574.0 - 16.0 - 32.0 - 4.0 - 60.0;
        let lang_y = 20.0;
        
        let lang_rect = egui::Rect::from_min_size(
            egui::pos2(lang_x, lang_y),
            egui::vec2(60.0, 20.0),
        );
        
        let response = ui.interact(lang_rect, ui.id().with("lang_selector"), egui::Sense::click());
        
        // 显示当前语言
        let lang_display = self.get_language_display_name(&self.current_language);
        ui.painter().text(
            egui::pos2(lang_x + 30.0, lang_y + 10.0),
            egui::Align2::CENTER_CENTER,
            lang_display,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
        
        // 显示下拉菜单
        if response.clicked() {
            // TODO: 显示语言选择下拉菜单
        }
    }
    
    /// 获取语言显示名称
    fn get_language_display_name<'a>(&self, locale: &'a str) -> &'a str {
        match locale {
            "zh-CN" => "中文",
            "en-US" => "English",
            "ja" => "日本語",
            "ko" => "한국어",
            "th" => "ไทย",
            "vi" => "Tiếng Việt",
            "id" => "Bahasa Indonesia",
            "es" => "Español",
            "pt" => "Português",
            "ru" => "Русский",
            _ => locale,
        }
    }
    
    /// 处理布局渲染结果
    fn handle_layout_result(&mut self, result: crate::ui::layout_renderer::RenderResult) {
        // 处理按钮点击
        for (button_id, clicked) in result.button_clicks {
            if clicked {
                self.handle_button_click(&button_id);
            }
        }
        
        // 处理复选框状态
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
        match button_id {
            "next" => self.wizard.next(),
            "back" => self.wizard.previous(),
            "cancel" => {
                // TODO: 显示取消确认对话框
            }
            "install" => {
                if self.agree_to_terms {
                    self.wizard.set_page(WizardPage::Installing);
                    // TODO: 启动安装任务
                }
            }
            "finish" => {
                // TODO: 关闭程序
            }
            "browse" => {
                // TODO: 打开文件夹选择对话框
            }
            _ => {
                tracing::warn!("Unhandled button click: {}", button_id);
            }
        }
    }
    
    /// 处理复选框变化
    fn handle_checkbox_change(&mut self, checkbox_id: &str, checked: bool) {
        match checkbox_id {
            "agree_terms" => self.agree_to_terms = checked,
            "desktop_shortcut" => self.create_desktop_shortcut = checked,
            "start_menu_shortcut" => self.create_start_menu_shortcut = checked,
            _ => {
                tracing::warn!("Unhandled checkbox change: {}", checkbox_id);
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
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 强制设置为 1.0 缩放
        ctx.set_pixels_per_point(1.0);
        
        // 更新 DPI 配置
        self.dpi_config.scale_factor = 1.0;
        self.dpi_config.use_2x = false;
        
        // 加载资源
        self.load_background_textures(ctx);
        
        // 设置暗色主题
        ctx.set_visuals(egui::Visuals::dark());
        
        // 渲染消息框
        let _message_results = self.message_box_manager.render(ctx);
        // TODO: 处理消息框结果
        
        // 主面板
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 27, 34)))
            .show(ctx, |ui| {
                // 绘制背景
                let bg_texture = match self.wizard.current_page() {
                    WizardPage::Installing => self.bg_installing_texture.as_ref(),
                    _ => self.bg_main_texture.as_ref(),
                };
                
                if let Some(texture) = bg_texture {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(0.0, 0.0),
                        egui::vec2(574.0, 358.0),
                    );
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                
                // 渲染标题栏（除了安装进度页）
                if self.wizard.current_page() != WizardPage::Installing {
                    self.render_custom_titlebar(ui, frame);
                }
                
                // 渲染当前页面的 XML 布局
                let current_page = self.wizard.current_page();
                // 确保布局已加载
                self.get_page_layout(current_page);
                
                // 现在分别借用不同的字段
                let layout_opt = self.layout_cache.get(&current_page);
                if let Some(layout) = layout_opt {
                    let render_result = if let Some(ref mut renderer) = self.layout_renderer {
                        renderer.render(ui, layout)
                    } else {
                        crate::ui::layout_renderer::RenderResult::new()
                    };
                    self.handle_layout_result(render_result);
                } else {
                    // 如果布局加载失败，显示错误信息
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(format!("布局文件未找到: {:?}", current_page))
                                .size(16.0)
                                .color(egui::Color32::RED)
                        );
                    });
                }
            });
    }
}

