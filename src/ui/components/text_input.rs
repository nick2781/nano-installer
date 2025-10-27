// 文本输入组件

pub struct TextInputConfig {
    pub value: String,
    pub placeholder: String,
    pub readonly: bool,
    pub width: f32,
    pub height: f32,
}

impl Default for TextInputConfig {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            readonly: false,
            width: 410.0,  // NSIS 设计宽度
            height: 32.0,
        }
    }
}

// TODO: 使用 gpui-component 的 Input
// 安装路径输入框设置为 readonly
