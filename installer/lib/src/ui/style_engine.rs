//! 样式引擎
//!
//! 处理UI元素的样式和主题

use crate::layout::element::ElementAttributes;
use egui::{Color32, Stroke, Style};

/// 样式类型
#[derive(Debug, Clone, PartialEq)]
pub enum StyleType {
    Primary,
    Link,
    Text,
    Custom(String),
}

impl From<&str> for StyleType {
    fn from(s: &str) -> Self {
        match s {
            "primary" => StyleType::Primary,
            "link" => StyleType::Link,
            "text" => StyleType::Text,
            _ => StyleType::Custom(s.to_string()),
        }
    }
}

/// 样式配置
#[derive(Debug, Clone)]
pub struct StyleConfig {
    /// 主色调
    pub primary_color: Color32,
    /// 悬停颜色
    pub hover_color: Color32,
    /// 禁用颜色
    pub disabled_color: Color32,
    /// 文字颜色
    pub text_color: Color32,
    /// 背景颜色
    pub background_color: Color32,
    /// 边框颜色
    pub border_color: Color32,
    /// 圆角半径
    pub corner_radius: f32,
    /// 边框宽度
    pub stroke_width: f32,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            primary_color: Color32::from_rgb(0, 196, 178), // #00C4B2
            hover_color: Color32::from_rgb(0, 220, 200),
            disabled_color: Color32::from_rgb(100, 100, 100),
            text_color: Color32::from_rgb(255, 255, 255),
            background_color: Color32::from_rgb(24, 27, 34), // #181B22
            border_color: Color32::from_rgb(60, 60, 60),
            corner_radius: 3.0,
            stroke_width: 1.0,
        }
    }
}

/// 样式引擎
pub struct StyleEngine {
    /// 样式配置
    config: StyleConfig,
    /// 自定义样式
    custom_styles: std::collections::HashMap<String, StyleConfig>,
}

impl StyleEngine {
    /// 创建新的样式引擎
    pub fn new() -> Self {
        Self {
            config: StyleConfig::default(),
            custom_styles: std::collections::HashMap::new(),
        }
    }

    /// 创建带配置的样式引擎
    pub fn with_config(config: StyleConfig) -> Self {
        Self {
            config,
            custom_styles: std::collections::HashMap::new(),
        }
    }

    /// 添加自定义样式
    pub fn add_custom_style(&mut self, name: String, style: StyleConfig) {
        self.custom_styles.insert(name, style);
    }

    /// 获取按钮样式
    pub fn get_button_style(&self, style_type: &StyleType, enabled: bool) -> ButtonStyle {
        let base_config = self.get_style_config(style_type);

        ButtonStyle {
            background_color: if enabled {
                base_config.primary_color
            } else {
                base_config.disabled_color
            },
            hover_color: if enabled {
                base_config.hover_color
            } else {
                base_config.disabled_color
            },
            text_color: if enabled {
                base_config.text_color
            } else {
                base_config.disabled_color
            },
            border_color: base_config.border_color,
            corner_radius: base_config.corner_radius,
            stroke_width: base_config.stroke_width,
            enabled,
        }
    }

    /// 获取标签样式
    pub fn get_label_style(&self, style_type: &StyleType) -> LabelStyle {
        let base_config = self.get_style_config(style_type);

        LabelStyle {
            text_color: base_config.text_color,
            background_color: None,
            font_size: 14.0,
        }
    }

    /// 获取输入框样式
    pub fn get_input_style(&self, style_type: &StyleType, enabled: bool) -> InputStyle {
        let base_config = self.get_style_config(style_type);

        InputStyle {
            background_color: if enabled {
                Color32::from_rgba_unmultiplied(255, 255, 255, 26) // 10% 透明度
            } else {
                base_config.disabled_color
            },
            text_color: if enabled {
                base_config.text_color
            } else {
                base_config.disabled_color
            },
            border_color: base_config.border_color,
            corner_radius: base_config.corner_radius,
            stroke_width: base_config.stroke_width,
            enabled,
        }
    }

    /// 获取复选框样式
    pub fn get_checkbox_style(
        &self,
        style_type: &StyleType,
        checked: bool,
        enabled: bool,
    ) -> CheckboxStyle {
        let base_config = self.get_style_config(style_type);

        CheckboxStyle {
            background_color: if checked {
                base_config.primary_color
            } else {
                Color32::TRANSPARENT
            },
            border_color: base_config.border_color,
            check_color: if checked {
                base_config.text_color
            } else {
                Color32::TRANSPARENT
            },
            corner_radius: base_config.corner_radius,
            stroke_width: base_config.stroke_width,
            checked,
            enabled,
        }
    }

    /// 获取进度条样式
    pub fn get_progress_style(&self, style_type: &StyleType) -> ProgressStyle {
        let base_config = self.get_style_config(style_type);

        ProgressStyle {
            track_color: base_config.background_color,
            fill_color: base_config.primary_color,
            corner_radius: base_config.corner_radius,
        }
    }

    /// 获取分隔线样式
    pub fn get_divider_style(&self, style_type: &StyleType) -> DividerStyle {
        let base_config = self.get_style_config(style_type);

        DividerStyle {
            color: Color32::from_rgba_unmultiplied(0, 196, 178, 51), // 20% 透明度
            thickness: 1.0,
        }
    }

    /// 获取基础样式配置
    fn get_style_config(&self, style_type: &StyleType) -> &StyleConfig {
        match style_type {
            StyleType::Custom(name) => self.custom_styles.get(name).unwrap_or(&self.config),
            _ => &self.config,
        }
    }

    /// 应用全局样式到egui
    pub fn apply_global_style(&self, style: &mut Style) {
        let visuals = &mut style.visuals;

        // 设置基础颜色
        visuals.window_fill = self.config.background_color;
        visuals.panel_fill = self.config.background_color;
        visuals.window_stroke = Stroke::new(self.config.stroke_width, self.config.border_color);

        // 设置交互颜色
        visuals.hyperlink_color = self.config.primary_color;
        visuals.selection.bg_fill = self.config.primary_color;
        visuals.selection.stroke = Stroke::new(1.0, self.config.primary_color);

        // 设置文本颜色（通过覆盖文本样式）
        // 注意：egui 的 Visuals 不直接支持设置 text_color 和 weak_text_color
        // 这些需要通过其他方式实现

        // 设置按钮样式
        visuals.button_frame = true;
        // 在 egui 0.33 中，window_rounding 字段可能不存在，使用其他方式设置
        // visuals.window_rounding = CornerRadius::same(self.config.corner_radius as u8);
        visuals.button_frame = true;
        visuals.collapsing_header_frame = true;
    }

    /// 解析颜色字符串
    pub fn parse_color(color_str: &str) -> Option<Color32> {
        if color_str.starts_with('#') {
            Self::parse_hex_color(&color_str[1..])
        } else if color_str.starts_with("rgba(") && color_str.ends_with(')') {
            Self::parse_rgba_color(&color_str[5..color_str.len() - 1])
        } else {
            None
        }
    }

    /// 解析十六进制颜色
    fn parse_hex_color(hex: &str) -> Option<Color32> {
        let hex = hex.trim();
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Some(Color32::from_rgb(r, g, b));
            }
        }
        None
    }

    /// 解析RGBA颜色
    fn parse_rgba_color(rgba: &str) -> Option<Color32> {
        let parts: Vec<&str> = rgba.split(',').map(|s| s.trim()).collect();
        if parts.len() == 4 {
            if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                parts[0].parse::<u8>(),
                parts[1].parse::<u8>(),
                parts[2].parse::<u8>(),
                parts[3].parse::<f32>(),
            ) {
                let alpha = (a * 255.0) as u8;
                return Some(Color32::from_rgba_unmultiplied(r, g, b, alpha));
            }
        }
        None
    }
}

impl Default for StyleEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 按钮样式
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub background_color: Color32,
    pub hover_color: Color32,
    pub text_color: Color32,
    pub border_color: Color32,
    pub corner_radius: f32,
    pub stroke_width: f32,
    pub enabled: bool,
}

/// 标签样式
#[derive(Debug, Clone)]
pub struct LabelStyle {
    pub text_color: Color32,
    pub background_color: Option<Color32>,
    pub font_size: f32,
}

/// 输入框样式
#[derive(Debug, Clone)]
pub struct InputStyle {
    pub background_color: Color32,
    pub text_color: Color32,
    pub border_color: Color32,
    pub corner_radius: f32,
    pub stroke_width: f32,
    pub enabled: bool,
}

/// 复选框样式
#[derive(Debug, Clone)]
pub struct CheckboxStyle {
    pub background_color: Color32,
    pub border_color: Color32,
    pub check_color: Color32,
    pub corner_radius: f32,
    pub stroke_width: f32,
    pub checked: bool,
    pub enabled: bool,
}

/// 进度条样式
#[derive(Debug, Clone)]
pub struct ProgressStyle {
    pub track_color: Color32,
    pub fill_color: Color32,
    pub corner_radius: f32,
}

/// 分隔线样式
#[derive(Debug, Clone)]
pub struct DividerStyle {
    pub color: Color32,
    pub thickness: f32,
}

/// 从元素属性创建样式类型
impl From<&ElementAttributes> for StyleType {
    fn from(attrs: &ElementAttributes) -> Self {
        attrs
            .style
            .as_ref()
            .map(|s| StyleType::from(s.as_str()))
            .unwrap_or(StyleType::Primary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_type_from_str() {
        assert_eq!(StyleType::from("primary"), StyleType::Primary);
        assert_eq!(StyleType::from("link"), StyleType::Link);
        assert_eq!(StyleType::from("text"), StyleType::Text);
        assert_eq!(
            StyleType::from("custom"),
            StyleType::Custom("custom".to_string())
        );
    }

    #[test]
    fn test_color_parsing() {
        assert_eq!(
            StyleEngine::parse_color("#FF0000"),
            Some(Color32::from_rgb(255, 0, 0))
        );
        assert_eq!(
            StyleEngine::parse_color("#00FF00"),
            Some(Color32::from_rgb(0, 255, 0))
        );
        assert_eq!(
            StyleEngine::parse_color("rgba(255, 0, 0, 0.5)"),
            Some(Color32::from_rgba_unmultiplied(255, 0, 0, 127))
        );
    }

    #[test]
    fn test_style_engine_creation() {
        let engine = StyleEngine::new();
        let button_style = engine.get_button_style(&StyleType::Primary, true);
        assert!(button_style.enabled);
        assert_eq!(button_style.text_color, Color32::from_rgb(255, 255, 255));
    }

    #[test]
    fn test_custom_style() {
        let mut engine = StyleEngine::new();
        let custom_config = StyleConfig {
            primary_color: Color32::from_rgb(255, 0, 0),
            ..Default::default()
        };
        engine.add_custom_style("red".to_string(), custom_config);

        let button_style = engine.get_button_style(&StyleType::Custom("red".to_string()), true);
        assert_eq!(button_style.background_color, Color32::from_rgb(255, 0, 0));
    }
}
