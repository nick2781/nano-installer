//! 布局渲染器
//! 
//! 将布局树渲染到egui UI

use egui::{Ui, Response, Rect, Vec2, Pos2, Color32, CornerRadius, Align};
use crate::layout::{LayoutTree, LayoutElement, ElementType, ElementAttributes};
use crate::ui::{dpi_handler::DpiConfig, style_engine::{StyleEngine, StyleType}};
use std::collections::HashMap;

/// 布局渲染器
pub struct LayoutRenderer {
    /// DPI配置
    dpi_config: DpiConfig,
    /// 样式引擎
    style_engine: StyleEngine,
    /// 资源缓存
    resource_cache: crate::ui::dpi_handler::ResourceCache,
    /// 国际化字符串
    i18n_strings: HashMap<String, String>,
    /// 交互状态
    interaction_state: InteractionState,
}

/// 交互状态
#[derive(Debug, Default)]
pub struct InteractionState {
    /// 按钮点击状态
    button_clicks: HashMap<String, bool>,
    /// 复选框状态
    checkbox_states: HashMap<String, bool>,
    /// 文本输入内容
    text_inputs: HashMap<String, String>,
    /// 悬停状态
    hover_states: HashMap<String, bool>,
}

impl LayoutRenderer {
    /// 创建新的布局渲染器
    pub fn new(dpi_config: DpiConfig, i18n_strings: HashMap<String, String>) -> Self {
        Self {
            dpi_config,
            style_engine: StyleEngine::new(),
            resource_cache: crate::ui::dpi_handler::ResourceCache::new(),
            i18n_strings,
            interaction_state: InteractionState::default(),
        }
    }

    /// 创建带样式引擎的布局渲染器
    pub fn with_style_engine(
        dpi_config: DpiConfig, 
        style_engine: StyleEngine, 
        i18n_strings: HashMap<String, String>
    ) -> Self {
        Self {
            dpi_config,
            style_engine,
            resource_cache: crate::ui::dpi_handler::ResourceCache::new(),
            i18n_strings,
            interaction_state: InteractionState::default(),
        }
    }

    /// 渲染布局树
    pub fn render(&mut self, ui: &mut Ui, layout_tree: &LayoutTree) -> RenderResult {
        let mut result = RenderResult::new();
        
        // 应用全局样式
        self.style_engine.apply_global_style(ui.style_mut());
        
        // 渲染根元素
        self.render_element(ui, &layout_tree.root, &mut result);
        
        result
    }

    /// 渲染单个元素
    fn render_element(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        match &element.element_type {
            ElementType::Page => self.render_page(ui, element, result),
            ElementType::VBox => self.render_vbox(ui, element, result),
            ElementType::HBox => self.render_hbox(ui, element, result),
            ElementType::Spacer => self.render_spacer(ui, element, result),
            ElementType::Flex => self.render_flex(ui, element, result),
            ElementType::Button => self.render_button(ui, element, result),
            ElementType::Label => self.render_label(ui, element, result),
            ElementType::Checkbox => self.render_checkbox(ui, element, result),
            ElementType::TextInput => self.render_text_input(ui, element, result),
            ElementType::Image => self.render_image(ui, element, result),
            ElementType::ProgressBar => self.render_progress_bar(ui, element, result),
            ElementType::Divider => self.render_divider(ui, element, result),
            ElementType::Overlay => self.render_overlay(ui, element, result),
        }
    }

    /// 渲染页面
    fn render_page(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 页面作为根容器，直接渲染子元素
        for child in &element.children {
            self.render_element(ui, child, result);
        }
    }

    /// 渲染垂直布局
    fn render_vbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);
            
            if let Some((top, _right, _bottom, _left)) = padding {
                ui.add_space(top);
            }
            
            for child in &element.children {
                self.render_element(ui, child, result);
            }
            
            if let Some((_top, _right, bottom, _left)) = padding {
                ui.add_space(bottom);
            }
        });
    }

    /// 渲染水平布局
    fn render_hbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);
            
            if let Some((_top, _right, _bottom, left)) = padding {
                ui.add_space(left);
            }
            
            for child in &element.children {
                self.render_element(ui, child, result);
            }
            
            if let Some((_top, right, _bottom, _left)) = padding {
                ui.add_space(right);
            }
        });
    }

    /// 渲染空白占位
    fn render_spacer(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let width = element.attributes.width.unwrap_or(0.0);
        let height = element.attributes.height.unwrap_or(0.0);
        
        if width > 0.0 && height > 0.0 {
            ui.add_space(height);
        } else if height > 0.0 {
            ui.add_space(height);
        } else {
            ui.add_space(10.0); // 默认间距
        }
    }

    /// 渲染弹性空间
    fn render_flex(&mut self, ui: &mut Ui, _element: &LayoutElement, _result: &mut RenderResult) {
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::left_to_right(Align::LEFT),
            |ui| {
                ui.allocate_ui(ui.available_size(), |_ui| {});
            }
        );
    }

    /// 渲染按钮
    fn render_button(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();
        let text = self.get_display_text(&element.attributes);
        let _style_type = StyleType::from(&element.attributes);
        let enabled = element.attributes.enabled.unwrap_or(true);
        
        let width = element.attributes.width.unwrap_or(120.0);
        let height = element.attributes.height.unwrap_or(40.0);
        
        // 暂时使用普通文字按钮，不使用图片背景
        // TODO: 实现自定义绘制的图片背景按钮
        let button = egui::Button::new(text);
        let button_response = ui.add_sized([width, height], button);
        
        if button_response.clicked() && enabled {
            result.button_clicks.insert(id.clone(), true);
        }
        
        // 记录按钮响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.button_responses.insert(id.clone(), button_response);
        }
    }

    /// 渲染标签
    fn render_label(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let text = self.get_display_text(&element.attributes);
        let _style_type = StyleType::from(&element.attributes);
        let _label_style = self.style_engine.get_label_style(&_style_type);
        
        let label = egui::Label::new(text);
        
        // 注意：egui::Label 不支持直接设置背景色和文字颜色
        // 这些需要通过 RichText 或其他方式实现
        
        ui.add(label);
    }

    /// 渲染复选框
    fn render_checkbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();
        let text = self.get_display_text(&element.attributes);
        let _style_type = StyleType::from(&element.attributes);
        let _enabled = element.attributes.enabled.unwrap_or(true);
        
        // 获取当前状态
        let current_checked = self.interaction_state.checkbox_states.get(&id).copied().unwrap_or(false);
        
        let checkbox_state = self.interaction_state.checkbox_states.entry(id.clone()).or_insert(current_checked);
        let checkbox = egui::Checkbox::new(checkbox_state, text);
        
        let response = ui.add(checkbox);
        
        if response.changed() {
            let new_checked = !current_checked;
            self.interaction_state.checkbox_states.insert(id.clone(), new_checked);
            result.checkbox_changes.insert(id.clone(), new_checked);
        }
        
        // 记录复选框响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.checkbox_responses.insert(id.clone(), response);
        }
    }

    /// 渲染文本输入框
    fn render_text_input(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();
        let style_type = StyleType::from(&element.attributes);
        let enabled = element.attributes.enabled.unwrap_or(true);
        
        // 获取当前内容
        let current_text = self.interaction_state.text_inputs.get(&id).cloned().unwrap_or_default();
        
        let mut text_input = egui::TextEdit::singleline(self.interaction_state.text_inputs.entry(id.clone()).or_insert_with(String::new));
        
        if let Some(width) = element.attributes.width {
            text_input = text_input.desired_width(width);
        }
        if let Some(height) = element.attributes.height {
            text_input = text_input.desired_rows((height / 20.0) as usize);
        }
        
        let response = ui.add(text_input);
        
        if response.changed() {
            result.text_input_changes.insert(id.clone(), self.interaction_state.text_inputs.get(&id).cloned().unwrap_or_default());
        }
        
        // 记录文本输入响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.text_input_responses.insert(id.clone(), response);
        }
    }

    /// 渲染图片
    fn render_image(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        if let Some(icon) = &element.attributes.icon {
            if let Some(texture) = self.resource_cache.get_background(ui.ctx(), &self.dpi_config, icon) {
                // 如果 XML 中指定了尺寸，使用指定的尺寸
                // 否则使用纹理的实际渲染尺寸（会自动处理 2x 资源）
                let size = if let (Some(w), Some(h)) = (element.attributes.width, element.attributes.height) {
                    egui::Vec2::new(w, h)
                } else {
                    self.dpi_config.get_render_size(texture)
                };
                
                ui.add(egui::Image::new(texture).max_size(size));
            }
        }
    }

    /// 渲染进度条
    fn render_progress_bar(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let progress = element.attributes.progress.unwrap_or(0.0);
        let _style_type = StyleType::from(&element.attributes);
        let _progress_style = self.style_engine.get_progress_style(&_style_type);
        
        let width = element.attributes.width.unwrap_or(ui.available_width());
        let height = element.attributes.height.unwrap_or(6.0);
        
        // 获取进度条图片
        if let Some(texture) = self.resource_cache.get_progress(ui.ctx(), &self.dpi_config) {
            let progress_width = width * progress;
            let progress_rect = Rect::from_min_size(
                ui.cursor().min,
                Vec2::new(progress_width, height)
            );
            
            if ui.is_rect_visible(progress_rect) {
                ui.painter().image(
                    texture.id(),
                    progress_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(progress, 1.0)),
                    Color32::WHITE,
                );
            }
        } else {
            // 使用默认进度条
            ui.add(egui::ProgressBar::new(progress).desired_width(width));
        }
    }

    /// 渲染分隔线
    fn render_divider(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let style_type = StyleType::from(&element.attributes);
        let divider_style = self.style_engine.get_divider_style(&style_type);
        
        let width = element.attributes.width.unwrap_or(ui.available_width());
        let height = element.attributes.height.unwrap_or(1.0);
        
        ui.add_space(5.0);
        ui.painter().rect_filled(
            Rect::from_min_size(ui.cursor().min, Vec2::new(width, height)),
            CornerRadius::same(0),
            divider_style.color,
        );
        ui.add_space(5.0);
    }

    /// 渲染浮动层
    fn render_overlay(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        if let Some((x, y)) = element.attributes.position {
            let width = element.attributes.width.unwrap_or(200.0);
            let height = element.attributes.height.unwrap_or(100.0);
            
            egui::Area::new(egui::Id::new(element.attributes.id.as_ref().unwrap_or(&"overlay".to_string())))
                .fixed_pos(Pos2::new(x, y))
                .show(ui.ctx(), |ui| {
                    ui.allocate_ui(Vec2::new(width, height), |ui| {
                        for child in &element.children {
                            self.render_element(ui, child, result);
                        }
                    });
                });
        }
    }

    /// 获取显示文本（处理国际化）
    fn get_display_text(&self, attrs: &ElementAttributes) -> String {
        if let Some(text) = &attrs.text {
            if text.starts_with('@') {
                // 国际化字符串
                let key = &text[1..];
                self.i18n_strings.get(key).cloned().unwrap_or_else(|| text.clone())
            } else {
                text.clone()
            }
        } else {
            String::new()
        }
    }

    /// 获取按钮点击状态
    pub fn get_button_clicked(&self, id: &str) -> bool {
        self.interaction_state.button_clicks.get(id).copied().unwrap_or(false)
    }

    /// 获取复选框状态
    pub fn get_checkbox_checked(&self, id: &str) -> bool {
        self.interaction_state.checkbox_states.get(id).copied().unwrap_or(false)
    }

    /// 获取文本输入内容
    pub fn get_text_input_value(&self, id: &str) -> String {
        self.interaction_state.text_inputs.get(id).cloned().unwrap_or_default()
    }

    /// 设置文本输入内容
    pub fn set_text_input_value(&mut self, id: &str, value: String) {
        self.interaction_state.text_inputs.insert(id.to_string(), value);
    }

    /// 清除交互状态
    pub fn clear_interaction_state(&mut self) {
        self.interaction_state = InteractionState::default();
    }
}

/// 渲染结果
#[derive(Debug, Default)]
pub struct RenderResult {
    /// 按钮点击事件
    pub button_clicks: HashMap<String, bool>,
    /// 复选框状态变化
    pub checkbox_changes: HashMap<String, bool>,
    /// 文本输入变化
    pub text_input_changes: HashMap<String, String>,
    /// 按钮响应
    pub button_responses: HashMap<String, Response>,
    /// 复选框响应
    pub checkbox_responses: HashMap<String, Response>,
    /// 文本输入响应
    pub text_input_responses: HashMap<String, Response>,
}

impl RenderResult {
    /// 创建新的渲染结果
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查按钮是否被点击
    pub fn is_button_clicked(&self, id: &str) -> bool {
        self.button_clicks.get(id).copied().unwrap_or(false)
    }

    /// 检查复选框是否被改变
    pub fn is_checkbox_changed(&self, id: &str) -> bool {
        self.checkbox_changes.contains_key(id)
    }

    /// 获取复选框新状态
    pub fn get_checkbox_new_state(&self, id: &str) -> Option<bool> {
        self.checkbox_changes.get(id).copied()
    }

    /// 检查文本输入是否被改变
    pub fn is_text_input_changed(&self, id: &str) -> bool {
        self.text_input_changes.contains_key(id)
    }

    /// 获取文本输入新值
    pub fn get_text_input_new_value(&self, id: &str) -> Option<&String> {
        self.text_input_changes.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InstallerConfig;

    #[test]
    fn test_layout_renderer_creation() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        let i18n_strings = HashMap::new();
        
        let renderer = LayoutRenderer::new(dpi_config, i18n_strings);
        assert!(renderer.interaction_state.button_clicks.is_empty());
        assert!(renderer.interaction_state.checkbox_states.is_empty());
        assert!(renderer.interaction_state.text_inputs.is_empty());
    }

    #[test]
    fn test_render_result() {
        let mut result = RenderResult::new();
        assert!(!result.is_button_clicked("test"));
        assert!(!result.is_checkbox_changed("test"));
        assert!(!result.is_text_input_changed("test"));
        
        result.button_clicks.insert("test".to_string(), true);
        assert!(result.is_button_clicked("test"));
    }

    #[test]
    fn test_display_text_resolution() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        let mut i18n_strings = HashMap::new();
        i18n_strings.insert("welcome.title".to_string(), "欢迎".to_string());
        
        let renderer = LayoutRenderer::new(dpi_config, i18n_strings);
        
        let attrs = ElementAttributes::new().with_text("@welcome.title");
        let text = renderer.get_display_text(&attrs);
        assert_eq!(text, "欢迎");
        
        let attrs = ElementAttributes::new().with_text("Normal text");
        let text = renderer.get_display_text(&attrs);
        assert_eq!(text, "Normal text");
    }
}
