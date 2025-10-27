// 欢迎页面
// 注意：NSIS 设计中没有独立的欢迎页，直接是配置页

use gpui::*;

pub struct WelcomePage {
    app_name: String,
    app_version: String,
}

impl WelcomePage {
    pub fn new(app_name: String, app_version: String) -> Self {
        Self {
            app_name,
            app_version,
        }
    }
}

// 如果需要独立的欢迎页，可以在这里实现
// 当前 NSIS 设计直接使用配置页作为首页
