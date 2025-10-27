// 完成页面

use gpui::*;

pub struct FinishPage {
    success: bool,
    can_launch: bool,
    launch_on_finish: bool,
    error_message: Option<String>,
}

impl FinishPage {
    pub fn new(success: bool) -> Self {
        Self {
            success,
            can_launch: success,
            launch_on_finish: false,
            error_message: None,
        }
    }
    
    pub fn with_error(error: String) -> Self {
        Self {
            success: false,
            can_launch: false,
            launch_on_finish: false,
            error_message: Some(error),
        }
    }
    
    pub fn is_success(&self) -> bool {
        self.success
    }
    
    pub fn can_launch(&self) -> bool {
        self.can_launch
    }
    
    pub fn should_launch(&self) -> bool {
        self.launch_on_finish
    }
    
    pub fn set_launch_on_finish(&mut self, launch: bool) {
        self.launch_on_finish = launch && self.can_launch;
    }
    
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}
