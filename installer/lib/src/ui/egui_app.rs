// egui 实现的安装程序界面 - 完全基于 XML 布局

use eframe::egui;
use crate::ui::wizard::{Wizard, WizardMode};
use crate::ui::message_box::MessageBoxManager;
use crate::ui::layout_renderer::{LayoutRenderer, RenderResult};
use crate::ui::dpi_handler::DpiConfig;
use crate::config::InstallerConfig;
use crate::installer::state::InstallState;
use crate::layout::{LayoutTree, XmlParser};
use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;

/// 安装程序主应用
pub struct InstallerApp {
    /// 向导管理器
    wizard: Wizard,
    /// 配置
    config: InstallerConfig,
    /// 配置文件基础路径
    config_base_path: PathBuf,
    /// 安装状态
    install_state: Option<Arc<InstallState>>,
    /// 当前语言
    current_language: String,
    
    // 背景纹理
    bg_main_texture: Option<egui::TextureHandle>,
    bg_installing_texture: Option<egui::TextureHandle>,
    btn_close_texture: Option<egui::TextureHandle>,
    
    // XML 布局系统
    layout_renderer: Option<LayoutRenderer>,
    layout_cache: HashMap<String, LayoutTree>,
    
    // 消息框
    message_box_manager: MessageBoxManager,
    
    // 交互状态
    install_path: String,
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
    agree_to_terms: bool,
    install_progress: f32,
}

impl InstallerApp {
    pub fn new(config: InstallerConfig, config_base_path: PathBuf) -> Self {
        let install_path = config.install.default_path.clone();
        
        Self {
            wizard: Wizard::new(WizardMode::Install),
            config,
            config_base_path,
            install_state: Some(Arc::new(InstallState::new(install_path.clone()))),
            current_language: "zh-CN".to_string(),
            
            bg_main_texture: None,
            bg_installing_texture: None,
            btn_close_texture: None,
            
            layout_renderer: None,
            layout_cache: HashMap::new(),
            
            message_box_manager: MessageBoxManager::new(),
            
            install_path,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            agree_to_terms: false,
            install_progress: 0.0,
        }
    }
    
    /// 加载资源
    fn load_resources(&mut self, ctx: &egui::Context) {
        // 加载背景图片
        if self.bg_main_texture.is_none() {
            self.bg_main_texture = self.load_texture(ctx, "assets/bg_main.png", "bg_main");
        }
        if self.bg_installing_texture.is_none() {
            self.bg_installing_texture = self.load_texture(ctx, "assets/bg_installing.png", "bg_installing");
        }
        if self.btn_close_texture.is_none() {
            self.btn_close_texture = self.load_texture(ctx, "assets/btn_close.png", "btn_close");
        }
        
        // 初始化布局渲染器
        if self.layout_renderer.is_none() {
            let dpi_config = DpiConfig::new(&self.config);
            let i18n_strings = self.load_i18n_strings();
            self.layout_renderer = Some(LayoutRenderer::new(dpi_config, i18n_strings));
        }
    }
    
    /// 加载纹理
    fn load_texture(&self, ctx: &egui::Context, relative_path: &str, name: &str) -> Option<egui::TextureHandle> {
        let assets_dir = self.config_base_path.join(&self.config.resources.assets_dir);
        let image_path = assets_dir.join(relative_path);
        
        // 尝试加载 2x 版本（如果存在）
        let dpi_scale = ctx.pixels_per_point();
        if dpi_scale >= 1.5 {
            let path_2x = image_path.with_file_name(
                format!("{}@2x.{}", 
                    image_path.file_stem()?.to_str()?,
                    image_path.extension()?.to_str()?)
            );
            if path_2x.exists() {
                if let Ok(image) = image::open(&path_2x) {
                    let size = [image.width() as usize, image.height() as usize];
                    let rgba = image.to_rgba8();
                    let pixels = rgba.as_flat_samples();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    return Some(ctx.load_texture(format!("{}@2x", name), color_image, Default::default()));
                }
            }
        }
        
        // 加载 1x 版本
        if let Ok(image) = image::open(&image_path) {
            let size = [image.width() as usize, image.height() as usize];
            let rgba = image.to_rgba8();
            let pixels = rgba.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            return Some(ctx.load_texture(name, color_image, Default::default()));
        }
        
        None
    }
    
    /// 加载国际化字符串
    fn load_i18n_strings(&self) -> HashMap<String, String> {
        let mut strings = HashMap::new();
        
        // 从配置加载基本字符串
        strings.insert("app_name".to_string(), self.config.project.name.clone());
        strings.insert("version".to_string(), self.config.project.version.clone());
        strings.insert("install_path".to_string(), self.install_path.clone());
        
        // TODO: 从语言包文件加载
        
        strings
    }
    
    /// 获取页面布局（返回引用）
    fn get_page_layout(&self, page: &WizardPage) -> Option<&LayoutTree> {
        // 查找页面配置
        let page_config = self.config.wizard.pages.iter()
            .find(|p| p.id == format!("{:?}", page).to_lowercase())?;
        
        let layout_id = page_config.layout.clone();
        self.layout_cache.get(&layout_id)
    }
    
    /// 加载页面布局（如果需要）
    fn load_page_layout(&mut self, page: &WizardPage) -> Option<&LayoutTree> {
        // 查找页面配置
        let page_config = self.config.wizard.pages.iter()
            .find(|p| p.id == format!("{:?}", page).to_lowercase())?;
        
        let layout_id = page_config.layout.clone();
        
        // 如果已经缓存，直接返回
        if !self.layout_cache.contains_key(&layout_id) {
            // 加载并解析 XML 布局文件
            let layouts_dir = self.config_base_path.join(&self.config.resources.layouts_dir);
            let layout_file = layouts_dir.join(&layout_id);
            
            let mut parser = XmlParser::new();
            match parser.parse_file(&layout_file) {
                Ok(layout_tree) => {
                    self.layout_cache.insert(layout_id.clone(), layout_tree);
                }
                Err(e) => {
                    eprintln!("Failed to load layout {}: {}", layout_id, e);
                    return None;
                }
            }
        }
        
        self.layout_cache.get(&layout_id)
    }
    
    /// 渲染自定义标题栏
    fn render_titlebar(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let titlebar_height = 48.0;
        let close_btn_size = 32.0;
        let close_btn_x = 574.0 - 16.0 - close_btn_size;
        
        // 可拖动区域（整个标题栏，除了关闭按钮）
        let drag_rect = egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(close_btn_x - 16.0, titlebar_height),
        );
        
        let drag_response = ui.interact(drag_rect, ui.id().with("titlebar_drag"), egui::Sense::drag());
        if drag_response.dragged() {
            // 使用 StartDrag 让 egui 处理窗口拖动
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        
        // 语言选择器（左侧）
        let lang_x = 16.0;
        let lang_y = 16.0;
        let lang_width = 80.0;
        let lang_height = 24.0;
        
        let lang_rect = egui::Rect::from_min_size(
            egui::pos2(lang_x, lang_y),
            egui::vec2(lang_width, lang_height),
        );
        
        ui.allocate_ui_at_rect(lang_rect, |ui| {
            let mut current_lang = self.current_language.clone();
            egui::ComboBox::from_id_source("language_selector")
                .selected_text(&current_lang)
                .show_ui(ui, |ui| {
                    for locale in &self.config.localization.supported_locales {
                        ui.selectable_value(&mut current_lang, locale.clone(), locale);
                    }
                });
            self.current_language = current_lang;
        });
        
        // 关闭按钮（右上角）
        if let Some(texture) = &self.btn_close_texture {
            let close_btn_rect = egui::Rect::from_min_size(
                egui::pos2(close_btn_x, 16.0),
                egui::vec2(close_btn_size, close_btn_size),
            );
            
            let close_response = ui.allocate_rect(close_btn_rect, egui::Sense::click());
            
            if ui.is_rect_visible(close_btn_rect) {
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
        }
    }
    
    /// 处理布局渲染结果
    fn handle_render_result(&mut self, result: &RenderResult) {
        // 处理按钮点击
        for (button_id, clicked) in &result.button_clicks {
            if *clicked {
                match button_id.as_str() {
                    "next" => self.wizard.next(),
                    "back" => self.wizard.previous(),
                    "cancel" => {
                        // TODO: 显示确认对话框
                    }
                    "install" => {
                        if self.agree_to_terms {
                            self.wizard.set_page(WizardPage::Installing);
                            self.start_installation();
                        }
                    }
                    "finish" => {
                        // TODO: 启动应用并关闭安装器
                    }
                    _ => {}
                }
            }
        }
        
        // 处理复选框状态
        for checkbox_id in result.checkbox_changes.keys() {
            if let Some(checked) = result.get_checkbox_new_state(checkbox_id) {
                match checkbox_id.as_str() {
                    "agree_terms" => self.agree_to_terms = checked,
                    "desktop_shortcut" => self.create_desktop_shortcut = checked,
                    "start_menu_shortcut" => self.create_start_menu_shortcut = checked,
                    _ => {}
                }
            }
        }
        
        // 处理文本输入
        for (input_id, text) in &result.text_input_changes {
            match input_id.as_str() {
                "install_path" => self.install_path = text.clone(),
                _ => {}
            }
        }
    }
    
    /// 开始安装
    fn start_installation(&mut self) {
        self.install_progress = 0.0;
        
        // 在新线程中执行安装
        let install_path = self.install_path.clone();
        let install_state = self.install_state.clone();
        let config = self.config.clone();
        
        std::thread::spawn(move || {
            if let Err(e) = Self::run_installation(install_path, install_state, config) {
                tracing::error!("Installation failed: {}", e);
            }
        });
    }
    
    /// 执行安装（在后台线程）
    fn run_installation(
        install_path: String,
        install_state: Option<Arc<InstallState>>,
        config: InstallerConfig,
    ) -> anyhow::Result<()> {
        use crate::installer::{InstallEngine, tasks::*};
        use crate::resources::RuntimeResources;
        
        tracing::info!("Starting installation to: {}", install_path);
        
        // 创建安装引擎
        let mut engine = InstallEngine::new(install_path.clone());
        
        // 1. 添加文件解压任务
        if let Some(payload_data) = RuntimeResources::get_payload() {
            engine.add_task(Box::new(ExtractFilesTask {
                payload_data,
            }));
        }
        
        // 2. 添加卸载器复制任务
        if let Some(uninst_data) = RuntimeResources::get_uninstaller() {
            let uninst_name = config.output.uninstaller_name.clone();
            engine.add_task(Box::new(CopyUninstallerTask {
                uninstaller_data: uninst_data,
                uninstaller_name: uninst_name,
            }));
        }
        
        // 3. 添加快捷方式创建任务
        let exe_path = format!("{}\\{}", install_path, config.install.exe_name);
        engine.add_task(Box::new(CreateShortcutsTask {
            app_name: config.project.name.clone(),
            exe_path,
        }));
        
        // 4. 添加注册表写入任务
        let uninst_path = format!("{}\\{}", install_path, config.output.uninstaller_name);
        engine.add_task(Box::new(WriteRegistryTask {
            app_name: config.project.name.clone(),
            app_version: config.project.version.clone(),
            install_path: install_path.clone(),
            publisher: config.project.publisher.clone(),
            uninstaller_path: uninst_path,
        }));
        
        // 执行安装
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create runtime: {}", e))?;
        
        runtime.block_on(async {
            engine.install().await
                .map_err(|e| anyhow::anyhow!("Installation failed: {}", e))
        })?;
        
        tracing::info!("Installation completed successfully");
        Ok(())
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 加载资源
        self.load_resources(ctx);
        
        // 设置深色主题
        ctx.set_visuals(egui::Visuals::dark());
        
        // 渲染消息框
        // 注意：egui_app.rs 是旧实现，现在使用 egui_app_xml.rs
        // 如果需要消息框功能，需要传递 DpiConfig 和 ResourceCache
        // let _message_results = self.message_box_manager.render(ctx, &dpi_config, &mut resource_cache);
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
                
                // 渲染标题栏（除了 Installing 页面）
                if self.wizard.current_page() != WizardPage::Installing {
                    self.render_titlebar(ui, frame);
                }
                
                // 渲染当前页面的 XML 布局
                let current_page = self.wizard.current_page();
                // 获取页面配置中的布局 ID
                let layout_id_opt = self.config.wizard.pages.iter()
                    .find(|p| p.id == format!("{:?}", current_page).to_lowercase())
                    .map(|p| p.layout.clone());
                
                if let Some(layout_id) = layout_id_opt {
                    // 确保布局已加载
                    self.load_page_layout(&current_page);
                    
                    // 现在分别借用不同的字段
                    let layout_tree_opt = self.layout_cache.get(&layout_id);
                    if let Some(layout_tree) = layout_tree_opt {
                        let render_result = if let Some(ref mut renderer) = self.layout_renderer {
                            renderer.render(ui, layout_tree)
                        } else {
                            RenderResult::new()
                        };
                        self.handle_render_result(&render_result);
                    } else {
                        // 如果布局加载失败，显示错误信息
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Failed to load layout: {}", layout_id))
                                    .size(16.0)
                                    .color(egui::Color32::RED)
                            );
                        });
                    }
                } else {
                    // 如果找不到页面配置，显示错误信息
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Page config not found: {:?}", current_page))
                                .size(16.0)
                                .color(egui::Color32::RED)
                        );
                    });
                }
            });
    }
}
