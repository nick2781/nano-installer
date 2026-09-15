#[cfg(not(target_arch = "x86_64"))]
compile_error!("nano-installer-native-x64 must be built for x86_64");

mod icon;
mod install;
mod shell;
mod version;

use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
    CreateRoundRectRgn, DeleteDC, DeleteObject, DrawTextW, EndPaint, GdiAlphaBlend, GetDC,
    GetDeviceCaps, GetTextExtentPoint32W, InvalidateRect, ReleaseDC, SelectObject, SetBkMode,
    SetTextColor, SetWindowRgn, UpdateWindow, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, FW_BOLD, FW_NORMAL, HDC, HFONT, LOGPIXELSX, OUT_DEFAULT_PRECIS, PAINTSTRUCT,
    SRCCOPY, TRANSPARENT,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory, IWICPalette,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnLoad,
};
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetSystemMetrics, LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassExW,
    SendMessageW, SetProcessDPIAware, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    HTCAPTION, ICON_BIG, ICON_SMALL, IDC_ARROW, MB_ICONERROR, MB_OK, MSG, SM_CXSCREEN, SM_CYSCREEN,
    SW_MINIMIZE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCLBUTTONDOWN, WM_PAINT, WM_SETICON, WNDCLASSEXW,
    WS_EX_APPWINDOW, WS_POPUP,
};

const BUNDLE_MAGIC: &[u8; 8] = b"NATVRS01";
const FOOTER_MAGIC: &[u8; 8] = b"NATVEND1";
const BUNDLE_VERSION: u16 = 1;
const BASE_DPI: u32 = 96;
const DEFAULT_DPI_THRESHOLD: u32 = 144;
const WM_MOUSELEAVE: u32 = 0x02A3;
static UI: OnceLock<Mutex<RuntimeState>> = OnceLock::new();
static TEMP_EXE_ID: AtomicU64 = AtomicU64::new(0);

struct NativeImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

struct ImageLayer {
    image: NativeImage,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    alpha: u8,
}

#[derive(Clone, Copy)]
struct LayerRect {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

struct RuntimeUi {
    width: i32,
    height: i32,
    corner_radius: i32,
    caption_height: i32,
    layers: Vec<ImageLayer>,
    texts: Vec<TextLayer>,
    overlay_layers: Vec<ImageLayer>,
    overlay_texts: Vec<TextLayer>,
    actions: Vec<ActionRegion>,
    hover_regions: Vec<HoverRegion>,
}

struct RuntimeState {
    files: HashMap<String, Vec<u8>>,
    dpi: DpiContext,
    locale: String,
    language_menu_open: bool,
    interaction: InteractionState,
    mode: RuntimeMode,
    ui: RuntimeUi,
}

#[derive(Clone, Copy)]
enum RuntimeMode {
    Installer,
    Uninstaller,
}

#[derive(Default)]
struct InteractionState {
    checkbox_states: HashMap<String, bool>,
    panel_visibility: HashMap<String, bool>,
    hovered_control: Option<String>,
    pressed_control: Option<String>,
    text_input_values: HashMap<String, String>,
}

struct TextLayer {
    runs: Vec<TextRun>,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    font_size: i32,
    bold: bool,
    alignment: TextAlignment,
    wrap: bool,
}

struct TextRun {
    text: String,
    color: COLORREF,
}

#[derive(Clone, Copy)]
enum TextAlignment {
    Left,
    Center,
    Right,
}

struct ActionRegion {
    action: WindowAction,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

struct HoverRegion {
    id: String,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Clone)]
enum WindowAction {
    Close,
    Minimize,
    ToggleLanguageMenu,
    SelectLanguage(String),
    Install,
    Uninstall,
    ToggleCheckbox { id: String, checked: bool },
    SetPanelVisibility { id: String, visible: bool },
}

#[derive(Clone, Copy)]
struct DpiContext {
    scale: f32,
    use_2x: bool,
}

struct ImageStyle<'a> {
    source: &'a str,
    destination: Option<LayerRect>,
    alpha: u8,
}

impl ImageStyle<'_> {
    fn plain(source: &str) -> ImageStyle<'_> {
        ImageStyle {
            source,
            destination: None,
            alpha: 255,
        }
    }
}

#[derive(Default)]
struct LayoutOutput {
    layers: Vec<ImageLayer>,
    texts: Vec<TextLayer>,
    overlay_layers: Vec<ImageLayer>,
    overlay_texts: Vec<TextLayer>,
    actions: Vec<ActionRegion>,
    hover_regions: Vec<HoverRegion>,
}

struct LayoutContext<'a> {
    dpi: DpiContext,
    files: &'a HashMap<String, Vec<u8>>,
    config: &'a serde_json::Value,
    locale: &'a str,
    translations: &'a HashMap<String, String>,
    interaction: &'a InteractionState,
}

#[derive(Clone, Copy)]
struct FlowItem {
    fixed_width: Option<i32>,
    flex_grow: f32,
    flex_shrink: f32,
    min_width: i32,
}

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub fn show_runtime_error(error: &anyhow::Error) {
    let message = HSTRING::from(format!("Native runtime failed:\n{error:#}"));
    unsafe {
        let _ = MessageBoxW(None, &message, w!("nano-installer"), MB_OK | MB_ICONERROR);
    }
}

pub fn run_installer_runtime() -> Result<()> {
    run_runtime(RuntimeMode::Installer)
}

pub fn run_uninstaller_runtime() -> Result<()> {
    run_runtime(RuntimeMode::Uninstaller)
}

fn run_runtime(mode: RuntimeMode) -> Result<()> {
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let bundle = BundleIndex::read(&exe)?.context("native resource bundle is missing")?;
    run_embedded(bundle, mode)
}

#[derive(Clone, Debug)]
pub struct BuildRequest {
    pub project_dir: PathBuf,
    pub output: Option<PathBuf>,
    pub stub_directory: Option<PathBuf>,
}

impl BuildRequest {
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
            output: None,
            stub_directory: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadFormat {
    SevenZip,
    Zip,
}

impl PayloadFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::SevenZip => "7z / LZMA",
            Self::Zip => "ZIP / Deflate",
        }
    }

    pub fn stub_name(self) -> &'static str {
        match self {
            Self::SevenZip => "lzma-stub-native.exe",
            Self::Zip => "zlib-stub-native.exe",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProjectSummary {
    pub project_dir: PathBuf,
    pub project_name: String,
    pub project_version: String,
    pub file_version: String,
    pub default_locale: String,
    pub output_path: PathBuf,
    pub installer_icon: Option<PathBuf>,
    pub uninstaller_name: String,
    pub uninstaller_icon: Option<PathBuf>,
    pub default_install_path: Option<String>,
    pub payload_path: PathBuf,
    pub payload_size: u64,
    pub payload_format: PayloadFormat,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildStage {
    Validating,
    SelectingStub,
    Packing,
    WritingResources,
    Complete,
}

#[derive(Clone, Debug)]
pub struct BuildEvent {
    pub stage: BuildStage,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct BuildResult {
    pub summary: ProjectSummary,
    pub stub_path: PathBuf,
    pub bundle_size: u64,
    pub output_size: u64,
}

pub fn inspect_project(project: impl AsRef<Path>) -> Result<ProjectSummary> {
    let project = project.as_ref();
    let config = read_project_config(project)?;
    let output_path = default_output_for_project(project, &config)?;
    let payload_path = project.join(
        config["resources"]["payload_file"]
            .as_str()
            .context("resources.payload_file is required")?,
    );
    let payload_format = payload_format(&payload_path)?;
    let payload_size = std::fs::metadata(&payload_path)?.len();
    let project_name = config["project"]["name"]
        .as_str()
        .or_else(|| config["project"]["output_name"].as_str())
        .unwrap_or("nano-installer package")
        .to_string();
    let project_version = config["project"]["version"]
        .as_str()
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string();
    let file_version = config["project"]["file_version"]
        .as_str()
        .unwrap_or(&project_version)
        .to_string();
    let default_locale = config["localization"]["default_locale"]
        .as_str()
        .unwrap_or("zh-CN")
        .to_string();
    let installer_icon = config["output"]["installer_icon"]
        .as_str()
        .map(|path| project.join(path));
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe")
        .to_string();
    let uninstaller_icon = uninstaller_icon_path(project, &config);
    let default_install_path = config["install"]["default_path"]
        .as_str()
        .map(str::to_string);
    validate_project_paths(project, &config, installer_icon.as_deref())?;
    let warnings = asset_scale_warnings(project, &config)?;
    Ok(ProjectSummary {
        project_dir: project.to_path_buf(),
        project_name,
        project_version,
        file_version,
        default_locale,
        output_path,
        installer_icon,
        uninstaller_name,
        uninstaller_icon,
        default_install_path,
        payload_path,
        payload_size,
        payload_format,
        warnings,
    })
}

pub fn build_project(request: BuildRequest) -> Result<BuildResult> {
    build_project_with_progress(request, |_| {})
}

pub fn build_project_with_progress(
    request: BuildRequest,
    mut progress: impl FnMut(BuildEvent),
) -> Result<BuildResult> {
    progress(BuildEvent {
        stage: BuildStage::Validating,
        message: "Validating project".to_string(),
    });
    let project = request.project_dir;
    let mut summary = inspect_project(&project)?;
    if let Some(output) = request.output {
        summary.output_path = output;
    }
    progress(BuildEvent {
        stage: BuildStage::Validating,
        message: format!(
            "Project: {} {}",
            summary.project_name, summary.project_version
        ),
    });
    progress(BuildEvent {
        stage: BuildStage::Validating,
        message: format!(
            "Payload: {} ({}; {})",
            summary.payload_path.display(),
            summary.payload_format.label(),
            format_build_size(summary.payload_size)
        ),
    });
    progress(BuildEvent {
        stage: BuildStage::Validating,
        message: format!("Output: {}", summary.output_path.display()),
    });
    let config = read_project_config(&project)?;
    let output = &summary.output_path;
    let version_info = installer_version_info(&config, output)?;
    progress(BuildEvent {
        stage: BuildStage::SelectingStub,
        message: format!("Selecting {}", summary.payload_format.stub_name()),
    });
    let stub = find_native_stub(
        summary.payload_format.stub_name(),
        request.stub_directory.as_deref(),
    )?;
    progress(BuildEvent {
        stage: BuildStage::SelectingStub,
        message: format!(
            "Installer stub: {} ({})",
            stub.display(),
            format_build_size(std::fs::metadata(&stub)?.len())
        ),
    });
    let uninstaller_path =
        find_native_stub("uninst-stub-native.exe", request.stub_directory.as_deref())?;
    let raw_uninstaller_size = std::fs::metadata(&uninstaller_path)?.len();
    progress(BuildEvent {
        stage: BuildStage::SelectingStub,
        message: format!(
            "Uninstaller stub: {} ({})",
            uninstaller_path.display(),
            format_build_size(raw_uninstaller_size)
        ),
    });
    let uninstaller_name = config["output"]["uninstaller_name"]
        .as_str()
        .unwrap_or("uninst.exe");
    validate_output_filename(uninstaller_name, "output.uninstaller_name")?;
    progress(BuildEvent {
        stage: BuildStage::SelectingStub,
        message: format!("Building self-contained uninstaller: {uninstaller_name}"),
    });
    progress(BuildEvent {
        stage: BuildStage::Packing,
        message: "Packing project resources".to_string(),
    });
    let uninstaller = build_uninstaller_executable(
        &project,
        &config,
        &uninstaller_path,
        uninstaller_name,
        |message| {
            progress(BuildEvent {
                stage: BuildStage::Packing,
                message,
            });
        },
    )?;
    let embedded_uninstaller_name = format!("runtime/{uninstaller_name}");
    progress(BuildEvent {
        stage: BuildStage::Packing,
        message: format!(
            "Uninstaller target: {uninstaller_name}; embedding as {embedded_uninstaller_name}"
        ),
    });
    progress(BuildEvent {
        stage: BuildStage::Packing,
        message: "Bundled runtime install handler (runs only after the user clicks Install): staged payload extraction, uninstaller deployment, manifest and registry registration"
            .to_string(),
    });
    let bundle = pack_project_with_progress(
        &project,
        Some((&embedded_uninstaller_name, uninstaller)),
        true,
        |message| {
            progress(BuildEvent {
                stage: BuildStage::Packing,
                message,
            });
        },
    )?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&stub, output)
        .with_context(|| format!("failed to create {}", output.display()))?;
    progress(BuildEvent {
        stage: BuildStage::Packing,
        message: format!("Copied installer stub to {}", output.display()),
    });
    progress(BuildEvent {
        stage: BuildStage::WritingResources,
        message: "Writing icon and version resources".to_string(),
    });
    if let Some(configured_icon) = config["output"]["installer_icon"].as_str() {
        let icon_path = project.join(configured_icon);
        progress(BuildEvent {
            stage: BuildStage::WritingResources,
            message: format!("Applying installer icon: {}", icon_path.display()),
        });
        icon::replace_exe_icon(output, &icon_path)?;
    }
    progress(BuildEvent {
        stage: BuildStage::WritingResources,
        message: format!("Writing VERSIONINFO: {}", summary.file_version),
    });
    version::replace_exe_version_info(output, &version_info)?;
    let mut file = OpenOptions::new().append(true).open(output)?;
    progress(BuildEvent {
        stage: BuildStage::WritingResources,
        message: format!(
            "Appending resource bundle ({})",
            format_build_size(bundle.len() as u64)
        ),
    });
    file.write_all(&bundle)?;
    file.write_all(&(bundle.len() as u64).to_le_bytes())?;
    file.write_all(FOOTER_MAGIC)?;
    progress(BuildEvent {
        stage: BuildStage::WritingResources,
        message: "Writing bundle size and footer marker".to_string(),
    });
    file.flush()?;
    let output_size = file.metadata()?.len();
    progress(BuildEvent {
        stage: BuildStage::Complete,
        message: format!("Created {}", output.display()),
    });
    Ok(BuildResult {
        summary,
        stub_path: stub,
        bundle_size: bundle.len() as u64,
        output_size,
    })
}

fn build_uninstaller_executable(
    project: &Path,
    config: &serde_json::Value,
    stub: &Path,
    output_name: &str,
    mut progress: impl FnMut(String),
) -> Result<Vec<u8>> {
    let bundle = pack_project_with_progress(project, None, false, |message| {
        progress(format!("Uninstaller bundle: {message}"));
    })?;
    let temporary = TemporaryExecutable::copy_from(stub)?;
    if let Some(icon_path) = uninstaller_icon_path(project, config) {
        progress(format!(
            "Applying uninstaller icon: {}",
            icon_path.display()
        ));
        icon::replace_exe_icon(&temporary.path, &icon_path)?;
    }
    let version_info = uninstaller_version_info(config, output_name);
    progress(format!(
        "Writing uninstaller VERSIONINFO: {}",
        version_info.file_version
    ));
    version::replace_exe_version_info(&temporary.path, &version_info)?;
    progress(format!(
        "Appending uninstaller UI bundle ({})",
        format_build_size(bundle.len() as u64)
    ));
    let mut file = OpenOptions::new().append(true).open(&temporary.path)?;
    file.write_all(&bundle)?;
    file.write_all(&(bundle.len() as u64).to_le_bytes())?;
    file.write_all(FOOTER_MAGIC)?;
    file.flush()?;
    drop(file);
    let executable = std::fs::read(&temporary.path)?;
    progress(format!(
        "Built self-contained {output_name} ({})",
        format_build_size(executable.len() as u64)
    ));
    Ok(executable)
}

struct TemporaryExecutable {
    path: PathBuf,
}

impl TemporaryExecutable {
    fn copy_from(source: &Path) -> Result<Self> {
        for _ in 0..100 {
            let id = TEMP_EXE_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "nano-installer-uninstaller-{}-{id}.exe",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    drop(file);
                    if let Err(error) = std::fs::copy(source, &path) {
                        let _ = std::fs::remove_file(&path);
                        return Err(error).with_context(|| {
                            format!("failed to prepare uninstaller stub: {}", source.display())
                        });
                    }
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error).context("failed to create temporary uninstaller"),
            }
        }
        bail!("failed to allocate a unique temporary uninstaller path")
    }
}

impl Drop for TemporaryExecutable {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn read_project_config(project: &Path) -> Result<serde_json::Value> {
    let path = project.join("installer_config.json");
    let data = std::fs::read(&path)
        .with_context(|| format!("missing project file: {}", path.display()))?;
    serde_json::from_slice(&data)
        .with_context(|| format!("invalid project config: {}", path.display()))
}

fn installer_version_info(
    config: &serde_json::Value,
    output: &Path,
) -> Result<version::VersionInfo> {
    let project = &config["project"];
    let product_name = project["name"]
        .as_str()
        .or_else(|| project["output_name"].as_str())
        .unwrap_or("nano-installer package")
        .to_string();
    let project_version = project["version"]
        .as_str()
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string();
    let file_version = project["file_version"]
        .as_str()
        .unwrap_or(&project_version)
        .to_string();
    let internal_name = project["output_name"]
        .as_str()
        .unwrap_or(&product_name)
        .to_string();
    let original_filename = output
        .file_name()
        .and_then(|name| name.to_str())
        .context("setup output filename must be valid Unicode")?
        .to_string();
    let file_description = project["description"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("{product_name} Installer"));
    let locale = config["localization"]["default_locale"]
        .as_str()
        .unwrap_or("en-US");
    Ok(version::VersionInfo {
        product_name,
        product_version: file_version.clone(),
        file_description,
        file_version,
        company_name: project["publisher"].as_str().map(str::to_string),
        copyright: project["copyright"].as_str().map(str::to_string),
        internal_name,
        original_filename,
        language_id: version::language_id(locale),
    })
}

fn uninstaller_version_info(config: &serde_json::Value, output_name: &str) -> version::VersionInfo {
    let project = &config["project"];
    let product_name = project["name"]
        .as_str()
        .or_else(|| project["output_name"].as_str())
        .unwrap_or("nano-installer package");
    let project_version = project["version"]
        .as_str()
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    let file_version = project["file_version"].as_str().unwrap_or(project_version);
    let internal_name = project["output_name"].as_str().unwrap_or(product_name);
    let locale = config["localization"]["default_locale"]
        .as_str()
        .unwrap_or("en-US");
    version::VersionInfo {
        product_name: product_name.to_string(),
        product_version: file_version.to_string(),
        file_description: format!("{product_name} Uninstaller"),
        file_version: file_version.to_string(),
        company_name: project["publisher"].as_str().map(str::to_string),
        copyright: project["copyright"].as_str().map(str::to_string),
        internal_name: format!("{internal_name}Uninstaller"),
        original_filename: output_name.to_string(),
        language_id: version::language_id(locale),
    }
}

fn uninstaller_icon_path(project: &Path, config: &serde_json::Value) -> Option<PathBuf> {
    config["output"]["uninstaller_icon"]
        .as_str()
        .or_else(|| config["resources"]["uninstaller_icon"].as_str())
        .map(|path| project.join(path))
}

fn default_output_for_project(project: &Path, config: &serde_json::Value) -> Result<PathBuf> {
    let name = config["output"]["installer_name"]
        .as_str()
        .context("output.installer_name is required")?;
    Ok(project.join("dist").join(name))
}

fn validate_output_filename(value: &str, key: &str) -> Result<()> {
    let path = Path::new(value);
    let mut components = path.components();
    if value.is_empty()
        || !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        bail!("{key} must be a file name without directory components")
    }
    Ok(())
}

fn payload_format(path: &Path) -> Result<PayloadFormat> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("failed to open payload: {}", path.display()))?;
    let mut signature = [0u8; 6];
    file.read_exact(&mut signature)
        .with_context(|| format!("payload is too small: {}", path.display()))?;
    if signature.starts_with(b"PK") {
        Ok(PayloadFormat::Zip)
    } else if signature == [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C] {
        Ok(PayloadFormat::SevenZip)
    } else {
        bail!("payload must be a ZIP or 7z archive")
    }
}

fn validate_project_paths(
    project: &Path,
    config: &serde_json::Value,
    installer_icon: Option<&Path>,
) -> Result<()> {
    if let Some(name) = config["output"]["uninstaller_name"].as_str() {
        validate_output_filename(name, "output.uninstaller_name")?;
    }
    for (key, fallback) in [
        ("layouts_dir", "layouts"),
        ("assets_dir", "assets"),
        ("locales_dir", "locales"),
    ] {
        let path = project.join(config["resources"][key].as_str().unwrap_or(fallback));
        if !path.is_dir() {
            bail!("missing project directory: {}", path.display());
        }
    }
    let layout = config["wizard"]["pages"][0]["layout"]
        .as_str()
        .context("wizard.pages[0].layout is required")?;
    let layout_path = project.join(layout);
    if !layout_path.is_file() {
        bail!("missing first-page layout: {}", layout_path.display());
    }
    let locales_dir = config["resources"]["locales_dir"]
        .as_str()
        .unwrap_or("locales");
    let default_locale = config["localization"]["default_locale"]
        .as_str()
        .unwrap_or("zh-CN");
    let locale_path = project
        .join(locales_dir)
        .join(format!("{default_locale}.json"));
    if !locale_path.is_file() {
        bail!("missing default locale: {}", locale_path.display());
    }
    if let Some(icon) = installer_icon {
        if !icon.is_file() {
            bail!("missing installer icon: {}", icon.display());
        }
    }
    if let Some(icon) = uninstaller_icon_path(project, config) {
        if !icon.is_file() {
            bail!("missing uninstaller icon: {}", icon.display());
        }
    }
    let version = config["project"]["file_version"]
        .as_str()
        .or_else(|| config["project"]["version"].as_str())
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    version::validate_version(version)?;
    Ok(())
}

fn asset_scale_warnings(project: &Path, config: &serde_json::Value) -> Result<Vec<String>> {
    let assets_dir = project.join(
        config["resources"]["assets_dir"]
            .as_str()
            .unwrap_or("assets"),
    );
    let mut pngs = Vec::new();
    collect_png_paths(&assets_dir, &assets_dir, &mut pngs)?;
    let set: HashSet<_> = pngs.iter().cloned().collect();
    let mut warnings = Vec::new();
    for path in pngs {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(stem) = file_name.strip_suffix(".png") else {
            continue;
        };
        let pair_name = if let Some(base) = stem.strip_suffix("@2x") {
            format!("{base}.png")
        } else {
            format!("{stem}@2x.png")
        };
        let pair = path.parent().unwrap_or(Path::new("")).join(pair_name);
        if !set.contains(&pair) {
            warnings.push(format!("missing DPI pair for assets/{}", path.display()));
        }
    }
    warnings.sort();
    Ok(warnings)
}

fn collect_png_paths(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_png_paths(root, &entry.path(), paths)?;
        } else if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            paths.push(entry.path().strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn find_native_stub(name: &str, override_directory: Option<&Path>) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(directory) = override_directory {
        candidates.push(directory.join(name));
        candidates.push(directory.join("stubs").join(name));
    }
    if let Some(directory) = std::env::var_os("NANO_INSTALLER_NATIVE_STUB_DIR") {
        let directory = PathBuf::from(directory);
        candidates.push(directory.join(name));
        candidates.push(directory.join("stubs").join(name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(directory) = exe.parent() {
            candidates.push(directory.join(name));
            candidates.push(directory.join("stubs").join(name));
        }
    }
    candidates.push(PathBuf::from("target/release/stubs").join(name));
    candidates.push(PathBuf::from("../../target/release/stubs").join(name));
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .with_context(|| format!("native runtime stub not found: {name}"))
}

#[cfg(test)]
fn pack_project(project: &Path, extra_file: Option<(&str, Vec<u8>)>) -> Result<Vec<u8>> {
    pack_project_with_progress(project, extra_file, true, |_| {})
}

fn pack_project_with_progress(
    project: &Path,
    extra_file: Option<(&str, Vec<u8>)>,
    include_payload: bool,
    mut progress: impl FnMut(String),
) -> Result<Vec<u8>> {
    let mut files = Vec::new();
    let config_path = project.join("installer_config.json");
    let config_data = std::fs::read(&config_path)
        .with_context(|| format!("missing project file: {}", config_path.display()))?;
    let config: serde_json::Value = serde_json::from_slice(&config_data)?;
    progress(format!(
        "Adding installer_config.json ({})",
        format_build_size(config_data.len() as u64)
    ));
    files.push(("installer_config.json".to_string(), config_data));

    for (key, default) in [
        ("layouts_dir", "layouts"),
        ("assets_dir", "assets"),
        ("locales_dir", "locales"),
    ] {
        let directory = config["resources"][key].as_str().unwrap_or(default);
        let file_count_before = files.len();
        let size_before = collected_size(&files);
        collect_directory(project, &project.join(directory), &mut files)?;
        let file_count = files.len() - file_count_before;
        let size = collected_size(&files) - size_before;
        progress(format!(
            "Collected {directory}/: {file_count} files ({})",
            format_build_size(size)
        ));
        if key == "assets_dir" {
            progress(
                "Skin assets are stored directly in the bundle; no intermediate skins.zip is created"
                    .to_string(),
            );
        }
    }
    let scripts = project.join("scripts");
    if scripts.is_dir() {
        let file_count_before = files.len();
        let size_before = collected_size(&files);
        collect_directory(project, &scripts, &mut files)?;
        progress(format!(
            "Collected scripts/: {} files ({})",
            files.len() - file_count_before,
            format_build_size(collected_size(&files) - size_before)
        ));
    }
    if include_payload {
        let payload = config["resources"]["payload_file"]
            .as_str()
            .context("resources.payload_file is required")?;
        collect_file(project, &project.join(payload), &mut files)?;
        let payload_size = files.last().map(|(_, data)| data.len()).unwrap_or_default();
        progress(format!(
            "Added payload {payload} ({}, already compressed)",
            format_build_size(payload_size as u64)
        ));
    }
    if let Some((name, data)) = extra_file {
        progress(format!(
            "Embedded {name} ({})",
            format_build_size(data.len() as u64)
        ));
        files.push((name.to_string(), data));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    progress(format!(
        "Encoding bundle index: {} entries ({} content)",
        files.len(),
        format_build_size(collected_size(&files))
    ));

    let mut bundle = Vec::new();
    bundle.extend_from_slice(BUNDLE_MAGIC);
    bundle.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
    bundle.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for (name, data) in files {
        let name = name.as_bytes();
        let name_len: u16 = name.len().try_into().context("bundle path is too long")?;
        bundle.extend_from_slice(&name_len.to_le_bytes());
        bundle.extend_from_slice(name);
        bundle.extend_from_slice(&(data.len() as u64).to_le_bytes());
        bundle.extend_from_slice(&data);
    }
    Ok(bundle)
}

fn collected_size(files: &[(String, Vec<u8>)]) -> u64 {
    files.iter().map(|(_, data)| data.len() as u64).sum()
}

fn format_build_size(bytes: u64) -> String {
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

fn collect_directory(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("missing project directory: {}", directory.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_directory(root, &entry.path(), files)?;
        } else {
            collect_file(root, &entry.path(), files)?;
        }
    }
    Ok(())
}

fn collect_file(root: &Path, path: &Path, files: &mut Vec<(String, Vec<u8>)>) -> Result<()> {
    if !path.is_file() {
        bail!("missing project file: {}", path.display());
    }
    let name = path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/");
    files.push((name, std::fs::read(path)?));
    Ok(())
}

/// A file stored in the bundle appended to a setup or uninstaller executable.
///
/// Only the offset and length are kept; contents are read from the executable
/// on demand so a large payload never lands in the process address space.
struct BundleEntry {
    offset: u64,
    size: u64,
}

/// Index of the bundle appended to an executable.
struct BundleIndex {
    exe: PathBuf,
    files: HashMap<String, BundleEntry>,
}

impl BundleIndex {
    /// Reads the bundle index from `exe`. Returns `None` when the executable
    /// carries no bundle footer.
    fn read(exe: &Path) -> Result<Option<Self>> {
        let mut file = std::fs::File::open(exe)
            .with_context(|| format!("failed to open {}", exe.display()))?;
        let length = file.metadata()?.len();
        if length < 16 {
            return Ok(None);
        }
        let mut footer = [0u8; 16];
        file.seek(SeekFrom::Start(length - 16))?;
        file.read_exact(&mut footer)?;
        if &footer[8..] != FOOTER_MAGIC {
            return Ok(None);
        }
        let size = u64::from_le_bytes(footer[..8].try_into()?);
        // checked arithmetic: a corrupt footer can carry a size that overflows
        // the addition before the subtraction ever applies.
        let footer_size = 16u64
            .checked_add(size)
            .context("invalid native bundle size")?;
        let start = length
            .checked_sub(footer_size)
            .context("invalid native bundle size")?;
        let files = parse_bundle_index(&mut file, start, size)?;
        Ok(Some(Self {
            exe: exe.to_path_buf(),
            files,
        }))
    }

    fn contains(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }

    fn read_file(&self, name: &str) -> Result<Vec<u8>> {
        let entry = self
            .files
            .get(name)
            .with_context(|| format!("missing from native bundle: {name}"))?;
        let mut file = std::fs::File::open(&self.exe)?;
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut contents = vec![0u8; entry.size as usize];
        file.read_exact(&mut contents)?;
        Ok(contents)
    }

    /// Streams one bundle entry to `destination` in fixed-size chunks.
    fn copy_file_to(&self, name: &str, destination: &Path) -> Result<u64> {
        let entry = self
            .files
            .get(name)
            .with_context(|| format!("missing from native bundle: {name}"))?;
        let mut source = std::fs::File::open(&self.exe)?;
        source.seek(SeekFrom::Start(entry.offset))?;
        let mut target = std::fs::File::create(destination)
            .with_context(|| format!("failed to create {}", destination.display()))?;
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut remaining = entry.size;
        while remaining > 0 {
            let chunk = remaining.min(buffer.len() as u64) as usize;
            source.read_exact(&mut buffer[..chunk])?;
            target.write_all(&buffer[..chunk])?;
            remaining -= chunk as u64;
        }
        target.flush()?;
        Ok(entry.size)
    }

    /// Parses `installer_config.json` from the bundle.
    fn read_config(&self) -> Result<serde_json::Value> {
        serde_json::from_slice(&self.read_file("installer_config.json")?)
            .context("invalid installer configuration")
    }
    /// Reads only the configuration, layout, asset, and locale entries that the
    /// runtime UI needs, leaving the payload on disk.
    fn read_ui_files(&self) -> Result<HashMap<String, Vec<u8>>> {
        let config: serde_json::Value =
            serde_json::from_slice(&self.read_file("installer_config.json")?)
                .context("invalid installer configuration")?;
        let prefixes = [
            config["resources"]["layouts_dir"]
                .as_str()
                .unwrap_or("layouts"),
            config["resources"]["assets_dir"]
                .as_str()
                .unwrap_or("assets"),
            config["resources"]["locales_dir"]
                .as_str()
                .unwrap_or("locales"),
        ]
        .map(|directory| format!("{}/", directory.trim_end_matches(['/', '\\'])));
        let mut files = HashMap::new();
        files.insert(
            "installer_config.json".to_string(),
            self.read_file("installer_config.json")?,
        );
        for name in self.files.keys() {
            if name == "installer_config.json" {
                continue;
            }
            if prefixes.iter().any(|prefix| name.starts_with(prefix)) {
                files.insert(name.clone(), self.read_file(name)?);
            }
        }
        Ok(files)
    }
}

/// Reads the bundle directory without loading any file contents.
fn parse_bundle_index(
    file: &mut std::fs::File,
    start: u64,
    size: u64,
) -> Result<HashMap<String, BundleEntry>> {
    let end = start
        .checked_add(size)
        .context("invalid native bundle size")?;
    let mut header = [0u8; 14];
    file.seek(SeekFrom::Start(start))?;
    file.read_exact(&mut header)?;
    if &header[..8] != BUNDLE_MAGIC {
        bail!("invalid native bundle magic");
    }
    if u16::from_le_bytes(header[8..10].try_into()?) != BUNDLE_VERSION {
        bail!("unsupported native bundle version");
    }
    let count = u32::from_le_bytes(header[10..14].try_into()?);
    let mut cursor = start + 14;
    let mut files = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let mut length = [0u8; 2];
        file.seek(SeekFrom::Start(cursor))?;
        file.read_exact(&mut length)?;
        let name_length = u16::from_le_bytes(length) as u64;
        cursor += 2;
        let mut name = vec![0u8; name_length as usize];
        file.seek(SeekFrom::Start(cursor))?;
        file.read_exact(&mut name)?;
        cursor += name_length;
        let mut length = [0u8; 8];
        file.seek(SeekFrom::Start(cursor))?;
        file.read_exact(&mut length)?;
        let entry_size = u64::from_le_bytes(length);
        cursor += 8;
        let offset = cursor;
        cursor = cursor
            .checked_add(entry_size)
            .context("native bundle entry size overflow")?;
        if cursor > end {
            bail!("native bundle entry exceeds the bundle boundary");
        }
        files.insert(
            String::from_utf8(name)?,
            BundleEntry {
                offset,
                size: entry_size,
            },
        );
    }
    Ok(files)
}

/// In-memory bundle parser used by tests to inspect a freshly packed bundle.
#[cfg(test)]
fn parse_bundle(data: &[u8]) -> Result<HashMap<String, Vec<u8>>> {
    let mut cursor = std::io::Cursor::new(data);
    let mut magic = [0u8; 8];
    cursor.read_exact(&mut magic)?;
    if &magic != BUNDLE_MAGIC {
        bail!("invalid native bundle magic");
    }
    let mut version = [0u8; 2];
    cursor.read_exact(&mut version)?;
    if u16::from_le_bytes(version) != BUNDLE_VERSION {
        bail!("unsupported native bundle version");
    }
    let mut count = [0u8; 4];
    cursor.read_exact(&mut count)?;
    let count = u32::from_le_bytes(count);
    let mut files = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let mut length = [0u8; 2];
        cursor.read_exact(&mut length)?;
        let mut name = vec![0u8; u16::from_le_bytes(length) as usize];
        cursor.read_exact(&mut name)?;
        let mut length = [0u8; 8];
        cursor.read_exact(&mut length)?;
        let mut contents = vec![0u8; u64::from_le_bytes(length) as usize];
        cursor.read_exact(&mut contents)?;
        files.insert(String::from_utf8(name)?, contents);
    }
    Ok(files)
}

fn run_embedded(bundle: BundleIndex, mode: RuntimeMode) -> Result<()> {
    // Only the configuration, layout, asset, and locale entries are read; the
    // payload stays on disk and is streamed when the install action runs.
    let files = bundle.read_ui_files()?;
    let dpi = configure_dpi(&files)?;
    let locale = initial_locale(&files)?;
    let interaction = initial_interaction(&files, mode)?;
    let ui = load_layout(&files, dpi, &locale, false, &interaction, mode)?;
    let (width, height) = (ui.width, ui.height);
    UI.set(Mutex::new(RuntimeState {
        files,
        dpi,
        locale,
        language_menu_open: false,
        interaction,
        mode,
        ui,
    }))
    .map_err(|_| anyhow::anyhow!("native UI was already initialized"))?;
    run_window(width, height)
}

fn initial_locale(files: &HashMap<String, Vec<u8>>) -> Result<String> {
    if let Ok(locale) = std::env::var("NANO_INSTALLER_TEST_LOCALE") {
        if !locale.is_empty() {
            return Ok(locale);
        }
    }
    let config: serde_json::Value = serde_json::from_slice(
        files
            .get("installer_config.json")
            .context("installer_config.json missing from native bundle")?,
    )?;
    Ok(config["localization"]["default_locale"]
        .as_str()
        .unwrap_or("zh-CN")
        .to_string())
}

fn initial_interaction(
    files: &HashMap<String, Vec<u8>>,
    mode: RuntimeMode,
) -> Result<InteractionState> {
    let config: serde_json::Value = serde_json::from_slice(
        files
            .get("installer_config.json")
            .context("installer_config.json missing from native bundle")?,
    )?;
    let layout_path = runtime_layout_path(&config, mode)?;
    let xml = std::str::from_utf8(
        files
            .get(layout_path)
            .with_context(|| format!("layout missing from native bundle: {layout_path}"))?,
    )?;
    let document = roxmltree::Document::parse(xml)?;
    let mut interaction = InteractionState::default();
    for checkbox in document
        .descendants()
        .filter(|node| node.has_tag_name("Checkbox"))
    {
        if let Some(id) = checkbox.attribute("id") {
            interaction.checkbox_states.insert(
                id.to_string(),
                checkbox.attribute("checked") == Some("true"),
            );
        }
    }
    for input in document
        .descendants()
        .filter(|node| node.has_tag_name("TextInput"))
    {
        let Some(id) = input.attribute("id") else {
            continue;
        };
        let value = input.attribute("value").map(str::to_string).or_else(|| {
            input
                .attribute("value-source")
                .and_then(|source| source.strip_prefix("config:"))
                .and_then(|path| config_value_as_string(&config, path))
        });
        if let Some(value) = value {
            interaction.text_input_values.insert(id.to_string(), value);
        }
    }
    Ok(interaction)
}

fn configure_dpi(files: &HashMap<String, Vec<u8>>) -> Result<DpiContext> {
    let config: serde_json::Value = serde_json::from_slice(
        files
            .get("installer_config.json")
            .context("installer_config.json missing from native bundle")?,
    )?;
    let aware = config["ui"]["dpi_aware"].as_bool().unwrap_or(true);
    if aware {
        unsafe {
            let _ = SetProcessDPIAware();
        }
    }
    let dpi = if aware { system_dpi() } else { BASE_DPI };
    let threshold = config["ui"]["dpi_threshold"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_DPI_THRESHOLD);
    Ok(DpiContext {
        scale: dpi as f32 / BASE_DPI as f32,
        use_2x: aware && dpi >= threshold,
    })
}

fn system_dpi() -> u32 {
    if let Ok(value) = std::env::var("NANO_INSTALLER_TEST_DPI") {
        if let Ok(dpi) = value.parse::<u32>() {
            if dpi > 0 {
                return dpi;
            }
        }
    }
    unsafe {
        let dc = GetDC(None);
        if dc.is_invalid() {
            return BASE_DPI;
        }
        let dpi = GetDeviceCaps(dc, LOGPIXELSX);
        let _ = ReleaseDC(None, dc);
        u32::try_from(dpi)
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(BASE_DPI)
    }
}

fn load_layout(
    files: &HashMap<String, Vec<u8>>,
    dpi: DpiContext,
    locale: &str,
    language_menu_open: bool,
    interaction: &InteractionState,
    mode: RuntimeMode,
) -> Result<RuntimeUi> {
    let config: serde_json::Value = serde_json::from_slice(
        files
            .get("installer_config.json")
            .context("installer_config.json missing from native bundle")?,
    )?;
    let layout_path = runtime_layout_path(&config, mode)?;
    let xml = std::str::from_utf8(
        files
            .get(layout_path)
            .with_context(|| format!("layout missing from native bundle: {layout_path}"))?,
    )?;
    let document = roxmltree::Document::parse(xml)?;
    let page = document
        .descendants()
        .find(|node| node.has_tag_name("Page"))
        .context("layout has no Page element")?;
    let width = scale_value(int_attribute(page, "width").unwrap_or(720), dpi.scale);
    let height = scale_value(int_attribute(page, "height").unwrap_or(450), dpi.scale);
    let corner_radius = scale_value(int_attribute(page, "border-radius").unwrap_or(0), dpi.scale);
    let mut output = LayoutOutput::default();
    let locales_dir = config["resources"]["locales_dir"]
        .as_str()
        .unwrap_or("locales");
    let locale_path = format!("{locales_dir}/{locale}.json");
    let translations: HashMap<String, String> = serde_json::from_slice(
        files
            .get(&locale_path)
            .with_context(|| format!("locale missing from native bundle: {locale_path}"))?,
    )?;
    let context = LayoutContext {
        dpi,
        files,
        config: &config,
        locale,
        translations: &translations,
        interaction,
    };
    let active_panel = interaction
        .panel_visibility
        .iter()
        .find(|(_, visible)| **visible)
        .and_then(|(id, _)| {
            document
                .descendants()
                .find(|node| node.is_element() && node.attribute("id") == Some(id.as_str()))
        })
        .map(|node| {
            let (left, top) = absolute_position(node, dpi);
            (
                node.range().start,
                LayerRect {
                    left,
                    top,
                    width: scale_value(int_attribute(node, "width").unwrap_or(0), dpi.scale),
                    height: scale_value(int_attribute(node, "height").unwrap_or(0), dpi.scale),
                },
            )
        });

    if let Some(path) = page.attribute("background-image") {
        push_layer(
            files,
            &mut output.layers,
            path,
            LayerRect {
                left: 0,
                top: 0,
                width,
                height,
            },
            dpi.use_2x,
            255,
        )?;
    }
    for node in page.descendants().filter(|node| node.is_element()) {
        if is_hidden(node, interaction) || is_inside_absolute_hbox(node) {
            continue;
        }
        let (left, top) = absolute_position(node, dpi);
        let layer_width = scale_value(int_attribute(node, "width").unwrap_or(0), dpi.scale);
        let layer_height = scale_value(int_attribute(node, "height").unwrap_or(0), dpi.scale);
        let rect = LayerRect {
            left,
            top,
            width: layer_width,
            height: layer_height,
        };
        if active_panel.is_some_and(|(panel_order, panel_rect)| {
            node.range().start < panel_order && rects_intersect(rect, panel_rect)
        }) {
            continue;
        }
        if node.has_tag_name("HBox")
            && node.attribute("position") == Some("absolute")
            && layer_width > 0
            && layer_height > 0
        {
            render_hbox(node, rect, &context, &mut output)?;
            continue;
        }
        if (node.has_tag_name("Box") || node.has_tag_name("Divider"))
            && node.attribute("position") == Some("absolute")
            && layer_width > 0
            && layer_height > 0
        {
            if let Some(background) = node.attribute("background") {
                push_solid_layer(
                    &mut output.layers,
                    rect,
                    background,
                    scale_value(int_attribute(node, "border-radius").unwrap_or(0), dpi.scale),
                )?;
            }
        }
        if (node.has_tag_name("Button") || node.has_tag_name("Select"))
            && layer_width > 0
            && layer_height > 0
        {
            push_action(node, rect, &mut output.actions, interaction);
            push_hover_region(node, rect, &context, &mut output.hover_regions);
        }
        if layer_width > 0 && layer_height > 0 {
            push_node_text(node, rect, &context, &mut output.texts);
        }
        if node.has_tag_name("Select") && layer_width > 0 && layer_height > 0 {
            render_language_select(node, rect, &context, language_menu_open, &mut output)?;
        }
        let style = match node.tag_name().name() {
            "Image" | "Icon" if node.attribute("position") == Some("absolute") => {
                node.attribute("src").map(ImageStyle::plain)
            }
            "Button" => button_image(node, interaction).map(parse_image_style),
            _ => None,
        };
        if let Some(style) = style.filter(|_| layer_width > 0 && layer_height > 0) {
            push_styled_layer(files, &mut output.layers, style, rect, dpi)?;
        }
    }
    Ok(RuntimeUi {
        width,
        height,
        corner_radius,
        caption_height: scale_value(64, dpi.scale),
        layers: output.layers,
        texts: output.texts,
        overlay_layers: output.overlay_layers,
        overlay_texts: output.overlay_texts,
        actions: output.actions,
        hover_regions: output.hover_regions,
    })
}

fn runtime_layout_path(config: &serde_json::Value, mode: RuntimeMode) -> Result<&str> {
    let (pages, field) = match mode {
        RuntimeMode::Installer => (&config["wizard"]["pages"], "wizard.pages[0].layout"),
        RuntimeMode::Uninstaller => (
            &config["wizard"]["uninstall_pages"],
            "wizard.uninstall_pages[0].layout",
        ),
    };
    pages[0]["layout"]
        .as_str()
        .with_context(|| format!("{field} is missing"))
}

fn is_hidden(node: roxmltree::Node<'_, '_>, interaction: &InteractionState) -> bool {
    if !node_is_visible(node, interaction) {
        return true;
    }
    let mut ancestor = node.parent();
    while let Some(current) = ancestor {
        if !node_is_visible(current, interaction) {
            return true;
        }
        ancestor = current.parent();
    }
    false
}

fn absolute_position(node: roxmltree::Node<'_, '_>, dpi: DpiContext) -> (i32, i32) {
    let mut left = int_attribute(node, "left").unwrap_or(0);
    let mut top = int_attribute(node, "top").unwrap_or(0);
    let mut ancestor = node.parent();
    while let Some(current) = ancestor {
        if current.has_tag_name("Page") {
            break;
        }
        if current.attribute("position") == Some("absolute") {
            left += int_attribute(current, "left").unwrap_or(0);
            top += int_attribute(current, "top").unwrap_or(0);
        }
        ancestor = current.parent();
    }
    (scale_value(left, dpi.scale), scale_value(top, dpi.scale))
}

fn rects_intersect(left: LayerRect, right: LayerRect) -> bool {
    left.left < right.left + right.width
        && left.left + left.width > right.left
        && left.top < right.top + right.height
        && left.top + left.height > right.top
}

fn node_is_visible(node: roxmltree::Node<'_, '_>, interaction: &InteractionState) -> bool {
    if let Some((panel, visible)) = node.attribute("action").and_then(parse_panel_action) {
        let expanded = interaction
            .panel_visibility
            .get(panel)
            .copied()
            .unwrap_or(false);
        return if visible { !expanded } else { expanded };
    }
    if let Some(visible) = node
        .attribute("id")
        .and_then(|id| interaction.panel_visibility.get(id))
    {
        return *visible;
    }
    node.attribute("visible") != Some("false")
}

fn parse_panel_action(action: &str) -> Option<(&str, bool)> {
    let value = action.strip_prefix("toggle_panel:")?;
    let (id, mode) = value.rsplit_once(':')?;
    match mode {
        "show" => Some((id, true)),
        "hide" => Some((id, false)),
        _ => None,
    }
}

fn is_inside_absolute_hbox(node: roxmltree::Node<'_, '_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(current) = ancestor {
        if current.has_tag_name("HBox") && current.attribute("position") == Some("absolute") {
            return true;
        }
        ancestor = current.parent();
    }
    false
}

fn push_action(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    actions: &mut Vec<ActionRegion>,
    interaction: &InteractionState,
) {
    if node.has_tag_name("Button") && !button_enabled(node, interaction) {
        return;
    }
    let action = if node.has_tag_name("Checkbox") {
        node.attribute("id").map(|id| WindowAction::ToggleCheckbox {
            id: id.to_string(),
            checked: checkbox_checked(node, interaction),
        })
    } else if let Some((id, visible)) = node.attribute("action").and_then(parse_panel_action) {
        Some(WindowAction::SetPanelVisibility {
            id: id.to_string(),
            visible,
        })
    } else {
        match node.attribute("action") {
            Some("minimize") => Some(WindowAction::Minimize),
            Some("close") | Some("close_confirm") => Some(WindowAction::Close),
            Some("switch_language") => Some(WindowAction::ToggleLanguageMenu),
            Some("install") => Some(WindowAction::Install),
            Some("uninstall") => Some(WindowAction::Uninstall),
            _ => None,
        }
    };
    if let Some(action) = action {
        actions.push(ActionRegion {
            action,
            left: rect.left,
            top: rect.top,
            right: rect.left + rect.width,
            bottom: rect.top + rect.height,
        });
    }
}

fn button_enabled(node: roxmltree::Node<'_, '_>, interaction: &InteractionState) -> bool {
    if node.attribute("enabled") == Some("false") {
        return false;
    }
    if let Some(condition) = node.attribute("enabled-when") {
        return evaluate_ui_condition(condition, interaction);
    }
    true
}

fn evaluate_ui_condition(condition: &str, interaction: &InteractionState) -> bool {
    let Some((id, expected)) = condition.rsplit_once(':') else {
        return false;
    };
    match expected {
        "checked" => interaction
            .checkbox_states
            .get(id)
            .copied()
            .unwrap_or(false),
        "unchecked" => !interaction
            .checkbox_states
            .get(id)
            .copied()
            .unwrap_or(false),
        "visible" => interaction
            .panel_visibility
            .get(id)
            .copied()
            .unwrap_or(false),
        "hidden" => !interaction
            .panel_visibility
            .get(id)
            .copied()
            .unwrap_or(false),
        _ => false,
    }
}

fn button_image<'a>(
    node: roxmltree::Node<'a, 'a>,
    interaction: &InteractionState,
) -> Option<&'a str> {
    let enabled = button_enabled(node, interaction);
    let id = node.attribute("id");
    let attribute = if !enabled {
        "disabled-image"
    } else if id.is_some_and(|id| interaction.pressed_control.as_deref() == Some(id)) {
        "pressed-image"
    } else if id.is_some_and(|id| interaction.hovered_control.as_deref() == Some(id)) {
        "hover-image"
    } else {
        "normal-image"
    };
    node.attribute(attribute)
        .or_else(|| node.attribute("normal-image"))
}

fn push_hover_region(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    regions: &mut Vec<HoverRegion>,
) {
    if !node.has_tag_name("Button") || !button_enabled(node, context.interaction) {
        return;
    }
    let Some(id) = node.attribute("id") else {
        return;
    };
    if node.attribute("hover-image").is_none() && node.attribute("pressed-image").is_none() {
        return;
    }
    regions.push(HoverRegion {
        id: id.to_string(),
        left: rect.left,
        top: rect.top,
        right: rect.left + rect.width,
        bottom: rect.top + rect.height,
    });
}

fn checkbox_checked(node: roxmltree::Node<'_, '_>, interaction: &InteractionState) -> bool {
    node.attribute("id")
        .and_then(|id| interaction.checkbox_states.get(id))
        .copied()
        .unwrap_or_else(|| node.attribute("checked") == Some("true"))
}

fn render_language_select(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    menu_open: bool,
    output: &mut LayoutOutput,
) -> Result<()> {
    let arrow_source = if menu_open {
        node.attribute("dropdown-open-image")
            .or_else(|| node.attribute("dropdown-image"))
    } else {
        node.attribute("dropdown-image")
    };
    if let Some(source) = arrow_source {
        let width = scaled_float_attribute(node, "dropdown-image-width", 8.0, context.dpi);
        let height = scaled_float_attribute(node, "dropdown-image-height", 5.0, context.dpi);
        let right_padding = scale_value(10, context.dpi.scale);
        push_styled_layer(
            context.files,
            &mut output.layers,
            ImageStyle::plain(source),
            LayerRect {
                left: rect.left + rect.width - width - right_padding,
                top: rect.top + (rect.height - height) / 2,
                width,
                height,
            },
            context.dpi,
        )?;
    }
    if !menu_open {
        return Ok(());
    }

    let options: Vec<_> = node
        .children()
        .filter(|child| child.has_tag_name("Option") && !is_hidden(*child, context.interaction))
        .collect();
    let popup_width = scale_value(
        int_attribute(node, "popup-width")
            .unwrap_or_else(|| (rect.width as f32 / context.dpi.scale).round() as i32),
        context.dpi.scale,
    );
    let row_height = scale_value(
        int_attribute(node, "popup-row-height").unwrap_or(26),
        context.dpi.scale,
    );
    let padding = scale_value(
        int_attribute(node, "popup-padding").unwrap_or(0),
        context.dpi.scale,
    );
    let popup = LayerRect {
        left: rect.left + rect.width - popup_width,
        top: rect.top + rect.height + scale_value(4, context.dpi.scale),
        width: popup_width,
        height: row_height * i32::try_from(options.len()).unwrap_or(0) + padding * 2,
    };
    push_solid_layer(
        &mut output.overlay_layers,
        popup,
        node.attribute("popup-background").unwrap_or("#FF303F4B"),
        scale_value(4, context.dpi.scale),
    )?;
    for (index, option) in options.into_iter().enumerate() {
        let row = LayerRect {
            left: popup.left + padding,
            top: popup.top + padding + row_height * index as i32,
            width: popup.width - padding * 2,
            height: row_height,
        };
        let option_locale = option.attribute("value").unwrap_or_default();
        if option_locale == context.locale {
            push_solid_layer(
                &mut output.overlay_layers,
                row,
                node.attribute("popup-selected-background")
                    .unwrap_or("#FF42515E"),
                scale_value(3, context.dpi.scale),
            )?;
        }
        output.overlay_texts.push(TextLayer {
            runs: vec![TextRun {
                text: option
                    .attribute("text")
                    .unwrap_or(option_locale)
                    .to_string(),
                color: parse_color(node.attribute("color").unwrap_or("#FFFFFFFF")),
            }],
            left: row.left + scale_value(10, context.dpi.scale),
            top: row.top,
            width: row.width - scale_value(20, context.dpi.scale),
            height: row.height,
            font_size: scale_value(
                int_attribute(node, "font-size").unwrap_or(12),
                context.dpi.scale,
            )
            .max(1),
            bold: false,
            alignment: TextAlignment::Left,
            wrap: false,
        });
        output.actions.push(ActionRegion {
            action: WindowAction::SelectLanguage(option_locale.to_string()),
            left: row.left,
            top: row.top,
            right: row.left + row.width,
            bottom: row.top + row.height,
        });
    }
    Ok(())
}

fn push_node_text(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    texts: &mut Vec<TextLayer>,
) {
    let Some((text, alignment)) = resolved_text_for_node(node, context) else {
        return;
    };
    let color = parse_color(node.attribute("color").unwrap_or("#FFFFFFFF"));
    let link_color = node.attribute("linkcolor").map(parse_color);
    texts.push(TextLayer {
        runs: parse_text_runs(&text, color, link_color),
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
        font_size: scale_value(
            int_attribute(node, "font-size").unwrap_or(12),
            context.dpi.scale,
        )
        .max(1),
        bold: node.attribute("font-weight") == Some("bold"),
        alignment,
        wrap: node.has_tag_name("Checkbox") || node.attribute("wrap") == Some("true"),
    });
}

fn resolved_text_for_node(
    node: roxmltree::Node<'_, '_>,
    context: &LayoutContext<'_>,
) -> Option<(String, TextAlignment)> {
    if node.has_tag_name("TextInput") {
        return text_input_value(node, context).map(|value| (value, TextAlignment::Left));
    }
    let (mut text, alignment) = text_for_node(node, context.locale, context.translations)?;
    if let Some(source) = node.attribute("value-source") {
        let value = resolve_value_source(source, node.attribute("value-format"), context)
            .unwrap_or_else(|| "--".to_string());
        text.push_str(&value);
    }
    Some((text, alignment))
}

fn text_input_value(node: roxmltree::Node<'_, '_>, context: &LayoutContext<'_>) -> Option<String> {
    node.attribute("id")
        .and_then(|id| context.interaction.text_input_values.get(id))
        .cloned()
        .or_else(|| node.attribute("value").map(str::to_string))
        .or_else(|| {
            node.attribute("value-source")
                .and_then(|source| source.strip_prefix("config:"))
                .and_then(|path| config_value_as_string(context.config, path))
        })
}

fn resolve_value_source(
    source: &str,
    format: Option<&str>,
    context: &LayoutContext<'_>,
) -> Option<String> {
    let value = if let Some(path) = source.strip_prefix("config:") {
        config_value(context.config, path).cloned()
    } else if let Some(control_id) = source.strip_prefix("disk-free:") {
        let path = context.interaction.text_input_values.get(control_id)?;
        disk_free_bytes(Path::new(path)).map(serde_json::Value::from)
    } else {
        None
    }?;
    match format {
        Some("size-mb") => value
            .as_u64()
            .and_then(|megabytes| megabytes.checked_mul(1024 * 1024))
            .map(format_size_bytes),
        Some("size") => value.as_u64().map(format_size_bytes),
        _ => json_value_as_string(&value),
    }
}

fn config_value<'a>(config: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.')
        .try_fold(config, |value, key| value.get(key))
}

fn config_value_as_string(config: &serde_json::Value, path: &str) -> Option<String> {
    config_value(config, path).and_then(json_value_as_string)
}

fn json_value_as_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn disk_free_bytes(path: &Path) -> Option<u64> {
    let root = disk_root(path)?;
    query_disk_free_bytes(&root).ok()
}

fn disk_root(path: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut root = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => root.push(component.as_os_str()),
            _ => break,
        }
    }
    if root.as_os_str().is_empty() {
        return None;
    }
    Some(root)
}

fn query_disk_free_bytes(root: &Path) -> windows::core::Result<u64> {
    let directory = HSTRING::from(root.to_string_lossy().as_ref());
    let mut available = 0u64;
    unsafe {
        GetDiskFreeSpaceExW(&directory, Some(&mut available), None, None)?;
    }
    Ok(available)
}

fn format_size_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;
    if bytes >= TIB {
        format!("{:.1} TB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.1} GB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{} MB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{} KB", bytes / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn render_hbox(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    let children: Vec<_> = node
        .children()
        .filter(|child| child.is_element() && !is_hidden(*child, context.interaction))
        .collect();
    if children.is_empty() {
        return Ok(());
    }
    let gap = scale_value(
        int_attribute(node, "item-spacing")
            .or_else(|| int_attribute(node, "gap"))
            .unwrap_or(0),
        context.dpi.scale,
    );
    let items: Vec<_> = children
        .iter()
        .map(|child| flow_item_for_node(*child, context))
        .collect();
    let widths = flow_widths(&items, rect.width, gap);
    let center_items = node.attribute("align-items") == Some("center");
    let mut left = rect.left;
    for (index, child) in children.into_iter().enumerate() {
        let width = widths[index];
        let height = int_attribute(child, "height")
            .map(|value| scale_value(value, context.dpi.scale))
            .unwrap_or(rect.height);
        let top = if center_items {
            rect.top + (rect.height - height) / 2
        } else {
            rect.top
        };
        if width > 0 && height > 0 {
            render_flow_item(
                child,
                LayerRect {
                    left,
                    top,
                    width,
                    height,
                },
                context,
                output,
            )?;
        }
        left += width + gap;
    }
    Ok(())
}

fn flow_item_for_node(node: roxmltree::Node<'_, '_>, context: &LayoutContext<'_>) -> FlowItem {
    let explicit_width =
        int_attribute(node, "width").map(|value| scale_value(value, context.dpi.scale));
    let has_intrinsic_text =
        node.has_tag_name("Checkbox") || node.has_tag_name("Label") || node.has_tag_name("Button");
    let intrinsic_width = if has_intrinsic_text {
        resolved_text_for_node(node, context).map(|(text, _)| {
            let font_size = scale_value(
                int_attribute(node, "font-size").unwrap_or(12),
                context.dpi.scale,
            );
            let text_width = measure_layout_text_width(
                &text,
                font_size,
                node.attribute("font-weight") == Some("bold"),
            );
            let image_and_gap = if node.has_tag_name("Checkbox") {
                node.attribute("checked-image")
                    .or_else(|| node.attribute("unchecked-image"))
                    .map(parse_image_style)
                    .and_then(|style| style.destination)
                    .map(|rect| scale_value(rect.left + rect.width + 8, context.dpi.scale))
                    .unwrap_or_else(|| scale_value(20, context.dpi.scale))
            } else {
                0
            };
            let width = text_width + image_and_gap;
            int_attribute(node, "max-width")
                .map(|maximum| width.min(scale_value(maximum, context.dpi.scale)))
                .unwrap_or(width)
        })
    } else {
        None
    };
    FlowItem {
        fixed_width: explicit_width.or(intrinsic_width),
        flex_grow: float_attribute(node, "flex-grow").unwrap_or(0.0),
        flex_shrink: float_attribute(node, "flex-shrink").unwrap_or(1.0),
        min_width: scale_value(
            int_attribute(node, "min-width").unwrap_or(0),
            context.dpi.scale,
        ),
    }
}

fn estimate_text_width(text: &str, font_size: i32) -> i32 {
    visible_text(text)
        .chars()
        .map(|character| {
            if character as u32 > 0x2e80 {
                font_size as f32
            } else {
                font_size as f32 * 0.55
            }
        })
        .sum::<f32>()
        .ceil() as i32
}

fn visible_text(text: &str) -> String {
    parse_text_runs(text, COLORREF(0), Some(COLORREF(0)))
        .into_iter()
        .map(|run| run.text)
        .collect()
}

fn measure_layout_text_width(text: &str, font_size: i32, bold: bool) -> i32 {
    let visible = visible_text(text);
    unsafe {
        let dc = GetDC(None);
        if dc.is_invalid() {
            return estimate_text_width(&visible, font_size);
        }
        let font = create_ui_font(font_size, bold);
        if font.is_invalid() {
            let _ = ReleaseDC(None, dc);
            return estimate_text_width(&visible, font_size);
        }
        let previous = SelectObject(dc, font);
        let utf16: Vec<u16> = visible.encode_utf16().collect();
        let width = measure_text_width(dc, &utf16) + (font_size / 8).max(2);
        let _ = SelectObject(dc, previous);
        let _ = DeleteObject(font);
        let _ = ReleaseDC(None, dc);
        width
    }
}

fn render_flow_item(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    match node.tag_name().name() {
        "Checkbox" => {
            let checked = checkbox_checked(node, context.interaction);
            let image = if checked {
                node.attribute("checked-image")
            } else {
                node.attribute("unchecked-image")
            };
            let mut text_left = rect.left;
            if let Some(value) = image {
                let mut style = parse_image_style(value);
                if let Some(destination) = style.destination {
                    text_left +=
                        scale_value(destination.left + destination.width + 8, context.dpi.scale);
                    let logical_control_height = rect.height as f32 / context.dpi.scale;
                    style.destination = Some(LayerRect {
                        top: ((logical_control_height - destination.height as f32) / 2.0)
                            .round()
                            .max(0.0) as i32,
                        ..destination
                    });
                }
                push_styled_layer(context.files, &mut output.layers, style, rect, context.dpi)?;
            }
            push_action(node, rect, &mut output.actions, context.interaction);
            push_node_text(
                node,
                LayerRect {
                    left: text_left,
                    top: rect.top,
                    width: (rect.left + rect.width - text_left).max(0),
                    height: rect.height,
                },
                context,
                &mut output.texts,
            );
        }
        "Button" => {
            push_action(node, rect, &mut output.actions, context.interaction);
            push_hover_region(node, rect, context, &mut output.hover_regions);
            if let Some(value) = button_image(node, context.interaction) {
                push_styled_layer(
                    context.files,
                    &mut output.layers,
                    parse_image_style(value),
                    rect,
                    context.dpi,
                )?;
            }
            push_node_text(node, rect, context, &mut output.texts);
            if let Some(content) = node.children().find(|child| {
                child.has_tag_name("Content") && !is_hidden(*child, context.interaction)
            }) {
                render_horizontal_content(content, rect, context, output)?;
            }
        }
        "Label" | "Select" => {
            push_node_text(node, rect, context, &mut output.texts);
        }
        "Image" | "Icon" => {
            if let Some(source) = node.attribute("src") {
                push_styled_layer(
                    context.files,
                    &mut output.layers,
                    ImageStyle::plain(source),
                    rect,
                    context.dpi,
                )?;
            }
        }
        "Box" => render_box_contents(node, rect, context, output)?,
        _ => {}
    }
    Ok(())
}

fn render_horizontal_content(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    let children: Vec<_> = node
        .children()
        .filter(|child| child.is_element() && !is_hidden(*child, context.interaction))
        .collect();
    let gap = scale_value(
        int_attribute(node, "item-spacing").unwrap_or(0),
        context.dpi.scale,
    );
    let widths: Vec<_> = children
        .iter()
        .map(|child| {
            int_attribute(*child, "width")
                .or_else(|| int_attribute(*child, "max-width"))
                .map(|value| scale_value(value, context.dpi.scale))
                .unwrap_or(0)
        })
        .collect();
    let total_width = widths.iter().sum::<i32>()
        + gap * i32::try_from(children.len().saturating_sub(1)).unwrap_or(0);
    let mut left = if node.attribute("horizontal-align") == Some("right") {
        rect.left + (rect.width - total_width).max(0)
    } else {
        rect.left
    };
    for (index, child) in children.into_iter().enumerate() {
        let width = widths[index];
        let height = int_attribute(child, "height")
            .map(|value| scale_value(value, context.dpi.scale))
            .unwrap_or(rect.height);
        let top = if node.attribute("vertical-align") == Some("center") {
            rect.top + (rect.height - height) / 2
        } else {
            rect.top
        };
        render_flow_item(
            child,
            LayerRect {
                left,
                top,
                width,
                height,
            },
            context,
            output,
        )?;
        left += width + gap;
    }
    Ok(())
}

fn render_box_contents(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    if let Some(background) = node.attribute("background") {
        push_solid_layer(
            &mut output.layers,
            rect,
            background,
            scale_value(
                int_attribute(node, "border-radius").unwrap_or(0),
                context.dpi.scale,
            ),
        )?;
    }
    for child in node
        .children()
        .filter(|child| child.is_element() && !is_hidden(*child, context.interaction))
    {
        let width = scale_value(
            int_attribute(child, "width").unwrap_or(0),
            context.dpi.scale,
        );
        let height = scale_value(
            int_attribute(child, "height").unwrap_or(0),
            context.dpi.scale,
        );
        let left = if child.attribute("position") == Some("absolute") {
            rect.left + scale_value(int_attribute(child, "left").unwrap_or(0), context.dpi.scale)
        } else {
            rect.left + (rect.width - width) / 2
        };
        let top = if child.attribute("position") == Some("absolute") {
            rect.top + scale_value(int_attribute(child, "top").unwrap_or(0), context.dpi.scale)
        } else {
            rect.top + (rect.height - height) / 2
        };
        let child_rect = LayerRect {
            left,
            top,
            width,
            height,
        };
        match child.tag_name().name() {
            "Image" | "Icon" => {
                if let Some(source) = child.attribute("src") {
                    push_styled_layer(
                        context.files,
                        &mut output.layers,
                        ImageStyle::plain(source),
                        child_rect,
                        context.dpi,
                    )?;
                }
            }
            "TextInput" => push_node_text(child, child_rect, context, &mut output.texts),
            _ => {}
        }
    }
    Ok(())
}

fn flow_widths(items: &[FlowItem], available_width: i32, gap: i32) -> Vec<i32> {
    let gap_width = gap * i32::try_from(items.len().saturating_sub(1)).unwrap_or(0);
    let fixed_width = items
        .iter()
        .filter_map(|item| item.fixed_width)
        .sum::<i32>();
    let available_for_items = (available_width - gap_width).max(0);
    if fixed_width > available_for_items {
        let overflow = fixed_width - available_for_items;
        let total_capacity = items
            .iter()
            .map(|item| {
                item.fixed_width
                    .map(|width| {
                        ((width - item.min_width).max(0) as f32 * item.flex_shrink.max(0.0)).round()
                            as i32
                    })
                    .unwrap_or(0)
            })
            .sum::<i32>();
        let last_shrinkable = items.iter().rposition(|item| {
            item.fixed_width.is_some_and(|width| width > item.min_width) && item.flex_shrink > 0.0
        });
        let mut reduced = 0;
        return items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let Some(width) = item.fixed_width else {
                    return 0;
                };
                let capacity = ((width - item.min_width).max(0) as f32 * item.flex_shrink.max(0.0))
                    .round() as i32;
                if capacity == 0 || total_capacity == 0 {
                    return width;
                }
                let reduction = if Some(index) == last_shrinkable {
                    (overflow - reduced).min(capacity)
                } else {
                    ((overflow as f32 * capacity as f32 / total_capacity as f32).round() as i32)
                        .min(capacity)
                        .min(overflow - reduced)
                };
                reduced += reduction;
                width - reduction
            })
            .collect();
    }
    let remaining = available_for_items - fixed_width;
    let total_flex = items
        .iter()
        .filter(|item| item.fixed_width.is_none())
        .map(|item| item.flex_grow.max(0.0))
        .sum::<f32>();
    let last_flexible = items
        .iter()
        .rposition(|item| item.fixed_width.is_none() && item.flex_grow > 0.0);
    let mut distributed = 0;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            if let Some(width) = item.fixed_width {
                return width;
            }
            if total_flex <= 0.0 || item.flex_grow <= 0.0 {
                return 0;
            }
            let width = if Some(index) == last_flexible {
                remaining - distributed
            } else {
                ((remaining as f32 * item.flex_grow / total_flex).round() as i32)
                    .min(remaining - distributed)
            };
            distributed += width;
            width
        })
        .collect()
}

fn text_for_node(
    node: roxmltree::Node<'_, '_>,
    locale: &str,
    translations: &HashMap<String, String>,
) -> Option<(String, TextAlignment)> {
    let raw = match node.tag_name().name() {
        "Button" | "Checkbox" | "Label" => {
            node.attribute("text").or_else(|| node.attribute("value"))?
        }
        "Select" => node
            .children()
            .find(|option| {
                option.has_tag_name("Option") && option.attribute("value") == Some(locale)
            })
            .or_else(|| node.children().find(|option| option.has_tag_name("Option")))?
            .attribute("text")?,
        _ => return None,
    };
    let text = raw
        .strip_prefix('@')
        .and_then(|key| translations.get(key))
        .cloned()
        .unwrap_or_else(|| raw.to_string());
    let explicit_alignment = node
        .attribute("textalign")
        .or_else(|| node.attribute("text-align"));
    let alignment = match explicit_alignment {
        Some("center") => TextAlignment::Center,
        Some("right") => TextAlignment::Right,
        _ if node.has_tag_name("Button") => TextAlignment::Center,
        _ => TextAlignment::Left,
    };
    Some((text, alignment))
}

fn parse_text_runs(text: &str, color: COLORREF, link_color: Option<COLORREF>) -> Vec<TextRun> {
    let Some(link_color) = link_color else {
        return vec![TextRun {
            text: text.to_string(),
            color,
        }];
    };
    let mut runs = Vec::new();
    let mut remaining = text;
    while let Some(open) = remaining.find('[') {
        let label_start = open + 1;
        let Some(label_end_relative) = remaining[label_start..].find("](") else {
            break;
        };
        let label_end = label_start + label_end_relative;
        let target_start = label_end + 2;
        let Some(target_end_relative) = remaining[target_start..].find(')') else {
            break;
        };
        let target_end = target_start + target_end_relative;
        if open > 0 {
            runs.push(TextRun {
                text: remaining[..open].to_string(),
                color,
            });
        }
        runs.push(TextRun {
            text: remaining[label_start..label_end].to_string(),
            color: link_color,
        });
        remaining = &remaining[target_end + 1..];
    }
    if !remaining.is_empty() {
        runs.push(TextRun {
            text: remaining.to_string(),
            color,
        });
    }
    if runs.is_empty() {
        runs.push(TextRun {
            text: text.to_string(),
            color,
        });
    }
    runs
}

fn parse_color(value: &str) -> COLORREF {
    let (_, red, green, blue) = parse_argb(value);
    COLORREF(red as u32 | ((green as u32) << 8) | ((blue as u32) << 16))
}

fn parse_argb(value: &str) -> (u8, u8, u8, u8) {
    let hex = value.trim_start_matches('#');
    let parsed = u32::from_str_radix(hex, 16).unwrap_or(0xFFFF_FFFF);
    match hex.len() {
        8 => (
            (parsed >> 24) as u8,
            (parsed >> 16) as u8,
            (parsed >> 8) as u8,
            parsed as u8,
        ),
        6 => (255, (parsed >> 16) as u8, (parsed >> 8) as u8, parsed as u8),
        _ => (255, 255, 255, 255),
    }
}

fn parse_image_style(value: &str) -> ImageStyle<'_> {
    let source = style_attribute(value, "file").unwrap_or(value);
    let destination = style_attribute(value, "dest").and_then(parse_style_rect);
    let alpha = style_attribute(value, "fade")
        .and_then(|raw| raw.parse::<u8>().ok())
        .unwrap_or(255);
    ImageStyle {
        source,
        destination,
        alpha,
    }
}

fn style_attribute<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}='");
    let rest = value.split_once(&marker)?.1;
    rest.split_once('\'').map(|(attribute, _)| attribute)
}

fn parse_style_rect(value: &str) -> Option<LayerRect> {
    let mut values = value.split(',').map(|item| item.trim().parse::<i32>());
    let left = values.next()?.ok()?;
    let top = values.next()?.ok()?;
    let right = values.next()?.ok()?;
    let bottom = values.next()?.ok()?;
    if values.next().is_some() || right <= left || bottom <= top {
        return None;
    }
    Some(LayerRect {
        left,
        top,
        width: right - left,
        height: bottom - top,
    })
}

fn int_attribute(node: roxmltree::Node<'_, '_>, name: &str) -> Option<i32> {
    node.attribute(name)?.parse().ok()
}

fn float_attribute(node: roxmltree::Node<'_, '_>, name: &str) -> Option<f32> {
    node.attribute(name)?.parse().ok()
}

fn scaled_float_attribute(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    default: f32,
    dpi: DpiContext,
) -> i32 {
    (float_attribute(node, name).unwrap_or(default) * dpi.scale).round() as i32
}

fn scale_value(value: i32, scale: f32) -> i32 {
    (value as f32 * scale).round() as i32
}

fn resolve_asset_path<'a>(
    files: &'a HashMap<String, Vec<u8>>,
    source: &str,
    use_2x: bool,
) -> Option<&'a str> {
    let (stem, extension) = match source.rsplit_once('.') {
        Some((stem, extension)) if !extension.contains(['/', '\\']) => (stem, Some(extension)),
        _ => (source, None),
    };
    let base_stem = stem.strip_suffix("@2x").unwrap_or(stem);
    let suffix = extension
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let base = format!("{base_stem}{suffix}");
    let high_dpi = format!("{base_stem}@2x{suffix}");
    let preferred = if use_2x { &high_dpi } else { &base };
    let fallback = if use_2x { &base } else { &high_dpi };
    files
        .get_key_value(preferred)
        .or_else(|| files.get_key_value(fallback))
        .map(|(path, _)| path.as_str())
}

fn push_styled_layer(
    files: &HashMap<String, Vec<u8>>,
    layers: &mut Vec<ImageLayer>,
    style: ImageStyle<'_>,
    control_rect: LayerRect,
    dpi: DpiContext,
) -> Result<()> {
    let rect = if let Some(destination) = style.destination {
        LayerRect {
            left: control_rect.left + scale_value(destination.left, dpi.scale),
            top: control_rect.top + scale_value(destination.top, dpi.scale),
            width: scale_value(destination.width, dpi.scale),
            height: scale_value(destination.height, dpi.scale),
        }
    } else {
        control_rect
    };
    push_layer(files, layers, style.source, rect, dpi.use_2x, style.alpha)
}

fn push_solid_layer(
    layers: &mut Vec<ImageLayer>,
    rect: LayerRect,
    color: &str,
    radius: i32,
) -> Result<()> {
    let width = u32::try_from(rect.width).context("solid layer width must be positive")?;
    let height = u32::try_from(rect.height).context("solid layer height must be positive")?;
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .context("solid layer size overflow")?;
    let mut pixels = vec![
        0u8;
        pixel_count
            .checked_mul(4)
            .context("solid layer size overflow")?
    ];
    let (alpha, red, green, blue) = parse_argb(color);
    let premultiply = |component: u8| ((component as u16 * alpha as u16) / 255) as u8;
    let radius = radius.min(rect.width / 2).min(rect.height / 2).max(0);
    for y in 0..rect.height {
        for x in 0..rect.width {
            if !inside_rounded_rect(x, y, rect.width, rect.height, radius) {
                continue;
            }
            let offset = ((y as usize * width as usize) + x as usize) * 4;
            pixels[offset] = premultiply(blue);
            pixels[offset + 1] = premultiply(green);
            pixels[offset + 2] = premultiply(red);
            pixels[offset + 3] = alpha;
        }
    }
    layers.push(ImageLayer {
        image: NativeImage {
            width,
            height,
            pixels,
        },
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
        alpha: 255,
    });
    Ok(())
}

fn inside_rounded_rect(x: i32, y: i32, width: i32, height: i32, radius: i32) -> bool {
    if radius <= 0 {
        return true;
    }
    let center_x = if x < radius {
        radius
    } else if x >= width - radius {
        width - radius - 1
    } else {
        x
    };
    let center_y = if y < radius {
        radius
    } else if y >= height - radius {
        height - radius - 1
    } else {
        y
    };
    let delta_x = x - center_x;
    let delta_y = y - center_y;
    delta_x * delta_x + delta_y * delta_y <= radius * radius
}

fn push_layer(
    files: &HashMap<String, Vec<u8>>,
    layers: &mut Vec<ImageLayer>,
    source: &str,
    rect: LayerRect,
    use_2x: bool,
    alpha: u8,
) -> Result<()> {
    let resolved = resolve_asset_path(files, source, use_2x)
        .with_context(|| format!("layout asset missing from native bundle: {source}"))?;
    let encoded = files.get(resolved).expect("resolved asset path must exist");
    layers.push(ImageLayer {
        image: decode_image(encoded)?,
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
        alpha,
    });
    Ok(())
}

fn decode_image(encoded: &[u8]) -> Result<NativeImage> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let _guard = ComGuard;
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = factory.CreateStream()?;
        stream.InitializeFromMemory(encoded)?;
        let decoder = factory.CreateDecoderFromStream(
            &stream,
            std::ptr::null(),
            WICDecodeMetadataCacheOnLoad,
        )?;
        let frame = decoder.GetFrame(0)?;
        let converter = factory.CreateFormatConverter()?;
        converter.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None::<&IWICPalette>,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let mut width = 0;
        let mut height = 0;
        converter.GetSize(&mut width, &mut height)?;
        let stride = width.checked_mul(4).context("bitmap stride overflow")?;
        let size = stride.checked_mul(height).context("bitmap size overflow")? as usize;
        let mut pixels = vec![0; size];
        converter.CopyPixels(std::ptr::null(), stride, &mut pixels)?;
        Ok(NativeImage {
            width,
            height,
            pixels,
        })
    }
}

fn run_window(client_width: i32, client_height: i32) -> Result<()> {
    unsafe {
        let module = GetModuleHandleW(None)?;
        let instance = HINSTANCE(module.0);
        let class_name = w!("NanoInstallerNativeRuntime");
        // The builder injects the project icon as RT_GROUP_ICON resource ID 1.
        // Raw stubs remain resource-free when executed on their own.
        let project_icon = LoadIconW(instance, win32_resource_id(1)).unwrap_or_default();
        let window_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hIcon: project_icon,
            hIconSm: project_icon,
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassExW(&window_class) == 0 {
            return Err(windows::core::Error::from_win32().into());
        }
        let style = WS_POPUP;
        let left = (GetSystemMetrics(SM_CXSCREEN) - client_width) / 2;
        let top = (GetSystemMetrics(SM_CYSCREEN) - client_height) / 2;
        let window = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name,
            w!("nano-installer native Win32"),
            style,
            left,
            top,
            client_width,
            client_height,
            None,
            None,
            instance,
            None,
        )?;
        let _ = SendMessageW(
            window,
            WM_SETICON,
            WPARAM(ICON_BIG as usize),
            LPARAM(project_icon.0 as isize),
        );
        let _ = SendMessageW(
            window,
            WM_SETICON,
            WPARAM(ICON_SMALL as usize),
            LPARAM(project_icon.0 as isize),
        );
        if let Some(state) = UI.get().and_then(|state| state.lock().ok()) {
            if state.ui.corner_radius > 0 {
                let region = CreateRoundRectRgn(
                    0,
                    0,
                    client_width + 1,
                    client_height + 1,
                    state.ui.corner_radius * 2,
                    state.ui.corner_radius * 2,
                );
                if !region.is_invalid() {
                    let _ = SetWindowRgn(window, region, true);
                }
            }
        }
        let _ = ShowWindow(window, SW_SHOW);
        let _ = UpdateWindow(window);
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

fn win32_resource_id(id: u16) -> PCWSTR {
    PCWSTR::from_raw(id as usize as *const u16)
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(window, &mut paint);
            if let Some(state) = UI.get().and_then(|state| state.lock().ok()) {
                let mut client = RECT::default();
                let _ = GetClientRect(window, &mut client);
                paint_ui_frame(dc, &client, &state.ui);
            }
            let _ = EndPaint(window, &paint);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = client_point(lparam);
            let _ = set_pressed_control(window, None);
            if let Some(action) = window_action_at(x, y) {
                handle_window_action(window, action);
            } else {
                let _ = set_language_menu_open(window, false);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = client_point(lparam);
            if let Some(id) = hover_control_at(x, y) {
                let _ = set_pressed_control(window, Some(id));
                return LRESULT(0);
            }
            let over_action = window_action_at(x, y).is_some();
            let over_caption = UI
                .get()
                .and_then(|state| state.lock().ok())
                .is_some_and(|state| y < state.ui.caption_height);
            if over_caption && !over_action {
                let _ = ReleaseCapture();
                let _ = SendMessageW(window, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), lparam);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = client_point(lparam);
            let _ = set_hovered_control(window, hover_control_at(x, y));
            let mut tracking = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: window,
                dwHoverTime: 0,
            };
            let _ = TrackMouseEvent(&mut tracking);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            let _ = set_hovered_control(window, None);
            let _ = set_pressed_control(window, None);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u32 == 0x1B && !install::busy() => {
            let _ = DestroyWindow(window);
            LRESULT(0)
        }
        WM_CLOSE if install::busy() => LRESULT(0),
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

unsafe fn paint_ui_frame(destination: HDC, client: &RECT, ui: &RuntimeUi) {
    let width = client.right - client.left;
    let height = client.bottom - client.top;
    let backbuffer = CreateCompatibleDC(destination);
    if backbuffer.is_invalid() {
        draw_ui_frame(destination, ui);
        return;
    }
    let bitmap = CreateCompatibleBitmap(destination, width, height);
    if bitmap.is_invalid() {
        let _ = DeleteDC(backbuffer);
        draw_ui_frame(destination, ui);
        return;
    }
    let previous = SelectObject(backbuffer, bitmap);
    draw_ui_frame(backbuffer, ui);
    let _ = BitBlt(destination, 0, 0, width, height, backbuffer, 0, 0, SRCCOPY);
    let _ = SelectObject(backbuffer, previous);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(backbuffer);
}

unsafe fn draw_ui_frame(destination: HDC, ui: &RuntimeUi) {
    for layer in &ui.layers {
        draw_layer(destination, layer);
    }
    for text in &ui.texts {
        draw_text(destination, text);
    }
    for layer in &ui.overlay_layers {
        draw_layer(destination, layer);
    }
    for text in &ui.overlay_texts {
        draw_text(destination, text);
    }
}

fn window_action_at(x: i32, y: i32) -> Option<WindowAction> {
    let state = UI.get()?.lock().ok()?;
    state
        .ui
        .actions
        .iter()
        .rev()
        .find(|region| x >= region.left && x < region.right && y >= region.top && y < region.bottom)
        .map(|region| region.action.clone())
}

fn hover_control_at(x: i32, y: i32) -> Option<String> {
    let state = UI.get()?.lock().ok()?;
    state
        .ui
        .hover_regions
        .iter()
        .rev()
        .find(|region| x >= region.left && x < region.right && y >= region.top && y < region.bottom)
        .map(|region| region.id.clone())
}

unsafe fn handle_window_action(window: HWND, action: WindowAction) {
    match action {
        WindowAction::Close => {
            if !install::busy() {
                let _ = DestroyWindow(window);
            }
        }
        WindowAction::Minimize => {
            let _ = ShowWindow(window, SW_MINIMIZE);
        }
        WindowAction::ToggleLanguageMenu => {
            let open = UI
                .get()
                .and_then(|state| state.lock().ok())
                .is_some_and(|state| !state.language_menu_open);
            if let Err(error) = set_language_menu_open(window, open) {
                show_runtime_error(&error);
            }
        }
        WindowAction::SelectLanguage(locale) => {
            if let Err(error) = select_language(window, locale) {
                show_runtime_error(&error);
            }
        }
        WindowAction::Install => install::start_install(window),
        WindowAction::Uninstall => install::start_uninstall(window),
        WindowAction::ToggleCheckbox { id, checked } => {
            if let Err(error) = set_checkbox_state(window, id, !checked) {
                show_runtime_error(&error);
            }
        }
        WindowAction::SetPanelVisibility { id, visible } => {
            if let Err(error) = set_panel_visibility(window, id, visible) {
                show_runtime_error(&error);
            }
        }
    }
}

unsafe fn set_hovered_control(window: HWND, id: Option<String>) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    if state.interaction.hovered_control == id {
        return Ok(());
    }
    state.interaction.hovered_control = id;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

unsafe fn set_pressed_control(window: HWND, id: Option<String>) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    if state.interaction.pressed_control == id {
        return Ok(());
    }
    state.interaction.pressed_control = id;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

unsafe fn set_language_menu_open(window: HWND, open: bool) -> Result<()> {
    let Some(runtime) = UI.get() else {
        return Ok(());
    };
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    if state.language_menu_open == open {
        return Ok(());
    }
    state.language_menu_open = open;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

unsafe fn select_language(window: HWND, locale: String) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    state.locale = locale;
    state.language_menu_open = false;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

unsafe fn set_checkbox_state(window: HWND, id: String, checked: bool) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    state.interaction.checkbox_states.insert(id, checked);
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

unsafe fn set_panel_visibility(window: HWND, id: String, visible: bool) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    state.interaction.panel_visibility.insert(id, visible);
    state.language_menu_open = false;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

fn rebuild_runtime_ui(state: &mut RuntimeState) -> Result<()> {
    state.ui = load_layout(
        &state.files,
        state.dpi,
        &state.locale,
        state.language_menu_open,
        &state.interaction,
        state.mode,
    )?;
    Ok(())
}

fn client_point(lparam: LPARAM) -> (i32, i32) {
    let x = lparam.0 as i16 as i32;
    let y = (lparam.0 >> 16) as i16 as i32;
    (x, y)
}

unsafe fn draw_text(destination: HDC, layer: &TextLayer) {
    let font = create_ui_font(layer.font_size, layer.bold);
    if font.is_invalid() {
        return;
    }
    let previous = SelectObject(destination, font);
    let _ = SetBkMode(destination, TRANSPARENT);
    if layer.wrap {
        draw_wrapped_text(destination, layer);
    } else {
        draw_single_line_text(destination, layer);
    }
    let _ = SelectObject(destination, previous);
    let _ = DeleteObject(font);
}

unsafe fn create_ui_font(font_size: i32, bold: bool) -> HFONT {
    CreateFontW(
        -font_size,
        0,
        0,
        0,
        if bold {
            FW_BOLD.0 as i32
        } else {
            FW_NORMAL.0 as i32
        },
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        w!("Microsoft YaHei"),
    )
}

unsafe fn draw_single_line_text(destination: HDC, layer: &TextLayer) {
    let mut measured_runs = Vec::with_capacity(layer.runs.len());
    let mut total_width = 0;
    for run in &layer.runs {
        let text: Vec<u16> = run.text.encode_utf16().collect();
        let width = measure_text_width(destination, &text);
        total_width += width;
        measured_runs.push((run, text, width));
    }
    let mut left = match layer.alignment {
        TextAlignment::Left => layer.left,
        TextAlignment::Center => layer.left + (layer.width - total_width) / 2,
        TextAlignment::Right => layer.left + layer.width - total_width,
    };
    for (run, mut text, width) in measured_runs {
        if left >= layer.left + layer.width {
            break;
        }
        let _ = SetTextColor(destination, run.color);
        let mut bounds = RECT {
            left,
            top: layer.top,
            right: layer.left + layer.width,
            bottom: layer.top + layer.height,
        };
        let _ = DrawTextW(
            destination,
            &mut text,
            &mut bounds,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        left += width;
    }
}

unsafe fn draw_wrapped_text(destination: HDC, layer: &TextLayer) {
    let mut lines: Vec<Vec<(COLORREF, Vec<u16>, i32)>> = vec![Vec::new()];
    let mut line_widths = vec![0i32];
    for run in &layer.runs {
        for character in run.text.chars() {
            if character == '\n' {
                lines.push(Vec::new());
                line_widths.push(0);
                continue;
            }
            let text: Vec<u16> = character.to_string().encode_utf16().collect();
            let width = measure_text_width(destination, &text);
            let line_index = lines.len() - 1;
            if line_widths[line_index] + width > layer.width && !lines[line_index].is_empty() {
                lines.push(Vec::new());
                line_widths.push(0);
            }
            let line_index = lines.len() - 1;
            line_widths[line_index] += width;
            lines[line_index].push((run.color, text, width));
        }
    }
    let line_height = (layer.font_size * 3 / 2).max(layer.font_size);
    let total_height = line_height * i32::try_from(lines.len()).unwrap_or(1);
    let mut top = layer.top + (layer.height - total_height).max(0) / 2;
    for (line, line_width) in lines.into_iter().zip(line_widths) {
        if top >= layer.top + layer.height {
            break;
        }
        let mut left = match layer.alignment {
            TextAlignment::Left => layer.left,
            TextAlignment::Center => layer.left + (layer.width - line_width) / 2,
            TextAlignment::Right => layer.left + layer.width - line_width,
        };
        for (color, mut text, width) in line {
            let _ = SetTextColor(destination, color);
            let mut bounds = RECT {
                left,
                top,
                right: layer.left + layer.width,
                bottom: (top + line_height).min(layer.top + layer.height),
            };
            let _ = DrawTextW(
                destination,
                &mut text,
                &mut bounds,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            left += width;
        }
        top += line_height;
    }
}

unsafe fn measure_text_width(destination: HDC, text: &[u16]) -> i32 {
    let mut size = SIZE::default();
    if GetTextExtentPoint32W(destination, text, &mut size).as_bool() {
        size.cx.max(0)
    } else {
        0
    }
}

unsafe fn draw_layer(destination: HDC, layer: &ImageLayer) {
    let source = CreateCompatibleDC(destination);
    if source.is_invalid() {
        return;
    }
    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: layer.image.width as i32,
            biHeight: -(layer.image.height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let Ok(bitmap) = CreateDIBSection(
        destination,
        &bitmap_info,
        DIB_RGB_COLORS,
        &mut bits,
        None,
        0,
    ) else {
        let _ = DeleteDC(source);
        return;
    };
    std::ptr::copy_nonoverlapping(
        layer.image.pixels.as_ptr(),
        bits.cast(),
        layer.image.pixels.len(),
    );
    let previous = SelectObject(source, bitmap);
    let _ = GdiAlphaBlend(
        destination,
        layer.left,
        layer.top,
        layer.width,
        layer.height,
        source,
        0,
        0,
        layer.image.width as i32,
        layer.image.height as i32,
        BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: layer.alpha,
            AlphaFormat: AC_SRC_ALPHA as u8,
        },
    );
    let _ = SelectObject(source, previous);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(source);
}

#[cfg(test)]
mod tests {
    use super::{
        button_image, disk_root, flow_widths, format_size_bytes, initial_interaction,
        inspect_project, installer_version_info, load_layout, measure_layout_text_width,
        pack_project, pack_project_with_progress, parse_bundle, parse_color, parse_image_style,
        parse_text_runs, query_disk_free_bytes, resolve_asset_path, runtime_layout_path,
        scale_value, uninstaller_version_info, validate_output_filename, BundleIndex, DpiContext,
        FlowItem, InteractionState, PayloadFormat, RuntimeMode, TextAlignment, WindowAction,
        FOOTER_MAGIC,
    };
    use anyhow::Context;
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn project_bundle_roundtrips_layout_assets_and_locales() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path();
        for directory in ["layouts", "assets", "locales"] {
            std::fs::create_dir(project.join(directory))?;
        }
        std::fs::write(
            project.join("installer_config.json"),
            br#"{"resources":{"payload_file":"payload/app.7z"},"wizard":{"pages":[{"layout":"layouts/config.xml"}]}}"#,
        )?;
        std::fs::write(
            project.join("layouts/config.xml"),
            br#"<Page width="720" height="450" />"#,
        )?;
        std::fs::write(project.join("assets/background.png"), b"png")?;
        std::fs::write(project.join("locales/zh-CN.json"), b"{}")?;
        std::fs::create_dir(project.join("payload"))?;
        std::fs::write(project.join("payload/app.7z"), b"payload")?;

        let packed = pack_project(project, None)?;
        let files = parse_bundle(&packed)?;

        assert_eq!(files.len(), 5);
        assert!(files.contains_key("installer_config.json"));
        assert!(files.contains_key("layouts/config.xml"));
        assert!(files.contains_key("assets/background.png"));
        assert!(files.contains_key("locales/zh-CN.json"));
        assert_eq!(files.get("payload/app.7z").unwrap(), b"payload");
        Ok(())
    }

    fn write_setup_image(path: &Path, bundle: &[u8]) -> anyhow::Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)?;
        file.write_all(b"stub-image")?;
        file.write_all(bundle)?;
        file.write_all(&(bundle.len() as u64).to_le_bytes())?;
        file.write_all(FOOTER_MAGIC)?;
        file.flush()?;
        Ok(())
    }

    #[test]
    fn bundle_index_streams_entries_without_loading_the_payload() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        for directory in ["layouts", "assets", "locales", "payload"] {
            std::fs::create_dir_all(project.join(directory))?;
        }
        std::fs::write(
            project.join("installer_config.json"),
            br#"{"resources":{"payload_file":"payload/app.7z"}}"#,
        )?;
        std::fs::write(project.join("layouts/config.xml"), b"layout")?;
        std::fs::write(project.join("assets/background.png"), b"png")?;
        std::fs::write(project.join("locales/zh-CN.json"), b"{}")?;
        // Larger than the streaming chunk size, so the copy loop wraps.
        let payload = vec![0xABu8; 3 * 1024 * 1024 + 7];
        std::fs::write(project.join("payload/app.7z"), &payload)?;

        let bundle = pack_project(&project, None)?;
        let image = temp.path().join("setup.exe");
        write_setup_image(&image, &bundle)?;

        let index = BundleIndex::read(&image)?.context("setup image carries a bundle")?;
        assert!(index.contains("payload/app.7z"));

        // Reading one entry returns exactly the packed bytes.
        assert_eq!(index.read_file("payload/app.7z")?, payload);

        // The UI file set excludes the payload, which is the whole point of the
        // change: the payload is never materialized in memory.
        let ui = index.read_ui_files()?;
        assert_eq!(ui.len(), 4);
        assert!(ui.contains_key("installer_config.json"));
        assert!(ui.contains_key("layouts/config.xml"));
        assert!(ui.contains_key("assets/background.png"));
        assert!(ui.contains_key("locales/zh-CN.json"));
        assert!(!ui.contains_key("payload/app.7z"));

        // Streaming copy reproduces the payload byte for byte.
        let streamed = temp.path().join("streamed.7z");
        assert_eq!(
            index.copy_file_to("payload/app.7z", &streamed)?,
            payload.len() as u64
        );
        assert_eq!(std::fs::read(&streamed)?, payload);

        let missing = temp.path().join("missing.7z");
        assert!(index.copy_file_to("payload/absent.7z", &missing).is_err());
        assert!(!missing.exists());
        Ok(())
    }

    #[test]
    fn bundle_index_ignores_images_without_a_footer() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let plain = temp.path().join("plain.exe");
        std::fs::write(&plain, b"not a setup image")?;
        assert!(BundleIndex::read(&plain)?.is_none());

        // A footer marker whose length field is larger than the file must not
        // be accepted as a valid bundle.
        let truncated = temp.path().join("truncated.exe");
        let mut bytes = b"stub".to_vec();
        bytes.extend_from_slice(&u64::MAX.to_le_bytes());
        bytes.extend_from_slice(FOOTER_MAGIC);
        std::fs::write(&truncated, &bytes)?;
        assert!(BundleIndex::read(&truncated).is_err());
        Ok(())
    }

    #[test]
    fn project_pack_progress_describes_assets_payload_and_uninstaller() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path();
        for directory in ["layouts", "assets", "locales", "payload"] {
            std::fs::create_dir(project.join(directory))?;
        }
        std::fs::write(
            project.join("installer_config.json"),
            br#"{"resources":{"payload_file":"payload/app.7z"}}"#,
        )?;
        std::fs::write(project.join("layouts/config.xml"), b"layout")?;
        std::fs::write(project.join("assets/background.png"), b"png")?;
        std::fs::write(project.join("locales/zh-CN.json"), b"{}")?;
        std::fs::write(project.join("payload/app.7z"), b"payload")?;

        let mut messages = Vec::new();
        let bundle = pack_project_with_progress(
            project,
            Some(("runtime/uninst-stub-native.exe", b"uninstaller".to_vec())),
            true,
            |message| messages.push(message),
        )?;
        let files = parse_bundle(&bundle)?;

        assert!(files.contains_key("runtime/uninst-stub-native.exe"));
        assert!(messages
            .iter()
            .any(|message| message.contains("Collected assets/: 1 files")));
        assert!(messages
            .iter()
            .any(|message| message.contains("no intermediate skins.zip")));
        assert!(messages
            .iter()
            .any(|message| message.contains("Added payload payload/app.7z")));
        assert!(messages
            .iter()
            .any(|message| message.contains("Embedded runtime/uninst-stub-native.exe")));
        Ok(())
    }

    #[test]
    fn image_style_supports_plain_path_destination_and_fade() {
        let plain = parse_image_style("assets/button.png");
        assert_eq!(plain.source, "assets/button.png");
        assert!(plain.destination.is_none());
        assert_eq!(plain.alpha, 255);

        let styled = parse_image_style("file='assets/button.png' dest='7,7,27,27' fade='160'");
        let destination = styled.destination.unwrap();
        assert_eq!(styled.source, "assets/button.png");
        assert_eq!(
            (
                destination.left,
                destination.top,
                destination.width,
                destination.height
            ),
            (7, 7, 20, 20)
        );
        assert_eq!(styled.alpha, 160);
    }

    #[test]
    fn dpi_asset_resolution_prefers_requested_density_and_falls_back() {
        let mut files = HashMap::new();
        files.insert("assets/logo.png".to_string(), Vec::new());
        files.insert("assets/logo@2x.png".to_string(), Vec::new());
        files.insert("assets/base-only.png".to_string(), Vec::new());

        assert_eq!(
            resolve_asset_path(&files, "assets/logo.png", false),
            Some("assets/logo.png")
        );
        assert_eq!(
            resolve_asset_path(&files, "assets/logo.png", true),
            Some("assets/logo@2x.png")
        );
        assert_eq!(
            resolve_asset_path(&files, "assets/logo@2x.png", false),
            Some("assets/logo.png")
        );
        assert_eq!(
            resolve_asset_path(&files, "assets/base-only.png", true),
            Some("assets/base-only.png")
        );
    }

    #[test]
    fn dpi_scaling_rounds_layout_coordinates() {
        assert_eq!(scale_value(720, 1.5), 1080);
        assert_eq!(scale_value(13, 1.5), 20);
        assert_eq!(scale_value(-3, 2.0), -6);
    }

    #[test]
    fn bottom_hbox_distributes_fixed_and_flexible_items() {
        let widths = flow_widths(
            &[
                FlowItem {
                    fixed_width: None,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: 0,
                },
                FlowItem {
                    fixed_width: None,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: 0,
                },
                FlowItem {
                    fixed_width: Some(184),
                    flex_grow: 0.0,
                    flex_shrink: 0.0,
                    min_width: 0,
                },
            ],
            640,
            32,
        );
        assert_eq!(widths, [196, 196, 184]);
    }

    #[test]
    fn hbox_shrinks_text_before_fixed_button() {
        let widths = flow_widths(
            &[
                FlowItem {
                    fixed_width: Some(500),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: 0,
                },
                FlowItem {
                    fixed_width: None,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: 0,
                },
                FlowItem {
                    fixed_width: Some(184),
                    flex_grow: 0.0,
                    flex_shrink: 0.0,
                    min_width: 0,
                },
            ],
            640,
            32,
        );
        assert_eq!(widths, [392, 0, 184]);
    }

    #[test]
    fn agreement_markdown_becomes_colored_visible_runs() {
        let base = parse_color("#CCFFFFFF");
        let link = parse_color("#00C4B2");
        let runs = parse_text_runs(
            "我已阅读并同意[《服务协议》](agreement)＆[《隐私政策》](policy)",
            base,
            Some(link),
        );
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "我已阅读并同意《服务协议》＆《隐私政策》"
        );
        assert_eq!(runs.len(), 4);
        assert_eq!(runs[1].color.0, link.0);
        assert_eq!(runs[3].color.0, link.0);
    }

    #[test]
    fn taptap_first_page_places_controls_at_192_dpi() -> anyhow::Result<()> {
        let mut files = HashMap::new();
        files.insert(
            "installer_config.json".to_string(),
            include_bytes!("../../../examples/TapTap/installer_config.json").to_vec(),
        );
        files.insert(
            "layouts/configpage.xml".to_string(),
            include_bytes!("../../../examples/TapTap/layouts/configpage.xml").to_vec(),
        );
        files.insert(
            "locales/zh-CN.json".to_string(),
            include_bytes!("../../../examples/TapTap/locales/zh-CN.json").to_vec(),
        );
        files.insert(
            "locales/en-US.json".to_string(),
            include_bytes!("../../../examples/TapTap/locales/en-US.json").to_vec(),
        );
        files.insert(
            "locales/ru.json".to_string(),
            include_bytes!("../../../examples/TapTap/locales/ru.json").to_vec(),
        );
        let assets: [(&str, &[u8]); 15] = [
            (
                "bg_main@2x.png",
                include_bytes!("../../../examples/TapTap/assets/bg_main@2x.png"),
            ),
            (
                "btn_minimize@2x.png",
                include_bytes!("../../../examples/TapTap/assets/btn_minimize@2x.png"),
            ),
            (
                "btn_close@2x.png",
                include_bytes!("../../../examples/TapTap/assets/btn_close@2x.png"),
            ),
            (
                "logo@2x.png",
                include_bytes!("../../../examples/TapTap/assets/logo@2x.png"),
            ),
            (
                "v2_logo_tagline@2x.png",
                include_bytes!("../../../examples/TapTap/assets/v2_logo_tagline@2x.png"),
            ),
            (
                "btn_primary@2x.png",
                include_bytes!("../../../examples/TapTap/assets/btn_primary@2x.png"),
            ),
            (
                "btn_disabled@2x.png",
                include_bytes!("../../../examples/TapTap/assets/btn_disabled@2x.png"),
            ),
            (
                "btn_hover@2x.png",
                include_bytes!("../../../examples/TapTap/assets/btn_hover@2x.png"),
            ),
            (
                "checkbox-2@2x.png",
                include_bytes!("../../../examples/TapTap/assets/checkbox-2@2x.png"),
            ),
            (
                "checkbox-0@2x.png",
                include_bytes!("../../../examples/TapTap/assets/checkbox-0@2x.png"),
            ),
            (
                "folder-icon@2x.png",
                include_bytes!("../../../examples/TapTap/assets/folder-icon@2x.png"),
            ),
            (
                "arrow-down@2x.png",
                include_bytes!("../../../examples/TapTap/assets/arrow-down@2x.png"),
            ),
            (
                "arrow-up@2x.png",
                include_bytes!("../../../examples/TapTap/assets/arrow-up@2x.png"),
            ),
            (
                "select-arrow@2x.png",
                include_bytes!("../../../examples/TapTap/assets/select-arrow@2x.png"),
            ),
            (
                "select-arrow-up@2x.png",
                include_bytes!("../../../examples/TapTap/assets/select-arrow-up@2x.png"),
            ),
        ];
        for (name, data) in assets {
            files.insert(format!("assets/{name}"), data.to_vec());
        }
        let default_interaction = initial_interaction(&files, RuntimeMode::Installer)?;

        let ui = load_layout(
            &files,
            DpiContext {
                scale: 2.0,
                use_2x: true,
            },
            "zh-CN",
            false,
            &default_interaction,
            RuntimeMode::Installer,
        )?;

        assert_eq!((ui.width, ui.height), (1440, 900));
        assert_eq!(ui.layers.len(), 10);
        let minimize = &ui.layers[2];
        assert_eq!(
            (minimize.left, minimize.top, minimize.width, minimize.height),
            (1294, 38, 40, 40)
        );
        assert_eq!(minimize.alpha, 160);
        let close = &ui.layers[3];
        assert_eq!(
            (close.left, close.top, close.width, close.height),
            (1362, 38, 40, 40)
        );
        assert_eq!(close.alpha, 160);
        let checkbox = &ui.layers[7];
        assert_eq!(
            (checkbox.left, checkbox.top, checkbox.width, checkbox.height),
            (80, 796, 32, 32)
        );
        let badge = &ui.layers[8];
        assert_eq!(
            (badge.left, badge.top, badge.width, badge.height),
            (1312, 788, 48, 48)
        );
        let arrow = &ui.layers[9];
        assert_eq!(
            (arrow.left, arrow.top, arrow.width, arrow.height),
            (1324, 800, 24, 24)
        );
        assert_eq!(ui.texts.len(), 5);
        assert!(ui.texts[3].wrap);
        assert!(ui.texts[3].width > 450);
        assert!(matches!(ui.texts[4].alignment, TextAlignment::Right));
        assert!(ui.overlay_layers.is_empty());
        assert!(ui.overlay_texts.is_empty());
        assert!(ui.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::ToggleCheckbox {
                ref id,
                checked: false
            } if id == "chkAgree"
        )));
        assert!(!ui
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::Install)));

        let open_menu = load_layout(
            &files,
            DpiContext {
                scale: 2.0,
                use_2x: true,
            },
            "zh-CN",
            true,
            &default_interaction,
            RuntimeMode::Installer,
        )?;
        assert_eq!(open_menu.overlay_layers.len(), 2);
        assert_eq!(open_menu.overlay_texts.len(), 3);
        assert_eq!(visible_text(&open_menu.overlay_texts[0]), "简体中文");
        assert_eq!(visible_text(&open_menu.overlay_texts[1]), "English");
        assert_eq!(visible_text(&open_menu.overlay_texts[2]), "Русский");

        let english = load_layout(
            &files,
            DpiContext {
                scale: 2.0,
                use_2x: true,
            },
            "en-US",
            false,
            &default_interaction,
            RuntimeMode::Installer,
        )?;
        assert_eq!(visible_text(&english.texts[0]), "English");
        assert_eq!(visible_text(&english.texts[1]), "Install Now");
        assert!(visible_text(&english.texts[3]).contains("Terms of Service"));

        let mut interaction = initial_interaction(&files, RuntimeMode::Installer)?;
        interaction
            .checkbox_states
            .insert("chkAgree".to_string(), true);
        interaction
            .panel_visibility
            .insert("moreconfiginfo".to_string(), true);
        let expanded = load_layout(
            &files,
            DpiContext {
                scale: 2.0,
                use_2x: true,
            },
            "zh-CN",
            false,
            &interaction,
            RuntimeMode::Installer,
        )?;
        assert!(expanded.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::ToggleCheckbox {
                ref id,
                checked: true
            } if id == "chkAgree"
        )));
        assert!(expanded
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::Install)));
        assert!(expanded.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::SetPanelVisibility {
                ref id,
                visible: false
            } if id == "moreconfiginfo"
        )));
        let expanded_text = expanded.texts.iter().map(visible_text).collect::<Vec<_>>();
        assert!(!expanded_text.iter().any(|text| text == "立即安装"));
        assert!(!expanded_text.iter().any(|text| text.starts_with("版本号")));
        assert!(expanded_text.iter().any(|text| text == "开始安装"));
        assert!(expanded_text.iter().any(|text| text == "收起"));
        assert!(expanded_text
            .iter()
            .any(|text| text == "C:\\Program Files\\TapTapTest"));
        assert!(expanded_text.iter().any(|text| text == "所需空间：200 MB"));
        assert!(
            expanded_text
                .iter()
                .any(|text| text.starts_with("可用空间：") && !text.ends_with("--")),
            "expanded text: {expanded_text:?}"
        );
        for layer in expanded.texts.iter().filter(|layer| {
            let text = visible_text(layer);
            text.starts_with("所需空间：") || text.starts_with("可用空间：")
        }) {
            let text = visible_text(layer);
            assert!(
                layer.width >= measure_layout_text_width(&text, layer.font_size, layer.bold),
                "label clips measured text: {text}"
            );
        }

        let russian = load_layout(
            &files,
            DpiContext {
                scale: 2.0,
                use_2x: true,
            },
            "ru",
            false,
            &interaction,
            RuntimeMode::Installer,
        )?;
        let agreement = russian
            .texts
            .iter()
            .find(|layer| visible_text(layer).starts_with("Принимаю"))
            .context("Russian agreement text")?;
        let collapse = russian
            .actions
            .iter()
            .find(|region| {
                matches!(
                    region.action,
                    WindowAction::SetPanelVisibility { visible: false, .. }
                )
            })
            .context("Russian collapse action")?;
        assert!(agreement.left + agreement.width <= collapse.left);
        assert!(collapse.right <= russian.width);
        let start_install = russian
            .texts
            .iter()
            .find(|layer| visible_text(layer) == "Начать установку")
            .context("Russian start install text")?;
        assert!(
            start_install.width
                >= measure_layout_text_width(
                    "Начать установку",
                    start_install.font_size,
                    start_install.bold,
                )
        );
        Ok(())
    }

    fn visible_text(layer: &super::TextLayer) -> String {
        layer.runs.iter().map(|run| run.text.as_str()).collect()
    }

    #[test]
    fn install_button_uses_xml_images_for_interaction_state() -> anyhow::Result<()> {
        let document = roxmltree::Document::parse(
            r#"<Button id="btnInstall" action="install" enabled-when="terms:checked"
                       normal-image="normal.png" hover-image="hover.png"
                       pressed-image="pressed.png" disabled-image="disabled.png" />"#,
        )?;
        let button = document.root_element();
        let mut interaction = InteractionState::default();
        interaction
            .checkbox_states
            .insert("terms".to_string(), false);
        assert_eq!(button_image(button, &interaction), Some("disabled.png"));

        interaction
            .checkbox_states
            .insert("terms".to_string(), true);
        assert_eq!(button_image(button, &interaction), Some("normal.png"));
        interaction.hovered_control = Some("btnInstall".to_string());
        assert_eq!(button_image(button, &interaction), Some("hover.png"));
        interaction.pressed_control = Some("btnInstall".to_string());
        assert_eq!(button_image(button, &interaction), Some("pressed.png"));

        let independent_document = roxmltree::Document::parse(
            r#"<Button id="standalone" action="install" normal-image="normal.png"
                       disabled-image="disabled.png" />"#,
        )?;
        assert_eq!(
            button_image(
                independent_document.root_element(),
                &InteractionState::default()
            ),
            Some("normal.png")
        );
        Ok(())
    }

    #[test]
    fn formats_bound_disk_sizes() {
        assert_eq!(format_size_bytes(200 * 1024 * 1024), "200 MB");
        assert_eq!(format_size_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn resolves_and_queries_windows_disk_root() -> anyhow::Result<()> {
        let root =
            disk_root(std::path::Path::new("C:\\Program Files\\TapTapTest")).expect("drive root");
        assert_eq!(root, std::path::PathBuf::from("C:\\"));
        let available = query_disk_free_bytes(&root)?;
        assert!(available > 0);
        Ok(())
    }

    #[test]
    fn project_file_version_overrides_release_version_for_pe() -> anyhow::Result<()> {
        let config: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../examples/TapTap/installer_config.json"
        ))?;
        let info = installer_version_info(&config, std::path::Path::new("dist/TapTap_Setup.exe"))?;
        assert_eq!(config["project"]["version"], "2026.9.22-rel.1");
        assert_eq!(info.file_version, "2026.9.22");
        assert_eq!(info.product_version, "2026.9.22");
        Ok(())
    }

    #[test]
    fn uninstaller_metadata_uses_project_version_and_configured_name() -> anyhow::Result<()> {
        let config: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../examples/TapTap/installer_config.json"
        ))?;
        let info = uninstaller_version_info(&config, "uninst.exe");
        assert_eq!(info.file_version, "2026.9.22");
        assert_eq!(info.original_filename, "uninst.exe");
        assert_eq!(info.file_description, "TapTap Uninstaller");
        assert_eq!(
            info.company_name.as_deref(),
            Some("易玩（上海）网络科技有限公司")
        );
        Ok(())
    }

    #[test]
    fn uninstaller_name_rejects_directory_components() {
        assert!(validate_output_filename("uninst.exe", "test").is_ok());
        assert!(validate_output_filename("runtime/uninst.exe", "test").is_err());
        assert!(validate_output_filename("..\\uninst.exe", "test").is_err());
    }

    #[test]
    fn runtime_modes_select_distinct_layout_lists() -> anyhow::Result<()> {
        let config: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../examples/TapTap/installer_config.json"
        ))?;
        assert_eq!(
            runtime_layout_path(&config, RuntimeMode::Installer)?,
            "layouts/configpage.xml"
        );
        assert_eq!(
            runtime_layout_path(&config, RuntimeMode::Uninstaller)?,
            "layouts/uninstallpage.xml"
        );
        Ok(())
    }

    #[test]
    fn taptap_uninstaller_buttons_have_distinct_hit_regions() -> anyhow::Result<()> {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap");
        let bundle = pack_project_with_progress(&project, None, false, |_| {})?;
        let files = parse_bundle(&bundle)?;
        let interaction = initial_interaction(&files, RuntimeMode::Uninstaller)?;
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            false,
            &interaction,
            RuntimeMode::Uninstaller,
        )?;
        let uninstall = ui
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::Uninstall))
            .context("uninstall button is not clickable")?;
        let close = ui
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::Close))
            .context("cancel button is not clickable")?;
        assert!(uninstall.left >= close.right);
        Ok(())
    }

    #[test]
    fn inspects_taptap_project_without_dpi_warnings() -> anyhow::Result<()> {
        let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("examples")
            .join("TapTap");
        let payload = project.join("payload").join("app.7z");
        if !payload.is_file() {
            eprintln!(
                "skipping TapTap inspection: {} is absent; example payloads are not tracked",
                payload.display()
            );
            return Ok(());
        }
        let summary = inspect_project(project)?;
        assert_eq!(summary.project_name, "TapTap");
        assert_eq!(summary.project_version, "2026.9.22-rel.1");
        assert_eq!(summary.file_version, "2026.9.22");
        assert_eq!(summary.payload_format, PayloadFormat::SevenZip);
        assert_eq!(summary.uninstaller_name, "uninst.exe");
        assert_eq!(
            summary.default_install_path.as_deref(),
            Some("C:\\Program Files\\TapTapTest")
        );
        assert_eq!(
            summary
                .uninstaller_icon
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str()),
            Some("uninst.ico")
        );
        assert!(summary.payload_size > 100 * 1024 * 1024);
        assert!(summary.warnings.is_empty(), "{:?}", summary.warnings);
        Ok(())
    }
}
