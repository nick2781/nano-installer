// 按钮组件
// 基于 gpui-component 的 Button 封装

// 注意：实际实现需要根据 gpui-component 的最新 API 调整
// 这里提供结构和逻辑框架

pub enum ButtonStyle {
    Primary,    // 主按钮（青色背景）
    Secondary,  // 次要按钮
    Link,       // 链接样式按钮
    Close,      // 关闭按钮
}

pub struct ButtonConfig {
    pub label: String,
    pub style: ButtonStyle,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub disabled: bool,
}

impl Default for ButtonConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            style: ButtonStyle::Primary,
            width: None,
            height: None,
            disabled: false,
        }
    }
}

// TODO: 实际的 GPUI 渲染实现
// 参考 docs/GPUI_COMPONENTS_GUIDE.md 中的示例
