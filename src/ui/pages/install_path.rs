// 安装路径选择页面
// 注意：NSIS 设计中路径选择是配置页的展开部分

use gpui::*;
use std::path::PathBuf;

pub struct InstallPathPage {
    install_path: PathBuf,
    required_space: u64,  // 字节
    available_space: u64, // 字节
}

impl InstallPathPage {
    pub fn new(default_path: PathBuf) -> Self {
        Self {
            install_path: default_path,
            required_space: 0,
            available_space: 0,
        }
    }
    
    pub fn install_path(&self) -> &PathBuf {
        &self.install_path
    }
    
    pub fn set_install_path(&mut self, path: PathBuf) {
        self.install_path = path;
        // 更新可用空间
        self.update_available_space();
    }
    
    pub fn set_required_space(&mut self, bytes: u64) {
        self.required_space = bytes;
    }
    
    pub fn has_enough_space(&self) -> bool {
        self.available_space >= self.required_space
    }
    
    fn update_available_space(&mut self) {
        // 获取磁盘可用空间
        // 实际实现需要使用系统 API
        self.available_space = 0;
    }
    
    pub fn required_space_mb(&self) -> u64 {
        self.required_space / 1024 / 1024
    }
    
    pub fn available_space_gb(&self) -> f64 {
        self.available_space as f64 / 1024.0 / 1024.0 / 1024.0
    }
}
