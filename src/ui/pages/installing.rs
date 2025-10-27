// 安装进度页面

use gpui::*;
use crate::installer::InstallProgress;

pub struct InstallingPage {
    progress: InstallProgress,
}

impl InstallingPage {
    pub fn new() -> Self {
        Self {
            progress: InstallProgress::default(),
        }
    }
    
    pub fn update_progress(&mut self, progress: InstallProgress) {
        self.progress = progress;
    }
    
    pub fn progress(&self) -> &InstallProgress {
        &self.progress
    }
    
    pub fn percentage(&self) -> f32 {
        self.progress.percentage
    }
    
    pub fn status_text(&self) -> &str {
        &self.progress.current_step
    }
    
    pub fn current_file(&self) -> Option<&str> {
        self.progress.current_file.as_deref()
    }
}

impl Default for InstallingPage {
    fn default() -> Self {
        Self::new()
    }
}
