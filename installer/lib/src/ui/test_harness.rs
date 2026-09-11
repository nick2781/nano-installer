use crate::config::InstallerConfig;
use crate::layout::taffy_bridge::{ComputedLayout, ComputedRect};
use crate::layout::LayoutTree;
use crate::ui::dpi_handler::DpiConfig;
use crate::ui::egui_app_xml::InstallerApp;
use crate::ui::message_box::{MessageBoxButton, MessageBoxConfig, MessageBoxType};
use crate::ui::resource_provider::FilesystemUiResourceProvider;
use crate::ui::wizard::WizardMode;
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;
use std::sync::Arc;

/// 单个页面的布局快照。
#[derive(Debug, Clone)]
pub struct LayoutSnapshot {
    page_id: String,
    computed: ComputedLayout,
}

impl LayoutSnapshot {
    /// 快照所属页面 ID。
    pub fn page_id(&self) -> &str {
        &self.page_id
    }

    /// 查询指定元素的计算后矩形。
    pub fn rect(&self, id: &str) -> Option<&ComputedRect> {
        self.computed.get_rect(id)
    }

    /// 获取完整布局计算结果。
    pub fn computed(&self) -> &ComputedLayout {
        &self.computed
    }
}

/// 待显示消息框的最小快照。
#[derive(Debug, Clone, PartialEq)]
pub struct PendingMessageBoxSnapshot {
    pub title: String,
    pub message: String,
    pub message_type: MessageBoxType,
    pub buttons: MessageBoxButton,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub ok_text: String,
    pub cancel_text: String,
}

impl From<MessageBoxConfig> for PendingMessageBoxSnapshot {
    fn from(config: MessageBoxConfig) -> Self {
        Self {
            title: config.title,
            message: config.message,
            message_type: config.message_type,
            buttons: config.buttons,
            width: config.width,
            height: config.height,
            ok_text: config.ok_text,
            cancel_text: config.cancel_text,
        }
    }
}

/// 可直接驱动 `InstallerApp` 的无窗口 UI harness。
pub struct UiHarness {
    ctx: egui::Context,
    app: InstallerApp,
}

impl UiHarness {
    /// 从项目目录创建 harness。
    pub fn from_project_dir(project_dir: impl AsRef<Path>, mode: WizardMode) -> Result<Self> {
        let project_dir = project_dir.as_ref().to_path_buf();
        let config_path = project_dir.join("installer_config.json");
        let config_content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        let config: InstallerConfig = serde_json::from_str(&config_content)
            .with_context(|| format!("failed to parse {}", config_path.display()))?;
        let dpi = DpiConfig::new(&config);
        let provider = Arc::new(FilesystemUiResourceProvider::new(&project_dir));
        let app = InstallerApp::new_with_provider(config, dpi, mode, provider);

        Ok(Self {
            ctx: egui::Context::default(),
            app,
        })
    }

    /// 当前向导页面 ID。
    pub fn current_page_id(&self) -> &str {
        self.app.harness_current_page_id()
    }

    /// 加载指定页面的布局树。
    pub fn load_page_layout(&mut self, page_id: &str) -> Result<LayoutTree> {
        self.app
            .harness_load_page_layout(page_id)
            .cloned()
            .with_context(|| format!("failed to load page layout: {}", page_id))
    }

    /// 加载当前页面布局树。
    pub fn current_layout(&mut self) -> Result<LayoutTree> {
        let page_id = self.current_page_id().to_string();
        self.load_page_layout(&page_id)
    }

    /// 计算指定页面的布局快照。
    pub fn snapshot_page(&mut self, page_id: &str) -> Result<LayoutSnapshot> {
        let layout = self.load_page_layout(page_id)?;
        let computed = self
            .app
            .harness_layout_renderer_mut()
            .and_then(|renderer| renderer.compute_layout_snapshot(&layout))
            .with_context(|| format!("failed to compute layout snapshot: {}", page_id))?;

        Ok(LayoutSnapshot {
            page_id: page_id.to_string(),
            computed,
        })
    }

    /// 计算当前页面的布局快照。
    pub fn snapshot_current_page(&mut self) -> Result<LayoutSnapshot> {
        let page_id = self.current_page_id().to_string();
        self.snapshot_page(&page_id)
    }

    /// 直接分发一个 XML action。
    pub fn dispatch_action(&mut self, action: &str) {
        self.app.harness_dispatch_action(action, &self.ctx);
    }

    /// 切换当前语言。
    pub fn switch_language(&mut self, locale: &str) {
        self.app.harness_switch_language(locale, &self.ctx);
    }

    /// 获取当前语言下的翻译文本。
    pub fn translated(&self, key: &str) -> String {
        self.app.harness_i18n(key)
    }

    /// 当前是否存在待显示的关闭确认框。
    pub fn has_pending_close_confirmation(&self) -> bool {
        self.app.harness_has_pending_close_confirmation()
    }

    /// 获取待显示关闭确认框的配置快照。
    pub fn pending_close_confirmation(&self) -> Option<PendingMessageBoxSnapshot> {
        self.app
            .harness_pending_close_confirmation_config()
            .map(Into::into)
    }

    /// 查询单个文本输入/选择框缓存值。
    pub fn text_input_value(&self, id: &str) -> Option<String> {
        self.app
            .harness_layout_renderer()
            .map(|renderer| renderer.get_text_input_value(id))
    }

    /// 获取当前所有文本输入/选择框缓存值。
    pub fn text_input_values(&self) -> Option<std::collections::HashMap<String, String>> {
        self.app
            .harness_layout_renderer()
            .map(|renderer| renderer.get_all_text_input_values())
    }

    /// 查询某个元素当前的 `visible` 配置值。
    pub fn element_visible(&mut self, page_id: &str, element_id: &str) -> Option<bool> {
        self.app
            .harness_load_page_layout(page_id)
            .and_then(|layout| layout.root.find_by_id(element_id))
            .and_then(|element| element.attributes.visible)
    }

    /// 获取选择框当前显示文案，而不是内部值。
    pub fn select_display_text(&mut self, page_id: &str, select_id: &str) -> Option<String> {
        let selected_value = self.text_input_value(select_id)?;
        let selected_attrs = {
            let layout = self.app.harness_load_page_layout(page_id)?;
            let select = layout.root.find_by_id(select_id)?;

            select
                .children
                .iter()
                .find(|child| {
                    child
                        .attributes
                        .get_custom("value")
                        .map(|value| value == &selected_value)
                        .unwrap_or(false)
                })
                .map(|child| child.attributes.clone())
        };
        let renderer = self.app.harness_layout_renderer()?;

        selected_attrs
            .as_ref()
            .map(|attrs| renderer.get_display_text_for_harness(attrs))
            .or_else(|| Some(selected_value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn example_project_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap")
    }

    #[test]
    fn test_ui_harness_loads_filesystem_layout_snapshot() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        assert_eq!(harness.current_page_id(), "config");

        let snapshot = harness
            .snapshot_current_page()
            .expect("snapshot current page");
        assert_eq!(snapshot.page_id(), "config");
        assert!(snapshot.rect("langSelect").is_some());
        assert!(snapshot.rect("btnShowMore_wrap").is_some());
    }

    #[test]
    fn test_ui_harness_can_switch_language_and_track_select_value() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        harness.switch_language("ru");

        let translated = harness.translated("close_confirm_message");
        assert!(translated.contains("Установка") || translated.contains("уверены"));
        assert_eq!(
            harness.text_input_value("langSelect").as_deref(),
            Some("ru")
        );
        assert_eq!(
            harness
                .select_display_text("config", "langSelect")
                .as_deref(),
            Some("Русский")
        );
    }

    #[test]
    fn test_ui_harness_can_dispatch_close_confirmation() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        harness.dispatch_action("close_confirm");

        assert!(harness.has_pending_close_confirmation());
        let snapshot = harness
            .pending_close_confirmation()
            .expect("pending close confirmation snapshot");
        assert_eq!(snapshot.message_type, MessageBoxType::Question);
        assert_eq!(snapshot.buttons, MessageBoxButton::OkCancel);
        assert_eq!(snapshot.message, "安装尚未完成，确定退出安装？");
        assert_eq!(snapshot.ok_text, "退出安装");
        assert_eq!(snapshot.cancel_text, "继续安装");
        assert_eq!(snapshot.width, Some(400.0));
        assert_eq!(snapshot.height, Some(230.0));
    }

    #[test]
    fn test_ui_harness_tracks_toggle_panel_visibility_state() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        assert_eq!(
            harness.element_visible("config", "moreconfiginfo"),
            Some(false)
        );
        assert_eq!(
            harness.element_visible("config", "btnShowMore_wrap"),
            Some(true)
        );
        assert_eq!(
            harness.element_visible("config", "btnHideMore_wrap"),
            Some(false)
        );

        harness.dispatch_action("toggle_panel:moreconfiginfo:show");

        assert_eq!(
            harness.element_visible("config", "moreconfiginfo"),
            Some(true)
        );
        assert_eq!(
            harness.element_visible("config", "btnShowMore_wrap"),
            Some(false)
        );
        assert_eq!(
            harness.element_visible("config", "btnHideMore_wrap"),
            Some(true)
        );

        harness.dispatch_action("toggle_panel:moreconfiginfo:hide");

        assert_eq!(
            harness.element_visible("config", "moreconfiginfo"),
            Some(false)
        );
        assert_eq!(
            harness.element_visible("config", "btnShowMore_wrap"),
            Some(true)
        );
        assert_eq!(
            harness.element_visible("config", "btnHideMore_wrap"),
            Some(false)
        );
    }

    #[test]
    fn test_ui_harness_preserves_layout_bounds_after_language_switch() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        let before = harness
            .snapshot_current_page()
            .expect("snapshot before switch");
        let before_lang = before.rect("langSelect").expect("lang select rect");
        let before_show_more = before
            .rect("btnShowMore_wrap")
            .expect("show more button rect");

        harness.switch_language("ru");

        let after = harness
            .snapshot_current_page()
            .expect("snapshot after switch");
        let after_lang = after.rect("langSelect").expect("lang select rect");
        let after_show_more = after
            .rect("btnShowMore_wrap")
            .expect("show more button rect");

        assert_eq!(before_lang.width, after_lang.width);
        assert_eq!(before_lang.height, after_lang.height);
        assert_eq!(before_show_more.height, after_show_more.height);
        assert!(after_show_more.width >= before_show_more.width);
        assert!(after_show_more.x + after_show_more.width <= after.computed.container_width + 0.1);
        assert_eq!(
            harness
                .select_display_text("config", "langSelect")
                .as_deref(),
            Some("Русский")
        );
    }

    #[test]
    fn test_close_confirmation_snapshot_switches_with_locale() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        harness.switch_language("ru");
        harness.dispatch_action("close_confirm");

        let snapshot = harness
            .pending_close_confirmation()
            .expect("pending close confirmation snapshot");
        assert_eq!(
            snapshot.message,
            "Установка ещё не завершена. Вы уверены, что хотите выйти?"
        );
        assert_eq!(snapshot.ok_text, "Выйти");
        assert_eq!(snapshot.cancel_text, "Продолжить");
        assert_eq!(snapshot.width, Some(400.0));
        assert_eq!(snapshot.height, Some(230.0));
    }

    #[test]
    fn test_show_more_button_content_stays_inside_button_bounds() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        let snapshot = harness
            .snapshot_current_page()
            .expect("snapshot current page");

        let button = snapshot.rect("btnShowMore_wrap").expect("button rect");
        let content = snapshot.rect("showMore_content").expect("content rect");
        let text = snapshot.rect("showMore_text").expect("text rect");
        let badge = snapshot.rect("showMore_badge").expect("badge rect");
        let icon = snapshot.rect("showMore_icon").expect("icon rect");

        assert!(content.x >= button.x);
        assert!(content.y >= button.y);
        assert!(content.x + content.width <= button.x + button.width + 0.1);
        assert!(content.y + content.height <= button.y + button.height + 0.1);
        assert!(text.x >= content.x);
        assert!(text.y >= content.y);
        assert!(text.x + text.width <= badge.x + 0.1);
        assert!(text.y + text.height <= content.y + content.height + 0.1);
        assert!(badge.x + badge.width <= content.x + content.width + 0.1);
        assert!(badge.y >= content.y);
        assert!(badge.y + badge.height <= content.y + content.height + 0.1);
        assert!(icon.x >= badge.x);
        assert!(icon.y >= badge.y);
        assert!(icon.x + icon.width <= badge.x + badge.width + 0.1);
        assert!(icon.y + icon.height <= badge.y + badge.height + 0.1);
    }

    #[test]
    fn test_show_more_button_expands_for_russian_but_remains_right_aligned() {
        let mut harness = UiHarness::from_project_dir(example_project_dir(), WizardMode::Install)
            .expect("create harness");

        let zh = harness.snapshot_current_page().expect("zh snapshot");
        let zh_button = zh.rect("btnShowMore_wrap").expect("zh button rect");

        harness.switch_language("ru");

        let ru = harness.snapshot_current_page().expect("ru snapshot");
        let button = ru.rect("btnShowMore_wrap").expect("button rect");
        let content = ru.rect("showMore_content").expect("content rect");
        let text = ru.rect("showMore_text").expect("text rect");
        let badge = ru.rect("showMore_badge").expect("badge rect");
        let icon = ru.rect("showMore_icon").expect("icon rect");

        assert!(button.width >= zh_button.width);
        assert!(button.width <= 184.0 + 0.1);
        assert!((content.x + content.width - (button.x + button.width)).abs() <= 0.1);
        assert!(text.x >= button.x);
        assert!(text.y >= button.y);
        assert!(text.y + text.height <= button.y + button.height + 0.1);
        assert!(text.x + text.width <= badge.x + 0.1);
        assert!(badge.x + badge.width <= button.x + button.width + 0.1);
        assert!(icon.x >= badge.x);
        assert!(icon.y >= badge.y);
        assert!(icon.x + icon.width <= badge.x + badge.width + 0.1);
        assert!(icon.y + icon.height <= badge.y + badge.height + 0.1);
    }
}
