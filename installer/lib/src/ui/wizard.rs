// 安装向导容器 — 配置驱动，零硬编码

use crate::config::installer_config::PageConfig;
use crate::layout::LayoutTree;
use std::collections::HashMap;

/// 向导模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardMode {
    Install,
    Update,
    Uninstall,
}

/// 向导状态（配置驱动，页面 ID 为 String）
pub struct Wizard {
    /// 安装流程页面列表 (来自 config.wizard.pages)
    install_pages: Vec<String>,
    /// 更新流程页面列表 (来自 config.wizard.update_pages, 跳过配置页)
    update_pages: Vec<String>,
    /// 卸载流程页面列表 (来自 config.wizard.uninstall_pages)
    uninstall_pages: Vec<String>,
    /// 当前页面索引
    current_index: usize,
    mode: WizardMode,
    page_history: Vec<usize>,
    page_layouts: HashMap<String, LayoutTree>,
    page_heights: HashMap<String, f32>,
    current_height: f32,
    can_proceed: bool,
    is_installing: bool,
    install_progress: f32,
    install_status: String,
}

impl Wizard {
    /// 从配置创建向导
    pub fn new(
        mode: WizardMode,
        install_pages: &[PageConfig],
        update_pages: &[PageConfig],
        uninstall_pages: &[PageConfig],
    ) -> Self {
        let install_ids: Vec<String> = install_pages.iter().map(|p| p.id.clone()).collect();
        let update_ids: Vec<String> = if update_pages.is_empty() {
            // 如果没有配置 update_pages，回退到 install_pages（跳过第一页 config）
            install_ids.iter().skip(1).cloned().collect()
        } else {
            update_pages.iter().map(|p| p.id.clone()).collect()
        };
        let uninstall_ids: Vec<String> = uninstall_pages.iter().map(|p| p.id.clone()).collect();

        Self {
            install_pages: install_ids,
            update_pages: update_ids,
            uninstall_pages: uninstall_ids,
            current_index: 0,
            mode,
            page_history: vec![0],
            page_layouts: HashMap::new(),
            page_heights: HashMap::new(),
            current_height: 400.0,
            can_proceed: true,
            is_installing: false,
            install_progress: 0.0,
            install_status: String::new(),
        }
    }

    /// 当前流程的页面列表
    fn pages(&self) -> &[String] {
        match self.mode {
            WizardMode::Install => &self.install_pages,
            WizardMode::Update => &self.update_pages,
            WizardMode::Uninstall => &self.uninstall_pages,
        }
    }

    /// 获取当前页面 ID
    pub fn current_page_id(&self) -> &str {
        self.pages()
            .get(self.current_index)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// 获取当前模式
    pub fn mode(&self) -> WizardMode {
        self.mode
    }

    /// 设置当前页面（按 ID）
    pub fn set_page(&mut self, page_id: &str) {
        if let Some(idx) = self.pages().iter().position(|p| p == page_id) {
            self.current_index = idx;
            self.update_window_height();
        }
    }

    /// 下一步
    pub fn next(&mut self) {
        let pages = self.pages();
        if self.current_index + 1 < pages.len() {
            self.page_history.push(self.current_index);
            self.current_index += 1;
            self.update_window_height();
        }
    }

    /// 上一步
    pub fn previous(&mut self) {
        if let Some(prev_index) = self.page_history.pop() {
            self.current_index = prev_index;
            self.update_window_height();
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

    /// 是否在最后一页
    pub fn is_finished(&self) -> bool {
        let pages = self.pages();
        self.current_index + 1 >= pages.len()
    }

    /// 开始安装
    pub fn start_installation(&mut self, preparing_status: &str) {
        self.is_installing = true;
        self.can_proceed = false;
        self.install_progress = 0.0;
        self.install_status = preparing_status.to_string();
    }

    /// 更新安装进度
    pub fn update_install_progress(&mut self, progress: f32, status: String) {
        self.install_progress = progress.clamp(0.0, 1.0);
        self.install_status = status;
    }

    /// 完成安装
    pub fn finish_installation(&mut self, complete_status: &str) {
        self.is_installing = false;
        self.install_progress = 1.0;
        self.install_status = complete_status.to_string();
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
    pub fn load_page_layout(&mut self, page_id: &str, layout: LayoutTree) {
        self.page_layouts.insert(page_id.to_string(), layout);
        self.update_window_height();
    }

    /// 获取页面布局
    pub fn get_page_layout(&self, page_id: &str) -> Option<&LayoutTree> {
        self.page_layouts.get(page_id)
    }

    /// 设置页面高度
    pub fn set_page_height(&mut self, page_id: &str, height: f32) {
        self.page_heights.insert(page_id.to_string(), height);
        self.update_window_height();
    }

    /// 获取当前窗口高度
    pub fn current_height(&self) -> f32 {
        self.current_height
    }

    /// 更新窗口高度
    fn update_window_height(&mut self) {
        let page_id = self.current_page_id().to_string();
        if let Some(&height) = self.page_heights.get(&page_id) {
            self.current_height = height;
        }
    }

    /// 重置向导
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.page_history.clear();
        self.page_history.push(0);
        self.can_proceed = true;
        self.is_installing = false;
        self.install_progress = 0.0;
        self.install_status.clear();
        self.update_window_height();
    }

    /// 切换模式
    pub fn switch_mode(&mut self, mode: WizardMode) {
        self.mode = mode;
        self.reset();
    }
}
