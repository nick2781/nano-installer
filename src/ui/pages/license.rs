// 许可协议页面
// 注意：NSIS 设计中许可协议是配置页中的链接，不是独立页面

use gpui::*;

pub struct LicensePage {
    license_text: String,
    accepted: bool,
}

impl LicensePage {
    pub fn new(license_text: String) -> Self {
        Self {
            license_text,
            accepted: false,
        }
    }
    
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }
    
    pub fn set_accepted(&mut self, accepted: bool) {
        self.accepted = accepted;
    }
}

// 如果需要显示完整的许可协议文本，可以在对话框中显示
// 当前 NSIS 设计使用链接到外部文档
