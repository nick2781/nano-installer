#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("nano-installer-gui is a Windows-only tool");

use eframe::egui;
use nano_installer_core::{BuildEvent, BuildRequest, BuildResult, BuildStage, ProjectSummary};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

const ACCENT: egui::Color32 = egui::Color32::from_rgb(0, 151, 137);
const ACCENT_DARK: egui::Color32 = egui::Color32::from_rgb(0, 112, 102);
const DISABLED_BG: egui::Color32 = egui::Color32::from_rgb(225, 229, 232);
const PAGE_BG: egui::Color32 = egui::Color32::from_rgb(244, 246, 248);
const PANEL_BG: egui::Color32 = egui::Color32::WHITE;
const BORDER: egui::Color32 = egui::Color32::from_rgb(214, 219, 224);
const PRIMARY_TEXT: egui::Color32 = egui::Color32::from_rgb(31, 39, 47);
const MUTED: egui::Color32 = egui::Color32::from_rgb(91, 101, 112);
const SUCCESS: egui::Color32 = egui::Color32::from_rgb(28, 130, 80);
const WARNING: egui::Color32 = egui::Color32::from_rgb(166, 108, 17);
const ERROR: egui::Color32 = egui::Color32::from_rgb(166, 47, 47);
const ERROR_BG: egui::Color32 = egui::Color32::from_rgb(255, 235, 235);
const PARAM_LABEL_WIDTH: f32 = 142.0;
const PARAM_BROWSE_WIDTH: f32 = 88.0;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 740.0])
            .with_min_inner_size([820.0, 600.0])
            .with_icon(tool_icon()),
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "nano-installer GUI",
        options,
        Box::new(|creation| Ok(Box::new(BuilderApp::new(creation)))),
    ) {
        let _ = rfd::MessageDialog::new()
            .set_title("nano-installer GUI")
            .set_description(format!("Failed to start: {error}"))
            .set_level(rfd::MessageLevel::Error)
            .show();
        std::process::exit(1);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkspaceTab {
    Build,
    Parameters,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiLanguage {
    English,
    SimplifiedChinese,
}

impl UiLanguage {
    fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
        }
    }
}

fn detect_language() -> UiLanguage {
    let locale = windows_ui_locale().unwrap_or_else(|| {
        std::env::var("LANG")
            .or_else(|_| std::env::var("LC_ALL"))
            .unwrap_or_default()
    });
    let locale = locale.to_ascii_lowercase();
    if locale.starts_with("zh") {
        UiLanguage::SimplifiedChinese
    } else {
        UiLanguage::English
    }
}

fn windows_ui_locale() -> Option<String> {
    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    let mut locale = [0u16; LOCALE_NAME_MAX_LENGTH];
    let length = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut locale) };
    if length <= 1 {
        return None;
    }
    Some(String::from_utf16_lossy(&locale[..length as usize - 1]))
}

fn tr(language: UiLanguage, key: &str) -> &str {
    match language {
        UiLanguage::English => match key {
            "file" => "File",
            "build" => "Build",
            "view" => "View",
            "help" => "Help",
            "open_project" => "Open project",
            "open_project_menu" => "Open project...",
            "browse" => "Browse...",
            "dialog_project" => "Select installer project",
            "dialog_output" => "Select setup output",
            "dialog_stub" => "Select native stub directory",
            "dialog_log" => "Save build log",
            "open_config" => "Open config",
            "open_output" => "Open output",
            "refresh" => "Refresh",
            "refresh_project" => "Refresh project",
            "build_setup" => "Build setup",
            "build_log" => "Build log",
            "retry" => "Retry",
            "retry_last" => "Retry last build",
            "clear" => "Clear",
            "copy_all" => "Copy all",
            "save_log" => "Save log...",
            "exit" => "Exit",
            "workspace" => "Build workspace",
            "parameters" => "Parameters",
            "project" => "Project",
            "project_name" => "Project name",
            "version" => "Version",
            "file_version" => "File version",
            "locale" => "Locale",
            "payload" => "Payload",
            "payload_size" => "Payload size",
            "runtime" => "Runtime",
            "icon" => "Icon",
            "no_warnings" => "No asset warnings",
            "warnings" => "Warnings",
            "no_project" => "Choose a project to inspect its configuration.",
            "open_config_file" => "Open installer_config.json",
            "workspace_intro" => "Inspect the project and build the setup.",
            "output" => "Output",
            "not_configured" => "Not configured",
            "custom" => "custom",
            "no_messages" => "No build messages yet.",
            "after_build" => "After build",
            "reveal_after_build" => "Open Explorer and select the generated setup",
            "command_preview" => "Command preview",
            "project_directory" => "Project directory",
            "output_executable" => "Output executable",
            "stub_directory" => "Native stub directory",
            "reset_output" => "Reset output",
            "automatic_stub" => "Use automatic stub search",
            "params_ready" => "Parameters are ready",
            "ready" => "Ready",
            "project_valid" => "Project is valid",
            "validation_failed" => "Validation failed",
            "project_not_ready" => "Project is not ready",
            "refresh_required" => "Refresh required",
            "params_attention" => "Build parameters need attention",
            "build_started" => "Build started",
            "retrying" => "Retrying last build",
            "build_complete" => "Build complete",
            "build_failed_retry" => "Build failed; Retry is available",
            "stage_validating" => "Validating project",
            "stage_stub" => "Selecting runtime stub",
            "stage_packing" => "Packing project resources",
            "stage_resources" => "Writing icon and version resources",
            "native_builder" => "Native x64 builder",
            "no_project_selected" => "No project selected",
            "language" => "Language",
            "project_prefix" => "Project",
            "build_blocked" => "Build blocked",
            "windows_executable" => "Windows executable",
            "text_file" => "Text file",
            "output_empty" => "Output executable path is empty",
            "output_extension" => "Output path must use the .exe extension",
            "stub_missing" => "Stub directory does not exist",
            "tip_open_project" => "Choose a directory containing installer_config.json",
            "tip_refresh" => "Re-read project configuration, layouts, assets, and payload metadata",
            "tip_retry" => "Run the last failed build with the same parameters",
            "tip_open_output" => "Open Explorer and select the generated setup",
            "tip_reset_output" => "Restore the output path from the project configuration",
            "setup_size" => "Setup",
            "bundle_size" => "bundle",
            "validation" => "Validation",
            "configuration_valid" => "Last inspection passed",
            "last_inspected" => "Inspected at",
            "build_warnings" => "Build warnings",
            "versions_locale" => "Versions and locale",
            "setup_locale" => "Setup locale",
            "package_inputs" => "Package inputs",
            "payload_file" => "Payload file",
            "archive_format" => "Format",
            "setup_stub" => "Setup stub",
            "setup_icon" => "Setup icon",
            "uninstaller_file" => "Uninstaller",
            "uninstaller_icon" => "Uninstall icon",
            "requires_admin" => "Administrator rights",
            "requires_admin_yes" => "Requested on start",
            "requires_admin_no" => "Not requested",
            "install_defaults" => "Installation",
            "default_install_path" => "Default folder",
            _ => key,
        },
        UiLanguage::SimplifiedChinese => match key {
            "file" => "文件",
            "build" => "构建",
            "view" => "视图",
            "help" => "帮助",
            "open_project" => "打开项目",
            "open_project_menu" => "打开项目...",
            "browse" => "浏览...",
            "dialog_project" => "选择安装项目",
            "dialog_output" => "选择安装包输出位置",
            "dialog_stub" => "选择 native stub 目录",
            "dialog_log" => "保存构建日志",
            "open_config" => "打开配置",
            "open_output" => "打开产物",
            "refresh" => "刷新",
            "refresh_project" => "刷新项目",
            "build_setup" => "构建安装包",
            "build_log" => "构建日志",
            "retry" => "重试",
            "retry_last" => "重试上次构建",
            "clear" => "清空",
            "copy_all" => "复制全部",
            "save_log" => "保存日志...",
            "exit" => "退出",
            "workspace" => "构建工作区",
            "parameters" => "参数设置",
            "project" => "项目",
            "project_name" => "项目名称",
            "version" => "版本",
            "file_version" => "文件版本",
            "locale" => "语言",
            "payload" => "Payload",
            "payload_size" => "Payload 大小",
            "runtime" => "运行时",
            "icon" => "图标",
            "no_warnings" => "资源检查通过",
            "warnings" => "警告",
            "no_project" => "选择项目后查看配置检查结果。",
            "open_config_file" => "打开 installer_config.json",
            "workspace_intro" => "检查项目并构建安装包。",
            "output" => "输出",
            "not_configured" => "未配置",
            "custom" => "自定义",
            "no_messages" => "还没有构建消息。",
            "after_build" => "构建完成后",
            "reveal_after_build" => "在资源管理器中选中生成的安装包",
            "command_preview" => "命令预览",
            "project_directory" => "项目目录",
            "output_executable" => "输出程序",
            "stub_directory" => "Native stub 目录",
            "reset_output" => "恢复默认输出",
            "automatic_stub" => "使用自动 stub 搜索",
            "params_ready" => "参数已就绪",
            "ready" => "就绪",
            "project_valid" => "项目有效",
            "validation_failed" => "校验失败",
            "project_not_ready" => "项目未就绪",
            "refresh_required" => "需要刷新",
            "params_attention" => "请检查构建参数",
            "build_started" => "开始构建",
            "retrying" => "正在重试上次构建",
            "build_complete" => "构建完成",
            "build_failed_retry" => "构建失败，可重试",
            "stage_validating" => "正在校验项目",
            "stage_stub" => "正在选择运行时 stub",
            "stage_packing" => "正在打包项目资源",
            "stage_resources" => "正在写入图标和版本资源",
            "native_builder" => "Native x64 构建器",
            "no_project_selected" => "未选择项目",
            "language" => "界面语言",
            "project_prefix" => "项目",
            "build_blocked" => "构建已阻止",
            "windows_executable" => "Windows 程序",
            "text_file" => "文本文件",
            "output_empty" => "输出程序路径不能为空",
            "output_extension" => "输出路径必须使用 .exe 扩展名",
            "stub_missing" => "Stub 目录不存在",
            "tip_open_project" => "选择包含 installer_config.json 的目录",
            "tip_refresh" => "重新读取项目配置、布局、资源和 payload 信息",
            "tip_retry" => "使用与上次失败构建完全相同的参数重试",
            "tip_open_output" => "在资源管理器中选中生成的安装包",
            "tip_reset_output" => "恢复项目配置中的默认输出路径",
            "setup_size" => "安装包",
            "bundle_size" => "资源包",
            "validation" => "配置检查",
            "configuration_valid" => "上次校验通过",
            "last_inspected" => "校验时间",
            "build_warnings" => "构建警告",
            "versions_locale" => "版本与语言",
            "setup_locale" => "安装包语言",
            "package_inputs" => "打包资源",
            "payload_file" => "Payload 文件",
            "archive_format" => "归档格式",
            "setup_stub" => "安装 Stub",
            "setup_icon" => "安装图标",
            "uninstaller_file" => "卸载程序",
            "uninstaller_icon" => "卸载图标",
            "requires_admin" => "管理员权限",
            "requires_admin_yes" => "启动时申请",
            "requires_admin_no" => "不申请",
            "install_defaults" => "安装配置",
            "default_install_path" => "默认目录",
            _ => key,
        },
    }
}

fn log_text(key: &str) -> &str {
    tr(UiLanguage::English, key)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatusTone {
    Neutral,
    Working,
    Success,
    Warning,
    Error,
}

struct BuilderApp {
    project_dir: String,
    output_path: String,
    stub_directory: String,
    output_custom: bool,
    project_dirty: bool,
    reveal_after_build: bool,
    tab: WorkspaceTab,
    language: UiLanguage,
    summary: Option<ProjectSummary>,
    last_inspected: Option<String>,
    validation_error: Option<String>,
    building: bool,
    progress: f32,
    status: String,
    status_tone: StatusTone,
    logs: Vec<String>,
    result: Option<BuildResult>,
    last_request: Option<BuildRequest>,
    last_build_failed: bool,
    receiver: Option<Receiver<WorkerMessage>>,
}

enum WorkerMessage {
    Progress(BuildEvent),
    Finished(Box<Result<BuildResult, String>>),
}

impl BuilderApp {
    fn new(creation: &eframe::CreationContext<'_>) -> Self {
        configure_style(&creation.egui_ctx);
        let stub_directory = std::env::var("NANO_INSTALLER_NATIVE_STUB_DIR").unwrap_or_default();
        let language = detect_language();
        Self {
            project_dir: String::new(),
            output_path: String::new(),
            stub_directory,
            output_custom: false,
            project_dirty: false,
            reveal_after_build: true,
            tab: WorkspaceTab::Build,
            language,
            summary: None,
            last_inspected: None,
            validation_error: None,
            building: false,
            progress: 0.0,
            status: tr(language, "ready").to_string(),
            status_tone: StatusTone::Neutral,
            logs: Vec::new(),
            result: None,
            last_request: None,
            last_build_failed: false,
            receiver: None,
        }
    }

    fn text<'a>(&self, key: &'a str) -> &'a str {
        tr(self.language, key)
    }

    fn sync_language_status(&mut self) {
        let key = if self.building {
            if self.progress >= stage_progress(BuildStage::WritingResources) {
                "stage_resources"
            } else if self.progress >= stage_progress(BuildStage::Packing) {
                "stage_packing"
            } else if self.progress >= stage_progress(BuildStage::SelectingStub) {
                "stage_stub"
            } else {
                "stage_validating"
            }
        } else if self.last_build_failed {
            "build_failed_retry"
        } else if self.project_dirty {
            "refresh_required"
        } else if self.result.is_some() {
            "build_complete"
        } else if self.validation_error.is_some() {
            "validation_failed"
        } else if self.summary.is_some() {
            "project_valid"
        } else {
            "ready"
        };
        self.status = self.text(key).to_string();
    }

    fn validate_project(&mut self) {
        if self.building {
            return;
        }
        self.result = None;
        self.last_build_failed = false;
        let project = PathBuf::from(self.project_dir.trim());
        let replace_output = should_replace_output(
            self.summary
                .as_ref()
                .map(|summary| summary.project_dir.as_path()),
            &project,
            &self.output_path,
        );
        match nano_installer_core::inspect_project(&project) {
            Ok(summary) => {
                if replace_output {
                    self.output_path = summary.output_path.display().to_string();
                    self.output_custom = false;
                }
                self.summary = Some(summary.clone());
                self.last_inspected = Some(inspection_time());
                self.validation_error = None;
                self.project_dirty = false;
                self.set_status(self.text("project_valid"), StatusTone::Success);
                self.append_log(format!("Project inspection passed: {}", project.display()));
                for warning in summary.warnings {
                    self.append_log(format!("Warning: {warning}"));
                }
            }
            Err(error) => {
                self.summary = None;
                self.last_inspected = None;
                self.validation_error = Some(format!("{error:#}"));
                self.set_status(self.text("validation_failed"), StatusTone::Error);
                self.append_log(format!("Validation failed: {error:#}"));
            }
        }
    }

    fn make_request(&self) -> Result<BuildRequest, String> {
        if self.summary.is_none() || self.project_dirty {
            return Err(self.text("refresh_required").to_string());
        }
        if let Some(error) = self.parameter_error() {
            return Err(error);
        }
        let mut request = BuildRequest::new(PathBuf::from(self.project_dir.trim()));
        if !self.output_path.trim().is_empty() {
            request.output = Some(PathBuf::from(self.output_path.trim()));
        }
        if !self.stub_directory.trim().is_empty() {
            request.stub_directory = Some(PathBuf::from(self.stub_directory.trim()));
        }
        Ok(request)
    }

    fn start_build(&mut self, context: &egui::Context) {
        if self.summary.is_none() || self.project_dirty {
            self.validate_project();
        }
        if self.summary.is_none() || self.building {
            return;
        }
        let request = match self.make_request() {
            Ok(request) => request,
            Err(error) => {
                self.validation_error = Some(error.clone());
                self.set_status(self.text("params_attention"), StatusTone::Error);
                let log_error = if self.summary.is_none() || self.project_dirty {
                    log_text("refresh_required").to_string()
                } else {
                    self.parameter_error_for(UiLanguage::English)
                        .unwrap_or(error)
                };
                self.append_log(format!("{}: {log_error}", log_text("build_blocked")));
                return;
            }
        };
        self.launch_build(request, context, "build_started");
    }

    fn retry_build(&mut self, context: &egui::Context) {
        let Some(request) = self.last_request.clone() else {
            return;
        };
        if !self.last_build_failed || self.building {
            return;
        }
        self.launch_build(request, context, "retrying");
    }

    fn launch_build(&mut self, request: BuildRequest, context: &egui::Context, status_key: &str) {
        let (sender, receiver) = mpsc::channel();
        let repaint = context.clone();
        let worker_request = request.clone();
        std::thread::spawn(move || {
            let progress_sender = sender.clone();
            let result =
                nano_installer_core::build_project_with_progress(worker_request, |event| {
                    let _ = progress_sender.send(WorkerMessage::Progress(event));
                    repaint.request_repaint();
                })
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(WorkerMessage::Finished(Box::new(result)));
            repaint.request_repaint();
        });
        self.last_request = Some(request);
        self.receiver = Some(receiver);
        self.building = true;
        self.progress = 0.05;
        self.result = None;
        self.validation_error = None;
        self.last_build_failed = false;
        self.set_status(self.text(status_key), StatusTone::Working);
        self.append_log(log_text(status_key).to_string());
    }

    fn poll_worker(&mut self) {
        let messages = self
            .receiver
            .as_ref()
            .map(|receiver| receiver.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for message in messages {
            match message {
                WorkerMessage::Progress(event) => {
                    self.progress = self.progress.max(stage_progress(event.stage));
                    self.set_status(
                        stage_status(self.language, event.stage),
                        StatusTone::Working,
                    );
                    self.append_log(event.message);
                }
                WorkerMessage::Finished(result) => {
                    self.building = false;
                    self.receiver = None;
                    match *result {
                        Ok(result) => {
                            self.progress = 1.0;
                            self.set_status(self.text("build_complete"), StatusTone::Success);
                            self.append_log(format!(
                                "Output: {} ({})",
                                result.summary.output_path.display(),
                                format_bytes(result.output_size)
                            ));
                            self.result = Some(result);
                            if self.reveal_after_build {
                                self.reveal_output();
                            }
                        }
                        Err(error) => {
                            self.progress = 0.0;
                            self.last_build_failed = true;
                            self.validation_error = Some(error.clone());
                            self.set_status(self.text("build_failed_retry"), StatusTone::Error);
                            self.append_log(format!("Error: {error}"));
                        }
                    }
                }
            }
        }
    }

    fn browse_project(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title(self.text("dialog_project"));
        let current = PathBuf::from(self.project_dir.trim());
        if current.is_dir() {
            dialog = dialog.set_directory(current);
        }
        if let Some(path) = dialog.pick_folder() {
            self.project_dir = path.display().to_string();
            self.project_dirty = true;
            self.validate_project();
        }
    }

    fn browse_output(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title(self.text("dialog_output"))
            .add_filter(self.text("windows_executable"), &["exe"]);
        if let Some(summary) = &self.summary {
            if let Some(parent) = summary.output_path.parent() {
                dialog = dialog.set_directory(parent);
            }
            if let Some(name) = summary
                .output_path
                .file_name()
                .and_then(|name| name.to_str())
            {
                dialog = dialog.set_file_name(name);
            }
        }
        if let Some(path) = dialog.save_file() {
            self.output_path = path.display().to_string();
            self.output_custom = true;
            self.result = None;
        }
    }

    fn browse_stub_directory(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title(self.text("dialog_stub"));
        let current = PathBuf::from(self.stub_directory.trim());
        if current.is_dir() {
            dialog = dialog.set_directory(current);
        }
        if let Some(path) = dialog.pick_folder() {
            self.stub_directory = path.display().to_string();
            self.result = None;
        }
    }

    fn reveal_output(&mut self) {
        let Some(result) = &self.result else {
            return;
        };
        let output = &result.summary.output_path;
        match std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", output.display()))
            .spawn()
        {
            Ok(_) => self.append_log("Opened output location".to_string()),
            Err(error) => self.append_log(format!("Failed to open output location: {error}")),
        }
    }

    fn open_project_config(&mut self) {
        let config = PathBuf::from(self.project_dir.trim()).join("installer_config.json");
        match std::process::Command::new("explorer.exe")
            .arg(config)
            .spawn()
        {
            Ok(_) => self.append_log("Opened installer_config.json".to_string()),
            Err(error) => self.append_log(format!("Failed to open project config: {error}")),
        }
    }

    fn save_log(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title(self.text("dialog_log"))
            .add_filter(self.text("text_file"), &["txt"])
            .set_file_name("nano-installer-build.log");
        if let Some(summary) = &self.summary {
            if let Some(parent) = summary.output_path.parent() {
                dialog = dialog.set_directory(parent);
            }
        }
        if let Some(path) = dialog.save_file() {
            match std::fs::write(&path, self.logs.join("\r\n")) {
                Ok(()) => self.append_log(format!("Saved build log to {}", path.display())),
                Err(error) => self.append_log(format!("Failed to save build log: {error}")),
            }
        }
    }

    fn append_log(&mut self, line: String) {
        const MAX_LOG_LINES: usize = 2_000;
        self.logs.push(format!("{} {line}", log_timestamp()));
        if self.logs.len() > MAX_LOG_LINES {
            let remove = self.logs.len() - MAX_LOG_LINES;
            self.logs.drain(0..remove);
        }
    }

    fn parameter_error(&self) -> Option<String> {
        self.parameter_error_for(self.language)
    }

    fn parameter_error_for(&self, language: UiLanguage) -> Option<String> {
        if self.output_path.trim().is_empty() {
            return Some(tr(language, "output_empty").to_string());
        }
        if Path::new(self.output_path.trim())
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| !extension.eq_ignore_ascii_case("exe"))
            .unwrap_or(true)
        {
            return Some(tr(language, "output_extension").to_string());
        }
        if !self.stub_directory.trim().is_empty() && !Path::new(self.stub_directory.trim()).is_dir()
        {
            return Some(tr(language, "stub_missing").to_string());
        }
        None
    }

    fn command_preview(&self) -> String {
        let mut command = format!(
            "nano-installer-native-x64.exe build --project {} --output {}",
            quote_arg(self.project_dir.trim()),
            quote_arg(self.output_path.trim())
        );
        if !self.stub_directory.trim().is_empty() {
            command.push_str(" --stubs ");
            command.push_str(&quote_arg(self.stub_directory.trim()));
        }
        command
    }

    fn set_status(&mut self, status: impl Into<String>, tone: StatusTone) {
        self.status = status.into();
        self.status_tone = tone;
    }

    fn show_menu(&mut self, context: &egui::Context, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(self.text("file"), |ui| {
                configure_menu(ui, 220.0);
                if ui
                    .add_enabled(
                        !self.building,
                        egui::Button::new(self.text("open_project_menu")),
                    )
                    .clicked()
                {
                    ui.close();
                    self.browse_project();
                }
                if ui
                    .add_enabled(
                        self.summary.is_some() && !self.building,
                        egui::Button::new(self.text("open_config")),
                    )
                    .clicked()
                {
                    ui.close();
                    self.open_project_config();
                }
                if ui
                    .add_enabled(
                        self.result.is_some(),
                        egui::Button::new(self.text("open_output")),
                    )
                    .clicked()
                {
                    ui.close();
                    self.reveal_output();
                }
                ui.separator();
                if ui.button(self.text("exit")).clicked() {
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(self.text("build"), |ui| {
                configure_menu(ui, 220.0);
                if ui
                    .add_enabled(
                        self.summary.is_some() && !self.building,
                        egui::Button::new(self.text("refresh_project")),
                    )
                    .clicked()
                {
                    ui.close();
                    self.validate_project();
                }
                if ui
                    .add_enabled(
                        self.summary.is_some() && !self.building,
                        egui::Button::new(self.text("build_setup")),
                    )
                    .clicked()
                {
                    ui.close();
                    self.start_build(context);
                }
                if ui
                    .add_enabled(
                        self.last_build_failed && !self.building,
                        egui::Button::new(self.text("retry_last")),
                    )
                    .clicked()
                {
                    ui.close();
                    self.retry_build(context);
                }
            });
            ui.menu_button(self.text("view"), |ui| {
                configure_menu(ui, 220.0);
                if ui.button(self.text("workspace")).clicked() {
                    ui.close();
                    self.tab = WorkspaceTab::Build;
                }
                if ui.button(self.text("parameters")).clicked() {
                    ui.close();
                    self.tab = WorkspaceTab::Parameters;
                }
                if ui.button(self.text("clear")).clicked() {
                    ui.close();
                    self.logs.clear();
                }
                ui.separator();
                let language_before = self.language;
                ui.menu_button(self.text("language"), |ui| {
                    configure_menu(ui, 150.0);
                    ui.selectable_value(
                        &mut self.language,
                        UiLanguage::English,
                        UiLanguage::English.label(),
                    );
                    ui.selectable_value(
                        &mut self.language,
                        UiLanguage::SimplifiedChinese,
                        UiLanguage::SimplifiedChinese.label(),
                    );
                });
                if self.language != language_before {
                    self.sync_language_status();
                }
            });
            ui.menu_button(self.text("help"), |ui| {
                configure_menu(ui, 280.0);
                ui.label("nano-installer GUI");
                ui.label(format!(
                    "{} {}",
                    self.text("version"),
                    env!("CARGO_PKG_VERSION")
                ));
                ui.label(self.text("native_builder"));
            });
        });
    }

    fn show_project_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(self.text("project"))
                    .size(15.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !self.building && !self.project_dir.trim().is_empty(),
                        egui::Button::new(self.text("refresh")),
                    )
                    .on_hover_text(self.text("tip_refresh"))
                    .clicked()
                {
                    self.validate_project();
                }
            });
        });
        ui.add_space(8.0);
        if let Some(summary) = self.summary.clone() {
            ui.label(
                egui::RichText::new(&summary.project_name)
                    .size(20.0)
                    .strong(),
            );
            let project_path = summary.project_dir.display().to_string();
            ui.add(
                egui::Label::new(
                    egui::RichText::new(compact_path(&summary.project_dir, 2))
                        .size(12.0)
                        .color(MUTED),
                )
                .truncate(),
            )
            .on_hover_text(&project_path);

            sidebar_section(ui, self.text("validation"));
            status_line(ui, StatusTone::Success, self.text("configuration_valid"));
            if let Some(time) = &self.last_inspected {
                sidebar_row(ui, self.text("last_inspected"), time, None);
            }
            sidebar_row(
                ui,
                self.text("build_warnings"),
                &summary.warnings.len().to_string(),
                None,
            );
            for warning in &summary.warnings {
                ui.add(
                    egui::Label::new(egui::RichText::new(warning).small().color(WARNING))
                        .wrap_mode(egui::TextWrapMode::Wrap),
                )
                .on_hover_text(warning);
            }

            sidebar_section(ui, self.text("versions_locale"));
            sidebar_row(ui, self.text("version"), &summary.project_version, None);
            sidebar_row(ui, self.text("file_version"), &summary.file_version, None);
            sidebar_row(ui, self.text("setup_locale"), &summary.default_locale, None);

            sidebar_section(ui, self.text("package_inputs"));
            let payload_path = summary.payload_path.display().to_string();
            let payload_file = file_label(&summary.payload_path, self.text("not_configured"));
            sidebar_row(
                ui,
                self.text("payload_file"),
                payload_file,
                Some(&payload_path),
            );
            sidebar_row(
                ui,
                self.text("payload_size"),
                &format_bytes(summary.payload_size),
                None,
            );
            sidebar_row(
                ui,
                self.text("archive_format"),
                summary.payload_format.label(),
                None,
            );
            sidebar_row(
                ui,
                self.text("setup_stub"),
                summary.payload_format.stub_name(),
                None,
            );
            sidebar_row(
                ui,
                self.text("setup_icon"),
                summary
                    .installer_icon
                    .as_ref()
                    .map(|path| file_label(path, self.text("not_configured")))
                    .unwrap_or_else(|| self.text("not_configured")),
                None,
            );
            sidebar_row(
                ui,
                self.text("uninstaller_file"),
                &summary.uninstaller_name,
                None,
            );
            sidebar_row(
                ui,
                self.text("uninstaller_icon"),
                summary
                    .uninstaller_icon
                    .as_ref()
                    .map(|path| file_label(path, self.text("not_configured")))
                    .unwrap_or_else(|| self.text("not_configured")),
                None,
            );

            sidebar_section(ui, self.text("install_defaults"));
            sidebar_row(
                ui,
                self.text("requires_admin"),
                if summary.require_admin {
                    self.text("requires_admin_yes")
                } else {
                    self.text("requires_admin_no")
                },
                None,
            );
            if let Some(path) = &summary.default_install_path {
                sidebar_row(
                    ui,
                    self.text("default_install_path"),
                    &compact_path(Path::new(path), 1),
                    Some(path),
                );
            } else {
                sidebar_row(
                    ui,
                    self.text("default_install_path"),
                    self.text("not_configured"),
                    None,
                );
            }
        } else if let Some(error) = &self.validation_error {
            status_line(ui, StatusTone::Error, self.text("validation_failed"));
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(egui::RichText::new(error).size(12.0).color(ERROR))
                    .wrap_mode(egui::TextWrapMode::Wrap),
            );
        } else {
            ui.label(egui::RichText::new(self.text("no_project")).color(MUTED));
            ui.add_space(10.0);
            if ui
                .add_enabled(!self.building, egui::Button::new(self.text("open_project")))
                .clicked()
            {
                self.browse_project();
            }
        }
        let config = PathBuf::from(self.project_dir.trim()).join("installer_config.json");
        if !self.project_dir.trim().is_empty() && config.is_file() {
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(122.0, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new("installer_config.json")
                                    .monospace()
                                    .size(12.0)
                                    .color(MUTED),
                            )
                            .truncate()
                            .halign(egui::Align::LEFT),
                        )
                        .on_hover_text("installer_config.json");
                    },
                );
                if ui
                    .add_enabled(!self.building, egui::Button::new(self.text("open_config")))
                    .clicked()
                {
                    self.open_project_config();
                }
            });
        }
    }

    fn show_build_tab(&mut self, context: &egui::Context, ui: &mut egui::Ui) {
        ui.heading(self.text("workspace"));
        ui.label(egui::RichText::new(self.text("workspace_intro")).color(MUTED));
        ui.add_space(14.0);

        egui::Frame::new()
            .fill(PANEL_BG)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .corner_radius(6)
            .inner_margin(14)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(self.text("output")).strong());
                    ui.label(
                        egui::RichText::new(if self.output_path.trim().is_empty() {
                            self.text("not_configured")
                        } else {
                            self.output_path.trim()
                        })
                        .color(MUTED),
                    );
                    if self.output_custom {
                        ui.label(
                            egui::RichText::new(self.text("custom"))
                                .small()
                                .color(ACCENT_DARK),
                        );
                    }
                });
                ui.add_space(8.0);
                if self.building || self.result.is_some() {
                    ui.add(
                        egui::ProgressBar::new(self.progress)
                            .desired_width(ui.available_width())
                            .text(format!("{}  {:.0}%", self.status, self.progress * 100.0)),
                    );
                } else {
                    status_line(ui, self.status_tone, &self.status);
                }
            });
        ui.add_space(12.0);
        let log_text = if self.logs.is_empty() {
            self.text("no_messages").to_string()
        } else {
            self.logs.join("\r\n")
        };
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(self.text("build_log"))
                    .size(16.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.text("clear")).clicked() {
                    self.logs.clear();
                }
                if ui.button(self.text("save_log")).clicked() {
                    self.save_log();
                }
                if ui.button(self.text("copy_all")).clicked() {
                    ui.ctx().copy_text(log_text.clone());
                }
            });
        });
        ui.add_space(6.0);
        let log_height = (ui.available_height() - 76.0).clamp(180.0, 360.0);
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(30, 35, 40))
            .corner_radius(5)
            .inner_margin(10)
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt("build_log")
                    .auto_shrink([false, false])
                    .max_height(log_height)
                    .stick_to_bottom(true)
                    .scroll_source(egui::scroll_area::ScrollSource {
                        drag: false,
                        ..Default::default()
                    })
                    .show(ui, |ui| {
                        let mut read_only_log = log_text.as_str();
                        let mut layouter = |ui: &egui::Ui,
                                            buffer: &dyn egui::TextBuffer,
                                            _: f32| {
                            ui.fonts_mut(|fonts| fonts.layout_job(highlight_log(buffer.as_str())))
                        };
                        ui.add(
                            egui::TextEdit::multiline(&mut read_only_log)
                                .font(egui::TextStyle::Monospace)
                                .layouter(&mut layouter)
                                .desired_width(f32::INFINITY)
                                .desired_rows(self.logs.len().max(1))
                                .frame(false),
                        );
                    });
            });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let can_build = self.summary.is_some() && !self.building;
            let build = ui.add_enabled(
                can_build,
                egui::Button::new(egui::RichText::new(self.text("build_setup")).strong())
                    .fill(if can_build { ACCENT } else { DISABLED_BG })
                    .min_size(egui::vec2(124.0, 34.0)),
            );
            if build.clicked() {
                self.start_build(context);
            }
            let retry = ui.add_enabled(
                self.last_build_failed && !self.building,
                egui::Button::new(self.text("retry_last")).min_size(egui::vec2(124.0, 34.0)),
            );
            if retry.clicked() {
                self.retry_build(context);
            }
            let output = ui.add_enabled(
                self.result.is_some(),
                egui::Button::new(self.text("open_output")).min_size(egui::vec2(112.0, 34.0)),
            );
            if output.clicked() {
                self.reveal_output();
            }
        });
        if let Some(result) = &self.result {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} {} | {} {}",
                    self.text("setup_size"),
                    format_bytes(result.output_size),
                    self.text("bundle_size"),
                    format_bytes(result.bundle_size)
                ))
                .size(12.0)
                .color(MUTED),
            );
        }
    }

    fn show_parameters_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("parameters"));
        ui.add_space(12.0);
        let field_width = parameter_field_width(ui.available_width(), ui.spacing().item_spacing.x);
        ui.horizontal(|ui| {
            parameter_label(ui, self.text("project_directory"));
            let response = ui
                .add_enabled_ui(!self.building, |ui| {
                    ui.add_sized(
                        [field_width, 30.0],
                        egui::TextEdit::singleline(&mut self.project_dir),
                    )
                })
                .inner;
            if response.changed() {
                self.project_dirty = true;
                self.summary = None;
                self.last_inspected = None;
                self.validation_error = None;
                self.set_status(self.text("refresh_required"), StatusTone::Warning);
            }
            if ui
                .add_enabled_ui(!self.building, |ui| {
                    ui.add_sized(
                        [PARAM_BROWSE_WIDTH, 30.0],
                        egui::Button::new(self.text("browse")),
                    )
                })
                .inner
                .clicked()
            {
                self.browse_project();
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            parameter_label(ui, self.text("output_executable"));
            let response = ui
                .add_enabled_ui(!self.building, |ui| {
                    ui.add_sized(
                        [field_width, 30.0],
                        egui::TextEdit::singleline(&mut self.output_path),
                    )
                })
                .inner;
            if response.changed() {
                self.output_custom = true;
                self.result = None;
            }
            if ui
                .add_enabled_ui(!self.building, |ui| {
                    ui.add_sized(
                        [PARAM_BROWSE_WIDTH, 30.0],
                        egui::Button::new(self.text("browse")),
                    )
                })
                .inner
                .clicked()
            {
                self.browse_output();
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            parameter_label(ui, self.text("stub_directory"));
            let response = ui
                .add_enabled_ui(!self.building, |ui| {
                    ui.add_sized(
                        [field_width, 30.0],
                        egui::TextEdit::singleline(&mut self.stub_directory),
                    )
                })
                .inner;
            if response.changed() {
                self.result = None;
            }
            if ui
                .add_enabled_ui(!self.building, |ui| {
                    ui.add_sized(
                        [PARAM_BROWSE_WIDTH, 30.0],
                        egui::Button::new(self.text("browse")),
                    )
                })
                .inner
                .clicked()
            {
                self.browse_stub_directory();
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.building, egui::Button::new(self.text("reset_output")))
                .on_hover_text(self.text("tip_reset_output"))
                .clicked()
            {
                if let Some(summary) = &self.summary {
                    self.output_path = summary.output_path.display().to_string();
                    self.output_custom = false;
                    self.result = None;
                }
            }
            if ui
                .add_enabled(
                    !self.building && !self.stub_directory.trim().is_empty(),
                    egui::Button::new(self.text("automatic_stub")),
                )
                .clicked()
            {
                self.stub_directory.clear();
                self.result = None;
            }
        });
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(10.0);
        let reveal_label = self.text("reveal_after_build");
        ui.label(egui::RichText::new(self.text("after_build")).strong());
        ui.checkbox(&mut self.reveal_after_build, reveal_label);
        ui.add_space(14.0);
        ui.label(egui::RichText::new(self.text("command_preview")).strong());
        let preview = self.command_preview();
        let mut read_only_preview = preview.as_str();
        ui.add_sized(
            [ui.available_width(), 92.0],
            egui::TextEdit::multiline(&mut read_only_preview)
                .font(egui::TextStyle::Monospace)
                .desired_rows(3),
        );
        ui.add_space(12.0);
        if let Some(error) = self.parameter_error() {
            egui::Frame::new()
                .fill(ERROR_BG)
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(220, 125, 125),
                ))
                .corner_radius(5)
                .inner_margin(9)
                .show(ui, |ui| {
                    ui.colored_label(ERROR, error);
                });
        } else {
            ui.colored_label(SUCCESS, self.text("params_ready"));
        }
    }
}

impl eframe::App for BuilderApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker();
        egui::TopBottomPanel::top("menu")
            .exact_height(28.0)
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(4))
            .show(context, |ui| self.show_menu(context, ui));
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(30.0)
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(8))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    status_line(ui, self.status_tone, &self.status);
                    if self.project_dirty {
                        ui.label(egui::RichText::new(self.text("refresh_required")).color(WARNING));
                    }
                });
            });
        egui::SidePanel::left("project_panel")
            .resizable(true)
            .default_width(310.0)
            .width_range(260.0..=380.0)
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(16))
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| self.show_project_panel(ui));
            });
        let workspace_label = self.text("workspace");
        let parameters_label = self.text("parameters");
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(PAGE_BG).inner_margin(20))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            [150.0, 30.0],
                            egui::Button::new(workspace_label)
                                .selected(self.tab == WorkspaceTab::Build),
                        )
                        .clicked()
                    {
                        self.tab = WorkspaceTab::Build;
                    }
                    if ui
                        .add_sized(
                            [120.0, 30.0],
                            egui::Button::new(parameters_label)
                                .selected(self.tab == WorkspaceTab::Parameters),
                        )
                        .clicked()
                    {
                        self.tab = WorkspaceTab::Parameters;
                    }
                });
                ui.separator();
                ui.add_space(12.0);
                match self.tab {
                    WorkspaceTab::Build => self.show_build_tab(context, ui),
                    WorkspaceTab::Parameters => self.show_parameters_tab(ui),
                }
            });
    }
}

fn configure_menu(ui: &mut egui::Ui, min_width: f32) {
    ui.set_min_width(min_width);
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
}

fn log_timestamp() -> String {
    let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format_log_timestamp(
        time.wYear,
        time.wMonth,
        time.wDay,
        time.wHour,
        time.wMinute,
        time.wSecond,
        time.wMilliseconds,
    )
}

fn inspection_time() -> String {
    let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format_inspection_time(time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute)
}

fn format_inspection_time(year: u16, month: u16, day: u16, hour: u16, minute: u16) -> String {
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
}

fn format_log_timestamp(
    year: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    millisecond: u16,
) -> String {
    format!("[{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millisecond:03}]")
}

fn highlight_log(log: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    for line in log.split_inclusive('\n') {
        let message = line
            .split_once("] ")
            .map(|(_, message)| message)
            .unwrap_or(line);
        let color = if message.starts_with("Error:")
            || message.starts_with("Build blocked:")
            || message.starts_with("Failed")
            || message.starts_with("Validation failed")
        {
            egui::Color32::from_rgb(255, 125, 135)
        } else if message.starts_with("Warning:") {
            egui::Color32::from_rgb(255, 204, 105)
        } else {
            egui::Color32::from_rgb(216, 225, 230)
        };
        job.append(
            line,
            0.0,
            egui::TextFormat::simple(egui::FontId::monospace(14.0), color),
        );
    }
    job
}

fn status_line(ui: &mut egui::Ui, tone: StatusTone, text: &str) {
    let color = match tone {
        StatusTone::Neutral => MUTED,
        StatusTone::Working => ACCENT_DARK,
        StatusTone::Success => SUCCESS,
        StatusTone::Warning => WARNING,
        StatusTone::Error => ERROR,
    };
    ui.colored_label(color, format!("● {text}"));
}

fn sidebar_section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(14.0);
    ui.separator();
    ui.add_space(7.0);
    ui.label(
        egui::RichText::new(label)
            .size(14.0)
            .strong()
            .color(PRIMARY_TEXT),
    );
    ui.add_space(5.0);
}

fn sidebar_row(ui: &mut egui::Ui, label: &str, value: &str, tooltip: Option<&str>) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(92.0, 20.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(label).size(12.0).color(MUTED))
                        .truncate()
                        .halign(egui::Align::LEFT),
                )
                .on_hover_text(label);
            },
        );
        let available = ui.available_width().max(24.0);
        ui.allocate_ui_with_layout(
            egui::vec2(available, 20.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(value).size(13.0).color(PRIMARY_TEXT))
                        .truncate()
                        .halign(egui::Align::LEFT),
                )
                .on_hover_text(tooltip.unwrap_or(value));
            },
        );
    });
}

fn parameter_label(ui: &mut egui::Ui, label: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(PARAM_LABEL_WIDTH, 30.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(label).strong())
                    .truncate()
                    .halign(egui::Align::LEFT),
            )
            .on_hover_text(label);
        },
    );
}

fn parameter_field_width(available: f32, item_spacing: f32) -> f32 {
    (available - PARAM_LABEL_WIDTH - PARAM_BROWSE_WIDTH - 2.0 * item_spacing).max(120.0)
}

fn file_label<'a>(path: &'a Path, fallback: &'a str) -> &'a str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(fallback)
}

fn compact_path(path: &Path, tail_components: usize) -> String {
    use std::path::Component;

    let full = path.display().to_string();
    if full.chars().count() <= 24 {
        return full;
    }
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return full;
    };
    let names = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if names.is_empty() {
        return full;
    }
    let tail = names
        .iter()
        .rev()
        .take(tail_components)
        .rev()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\\");
    format!("{}\\...\\{tail}", prefix.as_os_str().to_string_lossy())
}

fn stage_progress(stage: BuildStage) -> f32 {
    match stage {
        BuildStage::Validating => 0.12,
        BuildStage::SelectingStub => 0.28,
        BuildStage::Packing => 0.52,
        BuildStage::WritingResources => 0.82,
        BuildStage::Complete => 1.0,
    }
}

fn stage_status(language: UiLanguage, stage: BuildStage) -> &'static str {
    match stage {
        BuildStage::Validating => tr(language, "stage_validating"),
        BuildStage::SelectingStub => tr(language, "stage_stub"),
        BuildStage::Packing => tr(language, "stage_packing"),
        BuildStage::WritingResources => tr(language, "stage_resources"),
        BuildStage::Complete => tr(language, "build_complete"),
    }
}

fn should_replace_output(validated: Option<&Path>, project: &Path, output: &str) -> bool {
    validated.is_none_or(|validated| validated != project) || output.trim().is_empty()
}

fn quote_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn configure_style(context: &egui::Context) {
    configure_fonts(context);
    let mut visuals = egui::Visuals::light();
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke.color = egui::Color32::WHITE;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.window_corner_radius = 6.into();
    context.set_visuals(visuals);
    context.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(11.0, 6.0);
    });
}

fn configure_fonts(context: &egui::Context) {
    let Some(windows_dir) = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SystemRoot"))
    else {
        return;
    };
    let font_dir = PathBuf::from(windows_dir).join("Fonts");
    let font_data = ["msyh.ttc", "msyh.ttf", "simsun.ttc"]
        .into_iter()
        .find_map(|name| std::fs::read(font_dir.join(name)).ok());
    let Some(font_data) = font_data else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    let font_name = "system-cjk".to_string();
    fonts.font_data.insert(
        font_name.clone(),
        std::sync::Arc::new(egui::FontData::from_owned(font_data)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(fonts) = fonts.families.get_mut(&family) {
            fonts.push(font_name.clone());
        }
    }
    context.set_fonts(fonts);
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * KIB;
    const GIB: f64 = 1024.0 * MIB;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}

fn tool_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/nano-technology.png"))
        .expect("decode nano-installer window icon")
}

#[cfg(test)]
mod tests {
    use super::{
        compact_path, configure_menu, format_bytes, format_inspection_time, format_log_timestamp,
        highlight_log, log_text, parameter_field_width, quote_arg, should_replace_output,
        stage_progress, tool_icon, tr, BuilderApp, StatusTone, UiLanguage, WorkerMessage,
        WorkspaceTab,
    };
    use eframe::egui;
    use nano_installer_core::{BuildEvent, BuildRequest, BuildResult, BuildStage};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn formats_gui_sizes() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
    }

    #[test]
    fn build_progress_is_monotonic() {
        let stages = [
            BuildStage::Validating,
            BuildStage::SelectingStub,
            BuildStage::Packing,
            BuildStage::WritingResources,
            BuildStage::Complete,
        ];
        assert!(stages
            .windows(2)
            .all(|pair| stage_progress(pair[0]) < stage_progress(pair[1])));
    }

    #[test]
    fn validation_preserves_custom_output_for_same_project() {
        let project = std::path::Path::new("C:\\project");
        assert!(!should_replace_output(
            Some(project),
            project,
            "C:\\custom\\setup.exe"
        ));
        assert!(should_replace_output(Some(project), project, ""));
        assert!(should_replace_output(
            Some(project),
            std::path::Path::new("C:\\other"),
            "C:\\custom\\setup.exe"
        ));
    }

    #[test]
    fn command_arguments_are_quoted() {
        assert_eq!(
            quote_arg("C:\\Program Files\\TapTap"),
            "\"C:\\Program Files\\TapTap\""
        );
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn log_timestamp_has_fixed_width() {
        assert_eq!(
            format_log_timestamp(2026, 9, 15, 7, 5, 9, 42),
            "[2026-09-15 07:05:09.042]"
        );
    }

    #[test]
    fn log_severity_colors_preserve_selectable_text() {
        let log =
            "[2026-09-15 11:00:00.000] Warning: check\n[2026-09-15 11:00:01.000] Error: failed\n";
        let job = highlight_log(log);
        assert_eq!(job.text, log);
        assert_eq!(job.sections.len(), 2);
        assert_ne!(job.sections[0].format.color, job.sections[1].format.color);
    }

    #[test]
    fn log_commands_are_english_when_interface_is_chinese() {
        assert_eq!(
            tr(UiLanguage::SimplifiedChinese, "build_started"),
            "开始构建"
        );
        assert_eq!(log_text("build_started"), "Build started");
        assert_eq!(log_text("retrying"), "Retrying last build");
        assert_eq!(log_text("refresh_required"), "Refresh required");
    }

    #[test]
    fn sidebar_paths_keep_drive_and_relevant_tail() {
        assert_eq!(
            compact_path(
                std::path::Path::new("D:\\taptap-pc\\nano-installer\\examples\\TapTap"),
                2
            ),
            "D:\\...\\examples\\TapTap"
        );
        assert_eq!(
            compact_path(std::path::Path::new("C:\\Program Files\\TapTapTest"), 1),
            "C:\\...\\TapTapTest"
        );
        assert_eq!(
            compact_path(std::path::Path::new("C:\\projects\\App"), 2),
            "C:\\projects\\App"
        );
    }

    #[test]
    fn inspection_time_is_explicit_and_stable() {
        assert_eq!(
            format_inspection_time(2026, 9, 15, 13, 4),
            "2026-09-15 13:04"
        );
    }

    #[test]
    fn parameter_fields_use_the_available_work_area() {
        assert_eq!(parameter_field_width(900.0, 8.0), 654.0);
        assert_eq!(parameter_field_width(400.0, 8.0), 154.0);
    }

    #[test]
    fn startup_waits_for_project_selection() {
        let creation = eframe::CreationContext::_new_kittest(eframe::egui::Context::default());
        let app = BuilderApp::new(&creation);
        assert!(app.project_dir.is_empty());
        assert!(app.output_path.is_empty());
        assert!(app.summary.is_none());
        assert!(app.logs.is_empty());
        assert!(app.result.is_none());
    }

    /// Builds the window state without a desktop session.
    ///
    /// The app is created through the constructor eframe keeps for tests, so a
    /// case drives the state the window holds and never opens one.
    fn headless_app() -> BuilderApp {
        let creation = eframe::CreationContext::_new_kittest(egui::Context::default());
        BuilderApp::new(&creation)
    }

    /// A project folder on disk, removed when the case ends.
    ///
    /// The window reads a project rather than writing one, so a case needs the
    /// tree the inspection walks: `installer_config.json` beside the resource
    /// directories, a first-page layout, a default locale file, and a payload
    /// whose signature names the archive format.
    struct TestProject {
        path: PathBuf,
    }

    impl TestProject {
        fn create(label: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "nano-installer-gui-{}-{}-{label}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_dir_all(&path);
            for directory in ["layouts", "assets", "locales", "payload", "stubs"] {
                std::fs::create_dir_all(path.join(directory)).expect("project directory tree");
            }
            std::fs::write(path.join("locales/en-US.json"), "{}").expect("default locale file");
            std::fs::write(
                path.join("layouts/configpage.xml"),
                r#"<Page width="720" height="450"><Label text="Probe" /></Page>"#,
            )
            .expect("first page layout");
            // Six bytes is the shortest payload the inspection accepts: it
            // reads the signature and stops there, so no case needs an archive.
            std::fs::write(path.join("payload/app.zip"), b"PK\x03\x04\x00\x00").expect("payload");
            let project = Self { path };
            project.write_config("Probe", "Probe_Setup.exe", &["en-US"]);
            project
        }

        /// Writes the project file, which is where every name and default the
        /// window shows afterwards comes from.
        fn write_config(&self, name: &str, installer_name: &str, supported_locales: &[&str]) {
            let locales = supported_locales
                .iter()
                .map(|locale| format!("\"{locale}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let config = format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "1.0.0", "file_version": "1.0.0.0" }},
  "output": {{ "installer_name": "{installer_name}", "uninstaller_name": "uninst.exe" }},
  "localization": {{ "default_locale": "en-US", "supported_locales": [{locales}] }},
  "resources": {{ "layouts_dir": "layouts", "assets_dir": "assets", "locales_dir": "locales", "payload_file": "payload/app.zip" }},
  "wizard": {{ "pages": [{{ "id": "config", "layout": "layouts/configpage.xml" }}] }}
}}
"#
            );
            std::fs::write(self.config_path(), config).expect("project file");
        }

        fn config_path(&self) -> PathBuf {
            self.path.join("installer_config.json")
        }

        /// Where the inspection resolves the setup this project builds.
        fn output_path(&self, installer_name: &str) -> PathBuf {
            self.path.join("dist").join(installer_name)
        }

        fn display(&self) -> String {
            self.path.display().to_string()
        }
    }

    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// Waits for the build the app started in its worker thread.
    ///
    /// A case that reads the outcome has to let the thread reach it, and a
    /// thread that never finishes fails the case rather than leaving it to read
    /// a state no build produced.
    fn wait_for_build(app: &mut BuilderApp) {
        for _ in 0..600 {
            app.poll_worker();
            if !app.building {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the build worker did not finish");
    }

    /// Draws the window's panels, which is everything `update` does but the
    /// frame, because a frame needs a window.
    ///
    /// Laying the panels out is not allowed to change what they show, so the
    /// log and the status line are compared around every draw.
    fn draw_panels(app: &mut BuilderApp, context: &egui::Context) {
        let logs = app.logs.clone();
        let status = app.status.clone();
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::TopBottomPanel::top("menu").show(context, |ui| app.show_menu(context, ui));
            egui::SidePanel::left("project_panel").show(context, |ui| app.show_project_panel(ui));
            egui::CentralPanel::default().show(context, |ui| match app.tab {
                WorkspaceTab::Build => app.show_build_tab(context, ui),
                WorkspaceTab::Parameters => app.show_parameters_tab(ui),
            });
        });
        assert_eq!(app.logs, logs, "drawing changed the log");
        assert_eq!(app.status, status, "drawing changed the status line");
    }

    /// Opening a project reads the folder the user picked.
    ///
    /// Every name, path, and default the window shows afterwards comes out of
    /// that folder, so the case points the app at one and expects the summary
    /// to be filled from the file on disk.
    #[test]
    fn open_project_reads_the_folder_that_holds_installer_config_json() {
        let project = TestProject::create("open");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();

        app.validate_project();

        let summary = app.summary.as_ref().expect("project summary");
        assert_eq!(summary.project_name, "Probe");
        assert_eq!(summary.output_path, project.output_path("Probe_Setup.exe"));
        assert!(summary.warnings.is_empty(), "{:?}", summary.warnings);
        assert!(app.validation_error.is_none());
        assert!(app.last_inspected.is_some());
        assert!(!app.project_dirty);
        assert_eq!(app.status_tone, StatusTone::Success);
        assert!(app.logs.iter().any(
            |line| line.ends_with(&format!("Project inspection passed: {}", project.display()))
        ));
    }

    /// A folder that holds no project file is refused by name.
    ///
    /// Picking the wrong folder is the common mistake, and a window that
    /// reported nothing would leave the user guessing which file it wanted.
    #[test]
    fn open_project_names_the_missing_project_file() {
        let project = TestProject::create("no-config");
        std::fs::remove_file(project.config_path()).expect("remove the project file");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();

        app.validate_project();

        assert!(app.summary.is_none());
        assert!(app.last_inspected.is_none());
        assert_eq!(app.status_tone, StatusTone::Error);
        let error = app.validation_error.as_ref().expect("validation error");
        assert!(error.contains("installer_config.json"), "{error}");
        assert!(error.contains(&project.display()), "{error}");
        assert!(app.logs.iter().any(
            |line| line.contains("Validation failed") && line.contains("installer_config.json")
        ));
    }

    /// Refreshing keeps an output path chosen for this project.
    ///
    /// The field may point anywhere, and a refresh that reset it would move the
    /// next setup back into the project tree without saying so.
    #[test]
    fn refresh_keeps_a_custom_output_path_for_the_same_project() {
        let project = TestProject::create("refresh-same");
        let mut app = headless_app();
        app.project_dir = project.display();
        app.validate_project();
        app.output_path = "C:\\builds\\Probe_Setup.exe".to_string();
        app.output_custom = true;

        app.validate_project();

        assert_eq!(app.output_path, "C:\\builds\\Probe_Setup.exe");
        assert!(app.output_custom);
    }

    /// A custom output path belongs to the project it was chosen for.
    ///
    /// Switching projects makes that path meaningless, and keeping it would
    /// write the new project's setup into a folder the new project never
    /// configured.
    #[test]
    fn refresh_replaces_a_custom_output_path_when_the_project_changes() {
        let first = TestProject::create("refresh-first");
        let second = TestProject::create("refresh-second");
        second.write_config("Second", "Second_Setup.exe", &["en-US"]);
        let mut app = headless_app();
        app.project_dir = first.display();
        app.validate_project();
        app.output_path = "C:\\builds\\Probe_Setup.exe".to_string();
        app.output_custom = true;

        app.project_dir = second.display();
        app.validate_project();

        assert_eq!(
            app.output_path,
            second.output_path("Second_Setup.exe").display().to_string()
        );
        assert!(!app.output_custom);
    }

    /// Reset output puts the project's own output path back in the field.
    ///
    /// The button restores the summary's output path, so that path has to be
    /// the project's `dist/<output.installer_name>` and nothing else; a case
    /// that let it drift would hide a reset pointing the next build at the
    /// wrong file.
    #[test]
    fn reset_output_restores_the_dist_path_named_by_the_project() {
        let project = TestProject::create("reset-output");
        project.write_config("Probe", "Custom_Setup.exe", &["en-US"]);
        let mut app = headless_app();
        app.project_dir = project.display();
        app.validate_project();
        app.output_path = "C:\\builds\\elsewhere.exe".to_string();
        app.output_custom = true;

        let restored = app
            .summary
            .as_ref()
            .expect("project summary")
            .output_path
            .clone();

        assert_eq!(restored, project.output_path("Custom_Setup.exe"));
        assert_eq!(restored.parent(), Some(project.path.join("dist").as_path()));
    }

    /// Parameters carry the three paths into the build request.
    ///
    /// The request is all the worker thread sees, so every field has to be
    /// trimmed into it: a path that stayed on screen would send the build to
    /// the builder's own defaults instead.
    #[test]
    fn parameters_carry_the_project_output_and_runtime_directory_into_the_request() {
        let project = TestProject::create("request");
        let mut app = headless_app();
        app.project_dir = format!("  {}  ", project.display());
        app.output_path = format!(" {} ", project.output_path("Probe_Setup.exe").display());
        app.stub_directory = project.path.join("stubs").display().to_string();
        app.validate_project();

        let request = app.make_request().expect("build request");

        assert_eq!(request.project_dir, project.path);
        assert_eq!(request.output, Some(project.output_path("Probe_Setup.exe")));
        assert_eq!(request.stub_directory, Some(project.path.join("stubs")));
        assert_eq!(
            app.command_preview(),
            format!(
                "nano-installer-native-x64.exe build --project \"{}\" --output \"{}\" --stubs \"{}\"",
                project.display(),
                project.output_path("Probe_Setup.exe").display(),
                project.path.join("stubs").display()
            )
        );
    }

    /// An empty runtime directory leaves the stub search automatic.
    ///
    /// "Use automatic stub search" clears the field, and an empty path sent as
    /// a directory would be searched first and fail there, so the request has
    /// to carry no override at all and the preview has to offer none.
    #[test]
    fn an_empty_runtime_directory_leaves_the_stub_search_automatic() {
        let project = TestProject::create("auto-stub");
        let mut app = headless_app();
        app.project_dir = project.display();
        app.output_path = project.output_path("Probe_Setup.exe").display().to_string();
        app.validate_project();
        app.stub_directory = project.path.join("stubs").display().to_string();
        assert_eq!(
            app.make_request()
                .expect("request with an override")
                .stub_directory,
            Some(project.path.join("stubs"))
        );

        app.stub_directory.clear();
        let request = app.make_request().expect("request without an override");

        assert!(request.stub_directory.is_none());
        assert!(request.output.is_some());
        assert!(!app.command_preview().contains("--stubs"));
        assert!(app.command_preview().contains("--project"));
    }

    /// A build in flight refuses a second packaging task.
    ///
    /// The buttons are disabled while one runs, and this is the check behind
    /// them: two launches at once would write the same output file.
    #[test]
    fn a_running_build_refuses_a_second_one() {
        let project = TestProject::create("busy");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();
        app.output_path = project.output_path("Probe_Setup.exe").display().to_string();
        app.validate_project();
        app.building = true;
        app.last_build_failed = true;
        app.last_request = Some(BuildRequest::new(project.path.clone()));
        let context = egui::Context::default();
        let logs = app.logs.clone();

        app.start_build(&context);
        app.retry_build(&context);

        assert!(app.building);
        assert!(app.receiver.is_none());
        assert_eq!(
            app.last_request
                .as_ref()
                .map(|request| request.project_dir.clone()),
            Some(project.path.clone())
        );
        assert_eq!(app.logs, logs);
    }

    /// Retry repeats the failed build with the parameters it stored.
    ///
    /// The point of a retry is that only a file on disk changed, so the case
    /// edits the on-screen fields after the failure and expects the worker to
    /// receive the stored request rather than the form.
    #[test]
    fn retry_repeats_a_failed_build_with_the_parameters_it_stored() {
        let project = TestProject::create("retry");
        let missing = project.path.join("gone");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();
        app.output_path = project.output_path("Probe_Setup.exe").display().to_string();
        app.validate_project();
        app.last_request = Some(BuildRequest::new(missing.clone()));
        app.last_build_failed = true;
        app.project_dir = project.path.join("elsewhere").display().to_string();
        app.output_path = "C:\\builds\\other.exe".to_string();
        let context = egui::Context::default();

        app.retry_build(&context);
        wait_for_build(&mut app);

        assert_eq!(
            app.last_request
                .as_ref()
                .map(|request| request.project_dir.clone()),
            Some(missing.clone())
        );
        assert!(app
            .logs
            .iter()
            .any(|line| line.ends_with("Retrying last build")));
        let failure = app
            .logs
            .iter()
            .find(|line| line.contains("Error:"))
            .expect("the retried build failed");
        assert!(
            failure.contains(&missing.display().to_string()),
            "{failure}"
        );
        assert!(app.last_build_failed);
    }

    /// Build setup re-reads a project that changed on screen.
    ///
    /// Editing the project directory marks the summary as out of date, so the
    /// build has to inspect the folder again before it starts and refuse when
    /// the file no longer reads, rather than package what the sidebar still
    /// describes.
    #[test]
    fn build_setup_rechecks_a_project_marked_dirty_before_it_starts() {
        let project = TestProject::create("dirty");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();
        app.output_path = project.output_path("Probe_Setup.exe").display().to_string();
        app.validate_project();
        std::fs::write(project.config_path(), "{ \"project\": ").expect("break the project file");
        app.project_dirty = true;
        let context = egui::Context::default();

        app.start_build(&context);

        assert!(!app.building);
        assert!(app.last_request.is_none());
        assert!(app.summary.is_none());
        let error = app.validation_error.as_ref().expect("validation error");
        assert!(error.contains("invalid project config"), "{error}");
        assert!(app
            .logs
            .iter()
            .any(|line| line.contains("Validation failed")));
    }

    /// A build reads the project from disk in its worker thread.
    ///
    /// The sidebar summary is a snapshot of the last inspection. Deleting the
    /// project file behind that snapshot has to fail the build naming the file,
    /// because the worker inspects the folder again rather than trusting what
    /// the window still shows.
    #[test]
    fn a_build_reads_the_project_from_disk_in_its_worker_thread() {
        let project = TestProject::create("worker");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();
        app.output_path = project.output_path("Probe_Setup.exe").display().to_string();
        app.validate_project();
        std::fs::remove_file(project.config_path()).expect("remove the project file");
        let context = egui::Context::default();

        app.start_build(&context);
        wait_for_build(&mut app);

        assert!(app.summary.is_some(), "the screen kept its summary");
        assert!(app.logs.iter().any(|line| line.ends_with("Build started")));
        let failure = app
            .logs
            .iter()
            .find(|line| line.contains("Error:"))
            .expect("the build failed");
        assert!(failure.contains("installer_config.json"), "{failure}");
        assert!(failure.contains(&project.display()), "{failure}");
        let error = app.validation_error.as_ref().expect("build error");
        assert!(error.contains("installer_config.json"), "{error}");
        assert!(app.last_build_failed);
    }

    /// The log keeps the lines the view has scrolled past.
    ///
    /// Save log writes the stored lines and Copy all copies the same text, so
    /// what those export is the log itself rather than the handful of lines the
    /// panel can show at once: the case fills it well past that and checks
    /// every line is still there, in its original wording, under its own
    /// timestamp.
    #[test]
    fn the_log_keeps_the_lines_the_view_scrolled_past() {
        let mut app = headless_app();
        let timestamp_width = format_log_timestamp(2026, 9, 15, 7, 5, 9, 42).len() - 1;

        for index in 0..400 {
            app.append_log(format!("step {index} 中文 C:\\builds\\输出.exe"));
        }

        assert_eq!(app.logs.len(), 400);
        for (index, line) in app.logs.iter().enumerate() {
            let (timestamp, message) = line.split_once("] ").expect("a timestamped line");
            assert!(timestamp.starts_with('['), "{line}");
            assert_eq!(timestamp.len(), timestamp_width, "{line}");
            assert_eq!(message, format!("step {index} 中文 C:\\builds\\输出.exe"));
        }
        assert!(app
            .logs
            .first()
            .expect("first line")
            .ends_with("step 0 中文 C:\\builds\\输出.exe"));
        assert!(app
            .logs
            .last()
            .expect("last line")
            .ends_with("step 399 中文 C:\\builds\\输出.exe"));
    }

    /// Build messages reach the log in the order the worker sent them.
    ///
    /// The wording of these lines is the builder's, including the first pass
    /// under the `Uninstaller bundle:` heading and the repeated directory names
    /// of the second one. The window is a pass-through, so the case feeds that
    /// sequence and checks nothing is merged or dropped on the way, that the
    /// payload is named once, and that both passes stay visible.
    #[test]
    fn build_messages_reach_the_log_in_the_order_the_worker_sent_them() {
        let sent = [
            "Uninstaller bundle: Collected layouts/: 8 files (12.0 KiB)",
            "Uninstaller bundle: Collected assets/: 30 files (1.20 MiB)",
            "Uninstaller bundle: Collected locales/: 11 files (48.0 KiB)",
            "Uninstaller bundle: Collected scripts/: 2 files (9.0 KiB)",
            "Collected layouts/: 8 files (12.0 KiB)",
            "Collected assets/: 30 files (1.20 MiB)",
            "Collected locales/: 11 files (48.0 KiB)",
            "Collected scripts/: 2 files (9.0 KiB)",
            "Added payload payload/app.7z (150.00 MiB, already compressed)",
        ];
        let closing = "Created C:\\builds\\Probe_Setup.exe";
        let (sender, receiver) = mpsc::channel();
        for message in sent {
            sender
                .send(WorkerMessage::Progress(BuildEvent {
                    stage: BuildStage::Packing,
                    message: message.to_string(),
                }))
                .expect("send a build event");
        }
        sender
            .send(WorkerMessage::Progress(BuildEvent {
                stage: BuildStage::Complete,
                message: closing.to_string(),
            }))
            .expect("send the closing event");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.receiver = Some(receiver);

        app.poll_worker();

        let logged: Vec<&str> = app
            .logs
            .iter()
            .map(|line| {
                line.split_once("] ")
                    .map(|(_, message)| message)
                    .unwrap_or(line.as_str())
            })
            .collect();
        let mut expected = sent.to_vec();
        expected.push(closing);
        assert_eq!(logged, expected);
        assert_eq!(app.progress, 1.0);
        assert_eq!(app.status, tr(UiLanguage::English, "build_complete"));
        assert_eq!(
            app.logs
                .iter()
                .filter(|line| line.contains("Added payload"))
                .count(),
            1
        );
        assert_eq!(
            app.logs
                .iter()
                .filter(|line| line.ends_with("Collected assets/: 30 files (1.20 MiB)"))
                .count(),
            2
        );
    }

    /// Menus keep their width and their labels on one line.
    ///
    /// The menu bar is laid out once and then re-read after the interface
    /// language is switched, so a label that wrapped onto a second line would
    /// move the menus beside it every time the user changes the language.
    #[test]
    fn menu_labels_stay_on_one_line_in_both_languages() {
        const MENU_KEYS: [&str; 15] = [
            "file",
            "open_project_menu",
            "open_config",
            "open_output",
            "exit",
            "build",
            "refresh_project",
            "build_setup",
            "retry_last",
            "view",
            "workspace",
            "parameters",
            "clear",
            "language",
            "help",
        ];
        for key in MENU_KEYS {
            for language in [UiLanguage::English, UiLanguage::SimplifiedChinese] {
                let label = tr(language, key);
                assert!(!label.is_empty(), "{key} has no label");
                assert!(
                    !label.contains('\n') && !label.contains('\r'),
                    "{key} is not a single line"
                );
            }
        }
        for language in [UiLanguage::English, UiLanguage::SimplifiedChinese] {
            let label = language.label();
            assert!(!label.contains('\n') && !label.contains('\r'));
        }

        egui::__run_test_ui(|ui| {
            configure_menu(ui, 220.0);
            assert!(ui.min_rect().width() >= 220.0);
            assert_eq!(ui.style().wrap_mode, Some(egui::TextWrapMode::Extend));
        });
    }

    /// The panels draw in every state the guide describes.
    ///
    /// No case here can open a window, so the states are laid out against a
    /// headless context instead: nothing open, a project inspected, a build
    /// running, a build that failed, and a finished one, in both interface
    /// languages. A state that cannot be drawn at all would leave the user with
    /// a blank window, and a draw that changed the state it shows would be
    /// worse.
    #[test]
    fn the_panels_draw_in_every_state_they_can_be_in() {
        let project = TestProject::create("draw");
        let context = egui::Context::default();
        let mut app = headless_app();

        draw_panels(&mut app, &context);
        app.tab = WorkspaceTab::Parameters;
        draw_panels(&mut app, &context);
        app.tab = WorkspaceTab::Build;

        app.project_dir = project.display();
        app.output_path = project.output_path("Probe_Setup.exe").display().to_string();
        app.validate_project();
        draw_panels(&mut app, &context);

        app.building = true;
        app.progress = 0.52;
        draw_panels(&mut app, &context);
        app.building = false;

        app.last_build_failed = true;
        app.validation_error = Some("packing failed".to_string());
        draw_panels(&mut app, &context);

        app.result = Some(BuildResult {
            summary: app.summary.clone().expect("project summary"),
            stub_path: PathBuf::from("lzma-stub-native.exe"),
            bundle_size: 1_024,
            output_size: 4_096,
            update: None,
        });
        draw_panels(&mut app, &context);

        app.language = UiLanguage::SimplifiedChinese;
        app.tab = WorkspaceTab::Parameters;
        draw_panels(&mut app, &context);
        app.tab = WorkspaceTab::Build;
        draw_panels(&mut app, &context);

        assert!(app.summary.is_some());
        assert!(app.result.is_some());
    }

    /// What the inspection found reaches the sidebar and the log.
    ///
    /// The guide lists a 1x PNG without its 2x pair, a locale missing page text
    /// the default locale defines, and a language listed with no file behind
    /// it, so the case builds all three and expects the one inspection to fill
    /// the count the sidebar shows and the lines the log carries.
    #[test]
    fn inspection_findings_reach_the_sidebar_and_the_log() {
        let project = TestProject::create("warnings");
        project.write_config("Probe", "Probe_Setup.exe", &["en-US", "ja-JP", "ru-RU"]);
        std::fs::write(
            project.path.join("layouts/configpage.xml"),
            r#"<Page width="720" height="450"><Label text="@title" /></Page>"#,
        )
        .expect("first page layout");
        std::fs::write(
            project.path.join("locales/en-US.json"),
            r#"{"title":"Probe"}"#,
        )
        .expect("default locale file");
        std::fs::write(project.path.join("locales/ja-JP.json"), "{}").expect("locale file");
        std::fs::write(project.path.join("assets/logo.png"), b"png").expect("1x asset");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();

        app.validate_project();

        let warnings = app
            .summary
            .as_ref()
            .expect("project summary")
            .warnings
            .clone();
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings.contains(&"missing DPI pair for assets/logo.png".to_string()));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("locales/ja-JP.json is missing 1 page text(s)")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("localization.supported_locales lists ru-RU")));
        for warning in &warnings {
            assert!(
                app.logs
                    .iter()
                    .any(|line| line.ends_with(&format!("Warning: {warning}"))),
                "{warning} is not in the log"
            );
        }
    }

    /// A project file that cannot be read is reported, not fatal.
    ///
    /// These files are edited by hand, so the window has to survive one that
    /// went wrong: the case breaks the JSON and then a layout and expects the
    /// reason in the sidebar and in the log instead of a panic.
    #[test]
    fn a_broken_project_file_is_reported_to_the_user() {
        let project = TestProject::create("broken-json");
        let mut app = headless_app();
        app.language = UiLanguage::English;
        app.project_dir = project.display();
        std::fs::write(project.config_path(), "{ \"project\": ").expect("break the project file");

        app.validate_project();

        assert!(app.summary.is_none());
        assert_eq!(app.status_tone, StatusTone::Error);
        let error = app.validation_error.as_ref().expect("validation error");
        assert!(error.contains("invalid project config"), "{error}");
        assert!(error.contains("installer_config.json"), "{error}");

        project.write_config("Probe", "Probe_Setup.exe", &["en-US"]);
        std::fs::write(
            project.path.join("layouts/configpage.xml"),
            r#"<Page><Label text="@title">"#,
        )
        .expect("break the first page layout");

        app.validate_project();

        assert!(app.summary.is_none());
        let error = app.validation_error.as_ref().expect("validation error");
        assert!(error.contains("invalid layout"), "{error}");
        assert!(error.contains("configpage.xml"), "{error}");
    }

    #[test]
    fn window_icon_uses_the_brand_asset() {
        let icon = tool_icon();
        assert_eq!((icon.width, icon.height), (512, 512));
        assert_eq!(icon.rgba.len(), 512 * 512 * 4);
        assert_eq!(icon.rgba[3], 0);
    }
}
