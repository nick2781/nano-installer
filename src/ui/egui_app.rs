// egui 实现的安装程序界面

use eframe::egui;
use crate::ui::wizard::{Wizard, WizardPage};
use crate::common::config::InstallerConfig;
use crate::installer::InstallState;
use std::sync::Arc;

// 图片按钮组件
struct ImageButton {
    normal_texture: Option<egui::TextureHandle>,
    hover_texture: Option<egui::TextureHandle>,
    disabled_texture: Option<egui::TextureHandle>,
    size: egui::Vec2,
}

impl ImageButton {
    fn new(ctx: &egui::Context, base_path: &str, size: egui::Vec2) -> Self {
        let normal_path = format!("{}.png", base_path);
        let hover_path = format!("{}.png", base_path.replace("primary", "hover"));
        let disabled_path = format!("{}.png", base_path.replace("primary", "disabled"));
        
        Self {
            normal_texture: Self::load_embedded_texture(ctx, &normal_path),
            hover_texture: Self::load_embedded_texture(ctx, &hover_path),
            disabled_texture: Self::load_embedded_texture(ctx, &disabled_path),
            size,
        }
    }
    
    fn load_embedded_texture(ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
        // 获取DPI缩放比例
        let pixels_per_point = ctx.pixels_per_point();
        let should_use_2x = pixels_per_point >= 1.5;
        
        // 获取内嵌的图片数据
        let image_data: &[u8] = match path {
            "assets/btn_primary.png" => {
                if should_use_2x {
                    include_bytes!("../../assets/btn_primary@2x.png") as &[u8]
                } else {
                    include_bytes!("../../assets/btn_primary.png") as &[u8]
                }
            }
            "assets/btn_hover.png" => {
                if should_use_2x {
                    include_bytes!("../../assets/btn_hover@2x.png") as &[u8]
                } else {
                    include_bytes!("../../assets/btn_hover.png") as &[u8]
                }
            }
            "assets/btn_disabled.png" => {
                if should_use_2x {
                    include_bytes!("../../assets/btn_disabled@2x.png") as &[u8]
                } else {
                    include_bytes!("../../assets/btn_disabled.png") as &[u8]
                }
            }
            _ => return None,
        };
        
        if let Ok(image) = image::load_from_memory(image_data) {
            let size = [image.width() as _, image.height() as _];
            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                size,
                pixels.as_slice(),
            );
            return Some(ctx.load_texture(path, color_image, Default::default()));
        }
        None
    }
    
    fn show(&self, ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
        let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(self.size, sense);
        
        if ui.is_rect_visible(rect) {
            let texture = if !enabled {
                self.disabled_texture
                    .as_ref()
                    .or(self.normal_texture.as_ref())
            } else if response.hovered() {
                self.hover_texture
                    .as_ref()
                    .or(self.normal_texture.as_ref())
            } else {
                self.normal_texture.as_ref()
            };
            
            if let Some(texture) = texture {
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            
            // 绘制文字
            if !text.is_empty() {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(14.0),
                    if enabled { egui::Color32::WHITE } else { egui::Color32::from_rgb(180, 180, 180) },
                );
            }
        }
        
        response
    }
}

// 复选框组件
struct ImageCheckbox {
    unchecked_texture: Option<egui::TextureHandle>,
    checked_texture: Option<egui::TextureHandle>,
    size: egui::Vec2,
}

impl ImageCheckbox {
    fn new(ctx: &egui::Context) -> Self {
        Self {
            unchecked_texture: Self::load_embedded_texture(ctx, "assets/checkbox-0.png"),
            checked_texture: Self::load_embedded_texture(ctx, "assets/checkbox-2.png"),
            size: egui::vec2(16.0, 16.0),
        }
    }
    
    fn load_embedded_texture(ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
        // 获取DPI缩放比例
        let pixels_per_point = ctx.pixels_per_point();
        let should_use_2x = pixels_per_point >= 1.5;
        
        // 获取内嵌的图片数据
        let image_data: &[u8] = match path {
            "assets/checkbox-0.png" => {
                if should_use_2x {
                    include_bytes!("../../assets/checkbox-0@2x.png") as &[u8]
                } else {
                    include_bytes!("../../assets/checkbox-0.png") as &[u8]
                }
            }
            "assets/checkbox-2.png" => {
                if should_use_2x {
                    include_bytes!("../../assets/checkbox-2@2x.png") as &[u8]
                } else {
                    include_bytes!("../../assets/checkbox-2.png") as &[u8]
                }
            }
            _ => return None,
        };
        
        if let Ok(image) = image::load_from_memory(image_data) {
            let size = [image.width() as _, image.height() as _];
            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                size,
                pixels.as_slice(),
            );
            return Some(ctx.load_texture(path, color_image, Default::default()));
        }
        None
    }
    
    fn show(&self, ui: &mut egui::Ui, checked: &mut bool, text: &str) -> egui::Response {
        self.show_with_size(ui, checked, text, 12.0)
    }
    
    fn show_with_size(&self, ui: &mut egui::Ui, checked: &mut bool, text: &str, font_size: f32) -> egui::Response {
        let spacing = 4.0;  // 减小间距
        let text_galley = ui.fonts(|f| f.layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(font_size),
            egui::Color32::from_rgb(255, 255, 255),  // 纯白色
        ));
        let text_width = text_galley.rect.width();
        
        let total_size = egui::vec2(self.size.x + spacing + text_width, self.size.y.max(20.0));
        let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());
        
        if response.clicked() {
            *checked = !*checked;
        }
        
        if ui.is_rect_visible(rect) {
            let checkbox_rect = egui::Rect::from_min_size(
                rect.min,
                self.size,
            );
            
            let texture = if *checked {
                self.checked_texture.as_ref()
            } else {
                self.unchecked_texture.as_ref()
            };
            
            if let Some(texture) = texture {
                ui.painter().image(
                    texture.id(),
                    checkbox_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            
            // 绘制文字 - 纯白色
            let text_pos = egui::pos2(
                checkbox_rect.max.x + spacing,
                rect.center().y,
            );
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(255, 255, 255),  // 纯白色
            );
        }
        
        response
    }
}

pub struct InstallerApp {
    wizard: Wizard,
    config: InstallerConfig,
    install_state: Option<Arc<InstallState>>,
    install_path: String,
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
    agree_to_terms: bool,
    show_more: bool,
    current_language: String,
    
    // 纹理资源
    bg_main_texture: Option<egui::TextureHandle>,
    bg_installing_texture: Option<egui::TextureHandle>,
    bg_color_texture: Option<egui::TextureHandle>,
    logo_texture: Option<egui::TextureHandle>,
    btn_close_texture: Option<egui::TextureHandle>,
    progress_bar_texture: Option<egui::TextureHandle>,
    arrow_down_texture: Option<egui::TextureHandle>,
    arrow_up_texture: Option<egui::TextureHandle>,
    
    // UI组件
    btn_install: Option<ImageButton>,
    btn_run: Option<ImageButton>,
    checkbox: Option<ImageCheckbox>,
}

impl InstallerApp {
    pub fn new(config: InstallerConfig) -> Self {
        let install_path = config.default_install_path.clone();
        
        Self {
            wizard: Wizard::new(),
            config,
            install_state: Some(Arc::new(InstallState::new(install_path.clone()))),
            install_path,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            agree_to_terms: false,
            show_more: false,
            current_language: "zh-CN".to_string(),
            
            bg_main_texture: None,
            bg_installing_texture: None,
            bg_color_texture: None,
            logo_texture: None,
            btn_close_texture: None,
            progress_bar_texture: None,
            arrow_down_texture: None,
            arrow_up_texture: None,
            
            btn_install: None,
            btn_run: None,
            checkbox: None,
        }
    }
    
    /// 加载所有图片资源
    fn load_resources(&mut self, ctx: &egui::Context) {
        if self.bg_main_texture.is_none() {
            self.bg_main_texture = Self::load_texture(ctx, "assets/bg_main.png", "bg_main");
            if self.bg_main_texture.is_none() {
                eprintln!("警告: 无法加载 assets/bg_main.png");
            }
            
            self.bg_installing_texture = Self::load_texture(ctx, "assets/bg_installing.png", "bg_installing");
            if self.bg_installing_texture.is_none() {
                eprintln!("警告: 无法加载 assets/bg_installing.png");
            }
            
            self.logo_texture = Self::load_texture(ctx, "assets/logo.png", "logo");
            if self.logo_texture.is_none() {
                eprintln!("警告: 无法加载 assets/logo.png");
            }
            
            self.btn_close_texture = Self::load_texture(ctx, "assets/btn_close.png", "btn_close");
            self.progress_bar_texture = Self::load_texture(ctx, "assets/bar_installing.png", "progress_bar");
            self.bg_color_texture = Self::load_texture(ctx, "assets/bg_color.png", "bg_color");
            self.arrow_down_texture = Self::load_texture(ctx, "assets/arrow-down.png", "arrow_down");
            self.arrow_up_texture = Self::load_texture(ctx, "assets/arrow-up.png", "arrow_up");
            
            self.btn_install = Some(ImageButton::new(ctx, "assets/btn_primary", egui::vec2(240.0, 40.0)));
            self.btn_run = Some(ImageButton::new(ctx, "assets/btn_primary", egui::vec2(240.0, 40.0)));
            self.checkbox = Some(ImageCheckbox::new(ctx));
            
            eprintln!("资源加载完成");
            eprintln!("bg_main_texture: {}", self.bg_main_texture.is_some());
            eprintln!("logo_texture: {}", self.logo_texture.is_some());
        }
    }
    
    fn load_texture(ctx: &egui::Context, path: &str, name: &str) -> Option<egui::TextureHandle> {
        // 获取DPI缩放比例
        let pixels_per_point = ctx.pixels_per_point();
        
        // 根据DPI选择合适的图片
        let should_use_2x = pixels_per_point >= 1.5;
        
        // 从内嵌资源加载图片
        let image_data = Self::get_embedded_image(path, should_use_2x)?;
        
        if let Ok(image) = image::load_from_memory(image_data) {
            let size = [image.width() as _, image.height() as _];
            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                size,
                pixels.as_slice(),
            );
            eprintln!("成功加载内嵌图片: {} (DPI: {:.2})", name, pixels_per_point);
            return Some(ctx.load_texture(name, color_image, Default::default()));
        }
        
        eprintln!("无法加载内嵌图片: {} (DPI: {:.2})", path, pixels_per_point);
        None
    }
    
    /// 获取语言显示名称
    fn get_language_display_name(&self, locale: &str) -> String {
        match locale {
            "zh-CN" => "中文".to_string(),
            "zh-TW" => "繁體中文".to_string(),
            "en-US" => "English".to_string(),
            "ja" => "日本語".to_string(),
            "ko" => "한국어".to_string(),
            "vi" => "Tiếng Việt".to_string(),
            "th" => "ไทย".to_string(),
            "id" => "Bahasa Indonesia".to_string(),
            "es" => "Español".to_string(),
            "pt" => "Português".to_string(),
            "ru" => "Русский".to_string(),
            _ => locale.to_string(),
        }
    }
    
    /// 获取内嵌的图片资源
    fn get_embedded_image(path: &str, prefer_2x: bool) -> Option<&'static [u8]> {
        // 根据路径返回对应的内嵌资源
        match path {
            "assets/bg_main.png" => {
                if prefer_2x {
                    Some(include_bytes!("../../assets/bg_main@2x.png"))
                } else {
                    Some(include_bytes!("../../assets/bg_main.png"))
                }
            }
            "assets/bg_installing.png" => {
                if prefer_2x {
                    Some(include_bytes!("../../assets/bg_installing@2x.png"))
                } else {
                    Some(include_bytes!("../../assets/bg_installing.png"))
                }
            }
            "assets/logo.png" => {
                if prefer_2x {
                    Some(include_bytes!("../../assets/logo@2x.png"))
                } else {
                    Some(include_bytes!("../../assets/logo.png"))
                }
            }
            "assets/btn_close.png" => {
                if prefer_2x {
                    Some(include_bytes!("../../assets/btn_close@2x.png"))
                } else {
                    Some(include_bytes!("../../assets/btn_close.png"))
                }
            }
            "assets/bar_installing.png" => {
                if prefer_2x {
                    Some(include_bytes!("../../assets/bar_installing@2x.png"))
                } else {
                    Some(include_bytes!("../../assets/bar_installing.png"))
                }
            }
            "assets/arrow-down.png" => {
                if prefer_2x {
                    Some(include_bytes!("../../assets/arrow-down@2x.png"))
                } else {
                    Some(include_bytes!("../../assets/arrow-down.png"))
                }
            }
            "assets/arrow-up.png" => {
                if prefer_2x {
                    Some(include_bytes!("../../assets/arrow-up@2x.png"))
                } else {
                    Some(include_bytes!("../../assets/arrow-up.png"))
                }
            }
            "assets/bg_color.png" => {
                // 只有普通版本，没有@2x版本
                Some(include_bytes!("../../assets/bg_color.png"))
            }
            _ => None,
        }
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 加载所有资源
        self.load_resources(ctx);
        
        // 禁用默认样式，使用完全自定义的绘制
        ctx.set_visuals(egui::Visuals::dark());

        // 主面板 - 使用NSIS尺寸 574x358，完全自定义绘制
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 27, 34)))
            .show(ctx, |ui| {
                // 选择背景图片
                let bg_texture = match self.wizard.current_page() {
                    WizardPage::Installing => self.bg_installing_texture.as_ref(),
                    _ => self.bg_main_texture.as_ref(),
                };
                
                // 绘制背景图片（填充整个窗口）
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
                } else {
                    // 如果背景图加载失败，使用纯色背景
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(0.0, 0.0),
                        egui::vec2(574.0, 358.0),
                    );
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 27, 34));
                }
                
                // 自定义标题栏（可拖动）+ 关闭按钮
                self.render_custom_titlebar(ui, frame);
                
                // 主内容区域
                self.render_current_page(ui);
            });
    }
}

impl InstallerApp {
    /// 渲染自定义标题栏（可拖动 + 关闭按钮 + 语言选择）
    fn render_custom_titlebar(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let title_bar_height = 64.0;
        let title_bar_rect = egui::Rect::from_min_size(
            ui.min_rect().min,
            egui::vec2(ui.available_width(), title_bar_height),
        );
        
        // 标题栏区域（用于拖动）
        let title_bar_response = ui.allocate_rect(title_bar_rect, egui::Sense::click_and_drag());
        
        // 处理窗口拖动
        if title_bar_response.dragged() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        
        // 定义X按钮和语言框的共同参数 - 使用中心点对齐
        let close_btn_x = 574.0 - 16.0 - 32.0; // 右边距16px
        let close_btn_size = 32.0;
        let close_btn_center_y = 16.0 + close_btn_size / 2.0; // 中心Y坐标 = 16 + 16 = 32
        
        // 关闭按钮（右上角，绝对定位）- NSIS: inset="0,16,16,0"
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
        
        // 语言选择下拉框（X按钮左侧，间距4px，垂直居中对齐）
        let lang_width = 120.0;
        let lang_height = 32.0;
        let lang_x = close_btn_x - 4.0 - lang_width; // X按钮左侧，间距4px
        let lang_y = close_btn_center_y - lang_height / 2.0; // 使用相同的中心点
        
        // 使用allocate_ui_at_rect并移除所有内部spacing
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(
                egui::pos2(lang_x, lang_y),
                egui::vec2(lang_width, lang_height),
            ),
            |ui| {
                // 移除所有spacing和padding
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.spacing_mut().button_padding = egui::vec2(4.0, 0.0);
                
                // 垂直居中布局
                ui.centered_and_justified(|ui| {
                    // 设置ComboBox样式
                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 26);
                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 40);
                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 50);
                    ui.style_mut().visuals.window_fill = egui::Color32::from_rgba_premultiplied(24, 27, 34, 240);
                    ui.style_mut().visuals.popup_shadow = egui::epaint::Shadow::NONE;
                    ui.style_mut().visuals.widgets.inactive.fg_stroke.color = egui::Color32::WHITE;
                    ui.style_mut().visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
                    ui.style_mut().visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
                    
                    let current_display = self.get_language_display_name(&self.current_language);
                    egui::ComboBox::from_id_source("language_select")
                        .selected_text(egui::RichText::new(current_display).size(12.0).color(egui::Color32::WHITE))
                        .width(lang_width - 8.0)
                        .show_ui(ui, |ui| {
                            for locale in &self.config.supported_locales {
                                let display_name = self.get_language_display_name(locale);
                                ui.selectable_value(&mut self.current_language, locale.clone(), display_name);
                            }
                        });
                });
            },
        );
    }
    
    fn render_header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(&self.config.app_name);
            ui.label(format!("v{}", self.config.app_version));
        });
        ui.separator();
    }
    
    fn render_footer(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        
        ui.horizontal(|ui| {
            // 后退按钮
            if self.wizard.can_go_back() {
                if ui.button("← 上一步").clicked() {
                    self.wizard.previous();
                }
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 取消按钮
                if ui.button("取消").clicked() {
                    std::process::exit(0);
                }
                
                // 下一步/完成按钮
                if self.wizard.current_page() == WizardPage::Finish {
                    if ui.button("完成").clicked() {
                        std::process::exit(0);
                    }
                } else if self.can_proceed() {
                    if ui.button("下一步 →").clicked() {
                        self.wizard.next();
                    }
                }
            });
        });
    }
    
    fn can_proceed(&self) -> bool {
        match self.wizard.current_page() {
            WizardPage::License => self.agree_to_terms,
            WizardPage::Installing | WizardPage::Finish => false,
            _ => true,
        }
    }
    
    fn render_current_page(&mut self, ui: &mut egui::Ui) {
        match self.wizard.current_page() {
            WizardPage::Welcome => self.render_welcome_page(ui),
            WizardPage::Language => self.render_language_page(ui),
            WizardPage::License => self.render_license_page(ui),
            WizardPage::InstallPath => self.render_install_path_page(ui),
            WizardPage::Installing => self.render_installing_page(ui),
            WizardPage::Finish => self.render_finish_page(ui),
        }
    }
    
    fn render_welcome_page(&mut self, ui: &mut egui::Ui) {
        // 根据NSIS布局：
        // - Logo在顶部居中，距离顶部96px，高度80px
        // - 安装按钮在Logo下方55px，居中
        // - 底部87px高度区域包含复选框和更多选项按钮
        
        // Logo
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(213.0, 96.0), egui::vec2(148.0, 80.0)),
            |ui| {
                if let Some(texture) = &self.logo_texture {
                    let rect = ui.max_rect();
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            },
        );
        
        // 安装按钮（NSIS: padding="167,55,167,0" width="240" height="40"）
        // 位置：Logo底部(176) + 上边距(55) = 231
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(167.0, 231.0), egui::vec2(240.0, 40.0)),
            |ui| {
                if let Some(btn) = &self.btn_install {
                    if btn.show(ui, "立即安装", self.agree_to_terms).clicked() && self.agree_to_terms {
                        // 直接跳到安装进度页
                        self.wizard.set_page(WizardPage::Installing);
                    }
                }
            },
        );
        
        // 底部区域：复选框和更多选项（NSIS: HorizontalLayout height="87" inset="40,35,40,35"）
        // 位置：按钮底部(271)开始，高度87，内容从271+35=306开始
        let bottom_area_y = 231.0 + 40.0; // 按钮底部 = 271
        let bottom_area_inset_top = 35.0;
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(
                egui::pos2(40.0, bottom_area_y + bottom_area_inset_top), 
                egui::vec2(494.0, 87.0 - bottom_area_inset_top)
            ),
            |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0; // 移除默认间距
                    
                    // 同意协议复选框 - 使用图片（NSIS: width="110"）
                    if let Some(checkbox) = &self.checkbox {
                        checkbox.show(ui, &mut self.agree_to_terms, "我已阅读并同意");
                    }
                    
                    // 服务协议链接（NSIS: width="72"），紧贴复选框
                    if ui.link(egui::RichText::new("《服务协议》").size(12.0).color(egui::Color32::WHITE)).clicked() {
                        // TODO: 打开服务协议
                    }
                    
                    // "&" 文字（NSIS: width="10"），紧贴前面的元素
                    ui.label(egui::RichText::new("&").size(12.0).color(egui::Color32::from_rgb(228, 232, 236)));
                    
                    // 隐私政策链接（NSIS: width="72"），紧贴前面的元素
                    if ui.link(egui::RichText::new("《隐私政策》").size(12.0).color(egui::Color32::WHITE)).clicked() {
                        // TODO: 打开隐私政策
                    }
                    
                    // 右侧：自定义选项按钮（NSIS: width="80", with arrow image dest='68,2,80,15'）
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 文字始终显示"自定义安装"，只有箭头根据状态变化
                        let current_show_more = self.show_more;
                        let btn_text = "自定义安装"; // 始终显示"自定义安装"
                        let arrow_texture = if current_show_more {
                            &self.arrow_up_texture
                        } else {
                            &self.arrow_down_texture
                        };
                        
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(80.0, 20.0), egui::Sense::click());
                        
                        if ui.is_rect_visible(rect) {
                            // 绘制文字（NSIS: textpadding="0,0,16,0"）
                            ui.painter().text(
                                egui::pos2(rect.left(), rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                btn_text,
                                egui::FontId::proportional(12.0),
                                egui::Color32::WHITE,
                            );
                            
                            // 绘制箭头图标（NSIS: dest='68,2,80,15' or '68,3,80,16'）
                            if let Some(texture) = arrow_texture {
                                let arrow_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.left() + 68.0, rect.top() + 2.0),
                                    egui::vec2(12.0, 13.0),
                                );
                                ui.painter().image(
                                    texture.id(),
                                    arrow_rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                            }
                        }
                        
                        // 最后处理点击（在渲染之后改变状态）
                        if response.clicked() {
                            self.show_more = !self.show_more;
                            // 根据展开状态调整窗口大小（NSIS: 358 -> 518）
                            let new_height = if self.show_more { 518.0 } else { 358.0 };
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(574.0, new_height)));
                        }
                    });
                });
            },
        );
        
        // 更多选项区域（展开时显示）
        if self.show_more {
            self.render_more_config(ui);
        }
    }
    
    fn render_more_config(&mut self, ui: &mut egui::Ui) {
        // 更多配置区域（NSIS: pos="0,358,574,518" visible="false" float="true"）
        let start_y = 358.0;
        let expanded_height = 160.0; // 518 - 358 = 160
        
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(0.0, start_y), egui::vec2(574.0, expanded_height)),
            |ui| {
                let rect = ui.max_rect();
                
                // 背景（NSIS: bkcolor="#FF181B22" bkimage="assets/bg_color.png"）
                // 先绘制纯色背景
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 27, 34));
                
                // 然后绘制背景图片（如果加载成功）
                if let Some(texture) = &self.bg_color_texture {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                
                // 顶部分隔线（#00C4B2 · 20%）
                // #00C4B2 = rgb(0, 196, 178)，20% 透明度 = rgba(0, 196, 178, 51)
                let separator_rect = egui::Rect::from_min_size(
                    egui::pos2(16.0, start_y),
                    egui::vec2(574.0 - 32.0, 1.0),
                );
                ui.painter().rect_filled(
                    separator_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 196, 178, 51), // #00C4B2 · 20%
                );
                
                // 顶部空白（NSIS: Control height="24"）
                ui.add_space(25.0);
                
                // 安装路径编辑框和按钮（NSIS: HorizontalLayout height="32" inset="40,0,40,0"）
                // 布局：40px(margin) + 410px(path) + 弹性空间 + 60px(button) + 40px(margin) = 574px
                let row_y = start_y + 25.0; // 顶部24px空白 + 1px分隔线
                
                // 路径输入框（NSIS: RichEdit width="410" bkcolor="#1AFFFFFF"）
                let path_rect = egui::Rect::from_min_size(
                    egui::pos2(40.0, row_y),
                    egui::vec2(410.0, 32.0),
                );
                
                // 使用allocate_ui_at_rect来确保背景正确渲染
                ui.allocate_ui_at_rect(path_rect, |ui| {
                    // 绘制圆角背景（rgba(255, 255, 255, 0.10)）
                    // 0xFFFFFF 10% 透明度 = rgba(255, 255, 255, 0.10) = rgba(255, 255, 255, 26)
                    ui.painter().rect_filled(
                        path_rect,
                        8.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 26), // 10% = 25.5 ≈ 26
                    );
                    
                    // 显示路径文字（NSIS: inset="16,8,16,0" textcolor="#CCFFFFFF"）
                    let display_path = if self.install_path.is_empty() {
                        "C:\\Users\\Default\\AppData\\Local\\MyApp"
                    } else {
                        &self.install_path
                    };
                    
                    ui.painter().text(
                        egui::pos2(path_rect.left() + 16.0, path_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        display_path,
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 204), // #CCFFFFFF
                    );
                });
                
                // 更改位置按钮（NSIS: Button width="60" textcolor="#FF00C4B2" - 纯文字按钮）
                let btn_rect = egui::Rect::from_min_size(
                    egui::pos2(574.0 - 40.0 - 60.0, row_y), // 右对齐，右边距40px
                    egui::vec2(60.0, 32.0),
                );
                
                let btn_response = ui.allocate_rect(btn_rect, egui::Sense::click());
                
                // 绘制按钮文字（纯文字，无背景）
                let btn_color = if btn_response.hovered() {
                    egui::Color32::from_rgb(0, 255, 232) // hottextcolor="#FF00FFE8"
                } else {
                    egui::Color32::from_rgb(0, 196, 178) // textcolor="#FF00C4B2"
                };
                
                ui.painter().text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "更改位置",
                    egui::FontId::proportional(14.0), // font="2"
                    btn_color,
                );
                
                if btn_response.clicked() {
                    // TODO: 打开文件夹选择对话框
                }
                
                // 磁盘空间信息（NSIS: HorizontalLayout height="24" padding="40,5,0,0"）
                // lblRequiredSpace width="160" + local_space width="200"
                let space_y = row_y + 32.0 + 5.0;
                ui.painter().text(
                    egui::pos2(40.0, space_y + 12.0),
                    egui::Align2::LEFT_CENTER,
                    "所需空间: 100 MB",
                    egui::FontId::proportional(12.0), // font="0"
                    egui::Color32::from_rgb(150, 161, 169), // textcolor="#FF96A1A9"
                );
                
                ui.painter().text(
                    egui::pos2(40.0 + 160.0, space_y + 12.0),
                    egui::Align2::LEFT_CENTER,
                    "可用空间: 120.6 GB",
                    egui::FontId::proportional(12.0), // font="0"
                    egui::Color32::from_rgb(150, 161, 169), // textcolor="#FF96A1A9"
                );
                
                // 复选框选项（NSIS: HorizontalLayout height="20" padding="40,20,40,0"）
                // 注意：展开区域的复选框使用 font="2" (14px)
                let checkbox_y = space_y + 24.0 + 20.0;
                
                // 创建桌面快捷方式（NSIS: CheckBox width="150" font="2" size="14"）
                let checkbox_rect1 = egui::Rect::from_min_size(
                    egui::pos2(40.0, checkbox_y),
                    egui::vec2(150.0, 24.0),
                );
                ui.allocate_ui_at_rect(checkbox_rect1, |ui| {
                    if let Some(checkbox) = &self.checkbox {
                        checkbox.show_with_size(ui, &mut self.create_desktop_shortcut, "创建桌面快捷方式", 14.0);
                    }
                });
                
                // 开机启动（NSIS: CheckBox width="120" font="2" size="14"）
                let checkbox_rect2 = egui::Rect::from_min_size(
                    egui::pos2(40.0 + 150.0 + 24.0, checkbox_y), // 间距24px
                    egui::vec2(120.0, 24.0),
                );
                ui.allocate_ui_at_rect(checkbox_rect2, |ui| {
                    if let Some(checkbox) = &self.checkbox {
                        checkbox.show_with_size(ui, &mut self.create_start_menu_shortcut, "开机启动", 14.0);
                    }
                });
            },
        );
    }
    
    fn render_language_page(&self, ui: &mut egui::Ui) {
        ui.heading("选择语言");
        ui.add_space(10.0);
        
        ui.label("请选择安装程序使用的语言：");
        ui.add_space(10.0);
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            for locale in &self.config.supported_locales {
                if ui.selectable_label(false, locale).clicked() {
                    // TODO: 切换语言
                }
            }
        });
    }
    
    fn render_license_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("许可协议");
        ui.add_space(10.0);
        
        ui.label("请仔细阅读以下许可协议。如果您接受协议条款，请选中下面的复选框。");
        ui.add_space(10.0);
        
        // 许可文本框
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                ui.label("MIT License\n\nCopyright (c) 2024\n\n...");
            });
        
        ui.add_space(10.0);
        ui.checkbox(&mut self.agree_to_terms, "我接受许可协议的条款");
    }
    
    fn render_install_path_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("选择安装位置");
        ui.add_space(10.0);
        
        ui.label("安装程序将安装到以下文件夹：");
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.install_path);
            if ui.button("浏览...").clicked() {
                // TODO: 打开文件夹选择对话框
            }
        });
        
        ui.add_space(20.0);
        
        ui.label("选项：");
        ui.checkbox(&mut self.create_desktop_shortcut, "创建桌面快捷方式");
        ui.checkbox(&mut self.create_start_menu_shortcut, "创建开始菜单快捷方式");
    }
    
    fn render_installing_page(&self, ui: &mut egui::Ui) {
        // NSIS布局：进度条在距离顶部260px的位置，高度6px
        // 使用 bar_installing.png 作为进度条前景图
        
        // 进度条区域（NSIS: <Control height="260" /> 然后是进度条）
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(0.0, 260.0), egui::vec2(574.0, 6.0)),
            |ui| {
                if let Some(texture) = &self.progress_bar_texture {
                    let progress = 0.5; // TODO: 使用真实进度
                    let progress_width = 574.0 * progress;
                    let progress_rect = egui::Rect::from_min_size(
                        egui::pos2(0.0, 0.0),
                        egui::vec2(progress_width, 6.0),
                    );
                    
                    if ui.is_rect_visible(progress_rect) {
                        ui.painter().image(
                            texture.id(),
                            progress_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(progress, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            },
        );
        
        // 进度文字（NSIS: 进度条下方25px，居中，宽度200px）
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(187.0, 291.0), egui::vec2(200.0, 20.0)),
            |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new("正在安装 ...").size(14.0).color(egui::Color32::WHITE));
                });
            },
        );
    }
    
    fn render_finish_page(&mut self, ui: &mut egui::Ui) {
        // Logo
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(213.0, 80.0), egui::vec2(148.0, 80.0)),
            |ui| {
                if let Some(texture) = &self.logo_texture {
                    let rect = ui.max_rect();
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            },
        );
        
        // "安装完成"文字
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(187.0, 176.0), egui::vec2(200.0, 20.0)),
            |ui| {
                ui.painter().text(
                    egui::pos2(287.0, 186.0),
                    egui::Align2::CENTER_CENTER,
                    "安装完成",
                    egui::FontId::proportional(16.0),
                    egui::Color32::WHITE,
                );
            },
        );
        
        // 立即启动按钮
        ui.allocate_ui_at_rect(
            egui::Rect::from_min_size(egui::pos2(167.0, 212.0), egui::vec2(240.0, 40.0)),
            |ui| {
                if let Some(btn) = &self.btn_run {
                    if btn.show(ui, "立即启动", true).clicked() {
                        // TODO: 启动应用程序
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            },
        );
    }
}

