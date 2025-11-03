//! 向导流程配置
//! 
//! 定义安装和卸载的页面流程

use serde::{Deserialize, Serialize};

/// 向导配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardConfig {
    /// 安装页面列表
    pub pages: Vec<PageConfig>,
    /// 卸载页面列表
    pub uninstall_pages: Vec<PageConfig>,
}

/// 页面配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageConfig {
    /// 页面ID
    pub id: String,
    /// 布局文件
    pub layout: String,
    /// 页面标题
    pub title: String,
}

/// 页面跳转条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageTransition {
    /// 无条件跳转到下一页
    Next,
    /// 条件跳转
    Conditional {
        condition: String,
        next_page: String,
    },
    /// 结束流程
    Exit,
}

/// 流程定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    /// 流程名称
    pub name: String,
    /// 起始页面
    pub start_page: String,
    /// 页面跳转规则
    pub transitions: std::collections::HashMap<String, PageTransition>,
}

impl WizardConfig {
    /// 创建默认的安装向导配置
    pub fn default_install() -> Self {
        Self {
            pages: vec![
                PageConfig {
                    id: "config".to_string(),
                    layout: "configpage.xml".to_string(),
                    title: "安装配置".to_string(),
                },
                PageConfig {
                    id: "installing".to_string(),
                    layout: "installingpage.xml".to_string(),
                    title: "正在安装".to_string(),
                },
                PageConfig {
                    id: "finish".to_string(),
                    layout: "finishpage.xml".to_string(),
                    title: "安装完成".to_string(),
                },
            ],
            uninstall_pages: vec![
                PageConfig {
                    id: "uninstall_config".to_string(),
                    layout: "uninstallpage.xml".to_string(),
                    title: "卸载确认".to_string(),
                },
                PageConfig {
                    id: "uninstalling".to_string(),
                    layout: "uninstallingpage.xml".to_string(),
                    title: "正在卸载".to_string(),
                },
                PageConfig {
                    id: "uninstall_finish".to_string(),
                    layout: "uninstallfinishpage.xml".to_string(),
                    title: "卸载完成".to_string(),
                },
            ],
        }
    }

    /// 创建更新模式向导配置（跳过配置页）
    pub fn update_mode() -> Self {
        Self {
            pages: vec![
                PageConfig {
                    id: "installing".to_string(),
                    layout: "installingpage.xml".to_string(),
                    title: "正在更新".to_string(),
                },
                PageConfig {
                    id: "finish".to_string(),
                    layout: "finishpage.xml".to_string(),
                    title: "更新完成".to_string(),
                },
            ],
            uninstall_pages: vec![
                PageConfig {
                    id: "uninstall_config".to_string(),
                    layout: "uninstallpage.xml".to_string(),
                    title: "卸载确认".to_string(),
                },
                PageConfig {
                    id: "uninstalling".to_string(),
                    layout: "uninstallingpage.xml".to_string(),
                    title: "正在卸载".to_string(),
                },
                PageConfig {
                    id: "uninstall_finish".to_string(),
                    layout: "uninstallfinishpage.xml".to_string(),
                    title: "卸载完成".to_string(),
                },
            ],
        }
    }

    /// 获取页面配置
    pub fn get_page(&self, id: &str) -> Option<&PageConfig> {
        self.pages.iter().find(|p| p.id == id)
            .or_else(|| self.uninstall_pages.iter().find(|p| p.id == id))
    }

    /// 获取安装页面列表
    pub fn install_pages(&self) -> &[PageConfig] {
        &self.pages
    }

    /// 获取卸载页面列表
    pub fn uninstall_pages(&self) -> &[PageConfig] {
        &self.uninstall_pages
    }

    /// 检查页面是否存在
    pub fn has_page(&self, id: &str) -> bool {
        self.get_page(id).is_some()
    }
}
