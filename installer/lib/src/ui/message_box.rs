// 消息框系统

use eframe::egui;
use std::collections::HashMap;

/// 消息框类型
#[derive(Debug, Clone, PartialEq)]
pub enum MessageBoxType {
    Info,
    Warning,
    Error,
    Question,
}

/// 消息框按钮
#[derive(Debug, Clone, PartialEq)]
pub enum MessageBoxButton {
    Ok,
    Cancel,
    Yes,
    No,
    YesNo,
    OkCancel,
    YesNoCancel,
}

/// 消息框结果
#[derive(Debug, Clone, PartialEq)]
pub enum MessageBoxResult {
    Ok,
    Cancel,
    Yes,
    No,
}

/// 消息框配置
#[derive(Debug, Clone)]
pub struct MessageBoxConfig {
    pub title: String,
    pub message: String,
    pub message_type: MessageBoxType,
    pub buttons: MessageBoxButton,
    pub width: Option<f32>,
    pub height: Option<f32>,
    /// 是否使用 2x DPI（用于缩放字体、按钮、圆角等）
    pub use_2x: bool,
}

impl MessageBoxConfig {
    pub fn new(title: String, message: String) -> Self {
        Self {
            title,
            message,
            message_type: MessageBoxType::Info,
            buttons: MessageBoxButton::Ok,
            width: None,
            height: None,
            use_2x: false,
        }
    }

    pub fn error(title: String, message: String) -> Self {
        Self {
            title,
            message,
            message_type: MessageBoxType::Error,
            buttons: MessageBoxButton::Ok,
            width: None,
            height: None,
            use_2x: false,
        }
    }

    pub fn warning(title: String, message: String) -> Self {
        Self {
            title,
            message,
            message_type: MessageBoxType::Warning,
            buttons: MessageBoxButton::Ok,
            width: None,
            height: None,
            use_2x: false,
        }
    }

    pub fn question(title: String, message: String) -> Self {
        Self {
            title,
            message,
            message_type: MessageBoxType::Question,
            buttons: MessageBoxButton::YesNo,
            width: None,
            height: None,
            use_2x: false,
        }
    }

    pub fn with_buttons(mut self, buttons: MessageBoxButton) -> Self {
        self.buttons = buttons;
        self
    }

    pub fn with_type(mut self, message_type: MessageBoxType) -> Self {
        self.message_type = message_type;
        self
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn with_dpi(mut self, use_2x: bool) -> Self {
        self.use_2x = use_2x;
        self
    }
}

/// 消息框管理器
pub struct MessageBoxManager {
    active_dialogs: HashMap<String, MessageBoxConfig>,
    next_id: u32,
}

impl MessageBoxManager {
    pub fn new() -> Self {
        Self {
            active_dialogs: HashMap::new(),
            next_id: 0,
        }
    }

    /// 显示消息框
    pub fn show(&mut self, config: MessageBoxConfig) -> String {
        let id = format!("message_box_{}", self.next_id);
        self.next_id += 1;
        self.active_dialogs.insert(id.clone(), config);
        id
    }

    /// 显示错误消息框
    pub fn show_error(&mut self, title: &str, message: &str) -> String {
        self.show(MessageBoxConfig::error(title.to_string(), message.to_string()))
    }

    /// 显示警告消息框
    pub fn show_warning(&mut self, title: &str, message: &str) -> String {
        self.show(MessageBoxConfig::warning(title.to_string(), message.to_string()))
    }

    /// 显示问题消息框
    pub fn show_question(&mut self, title: &str, message: &str) -> String {
        self.show(MessageBoxConfig::question(title.to_string(), message.to_string()))
    }

    /// 渲染所有活跃的消息框
    pub fn render(&mut self, ctx: &egui::Context, dpi_config: &crate::ui::dpi_handler::DpiConfig, resource_cache: &mut crate::ui::dpi_handler::ResourceCache) -> HashMap<String, MessageBoxResult> {
        let mut results = HashMap::new();
        let mut to_remove = Vec::new();

        for (id, config) in &self.active_dialogs {
            if let Some(result) = self.render_dialog(ctx, id, config, dpi_config, resource_cache) {
                results.insert(id.clone(), result);
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.active_dialogs.remove(&id);
        }

        results
    }

    fn render_dialog(&self, ctx: &egui::Context, id: &str, config: &MessageBoxConfig, dpi_config: &crate::ui::dpi_handler::DpiConfig, resource_cache: &mut crate::ui::dpi_handler::ResourceCache) -> Option<MessageBoxResult> {
        let mut result = None;
        
        // 根据 DPI 缩放所有尺寸（匹配 NSIS msgbox 布局）
        let scale = if config.use_2x { 2.0 } else { 1.0 };
        let rounding = 16.0 * scale;  // 1x: 16, 2x: 32（匹配 NSIS borderround）
        let stroke_width = if config.use_2x { 2.0 } else { 1.0 };  // 1x: 1, 2x: 2（匹配 NSIS bordersize）
        let message_font_size = if config.use_2x { 32.0 } else { 16.0 };  // 1x: 16, 2x: 32（匹配 NSIS font id="1"）
        let button_font_size = if config.use_2x { 28.0 } else { 14.0 };  // 1x: 14, 2x: 28（匹配 NSIS font id="0"）
        let button_width = if config.use_2x { 320.0 } else { 160.0 };  // 1x: 160, 2x: 320（匹配 NSIS）
        let button_height = if config.use_2x { 80.0 } else { 40.0 };  // 1x: 40, 2x: 80（匹配 NSIS）
        let button_rounding = if config.use_2x { 24.0 } else { 12.0 };  // 1x: 12, 2x: 24（匹配 NSIS borderround）
        let top_spacing = if config.use_2x { 140.0 } else { 70.0 };  // 1x: 70, 2x: 140（匹配 NSIS Container height）
        let message_height = if config.use_2x { 48.0 } else { 24.0 };  // 1x: 24, 2x: 48（匹配 NSIS HorizontalLayout height）
        let middle_spacing = if config.use_2x { 128.0 } else { 64.0 };  // 1x: 64, 2x: 128（匹配 NSIS Container height）
        let button_spacing = if config.use_2x { 144.0 } else { 72.0 };  // 1x: 72, 2x: 144（匹配 NSIS centerPadding width）
        let side_margin = if config.use_2x { 64.0 } else { 32.0 };  // 1x: 32, 2x: 64（匹配 NSIS Container width）
        
        let width = config.width.unwrap_or(400.0);
        let height = config.height.unwrap_or(200.0);
        
        // 使用 Area 替代 Window 以实现无边框对话框
        // 注意：使用 viewport_rect 的中心点来居中对话框
        let viewport_rect = ctx.viewport_rect();
        let center = viewport_rect.center();
        // 对话框位置：中心点减去对话框宽度和高度的一半
        let dialog_pos = egui::pos2(center.x - width / 2.0, center.y - height / 2.0);
        
        // 将 rounding 转换为 CornerRadius（需要 u8，但我们可以使用 f32 并转换为合适的值）
        let corner_radius = egui::CornerRadius::same(rounding as u8);
        
        egui::Area::new(egui::Id::new(id))
            .fixed_pos(dialog_pos)
            .movable(false)
            .show(ctx, |ui| {
                // 设置 Area 的大小约束
                ui.set_min_size(egui::vec2(width, height));
                
                // 绘制对话框背景（带圆角）
                // 使用 ui.max_rect() 确保背景覆盖整个对话框区域
                let dialog_rect = ui.max_rect();
                ui.painter().rect_filled(dialog_rect, corner_radius, egui::Color32::from_rgb(42, 56, 68));
                ui.painter().rect_stroke(
                    dialog_rect, 
                    corner_radius, 
                    egui::Stroke::new(stroke_width, egui::Color32::from_rgb(71, 75, 89)),
                    egui::epaint::StrokeKind::Outside
                );
                
                // 创建内容区域（匹配 NSIS 布局结构）
                ui.allocate_ui(egui::vec2(width, height), |ui| {
                    ui.vertical(|ui| {
                        // 顶部空白（匹配 NSIS Container height="140"/"70"）
                        ui.add_space(top_spacing);
                        
                        // 消息文本区域（匹配 NSIS HorizontalLayout height="48"/"24"）
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, message_height),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                // 消息文本（白色，居中，粗体，匹配 NSIS font id="1"）
                                ui.label(
                                    egui::RichText::new(&config.message)
                                        .color(egui::Color32::WHITE)
                                        .size(message_font_size)
                                        .strong()  // 粗体（匹配 NSIS bold="true"）
                                );
                            }
                        );
                        
                        // 中间空白（匹配 NSIS Container height="128"/"64"）
                        ui.add_space(middle_spacing);
                        
                        // 按钮区域（匹配 NSIS HorizontalLayout height="80"/"40"）
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, button_height),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                // 左侧边距（匹配 NSIS Container width="64"/"32"）
                                ui.add_space(side_margin);
                                
                                // 按钮
                                ui.horizontal(|ui| {
                        match config.buttons {
                            MessageBoxButton::Ok => {
                                if ui.button("确定").clicked() {
                                    result = Some(MessageBoxResult::Ok);
                                }
                            }
                            MessageBoxButton::Cancel => {
                                if ui.button("取消").clicked() {
                                    result = Some(MessageBoxResult::Cancel);
                                }
                            }
                            MessageBoxButton::Yes => {
                                if ui.button("是").clicked() {
                                    result = Some(MessageBoxResult::Yes);
                                }
                            }
                            MessageBoxButton::No => {
                                if ui.button("否").clicked() {
                                    result = Some(MessageBoxResult::No);
                                }
                            }
                            MessageBoxButton::YesNo => {
                                if ui.button("是").clicked() {
                                    result = Some(MessageBoxResult::Yes);
                                }
                                ui.add_space(10.0);
                                if ui.button("否").clicked() {
                                    result = Some(MessageBoxResult::No);
                                }
                            }
                            MessageBoxButton::OkCancel => {
                                // 取消按钮（使用 btn_dialog 图片，次要按钮，在左侧）
                                // 匹配 NSIS btnCancel: width="320"/"160", height="80"/"40", borderround="24,24"/"12,12"
                                let cancel_button_path = if config.use_2x { "assets/btn_dialog@2x.png" } else { "assets/btn_dialog.png" };
                                let cancel_button_texture = resource_cache.get_background(ctx, dpi_config, cancel_button_path);
                                
                                // 分配按钮空间
                                let (cancel_button_rect, cancel_response) = ui.allocate_exact_size(
                                    egui::vec2(button_width, button_height),
                                    egui::Sense::click()
                                );
                                
                                // 绘制取消按钮背景图片
                                if let Some(texture) = cancel_button_texture {
                                    ui.painter().image(
                                        texture.id(),
                                        cancel_button_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                } else {
                                    // 回退：使用纯色背景
                                    ui.painter().rect_filled(
                                        cancel_button_rect,
                                        egui::CornerRadius::same(button_rounding as u8),
                                        egui::Color32::from_rgb(42, 56, 68)
                                    );
                                }
                                
                                // 绘制取消按钮文本
                                ui.painter().text(
                                    cancel_button_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "取消",
                                    egui::FontId::proportional(button_font_size),
                                    egui::Color32::WHITE,
                                );
                                
                                if cancel_response.clicked() {
                                    result = Some(MessageBoxResult::Cancel);
                                }
                                
                                // 按钮间距（匹配 NSIS centerPadding width="144"/"72"）
                                ui.add_space(button_spacing);
                                
                                // 确定按钮（使用 btn_dialog_primary 图片，主要按钮，在右侧）
                                // 匹配 NSIS btnOK: width="320"/"160", height="80"/"40", borderround="24,24"/"12,12"
                                let confirm_button_path = if config.use_2x { "assets/btn_dialog_primary@2x.png" } else { "assets/btn_dialog_primary.png" };
                                let confirm_button_texture = resource_cache.get_background(ctx, dpi_config, confirm_button_path);
                                
                                // 分配按钮空间
                                let (confirm_button_rect, confirm_response) = ui.allocate_exact_size(
                                    egui::vec2(button_width, button_height),
                                    egui::Sense::click()
                                );
                                
                                // 绘制确定按钮背景图片
                                if let Some(texture) = confirm_button_texture {
                                    ui.painter().image(
                                        texture.id(),
                                        confirm_button_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                } else {
                                    // 回退：使用纯色背景
                                    ui.painter().rect_filled(
                                        confirm_button_rect,
                                        egui::CornerRadius::same(button_rounding as u8),
                                        egui::Color32::from_rgb(0, 196, 178)
                                    );
                                }
                                
                                // 绘制确定按钮文本
                                ui.painter().text(
                                    confirm_button_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "确定",
                                    egui::FontId::proportional(button_font_size),
                                    egui::Color32::WHITE,
                                );
                                
                                if confirm_response.clicked() {
                                    result = Some(MessageBoxResult::Ok);
                                }
                            }
                            MessageBoxButton::YesNoCancel => {
                                if ui.button("是").clicked() {
                                    result = Some(MessageBoxResult::Yes);
                                }
                                ui.add_space(10.0);
                                if ui.button("否").clicked() {
                                    result = Some(MessageBoxResult::No);
                                }
                                ui.add_space(10.0);
                                if ui.button("取消").clicked() {
                                    result = Some(MessageBoxResult::Cancel);
                                }
                            }
                        }
                                });
                                
                                // 右侧边距（匹配 NSIS Container width="64"/"32"）
                                ui.add_space(side_margin);
                            }
                        );
                    });
                });
            });

        result
    }

    /// 检查是否有活跃的消息框
    pub fn has_active_dialogs(&self) -> bool {
        !self.active_dialogs.is_empty()
    }

    /// 关闭所有消息框
    pub fn close_all(&mut self) {
        self.active_dialogs.clear();
    }
}

impl Default for MessageBoxManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数
pub fn show_error(_ctx: &egui::Context, title: &str, message: &str) -> MessageBoxConfig {
    MessageBoxConfig::error(title.to_string(), message.to_string())
}

pub fn show_warning(_ctx: &egui::Context, title: &str, message: &str) -> MessageBoxConfig {
    MessageBoxConfig::warning(title.to_string(), message.to_string())
}

pub fn show_question(_ctx: &egui::Context, title: &str, message: &str) -> MessageBoxConfig {
    MessageBoxConfig::question(title.to_string(), message.to_string())
}
