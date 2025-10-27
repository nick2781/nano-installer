// 安装向导容器

/// 向导页面枚举（基于 NSIS 设计）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardPage {
    Config,      // 配置页（主页，包含协议同意和可展开的选项）
    Installing,  // 安装进度页
    Finish,      // 完成页
}

/// 向导状态
pub struct Wizard {
    current_page: WizardPage,
}

impl Wizard {
    /// 创建新的向导
    pub fn new() -> Self {
        Self {
            current_page: WizardPage::Config,
        }
    }
    
    /// 获取当前页面
    pub fn current_page(&self) -> WizardPage {
        self.current_page
    }
    
    /// 设置当前页面
    pub fn set_page(&mut self, page: WizardPage) {
        self.current_page = page;
    }
    
    /// 下一步
    pub fn next(&mut self) {
        self.current_page = match self.current_page {
            WizardPage::Config => WizardPage::Installing,
            WizardPage::Installing => WizardPage::Finish,
            WizardPage::Finish => WizardPage::Finish,
        };
    }
    
    /// 上一步（基于 NSIS 设计，不支持返回）
    pub fn previous(&mut self) {
        // NSIS 设计中，安装过程不支持返回
        // 可以在配置页时关闭窗口取消安装
    }
    
    /// 能否返回上一步
    pub fn can_go_back(&self) -> bool {
        // NSIS 设计中不支持返回
        false
    }
    
    /// 能否进入下一步
    pub fn can_go_forward(&self) -> bool {
        !matches!(self.current_page, WizardPage::Installing | WizardPage::Finish)
    }
    
    /// 是否在安装中
    pub fn is_installing(&self) -> bool {
        matches!(self.current_page, WizardPage::Installing)
    }
    
    /// 是否已完成
    pub fn is_finished(&self) -> bool {
        matches!(self.current_page, WizardPage::Finish)
    }
}

impl Default for Wizard {
    fn default() -> Self {
        Self::new()
    }
}

