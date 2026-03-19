// 消息框系统 — 支持 XML 布局渲染和代码渲染两种模式

use crate::layout::{LayoutElement, LayoutTree};
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
    pub use_2x: bool,
    /// 按钮文本 (从 locale 读取)
    pub ok_text: String,
    pub cancel_text: String,
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
            ok_text: "OK".to_string(),
            cancel_text: "Cancel".to_string(),
        }
    }

    pub fn error(title: String, message: String) -> Self {
        Self {
            message_type: MessageBoxType::Error,
            ..Self::new(title, message)
        }
    }

    pub fn warning(title: String, message: String) -> Self {
        Self {
            message_type: MessageBoxType::Warning,
            ..Self::new(title, message)
        }
    }

    pub fn question(title: String, message: String) -> Self {
        Self {
            message_type: MessageBoxType::Question,
            buttons: MessageBoxButton::YesNo,
            ..Self::new(title, message)
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

    pub fn with_button_texts(mut self, ok: &str, cancel: &str) -> Self {
        self.ok_text = ok.to_string();
        self.cancel_text = cancel.to_string();
        self
    }
}

/// 消息框管理器
pub struct MessageBoxManager {
    active_dialogs: HashMap<String, MessageBoxConfig>,
    /// 每个对话框的 XML 布局（从 msgBox.xml 克隆并配置）
    dialog_layouts: HashMap<String, LayoutTree>,
    next_id: u32,
    /// 预加载的 msgBox.xml 模板
    msgbox_template: Option<LayoutTree>,
}

impl MessageBoxManager {
    fn compute_dialog_width(config: &MessageBoxConfig) -> f32 {
        config.width.unwrap_or(400.0)
    }

    pub fn new() -> Self {
        Self {
            active_dialogs: HashMap::new(),
            dialog_layouts: HashMap::new(),
            next_id: 0,
            msgbox_template: None,
        }
    }

    /// 设置 msgBox.xml 模板（由 InstallerApp 在初始化时调用）
    pub fn set_template(&mut self, template: LayoutTree) {
        self.msgbox_template = Some(template);
    }

    /// 强制使用代码回退渲染，避免已知的 XML 对话框偏移问题。
    pub fn use_code_fallback(&mut self) {
        self.msgbox_template = None;
        self.dialog_layouts.clear();
    }

    /// 显示消息框
    pub fn show(&mut self, config: MessageBoxConfig) -> String {
        let id = format!("message_box_{}", self.next_id);
        self.next_id += 1;

        // 如果有 XML 模板，克隆并配置布局
        if let Some(ref template) = self.msgbox_template {
            let mut layout = template.clone();
            // 设置消息文本
            Self::update_element_text(&mut layout.root, "lblMsg", &config.message);

            // 配置按钮可见性和文本
            match &config.buttons {
                MessageBoxButton::OkCancel => {
                    Self::update_element_visible(&mut layout.root, "btnCancel", true);
                    Self::update_element_visible(&mut layout.root, "centerPadding", false);
                    Self::update_element_text(&mut layout.root, "btnOK", &config.ok_text);
                    Self::update_element_text(&mut layout.root, "btnCancel", &config.cancel_text);
                }
                MessageBoxButton::Ok => {
                    Self::update_element_visible(&mut layout.root, "btnCancel", false);
                    Self::update_element_visible(&mut layout.root, "centerPadding", false);
                    Self::update_element_text(&mut layout.root, "btnOK", &config.ok_text);
                }
                MessageBoxButton::YesNo => {
                    Self::update_element_visible(&mut layout.root, "btnCancel", true);
                    Self::update_element_visible(&mut layout.root, "centerPadding", false);
                    Self::update_element_text(&mut layout.root, "btnOK", &config.ok_text);
                    Self::update_element_text(&mut layout.root, "btnCancel", &config.cancel_text);
                }
                _ => {
                    Self::update_element_text(&mut layout.root, "btnOK", &config.ok_text);
                }
            }

            self.dialog_layouts.insert(id.clone(), layout);
        }

        self.active_dialogs.insert(id.clone(), config);
        id
    }

    /// 显示错误消息框
    pub fn show_error(&mut self, title: &str, message: &str) -> String {
        self.show(MessageBoxConfig::error(
            title.to_string(),
            message.to_string(),
        ))
    }

    /// 显示警告消息框
    pub fn show_warning(&mut self, title: &str, message: &str) -> String {
        self.show(MessageBoxConfig::warning(
            title.to_string(),
            message.to_string(),
        ))
    }

    /// 显示问题消息框
    pub fn show_question(&mut self, title: &str, message: &str) -> String {
        self.show(MessageBoxConfig::question(
            title.to_string(),
            message.to_string(),
        ))
    }

    /// 渲染所有活跃的消息框
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        dpi_config: &crate::ui::dpi_handler::DpiConfig,
        resource_cache: &mut crate::ui::dpi_handler::ResourceCache,
    ) -> HashMap<String, MessageBoxResult> {
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
            self.dialog_layouts.remove(&id);
        }

        results
    }

    fn render_dialog(
        &self,
        ctx: &egui::Context,
        id: &str,
        config: &MessageBoxConfig,
        dpi_config: &crate::ui::dpi_handler::DpiConfig,
        resource_cache: &mut crate::ui::dpi_handler::ResourceCache,
    ) -> Option<MessageBoxResult> {
        // 尝试使用 XML 渲染
        if let Some(layout) = self.dialog_layouts.get(id) {
            return self.render_dialog_xml(ctx, id, config, layout, dpi_config, resource_cache);
        }

        // 回退到代码渲染
        self.render_dialog_code(ctx, id, config, dpi_config, resource_cache)
    }

    /// XML 布局渲染的对话框
    fn render_dialog_xml(
        &self,
        ctx: &egui::Context,
        id: &str,
        config: &MessageBoxConfig,
        layout: &LayoutTree,
        dpi_config: &crate::ui::dpi_handler::DpiConfig,
        _resource_cache: &mut crate::ui::dpi_handler::ResourceCache,
    ) -> Option<MessageBoxResult> {
        let mut result = None;

        let width = config
            .width
            .unwrap_or(layout.root.attributes.width.unwrap_or(400.0));
        let height = config
            .height
            .unwrap_or(layout.root.attributes.height.unwrap_or(200.0));

        // 基于主窗口基础尺寸居中
        let base_w = dpi_config.window_width;
        let base_h = dpi_config.window_height;
        let dialog_pos = egui::pos2((base_w - width) / 2.0, (base_h - height) / 2.0);

        // 创建临时 LayoutRenderer — 用对话框自身尺寸做 Taffy 布局
        let mut dialog_dpi = dpi_config.clone();
        dialog_dpi.window_width = width;
        dialog_dpi.window_height = height;
        let mut renderer = crate::ui::layout_renderer::LayoutRenderer::new(
            dialog_dpi,
            HashMap::new(), // 对话框不需要 i18n (文本已经替换)
        );

        egui::Area::new(egui::Id::new(id))
            .fixed_pos(dialog_pos)
            .movable(false)
            .show(ctx, |ui| {
                ui.set_min_size(egui::vec2(width, height));
                ui.set_max_size(egui::vec2(width, height));

                let render_result = renderer.render(ui, layout);

                // 处理按钮点击
                for (btn_id, clicked) in &render_result.button_clicks {
                    if *clicked {
                        let action = render_result
                            .button_actions
                            .get(btn_id)
                            .map(|s| s.as_str())
                            .unwrap_or(btn_id.as_str());
                        match action {
                            "dialog_ok" | "btnOK" => {
                                result = Some(MessageBoxResult::Ok);
                            }
                            "dialog_cancel" | "btnCancel" => {
                                result = Some(MessageBoxResult::Cancel);
                            }
                            _ => {}
                        }
                    }
                }
            });

        result
    }

    /// 代码渲染的对话框（回退方案）
    fn render_dialog_code(
        &self,
        ctx: &egui::Context,
        id: &str,
        config: &MessageBoxConfig,
        dpi_config: &crate::ui::dpi_handler::DpiConfig,
        resource_cache: &mut crate::ui::dpi_handler::ResourceCache,
    ) -> Option<MessageBoxResult> {
        let mut result = None;

        let rounding = 16.0;
        let stroke_width = 1.0;
        let message_font_size = 14.0;
        let button_font_size = 14.0;
        let button_width = 160.0;
        let button_height = 40.0;
        let top_spacing = 50.0;
        let middle_spacing = 40.0;
        let button_gap = 16.0;

        let width = Self::compute_dialog_width(config);
        let height = config.height.unwrap_or(200.0);
        let message_height =
            (height - top_spacing - middle_spacing - button_height - 24.0).max(48.0);

        let base_w = dpi_config.window_width;
        let base_h = dpi_config.window_height;
        let dialog_pos = egui::pos2((base_w - width) / 2.0, (base_h - height) / 2.0);

        let corner_radius = egui::CornerRadius::same(rounding as u8);

        egui::Area::new(egui::Id::new(id))
            .fixed_pos(dialog_pos)
            .movable(false)
            .show(ctx, |ui| {
                ui.set_min_size(egui::vec2(width, height));
                ui.set_max_size(egui::vec2(width, height));

                let dialog_rect =
                    egui::Rect::from_min_size(ui.min_rect().min, egui::vec2(width, height));
                ui.painter().rect_filled(
                    dialog_rect,
                    corner_radius,
                    egui::Color32::from_rgb(42, 56, 68),
                );
                ui.painter().rect_stroke(
                    dialog_rect,
                    corner_radius,
                    egui::Stroke::new(stroke_width, egui::Color32::from_rgb(71, 75, 89)),
                    egui::epaint::StrokeKind::Outside,
                );

                ui.allocate_ui(egui::vec2(width, height), |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(top_spacing);

                        let message_rect = egui::Rect::from_min_size(
                            egui::pos2(dialog_rect.min.x + 32.0, dialog_rect.min.y + top_spacing),
                            egui::vec2(dialog_rect.width() - 64.0, message_height),
                        );
                        let mut job = egui::text::LayoutJob::single_section(
                            config.message.clone(),
                            egui::TextFormat {
                                font_id: egui::FontId::proportional(message_font_size),
                                color: egui::Color32::WHITE,
                                ..Default::default()
                            },
                        );
                        job.halign = egui::Align::Center;
                        job.wrap = egui::text::TextWrapping {
                            max_width: message_rect.width(),
                            ..Default::default()
                        };
                        ui.scope_builder(egui::UiBuilder::new().max_rect(message_rect), |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.centered_and_justified(|ui| {
                                ui.add_sized(message_rect.size(), egui::Label::new(job).wrap());
                            });
                        });

                        ui.add_space(middle_spacing);

                        {
                            let total_buttons_width = button_width * 2.0 + button_gap;
                            let buttons_x = dialog_rect.min.x
                                + (dialog_rect.width() - total_buttons_width) / 2.0;
                            let buttons_y =
                                dialog_rect.min.y + top_spacing + message_height + middle_spacing;

                            let cancel_rect = egui::Rect::from_min_size(
                                egui::pos2(buttons_x, buttons_y),
                                egui::vec2(button_width, button_height),
                            );
                            let ok_rect = egui::Rect::from_min_size(
                                egui::pos2(buttons_x + button_width + button_gap, buttons_y),
                                egui::vec2(button_width, button_height),
                            );

                            match config.buttons {
                                MessageBoxButton::OkCancel => {
                                    let cancel_response = ui.interact(
                                        cancel_rect,
                                        egui::Id::new(format!("{}_cancel", id)),
                                        egui::Sense::click(),
                                    );
                                    if let Some(tex) = resource_cache.get_background(
                                        ctx,
                                        dpi_config,
                                        "assets/btn_dialog.png",
                                    ) {
                                        ui.painter().image(
                                            tex.id(),
                                            cancel_rect,
                                            egui::Rect::from_min_max(
                                                egui::pos2(0.0, 0.0),
                                                egui::pos2(1.0, 1.0),
                                            ),
                                            egui::Color32::WHITE,
                                        );
                                    }
                                    ui.painter().text(
                                        cancel_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &config.cancel_text,
                                        egui::FontId::proportional(button_font_size),
                                        egui::Color32::WHITE,
                                    );
                                    if cancel_response.clicked() {
                                        result = Some(MessageBoxResult::Cancel);
                                    }
                                    if cancel_response.hovered() {
                                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }

                                    let ok_response = ui.interact(
                                        ok_rect,
                                        egui::Id::new(format!("{}_ok", id)),
                                        egui::Sense::click(),
                                    );
                                    if let Some(tex) = resource_cache.get_background(
                                        ctx,
                                        dpi_config,
                                        "assets/btn_dialog_primary.png",
                                    ) {
                                        ui.painter().image(
                                            tex.id(),
                                            ok_rect,
                                            egui::Rect::from_min_max(
                                                egui::pos2(0.0, 0.0),
                                                egui::pos2(1.0, 1.0),
                                            ),
                                            egui::Color32::WHITE,
                                        );
                                    }
                                    ui.painter().text(
                                        ok_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &config.ok_text,
                                        egui::FontId::proportional(button_font_size),
                                        egui::Color32::WHITE,
                                    );
                                    if ok_response.clicked() {
                                        result = Some(MessageBoxResult::Ok);
                                    }
                                    if ok_response.hovered() {
                                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                }
                                MessageBoxButton::Ok => {
                                    let single_x = dialog_rect.min.x
                                        + (dialog_rect.width() - button_width) / 2.0;
                                    let single_rect = egui::Rect::from_min_size(
                                        egui::pos2(single_x, buttons_y),
                                        egui::vec2(button_width, button_height),
                                    );
                                    let resp = ui.interact(
                                        single_rect,
                                        egui::Id::new(format!("{}_ok", id)),
                                        egui::Sense::click(),
                                    );
                                    if let Some(tex) = resource_cache.get_background(
                                        ctx,
                                        dpi_config,
                                        "assets/btn_dialog_primary.png",
                                    ) {
                                        ui.painter().image(
                                            tex.id(),
                                            single_rect,
                                            egui::Rect::from_min_max(
                                                egui::pos2(0.0, 0.0),
                                                egui::pos2(1.0, 1.0),
                                            ),
                                            egui::Color32::WHITE,
                                        );
                                    }
                                    ui.painter().text(
                                        single_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &config.ok_text,
                                        egui::FontId::proportional(button_font_size),
                                        egui::Color32::WHITE,
                                    );
                                    if resp.clicked() {
                                        result = Some(MessageBoxResult::Ok);
                                    }
                                }
                                _ => {
                                    ui.horizontal(|ui| {
                                        if ui.button(&config.ok_text).clicked() {
                                            result = Some(MessageBoxResult::Ok);
                                        }
                                    });
                                }
                            }
                            ui.allocate_space(egui::vec2(width, button_height));
                        }
                    });
                });
            });

        result
    }

    /// 递归更新元素文本
    fn update_element_text(element: &mut LayoutElement, target_id: &str, text: &str) {
        if let Some(id) = &element.attributes.id {
            if id == target_id {
                element.attributes.text = Some(text.to_string());
                return;
            }
        }
        for child in &mut element.children {
            Self::update_element_text(child, target_id, text);
        }
    }

    /// 递归更新元素可见性
    fn update_element_visible(element: &mut LayoutElement, target_id: &str, visible: bool) {
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
            Self::update_element_visible(child, target_id, visible);
        }
    }

    /// 检查是否有活跃的消息框
    pub fn has_active_dialogs(&self) -> bool {
        !self.active_dialogs.is_empty()
    }

    /// 获取指定对话框的配置快照。
    pub fn get_dialog_config(&self, id: &str) -> Option<&MessageBoxConfig> {
        self.active_dialogs.get(id)
    }

    /// 关闭所有消息框
    pub fn close_all(&mut self) {
        self.active_dialogs.clear();
        self.dialog_layouts.clear();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_width_stays_fixed_for_long_english_message() {
        let config = MessageBoxConfig::new(
            String::new(),
            "Installation is not complete. Are you sure you want to exit?".to_string(),
        )
        .with_size(400.0, 230.0);

        let width = MessageBoxManager::compute_dialog_width(&config);
        assert_eq!(
            width, 400.0,
            "expected english close-confirm dialog width to stay fixed"
        );
    }

    #[test]
    fn dialog_width_stays_fixed_for_long_russian_message() {
        let config = MessageBoxConfig::new(
            String::new(),
            "Установка ещё не завершена. Вы уверены, что хотите выйти?".to_string(),
        )
        .with_size(400.0, 230.0);

        let width = MessageBoxManager::compute_dialog_width(&config);
        assert_eq!(
            width, 400.0,
            "expected russian close-confirm dialog width to stay fixed"
        );
    }
}
