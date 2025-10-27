// 复选框组件

pub struct CheckboxConfig {
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
}

impl Default for CheckboxConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            checked: false,
            enabled: true,
        }
    }
}

// TODO: 使用 gpui-component 的 Checkbox
// 图片资源：
// - assets/checkbox-0.png (未选中)
// - assets/checkbox-2.png (已选中)
