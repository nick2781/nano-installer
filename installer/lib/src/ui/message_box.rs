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
    pub fn render(&mut self, ctx: &egui::Context) -> HashMap<String, MessageBoxResult> {
        let mut results = HashMap::new();
        let mut to_remove = Vec::new();

        for (id, config) in &self.active_dialogs {
            if let Some(result) = self.render_dialog(ctx, id, config) {
                results.insert(id.clone(), result);
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.active_dialogs.remove(&id);
        }

        results
    }

    fn render_dialog(&self, ctx: &egui::Context, id: &str, config: &MessageBoxConfig) -> Option<MessageBoxResult> {
        let mut result = None;

        egui::Window::new(&config.title)
            .id(egui::Id::new(id))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                let width = config.width.unwrap_or(400.0);
                let height = config.height.unwrap_or(200.0);
                
                ui.set_min_size(egui::Vec2::new(width, height));
                
                // 消息内容
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    
                    // 图标（根据类型）
                    match config.message_type {
                        MessageBoxType::Info => {
                            ui.label("ℹ️");
                        }
                        MessageBoxType::Warning => {
                            ui.label("⚠️");
                        }
                        MessageBoxType::Error => {
                            ui.label("❌");
                        }
                        MessageBoxType::Question => {
                            ui.label("❓");
                        }
                    }
                    
                    ui.add_space(10.0);
                    
                    // 消息文本
                    ui.label(&config.message);
                    
                    ui.add_space(30.0);
                    
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
                                if ui.button("确定").clicked() {
                                    result = Some(MessageBoxResult::Ok);
                                }
                                ui.add_space(10.0);
                                if ui.button("取消").clicked() {
                                    result = Some(MessageBoxResult::Cancel);
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
