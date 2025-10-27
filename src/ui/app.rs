// 主应用窗口

use gpui::*;
use crate::ui::wizard::{Wizard, WizardPage};
use crate::ui::styles;
use crate::installer::InstallState;
use crate::common::config::InstallerConfig;
use std::sync::Arc;

pub struct InstallerApp {
    wizard: Wizard,
    config: InstallerConfig,
    install_state: Option<Arc<InstallState>>,
}

impl InstallerApp {
    pub fn new(config: InstallerConfig) -> Self {
        let install_path = config.default_install_path.clone();
        
        Self {
            wizard: Wizard::new(),
            config,
            install_state: Some(Arc::new(InstallState::new(install_path))),
        }
    }
    
    pub fn current_page(&self) -> WizardPage {
        self.wizard.current_page()
    }
    
    pub fn next_page(&mut self) {
        self.wizard.next();
    }
    
    pub fn previous_page(&mut self) {
        self.wizard.previous();
    }
    
    pub fn can_go_back(&self) -> bool {
        self.wizard.can_go_back()
    }
    
    pub fn can_go_forward(&self) -> bool {
        self.wizard.can_go_forward()
    }
}

// TODO: 实现 GPUI Render trait
// 
// impl Render for InstallerApp {
//     fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
//         v_flex()
//             .size(px(styles::WINDOW_WIDTH as f32), px(styles::WINDOW_HEIGHT as f32))
//             .bg(styles::Colors::default().background)
//             .child(self.render_current_page(cx))
//             .child(self.render_close_button(cx))
//     }
// }

