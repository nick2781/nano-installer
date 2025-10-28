// 主应用窗口 - 使用GPUI实现

use crate::ui::wizard::{Wizard, WizardPage};
use crate::ui::styles;
use crate::installer::InstallState;
use crate::common::config::InstallerConfig;
use std::sync::Arc;
use gpui::*;
use gpui::prelude::*;

pub struct InstallerApp {
    wizard: Wizard,
    config: InstallerConfig,
    install_state: Option<Arc<InstallState>>,
    custom_install_expanded: bool,
    install_path: String,
    create_desktop_shortcut: bool,
    start_on_boot: bool,
    agree_to_terms: bool,
}

impl InstallerApp {
    pub fn new(config: InstallerConfig) -> Self {
        let install_path = config.default_install_path.clone();
        
        Self {
            wizard: Wizard::new(),
            config,
            install_state: Some(Arc::new(InstallState::new(install_path.clone()))),
            custom_install_expanded: false,
            install_path,
            create_desktop_shortcut: true,
            start_on_boot: false,
            agree_to_terms: false,
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
    
    pub fn toggle_custom_install(&mut self) {
        self.custom_install_expanded = !self.custom_install_expanded;
    }
    
    pub fn toggle_agree_to_terms(&mut self) {
        self.agree_to_terms = !self.agree_to_terms;
    }
    
    pub fn toggle_desktop_shortcut(&mut self) {
        self.create_desktop_shortcut = !self.create_desktop_shortcut;
    }
    
    pub fn toggle_start_on_boot(&mut self) {
        self.start_on_boot = !self.start_on_boot;
    }
}

// GPUI 0.2.2 使用不同的API结构
// 我们需要使用 View 而不是 App

impl Render for InstallerApp {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(rgb(0xf8f9fa))
            .child(
                // 标题栏
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .h_16()
                    .px_4()
                    .bg(rgb(0xffffff))
                    .border_b_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                img("assets/logo.png")
                                    .w_8()
                                    .h_8()
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x1f2937))
                                    .text(self.config.app_name.clone())
                            )
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x6b7280))
                            .text(format!("v{}", self.config.app_version))
                    )
            )
            .child(
                // 主内容区域
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .p_6()
                    .child(self.render_current_page(cx))
            )
            .child(
                // 底部按钮栏
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .h_16()
                    .px_6()
                    .bg(rgb(0xffffff))
                    .border_t_1()
                    .border_color(rgb(0xe5e7eb))
                    .child(
                        // 后退按钮
                        if self.can_go_back() {
                            Some(
                                button("上一步")
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(0x6b7280))
                                    .text_color(rgb(0xffffff))
                                    .rounded_md()
                                    .hover(|style| style.bg(rgb(0x4b5563)))
                                    .on_click(cx.listener(|this, _event, cx| {
                                        this.previous_page();
                                        cx.notify();
                                    }))
                            )
                        } else {
                            None
                        }
                    )
                    .child(
                        // 前进/完成按钮
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                if self.can_go_forward() {
                                    Some(
                                        button("下一步")
                                            .px_4()
                                            .py_2()
                                            .bg(rgb(0x3b82f6))
                                            .text_color(rgb(0xffffff))
                                            .rounded_md()
                                            .hover(|style| style.bg(rgb(0x2563eb)))
                                            .on_click(cx.listener(|this, _event, cx| {
                                                this.next_page();
                                                cx.notify();
                                            }))
                                    )
                                } else {
                                    Some(
                                        button("完成")
                                            .px_4()
                                            .py_2()
                                            .bg(rgb(0x10b981))
                                            .text_color(rgb(0xffffff))
                                            .rounded_md()
                                            .hover(|style| style.bg(rgb(0x059669)))
                                            .on_click(cx.listener(|this, _event, cx| {
                                                cx.quit();
                                            }))
                                    )
                                }
                            )
                            .child(
                                button("取消")
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(0xef4444))
                                    .text_color(rgb(0xffffff))
                                    .rounded_md()
                                    .hover(|style| style.bg(rgb(0xdc2626)))
                                    .on_click(cx.listener(|_this, _event, cx| {
                                        cx.quit();
                                    }))
                            )
                    )
            )
    }
}

impl InstallerApp {
    fn render_current_page(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        match self.current_page() {
            WizardPage::Welcome => self.render_welcome_page(),
            WizardPage::Language => self.render_language_page(),
            WizardPage::License => self.render_license_page(),
            WizardPage::InstallPath => self.render_install_path_page(),
            WizardPage::Installing => self.render_installing_page(),
            WizardPage::Finish => self.render_finish_page(),
        }
    }
    
    fn render_welcome_page(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w_full()
            .h_full()
            .gap_6()
            .child(
                div()
                    .text_4xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x1f2937))
                    .text("欢迎使用安装程序")
            )
            .child(
                div()
                    .text_lg()
                    .text_color(rgb(0x6b7280))
                    .text_center()
                    .text(format!("{} v{}", self.config.app_name, self.config.app_version))
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x4b5563))
                    .text_center()
                    .max_w_96()
                    .text("此向导将引导您完成安装过程。请按照屏幕上的说明进行操作。")
            )
    }
    
    fn render_language_page(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x1f2937))
                    .text("选择语言")
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x6b7280))
                    .text("请选择您偏好的语言：")
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        self.config.supported_locales.iter().map(|locale| {
                            div()
                                .flex()
                                .items_center()
                                .p_3()
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xe5e7eb))
                                .rounded_md()
                                .hover(|style| style.bg(rgb(0xf9fafb)))
                                .child(
                                    div()
                                        .text_base()
                                        .text_color(rgb(0x1f2937))
                                        .text(locale)
                                )
                        }).collect::<Vec<_>>()
                    )
            )
    }
    
    fn render_license_page(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x1f2937))
                    .text("许可协议")
            )
            .child(
                div()
                    .flex_1()
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xe5e7eb))
                    .rounded_md()
                    .p_4()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x4b5563))
                            .text("请仔细阅读以下许可协议。如果您同意这些条款，请勾选下面的复选框。")
                    )
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        input()
                            .r#type("checkbox")
                            .checked(self.agree_to_terms)
                    )
                    .child(
                        div()
                            .text_base()
                            .text_color(rgb(0x1f2937))
                            .text("我同意许可协议的条款")
                    )
            )
    }
    
    fn render_install_path_page(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x1f2937))
                    .text("安装路径")
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x6b7280))
                    .text("选择安装目录：")
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                input()
                                    .flex_1()
                                    .px_3()
                                    .py_2()
                                    .border_1()
                                    .border_color(rgb(0xd1d5db))
                                    .rounded_md()
                                    .text(self.install_path.clone())
                            )
                            .child(
                                button("浏览")
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(0x6b7280))
                                    .text_color(rgb(0xffffff))
                                    .rounded_md()
                                    .hover(|style| style.bg(rgb(0x4b5563)))
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        input()
                                            .r#type("checkbox")
                                            .checked(self.create_desktop_shortcut)
                                    )
                                    .child(
                                        div()
                                            .text_base()
                                            .text_color(rgb(0x1f2937))
                                            .text("创建桌面快捷方式")
                                    )
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        input()
                                            .r#type("checkbox")
                                            .checked(self.start_on_boot)
                                    )
                                    .child(
                                        div()
                                            .text_base()
                                            .text_color(rgb(0x1f2937))
                                            .text("开机自启动")
                                    )
                            )
                    )
            )
    }
    
    fn render_installing_page(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w_full()
            .h_full()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x1f2937))
                    .text("正在安装...")
            )
            .child(
                div()
                    .w_64()
                    .h_2()
                    .bg(rgb(0xe5e7eb))
                    .rounded_full()
                    .overflow_hidden()
                    .child(
                        div()
                            .w_32()
                            .h_full()
                            .bg(rgb(0x3b82f6))
                            .rounded_full()
                    )
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x6b7280))
                    .text("请稍候，正在安装文件...")
            )
    }
    
    fn render_finish_page(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w_full()
            .h_full()
            .gap_6()
            .child(
                div()
                    .text_4xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x10b981))
                    .text("安装完成！")
            )
            .child(
                div()
                    .text_lg()
                    .text_color(rgb(0x6b7280))
                    .text_center()
                    .text(format!("{} 已成功安装到您的计算机上。", self.config.app_name))
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x4b5563))
                    .text_center()
                    .max_w_96()
                    .text("您现在可以开始使用应用程序了。")
            )
    }
}