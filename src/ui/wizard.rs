// 安装向导容器

use std::collections::HashMap;
use crate::layout::LayoutTree;

/// 向导页面枚举（基于 NSIS 设计）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WizardPage {
    Welcome,     // 欢迎页
    Language,    // 语言选择页
    License,     // 许可协议页
    InstallPath, // 安装路径页
    Installing,  // 安装进度页
    Finish,      // 完成页
    UninstallConfirm, // 卸载确认页
    UninstallProgress, // 卸载进度页
    UninstallFinish,  // 卸载完成页
}

/// 向导模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardMode {
    Install,
    Update,
    Uninstall,
}

/// 向导状态
pub struct Wizard {
    current_page: WizardPage,
    mode: WizardMode,
    page_history: Vec<WizardPage>,
    page_layouts: HashMap<WizardPage, LayoutTree>,
    page_heights: HashMap<WizardPage, f32>,
    current_height: f32,
    can_proceed: bool,
    is_installing: bool,
    install_progress: f32,
    install_status: String,
}

impl Wizard {
    /// 创建新的向导
    pub fn new(mode: WizardMode) -> Self {
        let initial_page = match mode {
            WizardMode::Install | WizardMode::Update => WizardPage::Welcome,
            WizardMode::Uninstall => WizardPage::UninstallConfirm,
        };
        
        Self {
            current_page: initial_page,
            mode,
            page_history: vec![initial_page],
            page_layouts: HashMap::new(),
            page_heights: HashMap::new(),
            current_height: 400.0, // 默认高度
            can_proceed: true,
            is_installing: false,
            install_progress: 0.0,
            install_status: String::new(),
        }
    }
    
    /// 获取当前页面
    pub fn current_page(&self) -> WizardPage {
        self.current_page
    }
    
    /// 获取当前模式
    pub fn mode(&self) -> WizardMode {
        self.mode
    }
    
    /// 设置当前页面
    pub fn set_page(&mut self, page: WizardPage) {
        if page != self.current_page {
            self.page_history.push(self.current_page);
            self.current_page = page;
            self.update_window_height();
        }
    }
    
    /// 下一步
    pub fn next(&mut self) {
        let next_page = self.get_next_page();
        if next_page != self.current_page {
            self.set_page(next_page);
        }
    }
    
    /// 上一步
    pub fn previous(&mut self) {
        if let Some(prev_page) = self.page_history.pop() {
            self.current_page = prev_page;
            self.update_window_height();
        }
    }
    
    /// 获取下一页
    fn get_next_page(&self) -> WizardPage {
        match self.mode {
            WizardMode::Install | WizardMode::Update => {
                match self.current_page {
                    WizardPage::Welcome => WizardPage::InstallPath,
                    WizardPage::Language => WizardPage::InstallPath,
                    WizardPage::License => WizardPage::InstallPath,
                    WizardPage::InstallPath => WizardPage::Installing,
                    WizardPage::Installing => WizardPage::Finish,
                    WizardPage::Finish => WizardPage::Finish,
                    _ => self.current_page,
                }
            }
            WizardMode::Uninstall => {
                match self.current_page {
                    WizardPage::UninstallConfirm => WizardPage::UninstallProgress,
                    WizardPage::UninstallProgress => WizardPage::UninstallFinish,
                    WizardPage::UninstallFinish => WizardPage::UninstallFinish,
                    _ => self.current_page,
                }
            }
        }
    }
    
    /// 能否返回上一步
    pub fn can_go_back(&self) -> bool {
        !self.page_history.is_empty() && !self.is_installing
    }
    
    /// 能否进入下一步
    pub fn can_go_forward(&self) -> bool {
        self.can_proceed && !self.is_installing && !self.is_finished()
    }
    
    /// 是否在安装中
    pub fn is_installing(&self) -> bool {
        self.is_installing
    }
    
    /// 是否已完成
    pub fn is_finished(&self) -> bool {
        matches!(self.current_page, WizardPage::Finish | WizardPage::UninstallFinish)
    }
    
    /// 开始安装
    pub fn start_installation(&mut self) {
        self.is_installing = true;
        self.can_proceed = false;
        self.install_progress = 0.0;
        self.install_status = "准备安装...".to_string();
    }
    
    /// 更新安装进度
    pub fn update_install_progress(&mut self, progress: f32, status: String) {
        self.install_progress = progress.clamp(0.0, 1.0);
        self.install_status = status;
    }
    
    /// 完成安装
    pub fn finish_installation(&mut self) {
        self.is_installing = false;
        self.install_progress = 1.0;
        self.install_status = "安装完成".to_string();
        self.next();
    }
    
    /// 获取安装进度
    pub fn install_progress(&self) -> f32 {
        self.install_progress
    }
    
    /// 获取安装状态
    pub fn install_status(&self) -> &str {
        &self.install_status
    }
    
    /// 设置能否继续
    pub fn set_can_proceed(&mut self, can_proceed: bool) {
        self.can_proceed = can_proceed;
    }
    
    /// 加载页面布局
    pub fn load_page_layout(&mut self, page: WizardPage, layout: LayoutTree) {
        self.page_layouts.insert(page, layout);
        self.update_window_height();
    }
    
    /// 获取页面布局
    pub fn get_page_layout(&self, page: WizardPage) -> Option<&LayoutTree> {
        self.page_layouts.get(&page)
    }
    
    /// 设置页面高度
    pub fn set_page_height(&mut self, page: WizardPage, height: f32) {
        self.page_heights.insert(page, height);
        self.update_window_height();
    }
    
    /// 获取当前窗口高度
    pub fn current_height(&self) -> f32 {
        self.current_height
    }
    
    /// 更新窗口高度
    fn update_window_height(&mut self) {
        if let Some(&height) = self.page_heights.get(&self.current_page) {
            self.current_height = height;
        } else {
            // 默认高度
            self.current_height = match self.current_page {
                WizardPage::Welcome => 400.0,
                WizardPage::Language => 300.0,
                WizardPage::License => 500.0,
                WizardPage::InstallPath => 450.0,
                WizardPage::Installing => 350.0,
                WizardPage::Finish => 400.0,
                WizardPage::UninstallConfirm => 400.0,
                WizardPage::UninstallProgress => 350.0,
                WizardPage::UninstallFinish => 400.0,
            };
        }
    }
    
    /// 重置向导
    pub fn reset(&mut self) {
        let initial_page = match self.mode {
            WizardMode::Install | WizardMode::Update => WizardPage::Welcome,
            WizardMode::Uninstall => WizardPage::UninstallConfirm,
        };
        
        self.current_page = initial_page;
        self.page_history.clear();
        self.page_history.push(initial_page);
        self.can_proceed = true;
        self.is_installing = false;
        self.install_progress = 0.0;
        self.install_status.clear();
        self.update_window_height();
    }
    
    /// 切换到安装模式
    pub fn switch_to_install_mode(&mut self) {
        self.mode = WizardMode::Install;
        self.reset();
    }
    
    /// 切换到更新模式
    pub fn switch_to_update_mode(&mut self) {
        self.mode = WizardMode::Update;
        self.reset();
    }
    
    /// 切换到卸载模式
    pub fn switch_to_uninstall_mode(&mut self) {
        self.mode = WizardMode::Uninstall;
        self.reset();
    }
}

impl Default for Wizard {
    fn default() -> Self {
        Self::new(WizardMode::Install)
    }
}

/// 向导页面配置
pub struct WizardPageConfig {
    pub page: WizardPage,
    pub layout_file: String,
    pub default_height: f32,
    pub can_skip: bool,
    pub requires_confirmation: bool,
}

impl WizardPageConfig {
    pub fn new(page: WizardPage, layout_file: String, default_height: f32) -> Self {
        Self {
            page,
            layout_file,
            default_height,
            can_skip: false,
            requires_confirmation: false,
        }
    }
    
    pub fn with_skip(mut self, can_skip: bool) -> Self {
        self.can_skip = can_skip;
        self
    }
    
    pub fn with_confirmation(mut self, requires_confirmation: bool) -> Self {
        self.requires_confirmation = requires_confirmation;
        self
    }
}

/// 向导配置
pub struct WizardConfig {
    pub pages: Vec<WizardPageConfig>,
    pub default_width: f32,
    pub default_height: f32,
    pub min_width: f32,
    pub min_height: f32,
    pub max_width: f32,
    pub max_height: f32,
}

impl WizardConfig {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            default_width: 500.0,
            default_height: 400.0,
            min_width: 400.0,
            min_height: 300.0,
            max_width: 800.0,
            max_height: 600.0,
        }
    }
    
    pub fn add_page(mut self, page_config: WizardPageConfig) -> Self {
        self.pages.push(page_config);
        self
    }
    
    pub fn get_page_config(&self, page: WizardPage) -> Option<&WizardPageConfig> {
        self.pages.iter().find(|p| p.page == page)
    }
}

impl Default for WizardConfig {
    fn default() -> Self {
        Self::new()
    }
}