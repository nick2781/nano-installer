// 安装状态

use parking_lot::RwLock;
use std::sync::Arc;

/// 安装状态
#[derive(Debug, Clone)]
pub struct InstallState {
    inner: Arc<RwLock<InstallStateInner>>,
}

#[derive(Debug, Clone)]
struct InstallStateInner {
    /// 安装路径
    pub install_path: String,

    /// 是否创建桌面快捷方式
    pub create_desktop_shortcut: bool,

    /// 是否创建开始菜单快捷方式
    pub create_start_menu_shortcut: bool,

    /// 是否添加到 PATH
    pub add_to_path: bool,

    /// 是否启用开机自启动
    pub autostart_enabled: bool,

    /// 当前进度
    pub progress: InstallProgress,

    /// 是否已取消
    pub cancelled: bool,
}

impl InstallState {
    /// 创建新的安装状态
    pub fn new(install_path: String) -> Self {
        Self {
            inner: Arc::new(RwLock::new(InstallStateInner {
                install_path,
                create_desktop_shortcut: true,
                create_start_menu_shortcut: true,
                add_to_path: false,
                autostart_enabled: false,
                progress: InstallProgress::default(),
                cancelled: false,
            })),
        }
    }

    /// 获取安装路径
    pub fn install_path(&self) -> String {
        self.inner.read().install_path.clone()
    }

    /// 设置安装路径
    pub fn set_install_path(&self, path: String) {
        self.inner.write().install_path = path;
    }

    /// 是否创建桌面快捷方式
    pub fn create_desktop_shortcut(&self) -> bool {
        self.inner.read().create_desktop_shortcut
    }

    /// 设置是否创建桌面快捷方式
    pub fn set_create_desktop_shortcut(&self, create: bool) {
        self.inner.write().create_desktop_shortcut = create;
    }

    /// 是否创建开始菜单快捷方式
    pub fn create_start_menu_shortcut(&self) -> bool {
        self.inner.read().create_start_menu_shortcut
    }

    /// 设置是否创建开始菜单快捷方式
    pub fn set_create_start_menu_shortcut(&self, create: bool) {
        self.inner.write().create_start_menu_shortcut = create;
    }

    /// 是否添加到 PATH
    pub fn add_to_path(&self) -> bool {
        self.inner.read().add_to_path
    }

    /// 设置是否添加到 PATH
    pub fn set_add_to_path(&self, add: bool) {
        self.inner.write().add_to_path = add;
    }

    /// 是否启用开机自启
    pub fn autostart_enabled(&self) -> bool {
        self.inner.read().autostart_enabled
    }

    /// 设置是否启用开机自启
    pub fn set_autostart_enabled(&self, enabled: bool) {
        self.inner.write().autostart_enabled = enabled;
    }

    /// 获取进度
    pub fn progress(&self) -> InstallProgress {
        self.inner.read().progress.clone()
    }

    /// 更新进度
    pub fn update_progress(&self, progress: InstallProgress) {
        self.inner.write().progress = progress;
    }

    /// 是否已取消
    pub fn is_cancelled(&self) -> bool {
        self.inner.read().cancelled
    }

    /// 取消安装
    pub fn cancel(&self) {
        self.inner.write().cancelled = true;
    }
}

/// 安装进度
#[derive(Debug, Clone, Default)]
pub struct InstallProgress {
    /// 当前步骤
    pub current_step: String,

    /// 总百分比 (0-100)
    pub percentage: f32,

    /// 当前处理的文件（可选）
    pub current_file: Option<String>,

    /// 已处理的文件数
    pub files_processed: usize,

    /// 总文件数
    pub total_files: usize,
}

impl InstallProgress {
    /// 创建新的进度
    pub fn new(step: String) -> Self {
        Self {
            current_step: step,
            percentage: 0.0,
            current_file: None,
            files_processed: 0,
            total_files: 0,
        }
    }

    /// 更新百分比
    pub fn set_percentage(&mut self, percentage: f32) {
        self.percentage = percentage.clamp(0.0, 100.0);
    }
}
