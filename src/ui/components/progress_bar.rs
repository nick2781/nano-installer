// 进度条组件

pub struct ProgressBarConfig {
    pub value: f32,      // 0.0 - 100.0
    pub width: f32,
    pub height: f32,
    pub show_percentage: bool,
}

impl Default for ProgressBarConfig {
    fn default() -> Self {
        Self {
            value: 0.0,
            width: 574.0,  // 全宽
            height: 6.0,   // NSIS 设计高度
            show_percentage: false,
        }
    }
}

// TODO: 使用 gpui-component 的 ProgressBar
// 参考资源路径：assets/bar_installing.png
