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
        let align = element.attributes.align.as_deref().unwrap_or("top");
        let halign = element.attributes.get_custom("halign").map(|s| s.as_str()).unwrap_or("left");
        
        // 检查是否有 flex 子元素
        let has_flex_children = element.children.iter()
            .any(|child| child.attributes.flex.is_some());
        
        if has_flex_children {
            self.render_vbox_with_flex(ui, element, result);
        } else if align != "top" || halign != "left" {
            self.render_vbox_with_align(ui, element, result, align, halign);
        } else {
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
    }
    
    /// 渲染带对齐的垂直布局
    fn render_vbox_with_align(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, align: &str, halign: &str) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);
            
            if let Some((top, _right, _bottom, _left)) = padding {
                ui.add_space(top);
            }
            
            // 计算子元素总高度
            let mut total_height = 0.0;
            for child in &element.children {
                if let Some(height) = child.attributes.height {
                    total_height += height;
                }
            }
            total_height += spacing * (element.children.len() as f32 - 1.0).max(0.0);
            
            let available_height = ui.available_height();
            let remaining_height = (available_height - total_height).max(0.0);
            
            // 垂直对齐
            match align {
                "center" | "middle" => {
                    ui.add_space(remaining_height / 2.0);
                }
                "bottom" => {
                    ui.add_space(remaining_height);
                }
                "space-between" => {
                    let gap = if element.children.len() > 1 {
                        remaining_height / (element.children.len() as f32 - 1.0)
                    } else {
                        0.0
                    };
                    
                    for (i, child) in element.children.iter().enumerate() {
                        self.render_element_with_halign(ui, child, result, halign);
                        if i < element.children.len() - 1 {
                            ui.add_space(gap);
                        }
                    }
                    
                    if let Some((_top, _right, bottom, _left)) = padding {
                        ui.add_space(bottom);
                    }
                    return;
                }
                _ => {} // "top" - 默认，不添加空间
            }
            
            // 渲染子元素（带水平对齐）
            for child in &element.children {
                self.render_element_with_halign(ui, child, result, halign);
            }
            
            if let Some((_top, _right, bottom, _left)) = padding {
                ui.add_space(bottom);
            }
        });
    }
    
    /// 渲染元素（带水平对齐）
    fn render_element_with_halign(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, halign: &str) {
        match halign {
            "center" => {
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 50.0); // 简单居中，TODO: 精确计算
                    self.render_element(ui, element, result);
                });
            }
            "right" => {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    self.render_element(ui, element, result);
                });
            }
            _ => {
                self.render_element(ui, element, result);
            }
        }
    }
    
    /// 渲染带 flex 的垂直布局
    fn render_vbox_with_flex(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);
            
            // 第一遍：计算固定高度元素和总 flex 权重
            let mut fixed_height = 0.0;
            let mut total_flex = 0.0;
            
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    total_flex += flex;
                } else if let Some(height) = child.attributes.height {
                    fixed_height += height;
                }
            }
            
            // 计算剩余可用空间
            let available_height = ui.available_height();
            let spacing_total = spacing * (element.children.len() as f32 - 1.0).max(0.0);
            let remaining_height = (available_height - fixed_height - spacing_total).max(0.0);
            
            // 第二遍：渲染元素
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    // Flex 元素：分配剩余空间
                    let flex_height = if total_flex > 0.0 {
                        remaining_height * (flex / total_flex)
                    } else {
                        0.0
                    };
                    
                    if child.element_type == crate::layout::element::ElementType::Spacer {
                        // Spacer 占用空间但不渲染内容
                        ui.add_space(flex_height);
                    } else {
                        // 其他 flex 元素在分配的高度内渲染
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), flex_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                self.render_element(ui, child, result);
                            },
                        );
                    }
                } else {
                    // 非 flex 元素：正常渲染
                    self.render_element(ui, child, result);
                }
            }
        });
    }

    /// 渲染水平布局
    fn render_hbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        let align = element.attributes.align.as_deref().unwrap_or("left");
        let valign = element.attributes.get_custom("valign").map(|s| s.as_str()).unwrap_or("top");
        
        // 检查是否有 flex 子元素
        let has_flex_children = element.children.iter()
            .any(|child| child.attributes.flex.is_some());
        
        if has_flex_children {
            self.render_hbox_with_flex(ui, element, result);
        } else if align != "left" || valign != "top" {
            self.render_hbox_with_align(ui, element, result, align, valign);
        } else {
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
    }
    
    /// 渲染带对齐的水平布局
    fn render_hbox_with_align(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, align: &str, valign: &str) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        
        // 确定垂直对齐方式
        let vertical_align = match valign {
            "center" | "middle" => egui::Align::Center,
            "bottom" => egui::Align::Max,
            _ => egui::Align::Min,
        };
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);
            
            if let Some((_top, _right, _bottom, left)) = padding {
                ui.add_space(left);
            }
            
            // 计算子元素总宽度
            let mut total_width = 0.0;
            for child in &element.children {
                if let Some(width) = child.attributes.width {
                    total_width += width;
                } else if let Some(min_width) = child.attributes.min_width {
                    total_width += min_width;
                }
            }
            total_width += spacing * (element.children.len() as f32 - 1.0).max(0.0);
            
            let available_width = ui.available_width();
            let remaining_width = (available_width - total_width).max(0.0);
            
            // 水平对齐
            match align {
                "center" => {
                    ui.add_space(remaining_width / 2.0);
                }
                "right" => {
                    ui.add_space(remaining_width);
                }
                "space-between" => {
                    let gap = if element.children.len() > 1 {
                        remaining_width / (element.children.len() as f32 - 1.0)
                    } else {
                        0.0
                    };
                    
                    for (i, child) in element.children.iter().enumerate() {
                        self.render_element_with_valign(ui, child, result, vertical_align);
                        if i < element.children.len() - 1 {
                            ui.add_space(gap);
                        }
                    }
                    
                    if let Some((_top, right, _bottom, _left)) = padding {
                        ui.add_space(right);
                    }
                    return;
                }
                _ => {} // "left" - 默认，不添加空间
            }
            
            // 渲染子元素（带垂直对齐）
            for child in &element.children {
                self.render_element_with_valign(ui, child, result, vertical_align);
            }
            
            if let Some((_top, right, _bottom, _left)) = padding {
                ui.add_space(right);
            }
        });
    }
    
    /// 渲染元素（带垂直对齐）
    fn render_element_with_valign(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, valign: egui::Align) {
        ui.with_layout(egui::Layout::top_down(valign), |ui| {
            self.render_element(ui, element, result);
        });
    }
    
    /// 渲染带 flex 的水平布局
    fn render_hbox_with_flex(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);
            
            // 第一遍：计算固定宽度元素和总 flex 权重
            let mut fixed_width = 0.0;
            let mut total_flex = 0.0;
            
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    total_flex += flex;
                } else if let Some(width) = child.attributes.width {
                    fixed_width += width;
                }
            }
            
            // 计算剩余可用空间
            let available_width = ui.available_width();
            let spacing_total = spacing * (element.children.len() as f32 - 1.0).max(0.0);
            let remaining_width = (available_width - fixed_width - spacing_total).max(0.0);
            
            // 第二遍：渲染元素
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    // Flex 元素：分配剩余空间
                    let flex_width = if total_flex > 0.0 {
                        remaining_width * (flex / total_flex)
                    } else {
                        0.0
                    };
                    
                    if child.element_type == crate::layout::element::ElementType::Spacer {
                        // Spacer 占用空间但不渲染内容
                        ui.add_space(flex_width);
                    } else {
                        // 其他 flex 元素在分配的宽度内渲染
                        ui.allocate_ui_with_layout(
                            egui::vec2(flex_width, ui.available_height()),
                            egui::Layout::left_to_right(egui::Align::Min),
                            |ui| {
                                self.render_element(ui, child, result);
                            },
                        );
                    }
                } else {
                    // 非 flex 元素：正常渲染
                    self.render_element(ui, child, result);
                }
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
        
        // 计算按钮尺寸（支持 min_width）
        let width = if let Some(w) = element.attributes.width {
            w
        } else if let Some(min_w) = element.attributes.min_width {
            // 根据文本内容计算实际宽度，但不小于 min_width
            // 简单估算：每个字符约 7-8 像素（英文），中文约 14-16 像素
            let char_count = text.chars().count();
            let text_width = (char_count as f32) * 8.0;  // 简化计算
            (text_width + 30.0).max(min_w)  // 加30px padding
        } else {
            120.0  // 默认宽度
        };
        
        let height = element.attributes.height.unwrap_or(40.0);
        
        // 暂时使用普通文字按钮，不使用图片背景
        // TODO: 实现自定义绘制的图片背景按钮
        let button_response = ui.add_enabled(enabled, egui::Button::new(text).min_size(egui::vec2(width, height)));
        
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
        
        // 创建 RichText 以支持更多样式
        let mut rich_text = egui::RichText::new(text);
        
        // 设置字体大小（如果有 custom 属性中的 font_size）
        if let Some(font_size_str) = element.attributes.get_custom("font_size") {
            if let Ok(font_size) = font_size_str.parse::<f32>() {
                rich_text = rich_text.size(font_size);
            }
        }
        
        // 设置文字颜色
        if let Some(color_str) = &element.attributes.color {
            if let Some(color) = self.parse_color(color_str) {
                rich_text = rich_text.color(color);
            }
        }
        
        let mut label = egui::Label::new(rich_text);
        
        // 文本换行支持
        if element.attributes.wrap.unwrap_or(false) {
            label = label.wrap();
        }
        
        // 宽度约束
        let mut desired_width = None;
        if let Some(width) = element.attributes.width {
            desired_width = Some(width);
        } else if let Some(max_width) = element.attributes.max_width {
            desired_width = Some(max_width);
        }
        
        if let Some(width) = desired_width {
            ui.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.add(label);
                },
            );
        } else {
            ui.add(label);
        }
    }
    
    /// 解析颜色字符串
    fn parse_color(&self, color_str: &str) -> Option<egui::Color32> {
        // 支持 #RRGGBB 格式
        if color_str.starts_with('#') && color_str.len() == 7 {
            let r = u8::from_str_radix(&color_str[1..3], 16).ok()?;
            let g = u8::from_str_radix(&color_str[3..5], 16).ok()?;
            let b = u8::from_str_radix(&color_str[5..7], 16).ok()?;
            return Some(egui::Color32::from_rgb(r, g, b));
        }
        None
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
        let _style_type = StyleType::from(&element.attributes);
        let _enabled = element.attributes.enabled.unwrap_or(true);
        
        // 获取当前内容
        let _current_text = self.interaction_state.text_inputs.get(&id).cloned().unwrap_or_default();
        
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
        // 优先使用 text_i18n
        if let Some(i18n_key) = &attrs.text_i18n {
            if let Some(translated) = self.i18n_strings.get(i18n_key) {
                return translated.clone();
            }
        }
        
        // 其次使用 text（支持 @ 前缀）
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
