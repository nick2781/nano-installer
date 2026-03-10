//! 布局元素定义
//! 
//! 定义所有支持的UI元素类型和属性

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::layout::style_props::{FlexStyle, VisualStyle, WidgetProps};

/// 布局元素类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElementType {
    /// 页面根容器
    Page,
    /// 垂直布局容器
    VBox,
    /// 水平布局容器
    HBox,
    /// 空白占位
    Spacer,
    /// 弹性空间
    Flex,
    /// 按钮
    Button,
    /// 文本标签
    Label,
    /// 复选框
    Checkbox,
    /// 文本输入框
    TextInput,
    /// 图片
    Image,
    /// 进度条
    ProgressBar,
    /// 分隔线
    Divider,
    /// 下拉选择框
    Select,
    /// 浮动层
    Overlay,
}

/// 元素属性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementAttributes {
    /// 元素ID
    pub id: Option<String>,
    /// 文本内容（支持@国际化引用）
    pub text: Option<String>,
    /// 宽度
    pub width: Option<f32>,
    /// 高度
    pub height: Option<f32>,
    /// 内边距 (top, right, bottom, left)
    pub padding: Option<(f32, f32, f32, f32)>,
    /// 间距
    pub spacing: Option<f32>,
    /// 背景色
    pub background: Option<String>,
    /// 文字颜色
    pub color: Option<String>,
    /// 样式类型
    pub style: Option<String>,
    /// 对齐方式
    pub align: Option<String>,
    /// 图标/图片路径
    pub icon: Option<String>,
    /// 是否可见
    pub visible: Option<bool>,
    /// 是否启用
    pub enabled: Option<bool>,
    /// 是否选中（用于Checkbox）
    pub selected: Option<bool>,
    /// 进度值（0.0-1.0，用于ProgressBar）
    pub progress: Option<f32>,
    /// 位置（用于Overlay）
    pub position: Option<(f32, f32)>,
    /// 文本换行
    pub wrap: Option<bool>,
    /// 最大行数（用于文本换行）
    pub max_lines: Option<usize>,
    /// 最小宽度
    pub min_width: Option<f32>,
    /// 最大宽度
    pub max_width: Option<f32>,
    /// Flex 权重（用于 HBox/VBox 中的自动空间分配）
    pub flex: Option<f32>,
    /// i18n 键（优先于 text 属性）
    pub text_i18n: Option<String>,
    /// 其他自定义属性
    pub custom: HashMap<String, String>,
}

impl Default for ElementAttributes {
    fn default() -> Self {
        Self {
            id: None,
            text: None,
            width: None,
            height: None,
            padding: None,
            spacing: None,
            background: None,
            color: None,
            style: None,
            align: None,
            icon: None,
            visible: Some(true),
            enabled: Some(true),
            selected: None,
            progress: None,
            position: None,
            wrap: None,
            max_lines: None,
            min_width: None,
            max_width: None,
            flex: None,
            text_i18n: None,
            custom: HashMap::new(),
        }
    }
}

impl ElementAttributes {
    /// 创建新的属性
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设置文本
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// 设置尺寸
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// 设置内边距
    pub fn with_padding(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Some((top, right, bottom, left));
        self
    }

    /// 设置间距
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// 设置背景色
    pub fn with_background(mut self, background: impl Into<String>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// 设置文字颜色
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// 设置样式
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = Some(style.into());
        self
    }

    /// 设置对齐方式
    pub fn with_align(mut self, align: impl Into<String>) -> Self {
        self.align = Some(align.into());
        self
    }

    /// 设置图标
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// 设置可见性
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    /// 设置启用状态
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    /// 设置选中状态
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// 设置进度值
    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }

    /// 设置位置
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = Some((x, y));
        self
    }

    /// 设置自定义属性
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// 获取自定义属性
    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }

    /// 检查是否为国际化字符串引用
    pub fn is_i18n_text(&self) -> bool {
        self.text.as_ref().map_or(false, |t| t.starts_with("@"))
    }

    /// 获取国际化键名（去除@前缀）
    pub fn get_i18n_key(&self) -> Option<&str> {
        self.text.as_ref().and_then(|t| {
            if t.starts_with("@") {
                Some(&t[1..])
            } else {
                None
            }
        })
    }

    /// 检查是否支持子元素
    pub fn supports_children(&self, element_type: &ElementType) -> bool {
        matches!(element_type,
            ElementType::Page |
            ElementType::VBox |
            ElementType::HBox |
            ElementType::Overlay |
            ElementType::Select
        )
    }

    /// 验证属性
    pub fn validate(&self, element_type: &ElementType) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 检查必需属性
        match element_type {
            ElementType::Button => {
                // Button 可以有 text、icon 或 normalimage（图片按钮）
                let has_text = self.text.is_some() || self.text_i18n.is_some();
                let has_icon = self.icon.is_some();
                let has_image = self.custom.contains_key("normalimage");
                
                if !has_text && !has_icon && !has_image {
                    errors.push(format!("{:?} 元素必须设置 text、icon 或 normalimage 属性", element_type));
                }
            }
            ElementType::Label => {
                // Label 可以没有 text（空字符串），因为文本可能通过代码动态设置
                // 只要设置了 text 属性（即使是空字符串）或 icon 属性即可
                let has_text_attr = self.text.is_some();  // 即使为空字符串也算有属性
                let has_icon = self.icon.is_some();
                
                if !has_text_attr && !has_icon {
                    errors.push(format!("{:?} 元素必须设置 text 或 icon 属性", element_type));
                }
            }
            ElementType::Image => {
                if self.icon.is_none() {
                    errors.push("Image 元素必须设置 icon 属性".to_string());
                }
            }
            ElementType::TextInput => {
                if self.id.is_none() {
                    errors.push("TextInput 元素必须设置 id 属性".to_string());
                }
            }
            ElementType::ProgressBar => {
                // ProgressBar 的 progress 属性是可选的，因为进度值通常由代码动态设置
                // 不需要验证
            }
            _ => {}
        }

        // 检查数值范围
        if let Some(progress) = self.progress {
            if progress < 0.0 || progress > 1.0 {
                errors.push("progress 属性必须在 0.0-1.0 范围内".to_string());
            }
        }

        if let Some(width) = self.width {
            if width < 0.0 {
                errors.push("width 属性不能为负数".to_string());
            }
        }

        if let Some(height) = self.height {
            if height < 0.0 {
                errors.push("height 属性不能为负数".to_string());
            }
        }

        // 检查样式值
        if let Some(style) = &self.style {
            match style.as_str() {
                "primary" | "link" | "text" => {}
                _ => errors.push(format!("不支持的样式类型: {}", style)),
            }
        }

        // 检查对齐方式
        if let Some(align) = &self.align {
            match align.as_str() {
                "left" | "center" | "right" | "top" | "middle" | "bottom" => {}
                _ => errors.push(format!("不支持的对齐方式: {}", align)),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// 布局元素
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutElement {
    /// 元素类型
    pub element_type: ElementType,
    /// 元素属性 (旧格式, 向后兼容)
    pub attributes: ElementAttributes,
    /// 子元素
    pub children: Vec<LayoutElement>,

    // ── 新格式分层属性 (Phase 1) ──
    /// Flex 布局属性 (映射到 Taffy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex_style: Option<FlexStyle>,
    /// 视觉属性 (仅渲染用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_style: Option<VisualStyle>,
    /// 控件特有属性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_props: Option<WidgetProps>,
}

impl LayoutElement {
    /// 创建新元素
    pub fn new(element_type: ElementType) -> Self {
        Self {
            element_type,
            attributes: ElementAttributes::new(),
            children: Vec::new(),
            flex_style: None,
            visual_style: None,
            widget_props: None,
        }
    }

    /// 创建带属性的元素
    pub fn with_attributes(element_type: ElementType, attributes: ElementAttributes) -> Self {
        Self {
            element_type,
            attributes,
            children: Vec::new(),
            flex_style: None,
            visual_style: None,
            widget_props: None,
        }
    }

    /// 添加子元素
    pub fn add_child(mut self, child: LayoutElement) -> Self {
        self.children.push(child);
        self
    }

    /// 添加多个子元素
    pub fn add_children(mut self, children: Vec<LayoutElement>) -> Self {
        self.children.extend(children);
        self
    }

    /// 验证元素
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 验证属性
        if let Err(attr_errors) = self.attributes.validate(&self.element_type) {
            errors.extend(attr_errors);
        }

        // 验证子元素
        for child in &self.children {
            if let Err(child_errors) = child.validate() {
                errors.extend(child_errors);
            }
        }

        // 验证嵌套规则
        if !self.attributes.supports_children(&self.element_type) && !self.children.is_empty() {
            errors.push(format!("{:?} 元素不能包含子元素", self.element_type));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 查找子元素（按ID）
    pub fn find_by_id(&self, id: &str) -> Option<&LayoutElement> {
        if self.attributes.id.as_ref().map_or(false, |i| i == id) {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }

        None
    }

    /// 查找子元素（按类型）
    pub fn find_by_type(&self, element_type: &ElementType) -> Vec<&LayoutElement> {
        let mut result = Vec::new();

        if &self.element_type == element_type {
            result.push(self);
        }

        for child in &self.children {
            result.extend(child.find_by_type(element_type));
        }

        result
    }

    /// 获取所有文本元素
    pub fn get_text_elements(&self) -> Vec<&LayoutElement> {
        self.find_by_type(&ElementType::Label)
    }

    /// 获取所有按钮元素
    pub fn get_button_elements(&self) -> Vec<&LayoutElement> {
        self.find_by_type(&ElementType::Button)
    }

    /// 获取所有输入元素
    pub fn get_input_elements(&self) -> Vec<&LayoutElement> {
        let mut result = Vec::new();
        result.extend(self.find_by_type(&ElementType::TextInput));
        result.extend(self.find_by_type(&ElementType::Checkbox));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_attributes_default() {
        let attrs = ElementAttributes::new();
        assert_eq!(attrs.visible, Some(true));
        assert_eq!(attrs.enabled, Some(true));
        assert!(attrs.custom.is_empty());
    }

    #[test]
    fn test_element_attributes_builder() {
        let attrs = ElementAttributes::new()
            .with_id("test-button")
            .with_text("Click Me")
            .with_size(100.0, 30.0)
            .with_style("primary")
            .with_enabled(true);

        assert_eq!(attrs.id, Some("test-button".to_string()));
        assert_eq!(attrs.text, Some("Click Me".to_string()));
        assert_eq!(attrs.width, Some(100.0));
        assert_eq!(attrs.height, Some(30.0));
        assert_eq!(attrs.style, Some("primary".to_string()));
        assert_eq!(attrs.enabled, Some(true));
    }

    #[test]
    fn test_i18n_text_detection() {
        let attrs = ElementAttributes::new().with_text("@welcome.title");
        assert!(attrs.is_i18n_text());
        assert_eq!(attrs.get_i18n_key(), Some("welcome.title"));

        let attrs = ElementAttributes::new().with_text("Normal text");
        assert!(!attrs.is_i18n_text());
        assert_eq!(attrs.get_i18n_key(), None);
    }

    #[test]
    fn test_element_validation() {
        // 有效的按钮
        let button = LayoutElement::with_attributes(
            ElementType::Button,
            ElementAttributes::new()
                .with_text("Click Me")
                .with_style("primary"),
        );
        assert!(button.validate().is_ok());

        // 无效的按钮（缺少文本和图标）
        let invalid_button = LayoutElement::with_attributes(
            ElementType::Button,
            ElementAttributes::new().with_style("primary"),
        );
        assert!(invalid_button.validate().is_err());

        // 无效的进度条（进度值超出范围）
        let invalid_progress = LayoutElement::with_attributes(
            ElementType::ProgressBar,
            ElementAttributes::new().with_progress(1.5),
        );
        assert!(invalid_progress.validate().is_err());
    }

    #[test]
    fn test_element_find_by_id() {
        let page = LayoutElement::new(ElementType::Page)
            .add_child(
                LayoutElement::with_attributes(
                    ElementType::VBox,
                    ElementAttributes::new().with_id("main-container"),
                )
                .add_child(
                    LayoutElement::with_attributes(
                        ElementType::Button,
                        ElementAttributes::new().with_id("submit-button"),
                    ),
                ),
            );

        assert!(page.find_by_id("main-container").is_some());
        assert!(page.find_by_id("submit-button").is_some());
        assert!(page.find_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_element_find_by_type() {
        let page = LayoutElement::new(ElementType::Page)
            .add_child(
                LayoutElement::new(ElementType::VBox)
                    .add_child(LayoutElement::new(ElementType::Button))
                    .add_child(LayoutElement::new(ElementType::Label))
                    .add_child(LayoutElement::new(ElementType::Button)),
            );

        let buttons = page.find_by_type(&ElementType::Button);
        assert_eq!(buttons.len(), 2);

        let labels = page.find_by_type(&ElementType::Label);
        assert_eq!(labels.len(), 1);
    }
}
