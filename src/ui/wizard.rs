// 安装向导容器

/// 向导页面枚举（基于 NSIS 设计）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardPage {
    Welcome,     // 欢迎页
    Language,    // 语言选择页
    License,     // 许可协议页
    InstallPath, // 安装路径页
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
            current_page: WizardPage::Welcome,
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
            WizardPage::Welcome => WizardPage::InstallPath,
            WizardPage::Language => WizardPage::InstallPath,
            WizardPage::License => WizardPage::InstallPath,
            WizardPage::InstallPath => WizardPage::Installing,
            WizardPage::Installing => WizardPage::Finish,
            WizardPage::Finish => WizardPage::Finish,
        };
    }
    
    /// 上一步
    pub fn previous(&mut self) {
        self.current_page = match self.current_page {
            WizardPage::Welcome => WizardPage::Welcome,
            WizardPage::Language => WizardPage::Welcome,
            WizardPage::License => WizardPage::Welcome,
            WizardPage::InstallPath => WizardPage::License,
            WizardPage::Installing => WizardPage::InstallPath,
            WizardPage::Finish => WizardPage::Finish,
        };
    }
    
    /// 能否返回上一步
    pub fn can_go_back(&self) -> bool {
        self.current_page != WizardPage::Welcome && self.current_page != WizardPage::Finish
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

