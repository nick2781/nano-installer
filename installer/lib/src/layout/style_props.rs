//! CSS Flexbox 风格属性定义
//!
//! 将元素属性分为三层：布局属性（FlexStyle）、视觉属性（VisualStyle）、控件属性（WidgetProps）

use crate::layout::dimension::{Dimension, Edges};
use serde::{Deserialize, Serialize};

/// Flex 布局属性 (映射到 Taffy Style)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlexStyle {
    // 尺寸
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,

    // Flex 容器属性
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub gap: f32,

    // Flex 子项属性
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_self: AlignSelf,

    // 间距 (默认 0px, 不是 auto — auto margin 在 flexbox 中有特殊含义)
    pub margin: Edges<Dimension>,
    pub padding: Edges<Dimension>,

    // 定位
    pub position: Position,
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub left: Dimension,
}

impl Default for FlexStyle {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_width: Dimension::Auto,
            max_height: Dimension::Auto,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            gap: 0.0,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            align_self: AlignSelf::Auto,
            margin: Edges::zero(),
            padding: Edges::zero(),
            position: Position::Relative,
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: Dimension::Auto,
        }
    }
}

impl FlexStyle {
    /// 转换为 taffy::Style
    pub fn to_taffy(&self) -> taffy::Style {
        taffy::Style {
            display: taffy::Display::Flex,
            position: self.position.to_taffy(),
            flex_direction: self.flex_direction.to_taffy(),
            flex_wrap: self.flex_wrap.to_taffy(),
            justify_content: Some(self.justify_content.to_taffy()),
            align_items: Some(self.align_items.to_taffy()),
            align_content: Some(self.align_content.to_taffy()),
            align_self: self.align_self.to_taffy(),
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.to_taffy(),
            size: taffy::Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            min_size: taffy::Size {
                width: self.min_width.to_taffy(),
                height: self.min_height.to_taffy(),
            },
            max_size: taffy::Size {
                width: self.max_width.to_taffy(),
                height: self.max_height.to_taffy(),
            },
            margin: self.margin.to_taffy_lpa(),
            padding: self.padding.to_taffy_lp(),
            gap: taffy::Size {
                width: taffy::LengthPercentage::Length(self.gap),
                height: taffy::LengthPercentage::Length(self.gap),
            },
            inset: taffy::Rect {
                top: self.top.to_taffy_length_pct_auto(),
                right: self.right.to_taffy_length_pct_auto(),
                bottom: self.bottom.to_taffy_length_pct_auto(),
                left: self.left.to_taffy_length_pct_auto(),
            },
            ..Default::default()
        }
    }
}

/// 视觉属性 (仅渲染用, 不参与布局计算)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualStyle {
    pub background: Option<String>,
    pub background_image: Option<String>,
    pub color: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_family: Option<String>,
    pub text_align: Option<TextAlign>,
    pub border_radius: Option<f32>,
    pub border_color: Option<String>,
    pub border_width: Option<f32>,
    pub opacity: Option<f32>,
    pub visible: bool,
    pub enabled: bool,
}

impl Default for VisualStyle {
    fn default() -> Self {
        Self {
            background: None,
            background_image: None,
            color: None,
            font_size: None,
            font_weight: None,
            font_family: None,
            text_align: None,
            border_radius: None,
            border_color: None,
            border_width: None,
            opacity: None,
            visible: true,
            enabled: true,
        }
    }
}

/// 控件特有属性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetProps {
    pub id: Option<String>,
    pub text: Option<String>,
    pub value: Option<String>,
    /// 按钮行为声明 (例: "install", "close", "open_url:terms_of_service", "toggle_panel:moreconfiginfo:show")
    pub action: Option<String>,

    // Button 图片状态
    pub normal_image: Option<String>,
    pub hover_image: Option<String>,
    pub pressed_image: Option<String>,
    pub disabled_image: Option<String>,

    // Checkbox
    pub checked_image: Option<String>,
    pub unchecked_image: Option<String>,
    pub checked: Option<bool>,

    // ProgressBar
    pub progress: Option<f32>,
    pub bar_image: Option<String>,
    pub track_image: Option<String>,

    // Image
    pub src: Option<String>,

    // TextInput
    pub placeholder: Option<String>,
    pub readonly: Option<bool>,
    pub multiline: Option<bool>,

    // 通用
    pub max_lines: Option<usize>,
    pub wrap: Option<bool>,

    // 保留旧格式兼容 (NSIS 图片格式等)
    pub custom: std::collections::HashMap<String, String>,
}

impl Default for WidgetProps {
    fn default() -> Self {
        Self {
            id: None,
            text: None,
            value: None,
            action: None,
            normal_image: None,
            hover_image: None,
            pressed_image: None,
            disabled_image: None,
            checked_image: None,
            unchecked_image: None,
            checked: None,
            progress: None,
            bar_image: None,
            track_image: None,
            src: None,
            placeholder: None,
            readonly: None,
            multiline: None,
            max_lines: None,
            wrap: None,
            custom: std::collections::HashMap::new(),
        }
    }
}

impl WidgetProps {
    /// 检查文本是否为 i18n 引用 (@key)
    pub fn is_i18n_text(&self) -> bool {
        self.text.as_ref().map_or(false, |t| t.starts_with('@'))
    }

    /// 获取 i18n 键名 (去除 @ 前缀)
    pub fn get_i18n_key(&self) -> Option<&str> {
        self.text.as_ref().and_then(|t| t.strip_prefix('@'))
    }
}

// ─── 枚举类型 ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlexDirection {
    Row,
    Column,
}

impl Default for FlexDirection {
    fn default() -> Self {
        FlexDirection::Row
    }
}

impl FlexDirection {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "row" => Some(FlexDirection::Row),
            "column" | "col" => Some(FlexDirection::Column),
            _ => None,
        }
    }

    pub fn to_taffy(self) -> taffy::FlexDirection {
        match self {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
}

impl Default for FlexWrap {
    fn default() -> Self {
        FlexWrap::NoWrap
    }
}

impl FlexWrap {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "nowrap" | "no-wrap" => Some(FlexWrap::NoWrap),
            "wrap" => Some(FlexWrap::Wrap),
            _ => None,
        }
    }

    pub fn to_taffy(self) -> taffy::FlexWrap {
        match self {
            FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::FlexWrap::Wrap,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Default for JustifyContent {
    fn default() -> Self {
        JustifyContent::FlexStart
    }
}

impl JustifyContent {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace('-', "").as_str() {
            "flexstart" | "start" => Some(JustifyContent::FlexStart),
            "flexend" | "end" => Some(JustifyContent::FlexEnd),
            "center" => Some(JustifyContent::Center),
            "spacebetween" => Some(JustifyContent::SpaceBetween),
            "spacearound" => Some(JustifyContent::SpaceAround),
            "spaceevenly" => Some(JustifyContent::SpaceEvenly),
            _ => None,
        }
    }

    pub fn to_taffy(self) -> taffy::JustifyContent {
        match self {
            JustifyContent::FlexStart => taffy::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => taffy::JustifyContent::FlexEnd,
            JustifyContent::Center => taffy::JustifyContent::Center,
            JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

impl Default for AlignItems {
    fn default() -> Self {
        AlignItems::Stretch
    }
}

impl AlignItems {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace('-', "").as_str() {
            "flexstart" | "start" => Some(AlignItems::FlexStart),
            "flexend" | "end" => Some(AlignItems::FlexEnd),
            "center" => Some(AlignItems::Center),
            "stretch" => Some(AlignItems::Stretch),
            "baseline" => Some(AlignItems::Baseline),
            _ => None,
        }
    }

    pub fn to_taffy(self) -> taffy::AlignItems {
        match self {
            AlignItems::FlexStart => taffy::AlignItems::FlexStart,
            AlignItems::FlexEnd => taffy::AlignItems::FlexEnd,
            AlignItems::Center => taffy::AlignItems::Center,
            AlignItems::Stretch => taffy::AlignItems::Stretch,
            AlignItems::Baseline => taffy::AlignItems::Baseline,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignContent {
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
}

impl Default for AlignContent {
    fn default() -> Self {
        AlignContent::Stretch
    }
}

impl AlignContent {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace('-', "").as_str() {
            "flexstart" | "start" => Some(AlignContent::FlexStart),
            "flexend" | "end" => Some(AlignContent::FlexEnd),
            "center" => Some(AlignContent::Center),
            "stretch" => Some(AlignContent::Stretch),
            "spacebetween" => Some(AlignContent::SpaceBetween),
            "spacearound" => Some(AlignContent::SpaceAround),
            _ => None,
        }
    }

    pub fn to_taffy(self) -> taffy::AlignContent {
        match self {
            AlignContent::FlexStart => taffy::AlignContent::FlexStart,
            AlignContent::FlexEnd => taffy::AlignContent::FlexEnd,
            AlignContent::Center => taffy::AlignContent::Center,
            AlignContent::Stretch => taffy::AlignContent::Stretch,
            AlignContent::SpaceBetween => taffy::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => taffy::AlignContent::SpaceAround,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignSelf {
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

impl Default for AlignSelf {
    fn default() -> Self {
        AlignSelf::Auto
    }
}

impl AlignSelf {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace('-', "").as_str() {
            "auto" => Some(AlignSelf::Auto),
            "flexstart" | "start" => Some(AlignSelf::FlexStart),
            "flexend" | "end" => Some(AlignSelf::FlexEnd),
            "center" => Some(AlignSelf::Center),
            "stretch" => Some(AlignSelf::Stretch),
            "baseline" => Some(AlignSelf::Baseline),
            _ => None,
        }
    }

    pub fn to_taffy(self) -> Option<taffy::AlignSelf> {
        match self {
            AlignSelf::Auto => None,
            AlignSelf::FlexStart => Some(taffy::AlignSelf::FlexStart),
            AlignSelf::FlexEnd => Some(taffy::AlignSelf::FlexEnd),
            AlignSelf::Center => Some(taffy::AlignSelf::Center),
            AlignSelf::Stretch => Some(taffy::AlignSelf::Stretch),
            AlignSelf::Baseline => Some(taffy::AlignSelf::Baseline),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Relative,
    Absolute,
}

impl Default for Position {
    fn default() -> Self {
        Position::Relative
    }
}

impl Position {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "relative" => Some(Position::Relative),
            "absolute" => Some(Position::Absolute),
            _ => None,
        }
    }

    pub fn to_taffy(self) -> taffy::Position {
        match self {
            Position::Relative => taffy::Position::Relative,
            Position::Absolute => taffy::Position::Absolute,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    Normal,
    Bold,
}

impl FontWeight {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "normal" | "400" => Some(FontWeight::Normal),
            "bold" | "700" => Some(FontWeight::Bold),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl TextAlign {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "left" => Some(TextAlign::Left),
            "center" => Some(TextAlign::Center),
            "right" => Some(TextAlign::Right),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_style_to_taffy() {
        let style = FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(358.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        };

        let taffy_style = style.to_taffy();
        assert_eq!(taffy_style.flex_direction, taffy::FlexDirection::Column);
        assert_eq!(taffy_style.size.width, taffy::Dimension::Percent(1.0));
        assert_eq!(taffy_style.size.height, taffy::Dimension::Length(358.0));
    }

    #[test]
    fn test_widget_props_i18n() {
        let props = WidgetProps {
            text: Some("@install_button".to_string()),
            ..Default::default()
        };
        assert!(props.is_i18n_text());
        assert_eq!(props.get_i18n_key(), Some("install_button"));
    }

    #[test]
    fn test_enum_parsing() {
        assert_eq!(FlexDirection::parse("column"), Some(FlexDirection::Column));
        assert_eq!(
            JustifyContent::parse("space-between"),
            Some(JustifyContent::SpaceBetween)
        );
        assert_eq!(AlignItems::parse("flex-end"), Some(AlignItems::FlexEnd));
        assert_eq!(Position::parse("absolute"), Some(Position::Absolute));
    }
}
