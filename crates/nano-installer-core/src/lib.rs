#[cfg(not(target_arch = "x86_64"))]
compile_error!("nano-installer-native-x64 must be built for x86_64");

mod icon;
mod install;
mod manifest;
mod script;
mod shell;
mod version;

use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::GlobalFree;
use windows::Win32::Foundation::{
    COLORREF, HANDLE, HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
    CreateRoundRectRgn, DeleteDC, DeleteObject, DrawTextW, EndPaint, GdiAlphaBlend, GetDC,
    GetDeviceCaps, GetTextExtentPoint32W, InvalidateRect, ReleaseDC, ScreenToClient, SelectObject,
    SetBkMode, SetTextColor, SetWindowRgn, UpdateWindow, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
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
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::UI::Input::Ime::{
    ImmGetContext, ImmReleaseContext, ImmSetCandidateWindow, ImmSetCompositionWindow,
    CANDIDATEFORM, CFS_CANDIDATEPOS, CFS_POINT, COMPOSITIONFORM,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetDoubleClickTime, GetKeyState, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE,
    TRACKMOUSEEVENT, VIRTUAL_KEY, VK_A, VK_BACK, VK_C, VK_CONTROL, VK_DELETE, VK_END, VK_HOME,
    VK_LEFT, VK_RIGHT, VK_SHIFT, VK_V, VK_X, VK_Y, VK_Z,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, IShellItem, ShellExecuteW, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos,
    GetMessageW, GetSystemMetrics, KillTimer, LoadCursorW, LoadIconW, MessageBoxW, PostMessageW,
    PostQuitMessage, RegisterClassExW, SendMessageW, SetProcessDPIAware, SetTimer, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HTCAPTION, HTCLIENT, ICON_BIG, ICON_SMALL, IDC_ARROW,
    IDC_HAND, IDYES, MB_ICONERROR, MB_ICONQUESTION, MB_OK, MB_YESNO, MSG, SM_CXSCREEN, SM_CYSCREEN,
    SW_MINIMIZE, SW_SHOW, SW_SHOWNORMAL, WM_CHAR, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCLBUTTONDOWN, WM_PAINT, WM_SETCURSOR,
    WM_SETICON, WM_TIMER, WNDCLASSEXW, WS_EX_APPWINDOW, WS_POPUP,
};
use windows::Win32::UI::WindowsAndMessaging::{SetCursor, IDC_IBEAM};

const BUNDLE_MAGIC: &[u8; 8] = b"NATVRS01";
const FOOTER_MAGIC: &[u8; 8] = b"NATVEND1";
const BUNDLE_VERSION: u16 = 1;
const BASE_DPI: u32 = 96;
const DEFAULT_DPI_THRESHOLD: u32 = 144;
const WM_MOUSELEAVE: u32 = 0x02A3;
/// Timer that blinks the caret of the focused text field.
const CARET_TIMER: usize = 1;
/// Caret blink period in milliseconds, the usual Windows cadence.
const CARET_BLINK_MS: u32 = 530;
/// Posted by a worker thread when it changed runtime progress or page state.
const WM_APP_REFRESH: u32 = 0x8001;
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
    /// Region of `image` to draw. `None` draws the whole image; a progress bar
    /// fills by shrinking this region.
    source: Option<LayerRect>,
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
    text_hits: Vec<TextHit>,
    hover_regions: Vec<HoverRegion>,
    /// Editable text fields, so a click can focus one and place its caret.
    text_inputs: Vec<TextInputRegion>,
    /// Caret for the focused text field, positioned with the layout.
    caret: Option<ImageLayer>,
    /// Where that caret is, in client coordinates. The input method needs the
    /// position only, not the pixels, and the layer above owns a buffer.
    caret_rect: Option<LayerRect>,
    /// Highlight bands for the focused field's selection, drawn under the text.
    selection: Vec<ImageLayer>,
    /// Whether the caret is in the visible half of its blink cycle.
    caret_drawn: bool,
    /// Locale code of every option the language menu offers, in layout order.
    /// Keyboard navigation walks this list, which is why it is kept even while
    /// the menu is closed.
    language_options: Vec<String>,
    /// Localized question a `close_confirm` button asks before closing.
    close_confirm_message: String,
    /// Title for runtime dialogs, taken from `project.name`.
    product_name: String,
}

struct RuntimeState {
    files: HashMap<String, Vec<u8>>,
    dpi: DpiContext,
    locale: String,
    language_menu_open: bool,
    interaction: InteractionState,
    mode: RuntimeMode,
    ui: RuntimeUi,
    /// The window the runtime paints into, so worker threads can report back.
    window: isize,
    /// Executable an install deployed, launched from the finish page.
    installed_app: Option<PathBuf>,
}

#[derive(Clone, Copy)]
enum RuntimeMode {
    Installer,
    Uninstaller,
}

#[derive(Default, Clone)]
struct InteractionState {
    checkbox_states: HashMap<String, bool>,
    panel_visibility: HashMap<String, bool>,
    hovered_control: Option<String>,
    pressed_control: Option<String>,
    text_input_values: HashMap<String, String>,
    /// Wizard page currently rendered, as an index into the mode's page list.
    page_index: usize,
    /// Install or uninstall progress, 0-100, published by the worker thread.
    progress: Option<u8>,
    /// Locale key describing the running task, published by the worker thread.
    status_key: Option<String>,
    /// Literal status text a project script published. Takes precedence over
    /// `status_key`, because a script names its own steps.
    status_text: Option<String>,
    /// Option the open language menu highlights for keyboard selection.
    highlighted_option: Option<usize>,
    /// Text field that takes typed characters, if any.
    focused_text_input: Option<String>,
    /// Caret position inside the focused field, counted in characters.
    caret_index: usize,
    /// The other end of a selection, when the user has one. The caret is always
    /// the moving end, so dragging or holding Shift grows the range from here.
    selection_anchor: Option<usize>,
    /// Earlier values of the focused field, newest last, so Ctrl+Z can walk back
    /// over the edits of this run.
    undo_stack: Vec<TextSnapshot>,
    /// Values undone with Ctrl+Z, newest last, so Ctrl+Y can replay them.
    redo_stack: Vec<TextSnapshot>,
    /// Whether the caret is in the visible half of its blink cycle.
    caret_visible: bool,
    /// Set between a press inside a text field and the matching release, so a
    /// drag extends the selection instead of doing nothing.
    dragging_text_selection: bool,
    /// Whether the last edit was typing, so a run of keystrokes undoes in one
    /// step instead of one character at a time.
    typing_run: bool,
    /// The field and character index of the last press, so a second press on the
    /// same character within the double-click time selects the word under it.
    last_press: Option<(String, usize, std::time::Instant)>,
}

/// One remembered text field value, paired with the caret that goes with it.
#[derive(Clone)]
struct TextSnapshot {
    id: String,
    text: String,
    caret: usize,
}

/// How many edits a single text field remembers for undo.
const UNDO_DEPTH: usize = 64;

impl InteractionState {
    /// The progress the UI should draw for a control declared with `progress`.
    /// A bar without a bound worker keeps its authored value.
    fn progress_for(&self, authored: u8) -> u8 {
        self.progress.unwrap_or(authored)
    }

    /// The selected range inside the focused field, as `(start, end)` in
    /// characters, or `None` when nothing is selected. The caret is the moving
    /// end, so the range is ordered before it is handed out.
    fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        if anchor == self.caret_index {
            return None;
        }
        Some(if anchor < self.caret_index {
            (anchor, self.caret_index)
        } else {
            (self.caret_index, anchor)
        })
    }

    /// Drops the selection without moving the caret, which is what an edit or a
    /// plain arrow key does once it has consumed the range.
    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    /// Removes the selected text and parks the caret where it started.
    ///
    /// Returns whether there was a selection, so a caller can tell an edit that
    /// replaced a range from one that should fall back to a single character.
    fn remove_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection_range() else {
            return false;
        };
        if let Some(id) = self.focused_text_input.clone() {
            if let Some(text) = self.text_input_values.get_mut(&id) {
                let (start_byte, end_byte) = (byte_index(text, start), byte_index(text, end));
                text.replace_range(start_byte..end_byte, "");
            }
        }
        self.caret_index = start;
        self.clear_selection();
        true
    }

    /// Remembers the current value so `undo` can come back to it.
    ///
    /// `typing` marks a keystroke that inserts a character: consecutive ones
    /// share the snapshot taken before the run started, so Ctrl+Z undoes a word
    /// typed in one go rather than one letter. The redo stack belongs to the
    /// edit that is about to happen, so any fresh edit clears it.
    fn remember_for_undo(&mut self, typing: bool) {
        let Some(id) = self.focused_text_input.clone() else {
            return;
        };
        let coalesce = typing && self.typing_run;
        self.typing_run = typing;
        if coalesce {
            return;
        }
        let text = self.text_input_values.get(&id).cloned().unwrap_or_default();
        if self
            .undo_stack
            .last()
            .is_some_and(|snapshot| snapshot.id == id && snapshot.text == text)
        {
            return;
        }
        self.undo_stack.push(TextSnapshot {
            id,
            text,
            caret: self.caret_index,
        });
        if self.undo_stack.len() > UNDO_DEPTH {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }
    ///
    /// Any literal text a project script published belongs to that script's own
    /// step, so it is dropped here rather than shown for a library step.
    fn begin_step(&mut self, percent: u8, status_key: &str) {
        self.progress = Some(percent.min(100));
        self.status_key = Some(status_key.to_string());
        self.status_text = None;
    }
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
    /// Repository URL a rendered Markdown link points at, when the run came
    /// from `[label](target)` markup. Clicking such a run opens the target.
    link: Option<String>,
}

#[derive(Clone, Copy)]
enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Product name from `project.name`, used as the title of dialogs the runtime
/// opens so a user never sees an internal binary name.
fn product_name(files: &HashMap<String, Vec<u8>>) -> String {
    files
        .get("installer_config.json")
        .and_then(|encoded| serde_json::from_slice::<serde_json::Value>(encoded).ok())
        .and_then(|config| config["project"]["name"].as_str().map(str::to_string))
        .unwrap_or_else(|| "nano-installer".to_string())
}

/// A clickable span inside a text layer.
struct TextHit {
    action: WindowAction,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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
    CloseConfirm,
    Minimize,
    ToggleLanguageMenu,
    SelectLanguage(String),
    Install,
    Uninstall,
    LaunchApp,
    OpenLink(String),
    PickDirectory { id: String },
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
    /// Runs that respond to a click, resolved while text is laid out because
    /// only there is the exact glyph position known.
    text_hits: Vec<TextHit>,
    /// Editable text fields, so a click can put the caret in the one under it.
    text_inputs: Vec<TextInputRegion>,
    overlay_layers: Vec<ImageLayer>,
    overlay_texts: Vec<TextLayer>,
    actions: Vec<ActionRegion>,
    hover_regions: Vec<HoverRegion>,
}

/// An editable text field placed on the page.
#[derive(Clone)]
struct TextInputRegion {
    id: String,
    text: String,
    color: COLORREF,
    font_size: i32,
    bold: bool,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

struct LayoutContext<'a> {
    dpi: DpiContext,
    files: &'a HashMap<String, Vec<u8>>,
    config: &'a serde_json::Value,
    locale: &'a str,
    translations: &'a HashMap<String, String>,
    interaction: &'a InteractionState,
    language_menu_open: bool,
}

#[derive(Clone, Copy)]
struct FlowItem {
    fixed_width: Option<i32>,
    flex_grow: f32,
    flex_shrink: f32,
    min_width: i32,
    /// Main-axis size a flexible item starts from before `flex-grow` is shared
    /// out. Zero for items that do not declare `flex-basis`.
    flex_basis: i32,
}

impl FlowItem {
    /// The room an item asks for on its own, before free space is shared out.
    fn basis_size(&self) -> i32 {
        self.fixed_width.unwrap_or(self.flex_basis).max(0)
    }
}

/// Direction a flow container stacks its children in. `FlowItem` fields keep
/// their historical names; width there means the main axis of the container.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FlowAxis {
    Horizontal,
    Vertical,
}

impl FlowAxis {
    fn main(self, rect: LayerRect) -> i32 {
        match self {
            FlowAxis::Horizontal => rect.width,
            FlowAxis::Vertical => rect.height,
        }
    }

    fn cross(self, rect: LayerRect) -> i32 {
        match self {
            FlowAxis::Horizontal => rect.height,
            FlowAxis::Vertical => rect.width,
        }
    }

    /// Offsets `rect` by a distance along the container's main axis.
    fn shift(self, rect: LayerRect, distance: i32) -> LayerRect {
        match self {
            FlowAxis::Horizontal => LayerRect {
                left: rect.left + distance,
                ..rect
            },
            FlowAxis::Vertical => LayerRect {
                top: rect.top + distance,
                ..rect
            },
        }
    }

    /// Offsets `rect` by a distance along the container's cross axis.
    fn shift_cross(self, rect: LayerRect, distance: i32) -> LayerRect {
        match self {
            FlowAxis::Horizontal => LayerRect {
                top: rect.top + distance,
                ..rect
            },
            FlowAxis::Vertical => LayerRect {
                left: rect.left + distance,
                ..rect
            },
        }
    }

    /// Builds a child rect from its main and cross extents.
    fn place(self, rect: LayerRect, main: i32, cross: i32) -> LayerRect {
        match self {
            FlowAxis::Horizontal => LayerRect {
                width: main,
                height: cross,
                ..rect
            },
            FlowAxis::Vertical => LayerRect {
                width: cross,
                height: main,
                ..rect
            },
        }
    }

    /// The axis that measures a cross-axis extent. An HBox stretches height, so
    /// its cross sizes measure vertically, and the other way round for a VBox.
    fn cross_measure(self) -> FlowAxis {
        match self {
            FlowAxis::Horizontal => FlowAxis::Vertical,
            FlowAxis::Vertical => FlowAxis::Horizontal,
        }
    }
}

/// Padding and margin, in device pixels.
#[derive(Default, Clone, Copy)]
struct Insets {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Insets {
    fn horizontal(self) -> i32 {
        self.left + self.right
    }

    fn vertical(self) -> i32 {
        self.top + self.bottom
    }

    fn along(self, axis: FlowAxis) -> i32 {
        match axis {
            FlowAxis::Horizontal => self.horizontal(),
            FlowAxis::Vertical => self.vertical(),
        }
    }
}

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub fn show_runtime_error(error: &anyhow::Error) {
    let message = HSTRING::from(format!("Native runtime failed:\n{error:#}"));
    let title = UI
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| HSTRING::from(state.ui.product_name.clone()))
        .unwrap_or_else(|| HSTRING::from("nano-installer"));
    unsafe {
        let _ = MessageBoxW(None, &message, &title, MB_OK | MB_ICONERROR);
    }
}

pub fn run_installer_runtime() -> Result<()> {
    run_runtime(RuntimeMode::Installer)
}

/// The argument that turns the uninstaller into the post-uninstall cleaner.
pub(crate) const CLEANUP_FLAG: &str = "--cleanup";

pub fn run_uninstaller_runtime() -> Result<()> {
    // A finished uninstall leaves a copy of this executable in the temporary
    // directory to remove what the running process cannot remove about itself,
    // so the argument is handled before any window is created.
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if arguments.first().map(std::ffi::OsString::as_os_str)
        == Some(std::ffi::OsStr::new(CLEANUP_FLAG))
    {
        run_cleanup_helper(&arguments);
        return Ok(());
    }
    run_runtime(RuntimeMode::Uninstaller)
}

/// Removes what a completed uninstall could not remove about itself.
///
/// The uninstaller holds an exclusive lock on its own image, and the user may
/// still be looking at the finish page, so this runs as a separate copy. It
/// waits for that image to become deletable, deletes it, and then removes the
/// installation directory if nothing else is left in it, so files the user
/// added keep the directory alive.
///
/// Nothing here reports an error: by the time it runs the uninstall the user
/// asked for is already done, and a second dialog would only worry them.
fn run_cleanup_helper(arguments: &[std::ffi::OsString]) {
    let Some(directory) = arguments.get(1).map(PathBuf::from) else {
        return;
    };
    let Some(uninstaller) = arguments.get(2).map(PathBuf::from) else {
        return;
    };
    cleanup_after_uninstall(&directory, &uninstaller);
    self_delete_current_image();
}

/// Deletes a finished installation's uninstaller, then its directory.
///
/// The directory only goes when nothing else is left in it, so a file the user
/// put there keeps it, which is the same promise the manifest cleanup makes.
fn cleanup_after_uninstall(directory: &Path, uninstaller: &Path) {
    wait_until_deletable(uninstaller);
    let _ = std::fs::remove_file(uninstaller);
    let _ = std::fs::remove_dir(directory);
}

/// Deletes this helper copy once the process has ended.
///
/// Windows refuses to delete the image a running process was started from, and
/// it ignores a delete-on-close request for that image too, so the file is
/// handed to a short-lived command script instead. The script waits for this
/// process to end, deletes the copy, and then deletes itself, which leaves the
/// temporary directory as clean as the installation directory.
///
/// A script that cannot be written or started only costs a stale copy in the
/// temporary directory, so nothing here reports an error.
fn self_delete_current_image() {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let script =
        std::env::temp_dir().join(format!("nano-installer-cleanup-{}.cmd", std::process::id()));
    let body = format!(
        "@echo off\r\n\
         :wait\r\n\
         del /f /q \"{image}\" >nul 2>&1\r\n\
         if exist \"{image}\" (ping -n 2 127.0.0.1 >nul 2>&1 & goto wait)\r\n\
         del /f /q \"%~f0\" >nul 2>&1\r\n",
        image = exe.display()
    );
    if std::fs::write(&script, body).is_err() {
        return;
    }
    let _ = std::process::Command::new("cmd.exe")
        .raw_arg(format!("/c \"{}\"", script.display()))
        .creation_flags(CREATE_NO_WINDOW.0)
        .spawn();
}

/// Waits for a locked file to become deletable, up to ten minutes.
///
/// The uninstaller window stays open until the user closes it; the wait ends as
/// soon as the file can go, so a quick user pays nothing for the ceiling.
fn wait_until_deletable(path: &Path) {
    const ATTEMPTS: u32 = 2_400;
    const INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
    for _ in 0..ATTEMPTS {
        match std::fs::remove_file(path) {
            Ok(()) => return,
            // A file that is already gone needs no further waiting.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) => std::thread::sleep(INTERVAL),
        }
    }
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
    /// Whether the generated setup asks Windows for administrator rights.
    pub require_admin: bool,
    /// Whether the generated setup scales its interface with the display DPI.
    pub dpi_aware: bool,
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
    let manifest = manifest::ManifestSettings::from_config(&config);
    let mut warnings = asset_scale_warnings(project, &config)?;
    warnings.extend(locale_scale_warnings(project, &config)?);
    warnings.sort();
    warnings.dedup();
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
        require_admin: manifest.require_admin,
        dpi_aware: manifest.dpi_aware,
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
    let manifest = manifest::ManifestSettings::from_config(&config);
    progress(BuildEvent {
        stage: BuildStage::WritingResources,
        message: format!(
            "Writing application manifest (elevation: {}, DPI aware: {})",
            if manifest.require_admin {
                "requested"
            } else {
                "as the user"
            },
            if manifest.dpi_aware { "yes" } else { "no" }
        ),
    });
    manifest::replace_exe_manifest(output, &manifest)?;
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
    manifest::replace_exe_manifest(
        &temporary.path,
        &manifest::ManifestSettings::from_config(config),
    )?;
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

/// Compares the locale files with the pages and reports what has fallen behind.
///
/// A missing key is silent at run time: the runtime falls back to the default
/// locale, so a half-translated installer still runs. The build is the only
/// place where the gap is cheap to notice, so the missing keys are listed here.
fn locale_scale_warnings(project: &Path, config: &serde_json::Value) -> Result<Vec<String>> {
    let locales_dir = project.join(
        config["resources"]["locales_dir"]
            .as_str()
            .unwrap_or("locales"),
    );
    let default_locale = config["localization"]["default_locale"]
        .as_str()
        .unwrap_or("zh-CN");
    let required = required_locale_keys(project, config)?;
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&locales_dir)? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            files.push(path);
        }
    }
    files.sort();
    let mut keys_by_locale: Vec<(String, HashSet<String>)> = Vec::new();
    for path in &files {
        let Some(locale) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let keys: HashSet<String> =
            serde_json::from_slice::<HashMap<String, String>>(&std::fs::read(path)?)
                .with_context(|| format!("invalid locale file: {}", path.display()))?
                .into_keys()
                .collect();
        keys_by_locale.push((locale.to_string(), keys));
    }
    let default_keys = keys_by_locale
        .iter()
        .find(|(locale, _)| locale == default_locale)
        .map(|(_, keys)| keys.clone())
        .unwrap_or_default();
    let mut warnings = Vec::new();
    for (locale, keys) in &keys_by_locale {
        // Only keys the default locale answers are reported, so a page may ask
        // for a key the product intentionally leaves untranslated.
        let mut missing: Vec<&String> = required
            .iter()
            .filter(|key| default_keys.contains(*key) && !keys.contains(*key))
            .collect();
        missing.sort();
        if !missing.is_empty() {
            let listed = missing
                .iter()
                .map(|key| format!("`{key}`"))
                .collect::<Vec<_>>()
                .join(", ");
            warnings.push(format!(
                "locales/{locale}.json is missing {} page text(s): {listed}",
                missing.len()
            ));
        }
    }
    let mut configured: Vec<String> = config["localization"]["supported_locales"]
        .as_array()
        .map(|locales| {
            locales
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    configured.sort();
    configured.dedup();
    for locale in configured {
        if !keys_by_locale.iter().any(|(name, _)| *name == locale) {
            warnings.push(format!(
                "localization.supported_locales lists {locale}, but locales/{locale}.json is missing"
            ));
        }
    }
    Ok(warnings)
}

/// Every locale key the project's pages ask for.
///
/// Only attributes are read, and only ones that start with `@`, so an asset
/// reference such as `logo@2x.png` is never mistaken for a key.
fn required_locale_keys(project: &Path, config: &serde_json::Value) -> Result<HashSet<String>> {
    let layouts_dir = project.join(
        config["resources"]["layouts_dir"]
            .as_str()
            .unwrap_or("layouts"),
    );
    let mut layouts = Vec::new();
    collect_xml_paths(&layouts_dir, &mut layouts)?;
    layouts.sort();
    let mut keys = HashSet::new();
    for path in layouts {
        let xml = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read layout: {}", path.display()))?;
        let document = roxmltree::Document::parse(&xml)
            .with_context(|| format!("invalid layout: {}", path.display()))?;
        for node in document.descendants().filter(|node| node.is_element()) {
            for attribute in node.attributes() {
                if let Some(key) = attribute.value().trim().strip_prefix('@') {
                    keys.insert(key.to_string());
                }
            }
        }
    }
    Ok(keys)
}

fn collect_xml_paths(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_xml_paths(&path, paths)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
        {
            paths.push(path);
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
        window: 0,
        installed_app: None,
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
    let layout_path = runtime_layout_path_at(&config, mode, interaction.page_index)?;
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
        language_menu_open,
    };
    // The language menu is drawn only on the page that declares the Select, so
    // its option list is read from this layout.
    let language_options: Vec<String> = page
        .descendants()
        .filter(|node| {
            node.has_tag_name("Select") && node.attribute("action") == Some("switch_language")
        })
        .flat_map(|select| select.children())
        .filter(|option| option.has_tag_name("Option") && !is_hidden(*option, interaction))
        .map(|option| option.attribute("value").unwrap_or_default().to_string())
        .collect();
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

    // The page fill sits under its background image, so a layout can paint a
    // base colour and still lay artwork over it.
    if let Some(background) = page.attribute("background") {
        push_solid_layer(
            &mut output.layers,
            LayerRect {
                left: 0,
                top: 0,
                width,
                height,
            },
            background,
            corner_radius,
        )?;
    }
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
    push_node_border(
        page,
        LayerRect {
            left: 0,
            top: 0,
            width,
            height,
        },
        &context,
        &mut output.layers,
    )?;
    for node in page.descendants().filter(|node| node.is_element()) {
        if is_hidden(node, interaction) || is_inside_render_container(node) {
            continue;
        }
        let (left, top) = absolute_position(node, dpi);
        let layer_width = size_attribute(node, "width", width, &context).unwrap_or(0);
        let layer_height = size_attribute(node, "height", height, &context).unwrap_or(0);
        let own_left = scale_value(int_attribute(node, "left").unwrap_or(0), dpi.scale);
        let own_top = scale_value(int_attribute(node, "top").unwrap_or(0), dpi.scale);
        let rect = LayerRect {
            left: anchored_left(node, left - own_left, layer_width, width, &context),
            top: anchored_top(node, top - own_top, layer_height, height, &context),
            width: layer_width,
            height: layer_height,
        };
        if active_panel.is_some_and(|(panel_order, panel_rect)| {
            node.range().start < panel_order && rects_intersect(rect, panel_rect)
        }) {
            continue;
        }
        // Buttons, selects, and any element that declares an `action` respond to
        // clicks, so a project can make an icon or a label clickable that way.
        if (node.has_tag_name("Button")
            || node.has_tag_name("Select")
            || node.attribute("action").is_some())
            && layer_width > 0
            && layer_height > 0
        {
            push_action(node, rect, &mut output.actions, &context);
            push_hover_region(node, rect, &context, &mut output.hover_regions);
        }
        if node.has_tag_name("ProgressBar") && layer_width > 0 && layer_height > 0 {
            render_progress_bar(node, rect, &context, &mut output)?;
            continue;
        }
        // Containers own their subtree. `is_inside_render_container` already
        // skipped their children, so drawing here cannot double-place them.
        if let Some(axis) = flow_axis(node).filter(|_| layer_width > 0 && layer_height > 0) {
            render_flow(node, rect, axis, &context, &mut output)?;
            continue;
        }
        if layer_width > 0 && layer_height > 0 {
            push_node_text(node, rect, &context, &mut output);
        }
        if node.has_tag_name("Select") && layer_width > 0 && layer_height > 0 {
            render_language_select(node, rect, &context, &mut output)?;
        }
        let has_layer = layer_width > 0 && layer_height > 0;
        match node.tag_name().name() {
            "Image" | "Icon" if has_layer => {
                if let Some(source) = node.attribute("src") {
                    push_styled_layer(
                        files,
                        &mut output.layers,
                        ImageStyle::plain(source),
                        rect,
                        dpi,
                    )?;
                }
            }
            "Button" if has_layer => {
                if let Some(style) = button_image(node, interaction).map(parse_image_style) {
                    push_styled_layer(files, &mut output.layers, style, rect, dpi)?;
                }
                if node.attribute("normal-image").is_none() {
                    push_node_border(node, rect, &context, &mut output.layers)?;
                }
            }
            "Box" | "Divider" if has_layer => {
                render_box_contents(node, rect, &context, &mut output)?;
            }
            _ => {}
        }
    }
    // The caret and the selection follow the focused field, so they are built
    // here where the font measurement helpers are available.
    let focused_field = interaction
        .focused_text_input
        .as_deref()
        .and_then(|focused| output.text_inputs.iter().find(|field| field.id == focused));
    let caret = focused_field.map(|field| caret_layer(field, interaction.caret_index));
    let caret_rect = focused_field.map(|field| caret_rect(field, interaction.caret_index));
    let selection = match (focused_field, interaction.selection_range()) {
        (Some(field), Some((start, end))) => selection_layers(field, start, end),
        _ => Vec::new(),
    };
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
        text_hits: output.text_hits,
        hover_regions: output.hover_regions,
        text_inputs: output.text_inputs,
        caret,
        caret_rect,
        selection,
        caret_drawn: interaction.caret_visible,
        language_options,
        close_confirm_message: translations
            .get("close_confirm_message")
            .cloned()
            .unwrap_or_else(|| "Exit the installer?".to_string()),
        product_name: product_name(files),
    })
}

fn runtime_layout_path(config: &serde_json::Value, mode: RuntimeMode) -> Result<&str> {
    runtime_layout_path_at(config, mode, 0)
}

fn runtime_page_count(config: &serde_json::Value, mode: RuntimeMode) -> usize {
    let pages = match mode {
        RuntimeMode::Installer => &config["wizard"]["pages"],
        RuntimeMode::Uninstaller => &config["wizard"]["uninstall_pages"],
    };
    pages.as_array().map(Vec::len).unwrap_or(0)
}

/// The layout of the page at `index`, falling back to the first page so an
/// out-of-range index can never blank the window.
fn runtime_layout_path_at(
    config: &serde_json::Value,
    mode: RuntimeMode,
    index: usize,
) -> Result<&str> {
    let (pages, field) = match mode {
        RuntimeMode::Installer => (&config["wizard"]["pages"], "wizard.pages[0].layout"),
        RuntimeMode::Uninstaller => (
            &config["wizard"]["uninstall_pages"],
            "wizard.uninstall_pages[0].layout",
        ),
    };
    pages[index]["layout"]
        .as_str()
        .or_else(|| pages[0]["layout"].as_str())
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

/// Positions a node along the horizontal axis.
///
/// `left` measures from the near edge, `right` from the far one, and `inset`
/// is the shorthand that sets all four edges. A near edge wins when both are
/// declared, because a declared width already fixes the extent.
fn anchored_left(
    node: roxmltree::Node<'_, '_>,
    base: i32,
    size: i32,
    parent_width: i32,
    context: &LayoutContext<'_>,
) -> i32 {
    if let Some(left) = int_attribute(node, "left") {
        return base + scale_value(left, context.dpi.scale);
    }
    let inset = insets_for_node(node, "inset", context);
    if let Some(left) = int_attribute(node, "inset-left") {
        return base + scale_value(left, context.dpi.scale);
    }
    if let Some(right) = int_attribute(node, "right") {
        return base + parent_width - scale_value(right, context.dpi.scale) - size;
    }
    if node.attribute("inset").is_some() {
        return base + inset.left;
    }
    base
}

/// Vertical counterpart of [`anchored_left`].
fn anchored_top(
    node: roxmltree::Node<'_, '_>,
    base: i32,
    size: i32,
    parent_height: i32,
    context: &LayoutContext<'_>,
) -> i32 {
    if let Some(top) = int_attribute(node, "top") {
        return base + scale_value(top, context.dpi.scale);
    }
    let inset = insets_for_node(node, "inset", context);
    if let Some(top) = int_attribute(node, "inset-top") {
        return base + scale_value(top, context.dpi.scale);
    }
    if let Some(bottom) = int_attribute(node, "bottom") {
        return base + parent_height - scale_value(bottom, context.dpi.scale) - size;
    }
    if node.attribute("inset").is_some() {
        return base + inset.top;
    }
    base
}

/// Resolves `[label](target)` markup to a URL the shell can open.
///
/// A target that is already a URL or an absolute path is used as is. A bare
/// name such as `agreement` is looked up in the project's `links` table, which
/// keeps the localizable text free of raw URLs. The historical `agreement` and
/// `policy` names map onto the reference project's keys.
fn resolve_link_target(target: &str, context: &LayoutContext<'_>) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") || trimmed.starts_with("mailto:") {
        return Some(trimmed.to_string());
    }
    config_link(trimmed, context.config).or_else(|| {
        let fallback = match trimmed {
            "agreement" => "terms_of_service",
            "policy" => "privacy_policy",
            _ => return None,
        };
        config_link(fallback, context.config)
    })
}

fn config_link(key: &str, config: &serde_json::Value) -> Option<String> {
    config["links"]
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// The TextInput a `pick_directory` control edits.
///
/// A control may name its target, otherwise the layout's first writable
/// TextInput is used, because a page normally has exactly one path field.
fn pick_directory_target(
    node: roxmltree::Node<'_, '_>,
    interaction: &InteractionState,
) -> Option<String> {
    if let Some(id) = node.attribute("target") {
        return Some(id.to_string());
    }
    let document = node.document();
    document
        .descendants()
        .filter(|candidate| candidate.has_tag_name("TextInput"))
        .find(|candidate| candidate.attribute("readonly") != Some("true"))
        .or_else(|| {
            document
                .descendants()
                .find(|candidate| candidate.has_tag_name("TextInput"))
        })
        .and_then(|candidate| candidate.attribute("id").map(str::to_string))
        .or_else(|| interaction.text_input_values.keys().next().cloned())
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

/// Whether a container positions its own children. The page-level pass walks a
/// flat node list, so anything below such a container must be skipped there.
fn renders_own_children(node: roxmltree::Node<'_, '_>) -> bool {
    node.has_tag_name("HBox")
        || node.has_tag_name("VBox")
        || node.has_tag_name("Content")
        || node.has_tag_name("Box")
}

fn is_inside_render_container(node: roxmltree::Node<'_, '_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(current) = ancestor {
        if renders_own_children(current) {
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
    context: &LayoutContext<'_>,
) {
    let interaction = context.interaction;
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
            Some("close") => Some(WindowAction::Close),
            Some("close_confirm") => Some(WindowAction::CloseConfirm),
            Some("pick_directory") => pick_directory_target(node, interaction)
                .map(|id| WindowAction::PickDirectory { id }),
            // `open_url:` takes either a `links` key or a URL written out in
            // full, so a layout can link somewhere the config does not name.
            Some(action) if action.starts_with("open_url:") => {
                let target = action.trim_start_matches("open_url:");
                resolve_link_target(target, context).map(WindowAction::OpenLink)
            }
            Some("switch_language") => Some(WindowAction::ToggleLanguageMenu),
            Some("install") => Some(WindowAction::Install),
            Some("uninstall") => Some(WindowAction::Uninstall),
            Some("launch_app") => Some(WindowAction::LaunchApp),
            // The finish page closes the wizard; `finish` is that same action
            // under the name the example layouts use.
            Some("finish") => Some(WindowAction::Close),
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
    output: &mut LayoutOutput,
) -> Result<()> {
    let menu_open = context.language_menu_open;
    // Background first, then the outline, so the ring sits on top of the fill
    // and grows inwards from the control edge.
    let radius = scale_value(
        int_attribute(node, "border-radius").unwrap_or(0),
        context.dpi.scale,
    );
    if let Some(background) = node.attribute("background") {
        push_solid_layer(&mut output.layers, rect, background, radius)?;
    }
    push_node_border(node, rect, context, &mut output.layers)?;
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
        // A keyboard highlight wins over the current locale, so arrow keys stay
        // visible while they walk past the selected entry.
        let background = if context.interaction.highlighted_option == Some(index) {
            node.attribute("popup-highlight-background")
                .or_else(|| node.attribute("popup-selected-background"))
                .unwrap_or("#FF495A68")
        } else if option_locale == context.locale {
            node.attribute("popup-selected-background")
                .unwrap_or("#FF42515E")
        } else {
            ""
        };
        if !background.is_empty() {
            push_solid_layer(
                &mut output.overlay_layers,
                row,
                background,
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
                link: None,
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
    output: &mut LayoutOutput,
) {
    let Some((text, alignment)) = resolved_text_for_node(node, context) else {
        return;
    };
    let color = parse_color(node.attribute("color").unwrap_or("#FFFFFFFF"));
    let link_color = node.attribute("linkcolor").map(parse_color);
    output.texts.push(TextLayer {
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
    // Link markup only becomes clickable when the layout asks for a link colour,
    // which is how a project opts a label into clickable text.
    let layer = output.texts.last_mut().expect("text layer was just pushed");
    for run in &mut layer.runs {
        if let Some(target) = run.link.take() {
            run.link = resolve_link_target(&target, context);
        }
    }
    let layer = output.texts.last().expect("text layer was just pushed");
    output.text_hits.extend(unsafe { text_layer_hits(layer) });
    if node.has_tag_name("TextInput") && !is_readonly_text_input(node) {
        output.text_inputs.push(TextInputRegion {
            id: node.attribute("id").unwrap_or_default().to_string(),
            text,
            color: layer
                .runs
                .first()
                .map(|run| run.color)
                .unwrap_or(COLORREF(0)),
            font_size: layer.font_size,
            bold: layer.bold,
            left: rect.left,
            top: rect.top,
            width: rect.width,
            height: rect.height,
        });
    }
}

/// A `readonly` field shows a value the user cannot type into.
fn is_readonly_text_input(node: roxmltree::Node<'_, '_>) -> bool {
    node.attribute("readonly")
        .is_some_and(|value| value != "false")
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
        // `status` replaces the authored placeholder with what the running task
        // published. A project script names its own steps, so its literal text
        // wins; otherwise the published locale key is looked up.
        if source == "status" {
            let published = context
                .interaction
                .status_text
                .clone()
                .or_else(|| {
                    context
                        .interaction
                        .status_key
                        .as_deref()
                        .and_then(|key| context.translations.get(key))
                        .cloned()
                })
                .unwrap_or(text);
            return Some((published, alignment));
        }
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

/// The flow direction a container declares. Containers without `layout` are
/// horizontal, matching the first implementation slice.
fn flow_axis(node: roxmltree::Node<'_, '_>) -> Option<FlowAxis> {
    match node.tag_name().name() {
        "HBox" => Some(FlowAxis::Horizontal),
        "VBox" => Some(FlowAxis::Vertical),
        "Content" => match node.attribute("layout") {
            Some("vertical") => Some(FlowAxis::Vertical),
            _ => Some(FlowAxis::Horizontal),
        },
        _ => None,
    }
}

/// Lays out a flow container: padding inset, then children in order along the
/// main axis with flex sizing, gaps, `justify-content` and `align-items`.
fn render_flow(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    axis: FlowAxis,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    let padding = insets_for_node(node, "padding", context);
    let content = LayerRect {
        left: rect.left + padding.left,
        top: rect.top + padding.top,
        width: (rect.width - padding.horizontal()).max(0),
        height: (rect.height - padding.vertical()).max(0),
    };
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
    let available_main = axis.main(content);
    let items: Vec<_> = children
        .iter()
        .map(|child| flow_item_for_node(*child, axis, available_main, context))
        .collect();
    let cross_sizes: Vec<i32> = children
        .iter()
        .map(|child| cross_size_for_node(*child, axis, axis.cross(content), context))
        .collect();
    // A container that does not wrap keeps every item on one line, which is what
    // free space is shared across.
    let lines = if wraps(node) {
        wrap_lines(&items, available_main, gap)
    } else {
        vec![(0..items.len()).collect()]
    };
    let single_line = lines.len() == 1 && !wraps(node);
    let mut cross_cursor = 0;
    for line in lines {
        let line_items: Vec<FlowItem> = line.iter().map(|index| items[*index]).collect();
        let sizes = flow_widths(&line_items, available_main, gap);
        let used: i32 = sizes.iter().sum::<i32>()
            + gap * i32::try_from(line.len().saturating_sub(1)).unwrap_or(0);
        // `justify-content` places the whole run inside the leftover space; HBox
        // and Content spell the same idea `horizontal-align`.
        let offset = match main_alignment(node, axis) {
            Some("center") => (available_main - used) / 2,
            Some("end" | "right") => available_main - used,
            _ => 0,
        }
        .max(0);
        // A line of a wrapping container is only as tall as its tallest item, so
        // the next line starts right below it. A container that does not wrap
        // aligns inside its own cross extent, as it always has.
        let line_cross = if single_line {
            axis.cross(content)
        } else {
            line.iter()
                .map(|index| cross_sizes[*index])
                .max()
                .unwrap_or(0)
        };
        let mut cursor = offset;
        for (position, index) in line.iter().enumerate() {
            let child = children[*index];
            let margin = insets_for_node(child, "margin", context);
            let outer_main = sizes[position];
            let main = (outer_main - margin.along(axis)).max(0);
            let cross = cross_sizes[*index];
            // `align-self` overrides the container's `align-items` for one item.
            let cross_offset = match cross_alignment_for_item(child, node, axis) {
                Some("center") => (line_cross - cross) / 2,
                Some("end" | "bottom" | "right") => line_cross - cross,
                _ => 0,
            };
            let placed = axis.place(content, main, cross);
            let placed = axis.shift(placed, cursor + margin_main_start(margin, axis));
            let placed = axis.shift_cross(
                placed,
                cross_cursor + cross_offset.max(0) + margin_cross_start(margin, axis),
            );
            if main > 0 && cross > 0 {
                render_flow_item(child, placed, context, output)?;
            }
            cursor += outer_main + gap;
        }
        cross_cursor += line_cross + gap;
    }
    Ok(())
}

/// Whether a container asks its items to move onto another line when they run
/// out of room.
fn wraps(node: roxmltree::Node<'_, '_>) -> bool {
    matches!(
        node.attribute("flex-wrap"),
        Some("true" | "wrap" | "wrap-reverse")
    )
}

/// Splits items into the lines a wrapping container draws them on.
///
/// Items are added while they still fit and the item that no longer does starts
/// the next line, which is the behaviour a row of cards or tags needs. The size
/// used here is the one an item claims before free space is shared out, because
/// that is the room it needs to stay on the current line.
fn wrap_lines(items: &[FlowItem], available_main: i32, gap: i32) -> Vec<Vec<usize>> {
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut used = 0;
    for (index, item) in items.iter().enumerate() {
        let natural = item.basis_size();
        let with_gap = if current.is_empty() {
            natural
        } else {
            used + gap + natural
        };
        if !current.is_empty() && with_gap > available_main {
            lines.push(std::mem::take(&mut current));
            used = natural;
        } else {
            used = with_gap;
        }
        current.push(index);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Main-axis alignment for a container, accepting the spelling each tag uses.
fn main_alignment<'a>(node: roxmltree::Node<'a, '_>, axis: FlowAxis) -> Option<&'a str> {
    match axis {
        FlowAxis::Horizontal => node
            .attribute("justify-content")
            .or_else(|| node.attribute("horizontal-align")),
        FlowAxis::Vertical => node.attribute("justify-content"),
    }
}

/// Cross-axis alignment for one item: its own `align-self` when set, otherwise
/// the container's `align-items`.
fn cross_alignment_for_item<'a>(
    child: roxmltree::Node<'a, '_>,
    container: roxmltree::Node<'a, '_>,
    axis: FlowAxis,
) -> Option<&'a str> {
    child
        .attribute("align-self")
        .or_else(|| cross_alignment(container, axis))
}

/// Cross-axis alignment for a container, accepting the spelling each tag uses.
fn cross_alignment<'a>(node: roxmltree::Node<'a, '_>, axis: FlowAxis) -> Option<&'a str> {
    match axis {
        FlowAxis::Horizontal => node
            .attribute("align-items")
            .or_else(|| node.attribute("vertical-align")),
        FlowAxis::Vertical => node
            .attribute("align-items")
            .or_else(|| node.attribute("horizontal-align")),
    }
}

fn margin_main_start(margin: Insets, axis: FlowAxis) -> i32 {
    match axis {
        FlowAxis::Horizontal => margin.left,
        FlowAxis::Vertical => margin.top,
    }
}

fn margin_cross_start(margin: Insets, axis: FlowAxis) -> i32 {
    match axis {
        FlowAxis::Horizontal => margin.top,
        FlowAxis::Vertical => margin.left,
    }
}

/// The main-axis size an item claims, including its margins. Items without an
/// explicit or intrinsic size rely on `flex-grow`, as `Spacer` does.
fn flow_item_for_node(
    node: roxmltree::Node<'_, '_>,
    axis: FlowAxis,
    available_main: i32,
    context: &LayoutContext<'_>,
) -> FlowItem {
    let margin = insets_for_node(node, "margin", context);
    let padding = insets_for_node(node, "padding", context);
    let inset = padding.along(axis) + margin.along(axis);
    let explicit = size_attribute(node, main_axis_attribute(axis), available_main, context);
    let intrinsic = intrinsic_size(node, axis, context);
    let flex_grow = float_attribute(node, "flex-grow").unwrap_or(0.0);
    // `flex-basis` is the size an item starts from before free space is shared
    // out, so an item that declares it keeps growing instead of freezing at the
    // basis the way a plain width would.
    let basis = size_attribute(node, "flex-basis", available_main, context);
    let (fixed_width, flex_basis) = match basis {
        Some(basis) => (
            (flex_grow <= 0.0).then(|| basis + inset),
            (basis + inset).max(0),
        ),
        None => (explicit.or(intrinsic).map(|value| value + inset), 0),
    };
    let min_attribute = match axis {
        FlowAxis::Horizontal => "min-width",
        FlowAxis::Vertical => "min-height",
    };
    FlowItem {
        fixed_width,
        flex_grow,
        flex_shrink: float_attribute(node, "flex-shrink").unwrap_or(1.0),
        min_width: size_attribute(node, min_attribute, available_main, context).unwrap_or(0),
        flex_basis,
    }
}

fn main_axis_attribute(axis: FlowAxis) -> &'static str {
    match axis {
        FlowAxis::Horizontal => "width",
        FlowAxis::Vertical => "height",
    }
}

fn cross_axis_attribute(axis: FlowAxis) -> &'static str {
    match axis {
        FlowAxis::Horizontal => "height",
        FlowAxis::Vertical => "width",
    }
}

/// The content size a text control claims when measuring `axis`. Text measures
/// by width horizontally and by its line box vertically.
fn intrinsic_size(
    node: roxmltree::Node<'_, '_>,
    axis: FlowAxis,
    context: &LayoutContext<'_>,
) -> Option<i32> {
    // A nested container reports the extent its own children need, so an outer
    // flow can size it without the layout declaring a fixed size.
    if renders_own_children(node) {
        return Some(container_intrinsic_size(node, axis, context));
    }
    if !has_intrinsic_text(node) {
        return None;
    }
    match axis {
        FlowAxis::Vertical => Some(text_intrinsic_height(node, context)),
        FlowAxis::Horizontal => resolved_text_for_node(node, context).map(|(text, _)| {
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
        }),
    }
}

/// The height a text control needs: its line box, or the wrapped line count
/// when the layout asks it to wrap inside a declared width.
fn text_intrinsic_height(node: roxmltree::Node<'_, '_>, context: &LayoutContext<'_>) -> i32 {
    let line_height = line_height_for_node(node, context);
    if node.attribute("wrap") != Some("true") {
        return line_height;
    }
    let Some(width) = size_attribute(node, "width", 0, context).filter(|width| *width > 0) else {
        return line_height;
    };
    let Some((text, _)) = resolved_text_for_node(node, context) else {
        return line_height;
    };
    let font_size = scale_value(
        int_attribute(node, "font-size").unwrap_or(12),
        context.dpi.scale,
    );
    let bold = node.attribute("font-weight") == Some("bold");
    let lines = wrapped_line_count(&visible_text(&text), font_size, bold, width);
    line_height * lines
}

/// Counts the lines `text` occupies when wrapped at `width`.
///
/// The runtime breaks between any two characters, so this counts the same way
/// and mixed CJK and Latin text measures the same here as when it is drawn.
fn wrapped_line_count(text: &str, font_size: i32, bold: bool, width: i32) -> i32 {
    let mut lines = 1;
    let mut line_width = 0;
    for character in text.chars() {
        if character == '\n' {
            lines += 1;
            line_width = 0;
            continue;
        }
        let measured = measure_layout_text_width(&character.to_string(), font_size, bold);
        if line_width > 0 && line_width + measured > width {
            lines += 1;
            line_width = 0;
        }
        line_width += measured;
    }
    lines
}

/// The extent a container needs for `axis`, measured from its children.
///
/// Along its own flow direction the children add up with their gaps; across it
/// the largest child wins. Padding is added on both sides, which is what lets a
/// nested panel size itself from the controls it holds.
fn container_intrinsic_size(
    node: roxmltree::Node<'_, '_>,
    axis: FlowAxis,
    context: &LayoutContext<'_>,
) -> i32 {
    let padding = insets_for_node(node, "padding", context);
    let children: Vec<_> = node
        .children()
        .filter(|child| child.is_element() && !is_hidden(*child, context.interaction))
        .collect();
    if children.is_empty() {
        return padding.along(axis);
    }
    let declared_main = size_attribute(node, main_axis_attribute(axis), 0, context).unwrap_or(0);
    let declared_cross = size_attribute(node, cross_axis_attribute(axis), 0, context).unwrap_or(0);
    let content = if flow_axis(node) == Some(axis) {
        let gap = scale_value(
            int_attribute(node, "item-spacing")
                .or_else(|| int_attribute(node, "gap"))
                .unwrap_or(0),
            context.dpi.scale,
        );
        let items: Vec<FlowItem> = children
            .iter()
            .map(|child| flow_item_for_node(*child, axis, declared_main, context))
            .collect();
        if wraps(node) && declared_main > 0 {
            // A wrapping container is only as wide as its widest line, which is
            // what an outer flow needs to place it.
            wrap_lines(&items, declared_main, gap)
                .iter()
                .map(|line| {
                    let widths: i32 = line.iter().map(|index| items[*index].basis_size()).sum();
                    widths + gap * i32::try_from(line.len().saturating_sub(1)).unwrap_or(0)
                })
                .max()
                .unwrap_or(0)
        } else {
            let widths: i32 = items.iter().map(|item| item.basis_size()).sum();
            widths + gap * i32::try_from(children.len().saturating_sub(1)).unwrap_or(0)
        }
    } else {
        children
            .iter()
            .map(|child| {
                let margin = insets_for_node(*child, "margin", context);
                cross_size_for_node(*child, axis, declared_cross, context)
                    + margin.along(axis.cross_measure())
            })
            .max()
            .unwrap_or(0)
    };
    (content + padding.along(axis)).max(0)
}

fn has_intrinsic_text(node: roxmltree::Node<'_, '_>) -> bool {
    node.tag_name().name() == "Checkbox"
        || node.tag_name().name() == "Label"
        || node.tag_name().name() == "Button"
}

fn line_height_for_node(node: roxmltree::Node<'_, '_>, context: &LayoutContext<'_>) -> i32 {
    let font_size = scale_value(
        int_attribute(node, "font-size").unwrap_or(12),
        context.dpi.scale,
    );
    (font_size as f32 * 1.4).round() as i32
}

/// The cross-axis size of an item: explicit if declared, otherwise its content
/// size, otherwise the container's own extent (the default stretch).
fn cross_size_for_node(
    node: roxmltree::Node<'_, '_>,
    axis: FlowAxis,
    available_cross: i32,
    context: &LayoutContext<'_>,
) -> i32 {
    if let Some(explicit) =
        size_attribute(node, cross_axis_attribute(axis), available_cross, context)
    {
        return explicit;
    }
    if let Some(intrinsic) = intrinsic_size(node, axis.cross_measure(), context) {
        return intrinsic.min(available_cross.max(0));
    }
    available_cross
}

/// Reads an extent that may be a pixel count or a percentage of the parent.
fn size_attribute(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    base: i32,
    context: &LayoutContext<'_>,
) -> Option<i32> {
    let raw = node.attribute(name)?.trim();
    if let Some(percent) = raw.strip_suffix('%') {
        let percent: f32 = percent.trim().parse().ok()?;
        return Some((base as f32 * percent / 100.0).round() as i32);
    }
    let value: i32 = raw.parse().ok()?;
    Some(scale_value(value, context.dpi.scale))
}

/// Parses the one-to-four-value `padding`/`margin` shorthand, in device pixels.
fn insets_for_node(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    context: &LayoutContext<'_>,
) -> Insets {
    let mut insets = Insets::default();
    if let Some(raw) = node.attribute(name) {
        let values: Vec<i32> = raw
            .split_whitespace()
            .filter_map(|value| value.parse::<i32>().ok())
            .map(|value| scale_value(value, context.dpi.scale))
            .collect();
        match values.as_slice() {
            [all] => {
                insets = Insets {
                    left: *all,
                    top: *all,
                    right: *all,
                    bottom: *all,
                }
            }
            [vertical, horizontal] => {
                insets = Insets {
                    left: *horizontal,
                    top: *vertical,
                    right: *horizontal,
                    bottom: *vertical,
                }
            }
            [top, horizontal, bottom] => {
                insets = Insets {
                    left: *horizontal,
                    top: *top,
                    right: *horizontal,
                    bottom: *bottom,
                }
            }
            [top, right, bottom, left] => {
                insets = Insets {
                    left: *left,
                    top: *top,
                    right: *right,
                    bottom: *bottom,
                }
            }
            _ => {}
        }
    }
    // `margin-top` and friends override the shorthand, standing alone the way
    // the example layouts write them.
    if let Some(value) = edge_attribute(node, name, "top", context) {
        insets.top = value;
    }
    if let Some(value) = edge_attribute(node, name, "right", context) {
        insets.right = value;
    }
    if let Some(value) = edge_attribute(node, name, "bottom", context) {
        insets.bottom = value;
    }
    if let Some(value) = edge_attribute(node, name, "left", context) {
        insets.left = value;
    }
    insets
}

fn edge_attribute(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    side: &str,
    context: &LayoutContext<'_>,
) -> Option<i32> {
    let attribute = format!("{name}-{side}");
    int_attribute(node, &attribute).map(|value| scale_value(value, context.dpi.scale))
}

/// Shrinks `rect` by `insets` on each edge.
fn inset_rect(rect: LayerRect, insets: Insets) -> LayerRect {
    LayerRect {
        left: rect.left + insets.left,
        top: rect.top + insets.top,
        width: rect.width - insets.horizontal(),
        height: rect.height - insets.vertical(),
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
    // A leaf control's own padding insets what it draws, but not its place in
    // the flow, which the parent already accounted for. Containers and `Box`
    // apply their own padding where they lay out their children.
    let rect = if flow_axis(node).is_none() && !node.has_tag_name("Box") {
        let inset = inset_rect(rect, insets_for_node(node, "padding", context));
        if inset.width <= 0 || inset.height <= 0 {
            return Ok(());
        }
        inset
    } else {
        rect
    };
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
            push_action(node, rect, &mut output.actions, context);
            push_node_text(
                node,
                LayerRect {
                    left: text_left,
                    top: rect.top,
                    width: (rect.left + rect.width - text_left).max(0),
                    height: rect.height,
                },
                context,
                output,
            );
        }
        "Button" => {
            push_action(node, rect, &mut output.actions, context);
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
            push_node_text(node, rect, context, output);
            if let Some(content) = node.children().find(|child| {
                child.has_tag_name("Content") && !is_hidden(*child, context.interaction)
            }) {
                render_flow(content, rect, FlowAxis::Horizontal, context, output)?;
            }
        }
        "Label" | "Select" => {
            push_action(node, rect, &mut output.actions, context);
            push_hover_region(node, rect, context, &mut output.hover_regions);
            push_node_text(node, rect, context, output);
        }
        "TextInput" => push_node_text(node, rect, context, output),
        "ProgressBar" => render_progress_bar(node, rect, context, output)?,
        "Image" | "Icon" => {
            push_action(node, rect, &mut output.actions, context);
            push_hover_region(node, rect, context, &mut output.hover_regions);
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
        "Box" | "Divider" => render_box_contents(node, rect, context, output)?,
        _ => {
            if let Some(axis) = flow_axis(node) {
                render_flow(node, rect, axis, context, output)?;
            }
        }
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
    push_node_border(node, rect, context, &mut output.layers)?;
    let padding = insets_for_node(node, "padding", context);
    let content = LayerRect {
        left: rect.left + padding.left,
        top: rect.top + padding.top,
        width: (rect.width - padding.horizontal()).max(0),
        height: (rect.height - padding.vertical()).max(0),
    };
    for child in node
        .children()
        .filter(|child| child.is_element() && !is_hidden(*child, context.interaction))
    {
        let width = size_attribute(child, "width", content.width, context).unwrap_or(0);
        let height = size_attribute(child, "height", content.height, context).unwrap_or(0);
        // Children without `position="absolute"` are centered, which is how the
        // example's single-child wrappers declare their content boxes.
        let (left, top) = if child.attribute("position") == Some("absolute") {
            (
                anchored_left(child, content.left, width, content.width, context),
                anchored_top(child, content.top, height, content.height, context),
            )
        } else {
            (
                content.left + (content.width - width) / 2,
                content.top + (content.height - height) / 2,
            )
        };
        if width <= 0 || height <= 0 {
            continue;
        }
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
                    // A flexible item that overflows still keeps the basis it
                    // asked for; it simply gets no share of the free space.
                    return item.flex_basis;
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
    // Basis sizes are reserved before free space is shared, which is what lets
    // `flex-basis` and `flex-grow` work together.
    let basis_total = items
        .iter()
        .filter(|item| item.fixed_width.is_none())
        .map(|item| item.flex_basis)
        .sum::<i32>();
    let remaining = available_for_items - fixed_width - basis_total;
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
                return item.flex_basis;
            }
            let share = if Some(index) == last_flexible {
                (remaining - distributed).max(0)
            } else {
                ((remaining as f32 * item.flex_grow / total_flex).round() as i32)
                    .min((remaining - distributed).max(0))
            };
            distributed += share;
            item.flex_basis + share
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
            link: None,
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
                link: None,
            });
        }
        runs.push(TextRun {
            text: remaining[label_start..label_end].to_string(),
            color: link_color,
            link: Some(remaining[target_start..target_end].to_string()),
        });
        remaining = &remaining[target_end + 1..];
    }
    if !remaining.is_empty() {
        runs.push(TextRun {
            text: remaining.to_string(),
            color,
            link: None,
        });
    }
    if runs.is_empty() {
        runs.push(TextRun {
            text: text.to_string(),
            color,
            link: None,
        });
    }
    runs
}

/// How far into `field` the character at `index` starts, in device pixels.
///
/// The caret, the selection bands, and a click all measure the same prefix, so
/// they all agree on where a character begins.
fn text_offset_for_index(field: &TextInputRegion, index: usize) -> i32 {
    let visible = visible_text(&field.text);
    let prefix: String = visible.chars().take(index).collect();
    unsafe {
        let dc = GetDC(None);
        if dc.is_invalid() {
            return 0;
        }
        let font = create_ui_font(field.font_size, field.bold);
        let width = if font.is_invalid() {
            0
        } else {
            let previous = SelectObject(dc, font);
            let utf16: Vec<u16> = prefix.encode_utf16().collect();
            let width = measure_text_width(dc, &utf16);
            let _ = SelectObject(dc, previous);
            let _ = DeleteObject(font);
            width
        };
        let _ = ReleaseDC(None, dc);
        width
    }
}

/// The highlight a selected range draws, as one band per covered line.
///
/// The band is a solid premultiplied layer, so it blends through the same path
/// as the rest of the page and never hides the glyphs drawn on top of it.
fn selection_layers(field: &TextInputRegion, start: usize, end: usize) -> Vec<ImageLayer> {
    let visible = visible_text(&field.text);
    let count = visible.chars().count();
    let start = start.min(count);
    let end = end.min(count);
    if start >= end {
        return Vec::new();
    }
    let left = field.left + text_offset_for_index(field, start);
    let right = field.left + text_offset_for_index(field, end);
    let band_left = left.max(field.left);
    let band_right = right.min(field.left + field.width).max(band_left);
    let width = band_right - band_left;
    // The band is inset vertically so the field keeps a little of its own
    // background above and below the highlight.
    let inset = (field.height as f32 * 0.15).round() as i32;
    let top = field.top + inset;
    let height = (field.height - inset * 2).max(1);
    if width <= 0 || height <= 0 {
        return Vec::new();
    }
    // A light wash reads as a selection on both dark and light backgrounds.
    let (alpha, red, green, blue) = (96u8, 120u8, 190u8, 255u8);
    let premultiply = |component: u8| ((component as u16 * alpha as u16) / 255) as u8;
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[0] = premultiply(blue);
        pixel[1] = premultiply(green);
        pixel[2] = premultiply(red);
        pixel[3] = alpha;
    }
    vec![ImageLayer {
        image: NativeImage {
            width: width as u32,
            height: height as u32,
            pixels,
        },
        left: band_left,
        top,
        width,
        height,
        source: None,
        alpha: 255,
    }]
}

/// The caret for `field`, placed after `caret_index` characters.
///
/// The blink is a timer elsewhere; this only computes where the bar stands, so
/// the caret lines up with the glyphs the user sees.
fn caret_layer(field: &TextInputRegion, caret_index: usize) -> ImageLayer {
    let offset = text_offset_for_index(field, caret_index);
    // One device pixel wide, so a caret never looks like a selection handle.
    let caret_width = 1;
    let caret_height = (field.height as f32 * 0.7).round().max(1.0) as i32;
    let left = (field.left + offset).min(field.left + field.width - caret_width);
    // COLORREF is 0x00BBGGRR and the layer format is BGRA, so the components
    // already read in the order the pixel buffer wants them.
    let (blue, green, red) = (
        (field.color.0 >> 16) as u8,
        (field.color.0 >> 8) as u8,
        field.color.0 as u8,
    );
    let pixel_count = caret_width as usize * caret_height as usize;
    let mut pixels = vec![0u8; pixel_count * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[0] = blue;
        pixel[1] = green;
        pixel[2] = red;
        pixel[3] = 255;
    }
    ImageLayer {
        image: NativeImage {
            width: caret_width as u32,
            height: caret_height as u32,
            pixels,
        },
        left: left.max(field.left),
        top: field.top + (field.height - caret_height) / 2,
        width: caret_width,
        height: caret_height,
        source: None,
        alpha: 255,
    }
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
        source: None,
        alpha: 255,
    });
    Ok(())
}

/// Paints the outline a control declares, if it declares one.
///
/// Returned before any work when `border-color` is absent, so controls that do
/// not want an outline pay nothing for the feature.
fn push_node_border(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    layers: &mut Vec<ImageLayer>,
) -> Result<()> {
    let Some(color) = node.attribute("border-color") else {
        return Ok(());
    };
    let width = int_attribute(node, "border-width").unwrap_or(1);
    if width <= 0 {
        return Ok(());
    }
    let radius = scale_value(
        int_attribute(node, "border-radius").unwrap_or(0),
        context.dpi.scale,
    );
    push_border_layer(
        layers,
        rect,
        color,
        scale_value(width, context.dpi.scale),
        radius,
    )
}

/// Draws a rounded-rectangle outline.
///
/// The ring is a solid layer whose pixels survive only between the outer and
/// the inner rounded rectangle, so it blends with the same code path as fills
/// and needs no extra drawing primitive.
fn push_border_layer(
    layers: &mut Vec<ImageLayer>,
    rect: LayerRect,
    color: &str,
    width: i32,
    radius: i32,
) -> Result<()> {
    let width = width.max(1).min(rect.width / 2).min(rect.height / 2);
    if width <= 0 || rect.width <= 0 || rect.height <= 0 {
        return Ok(());
    }
    let pixel_width = u32::try_from(rect.width).context("border width must be positive")?;
    let pixel_height = u32::try_from(rect.height).context("border height must be positive")?;
    let pixel_count = (pixel_width as usize)
        .checked_mul(pixel_height as usize)
        .context("border size overflow")?;
    let mut pixels = vec![0u8; pixel_count.checked_mul(4).context("border size overflow")?];
    let (alpha, red, green, blue) = parse_argb(color);
    let premultiply = |component: u8| ((component as u16 * alpha as u16) / 255) as u8;
    let outer_radius = radius.min(rect.width / 2).min(rect.height / 2).max(0);
    let inner_width = rect.width - width * 2;
    let inner_height = rect.height - width * 2;
    let inner_radius = (outer_radius - width).max(0);
    for y in 0..rect.height {
        for x in 0..rect.width {
            if !inside_rounded_rect(x, y, rect.width, rect.height, outer_radius) {
                continue;
            }
            // The inner rectangle is tested in its own coordinates, which is
            // why the bounds are checked before the corner test.
            let inner_x = x - width;
            let inner_y = y - width;
            if inner_width > 0
                && inner_height > 0
                && inner_x >= 0
                && inner_y >= 0
                && inner_x < inner_width
                && inner_y < inner_height
                && inside_rounded_rect(inner_x, inner_y, inner_width, inner_height, inner_radius)
            {
                continue;
            }
            let offset = ((y as usize * pixel_width as usize) + x as usize) * 4;
            pixels[offset] = premultiply(blue);
            pixels[offset + 1] = premultiply(green);
            pixels[offset + 2] = premultiply(red);
            pixels[offset + 3] = alpha;
        }
    }
    layers.push(ImageLayer {
        image: NativeImage {
            width: pixel_width,
            height: pixel_height,
            pixels,
        },
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
        source: None,
        alpha: 255,
    });
    Ok(())
}

/// Draws a progress bar: a rounded track, then the filled portion on top.
///
/// `bar-image` is authored as a single full-width sprite, so the filled part is
/// drawn by clipping the sprite to the completed share of the control.
fn render_progress_bar(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    let radius = scale_value(
        int_attribute(node, "border-radius").unwrap_or(0),
        context.dpi.scale,
    );
    if let Some(background) = node.attribute("background") {
        push_solid_layer(&mut output.layers, rect, background, radius)?;
    }
    let authored = int_attribute(node, "progress").unwrap_or(0).clamp(0, 100) as u8;
    let progress = context.interaction.progress_for(authored);
    let filled = (rect.width as i64 * progress as i64 / 100) as i32;
    if filled <= 0 {
        return Ok(());
    }
    let Some(source) = node.attribute("bar-image") else {
        return Ok(());
    };
    let resolved = resolve_asset_path(context.files, source, context.dpi.use_2x)
        .with_context(|| format!("layout asset missing from native bundle: {source}"))?;
    let encoded = context
        .files
        .get(resolved)
        .expect("resolved asset path must exist");
    let image = decode_image(encoded)?;
    // Clip the sprite horizontally so the bar fills left to right.
    let source_width =
        ((image.width as i64 * progress as i64) / 100).clamp(1, image.width as i64) as i32;
    let source_height = image.height as i32;
    output.layers.push(ImageLayer {
        image,
        left: rect.left,
        top: rect.top,
        width: filled,
        height: rect.height,
        source: Some(LayerRect {
            left: 0,
            top: 0,
            width: source_width,
            height: source_height,
        }),
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
        source: None,
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
    // Taskbar, Alt+Tab, and any crash dialog show this title, so it is the
    // product name rather than an internal identifier.
    let title = UI
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| HSTRING::from(state.ui.product_name.clone()))
        .unwrap_or_else(|| HSTRING::from("nano-installer"));
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
            &title,
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
        // Worker threads post progress updates back to this window.
        if let Some(state) = UI.get() {
            if let Ok(mut state) = state.lock() {
                state.window = window.0 as isize;
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
            if end_text_selection_drag(window) {
                // A drag inside a field only ends the selection gesture.
            } else if text_input_at(x, y).is_some() {
                // The press already moved the caret into this field.
            } else if let Some(action) = window_action_at(x, y) {
                handle_window_action(window, action);
            } else {
                let _ = set_language_menu_open(window, false);
                // A click outside every field takes the keyboard focus away,
                // which is what hides the caret again.
                let _ = focus_text_input(window, None, 0);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = client_point(lparam);
            if let Some((id, index)) = text_input_at(x, y) {
                let extend = key_down(VK_SHIFT);
                let _ = begin_text_selection_drag(window, id, index, extend);
                let _ = SetCapture(window);
                return LRESULT(0);
            }
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
            if extend_text_selection_drag(x) {
                // Dragging a selection owns the pointer until it is released.
                return LRESULT(0);
            }
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
        // Layouts mark interactive controls with `cursor="hand"`; showing the
        // hand only over things that actually respond to a click matches that.
        WM_SETCURSOR if lparam.0 as u16 as u32 == HTCLIENT => {
            let mut point = POINT::default();
            if GetCursorPos(&mut point).is_ok() && ScreenToClient(window, &mut point).as_bool() {
                let editable = text_input_at(point.x, point.y).is_some();
                let clickable = window_action_at(point.x, point.y).is_some()
                    || hover_control_at(point.x, point.y).is_some();
                let cursor = if editable {
                    IDC_IBEAM
                } else if clickable {
                    IDC_HAND
                } else {
                    IDC_ARROW
                };
                if let Ok(handle) = LoadCursorW(None, cursor) {
                    let _ = SetCursor(handle);
                }
            }
            LRESULT(1)
        }
        // The language menu takes the keyboard while it is open: arrows move the
        // highlight, Enter confirms, and Escape closes only the menu.
        WM_CHAR if text_input_is_focused() => {
            handle_text_input_char(window, wparam.0 as u32);
            LRESULT(0)
        }
        WM_KEYDOWN if text_input_is_focused() => {
            handle_text_input_key(window, wparam.0 as u32);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == CARET_TIMER => {
            let _ = toggle_caret_blink();
            LRESULT(0)
        }
        WM_KEYDOWN if language_menu_is_open() => {
            handle_language_menu_key(window, wparam.0 as u32);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u32 == 0x1B && !install::busy() => {
            let _ = DestroyWindow(window);
            LRESULT(0)
        }
        WM_APP_REFRESH => {
            // The state is already rebuilt by whoever posted this message.
            let _ = InvalidateRect(window, None, false);
            // A worker may have moved the caret, and an input method is only
            // repositioned from the thread that owns the window.
            place_ime_windows(window);
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
    // Selection bands sit above the page art but under the glyphs they cover.
    for layer in &ui.selection {
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
    if ui.caret_drawn {
        if let Some(caret) = &ui.caret {
            draw_layer(destination, caret);
        }
    }
}

fn window_action_at(x: i32, y: i32) -> Option<WindowAction> {
    let state = UI.get()?.lock().ok()?;
    // Text hits are checked first: a link sits inside a label that may itself
    // overlap a panel, and the innermost target is the one the user aimed at.
    if let Some(hit) = state
        .ui
        .text_hits
        .iter()
        .rev()
        .find(|hit| x >= hit.left && x < hit.right && y >= hit.top && y < hit.bottom)
    {
        return Some(hit.action.clone());
    }
    state
        .ui
        .actions
        .iter()
        .rev()
        .find(|region| x >= region.left && x < region.right && y >= region.top && y < region.bottom)
        .map(|region| region.action.clone())
}

/// Focuses a field and starts a selection gesture from the press position.
///
/// A Shift-click extends the existing selection instead of starting a new one,
/// which is how a range is grown without dragging. A second press on the same
/// character within the system double-click time selects the word under it and
/// starts no gesture, so the press that follows does not wipe the word again.
unsafe fn begin_text_selection_drag(
    window: HWND,
    id: String,
    index: usize,
    extend: bool,
) -> Result<()> {
    focus_text_input(window, Some(id.clone()), index)?;
    update_runtime(|state| {
        let double_click = !extend && is_double_click(&state.interaction, &id, index);
        state.interaction.last_press = if double_click {
            None
        } else {
            Some((id.clone(), index, std::time::Instant::now()))
        };
        if double_click {
            select_word_at(state, index)?;
        } else {
            let extending = extend
                && state.interaction.selection_anchor.is_some()
                && state.interaction.focused_text_input.as_deref() == Some(id.as_str());
            if !extending {
                state.interaction.selection_anchor = Some(index);
            }
            state.interaction.caret_index = index;
        }
        // The gesture flag stays set either way: the release that follows has to
        // give the capture taken on this press back.
        state.interaction.dragging_text_selection = true;
        state.interaction.typing_run = false;
        Ok(())
    })
}

/// Whether this press repeats the previous one closely enough to be a double
/// click, which is measured with the same interval Windows uses elsewhere.
fn is_double_click(interaction: &InteractionState, id: &str, index: usize) -> bool {
    let Some((last_id, last_index, at)) = interaction.last_press.as_ref() else {
        return false;
    };
    if last_id != id || *last_index != index {
        return false;
    }
    let interval = unsafe { GetDoubleClickTime() }.max(1);
    at.elapsed() <= std::time::Duration::from_millis(u64::from(interval))
}

/// Selects the word around `index`, the way double-clicking a native edit
/// control does. A press beyond the last character selects nothing.
fn select_word_at(state: &mut RuntimeState, index: usize) -> Result<()> {
    let Some(id) = state.interaction.focused_text_input.clone() else {
        return Ok(());
    };
    let text = state
        .interaction
        .text_input_values
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let (start, end) = word_range(&text, index);
    state.interaction.selection_anchor = Some(start);
    state.interaction.caret_index = end;
    state.interaction.caret_visible = true;
    Ok(())
}

/// The character range of the word that contains `index`.
///
/// Characters are grouped the way Windows groups them for word selection:
/// letters, digits, and underscore form a word, whitespace forms a run of its
/// own, and every other character stands alone, so double-clicking a path
/// separator selects just that separator.
fn word_range(text: &str, index: usize) -> (usize, usize) {
    let characters: Vec<char> = text.chars().collect();
    if characters.is_empty() || index >= characters.len() {
        return (characters.len(), characters.len());
    }
    let mut start = index;
    while start > 0 && same_selection_run(characters[start - 1], characters[index]) {
        start -= 1;
    }
    let mut end = index + 1;
    while end < characters.len() && same_selection_run(characters[index], characters[end]) {
        end += 1;
    }
    (start, end)
}

/// Whether two neighbouring characters are selected as one run.
///
/// Words join up with words and whitespace with whitespace; anything else is a
/// separator, and separators never merge, not even two different ones, so `C:\`
/// selects either the colon or the backslash but never both.
fn same_selection_run(left: char, right: char) -> bool {
    (left.is_whitespace() && right.is_whitespace())
        || (is_word_character(left) && is_word_character(right))
}

/// Whether a character is part of a word for selection purposes.
fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Drags the selection end to the pointer while the button stays down.
///
/// Returns whether a drag was in progress, so the caller stops treating the
/// motion as hovering.
fn extend_text_selection_drag(x: i32) -> bool {
    if !UI
        .get()
        .and_then(|state| state.lock().ok())
        .is_some_and(|state| state.interaction.dragging_text_selection)
    {
        return false;
    }
    let _ = update_runtime(|state| {
        let Some(id) = state.interaction.focused_text_input.clone() else {
            state.interaction.dragging_text_selection = false;
            return Ok(());
        };
        let field = state
            .ui
            .text_inputs
            .iter()
            .find(|field| field.id == id)
            .cloned();
        if let Some(field) = field {
            let visible = visible_text(&field.text);
            // Dragging past either edge keeps extending to that end, so the
            // selection does not stall when the pointer leaves the field.
            let clamped = x.clamp(field.left, field.left + field.width);
            let index = unsafe { caret_index_for_x(&field, &visible, clamped) };
            state.interaction.caret_index = index;
        }
        state.interaction.caret_visible = true;
        Ok(())
    });
    true
}

/// Ends a selection gesture, if one was running.
fn end_text_selection_drag(window: HWND) -> bool {
    let dragging = UI
        .get()
        .and_then(|state| state.lock().ok())
        .is_some_and(|state| state.interaction.dragging_text_selection);
    if !dragging {
        return false;
    }
    let _ = update_runtime(|state| {
        // A press and release at the same spot is a plain click: the caret
        // stays, and nothing stays selected.
        if state.interaction.selection_anchor == Some(state.interaction.caret_index) {
            state.interaction.clear_selection();
        }
        state.interaction.dragging_text_selection = false;
        Ok(())
    });
    unsafe {
        let _ = ReleaseCapture();
        let _ = InvalidateRect(window, None, false);
    }
    true
}

/// The editable field under a point, with the caret index that point implies.
///
/// Clicking between two glyphs puts the caret on the nearer side, so a click
/// lands where the user aimed rather than always at the end of the text.
fn text_input_at(x: i32, y: i32) -> Option<(String, usize)> {
    let state = UI.get()?.lock().ok()?;
    let field = state.ui.text_inputs.iter().rev().find(|field| {
        x >= field.left
            && x < field.left + field.width
            && y >= field.top
            && y < field.top + field.height
    })?;
    let visible: String = visible_text(&field.text);
    let index = unsafe { caret_index_for_x(field, &visible, x) };
    Some((field.id.clone(), index))
}

/// The caret index a click at `x` selects inside `field`.
unsafe fn caret_index_for_x(field: &TextInputRegion, visible: &str, x: i32) -> usize {
    let dc = GetDC(None);
    if dc.is_invalid() {
        return visible.chars().count();
    }
    let font = create_ui_font(field.font_size, field.bold);
    if font.is_invalid() {
        let _ = ReleaseDC(None, dc);
        return visible.chars().count();
    }
    let previous = SelectObject(dc, font);
    let mut best = 0;
    let mut best_distance = i32::MAX;
    let mut prefix = String::new();
    for (index, character) in visible.chars().enumerate() {
        let utf16: Vec<u16> = prefix.encode_utf16().collect();
        let distance = (field.left + measure_text_width(dc, &utf16) - x).abs();
        if distance < best_distance {
            best_distance = distance;
            best = index;
        }
        prefix.push(character);
    }
    // The trailing position is only nearer than the last glyph when the click
    // is past the end of the text.
    let utf16: Vec<u16> = visible.encode_utf16().collect();
    if (field.left + measure_text_width(dc, &utf16) - x).abs() <= best_distance {
        best = visible.chars().count();
    }
    let _ = SelectObject(dc, previous);
    let _ = DeleteObject(font);
    let _ = ReleaseDC(None, dc);
    best
}

fn text_input_is_focused() -> bool {
    UI.get()
        .and_then(|state| state.lock().ok())
        .and_then(|state| state.interaction.focused_text_input.clone())
        .is_some()
}

/// Focuses `id` at `caret_index`, or clears the focus when `id` is `None`.
unsafe fn focus_text_input(window: HWND, id: Option<String>, caret_index: usize) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    let focused = id.as_deref().filter(|_| {
        state
            .ui
            .text_inputs
            .iter()
            .any(|field| Some(field.id.as_str()) == id.as_deref())
    });
    if state.interaction.focused_text_input.as_deref() == focused
        && (focused.is_none() || state.interaction.caret_index == caret_index)
    {
        return Ok(());
    }
    if state.interaction.focused_text_input.as_deref() != focused {
        // A new field starts without a selection carried over from the old one.
        state.interaction.clear_selection();
        state.interaction.typing_run = false;
    }
    state.interaction.focused_text_input = focused.map(str::to_string);
    state.interaction.caret_index = caret_index;
    state.interaction.caret_visible = true;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    // The blink timer only runs while a field owns the focus.
    let _ = KillTimer(window, CARET_TIMER);
    if focused.is_some() {
        let _ = SetTimer(window, CARET_TIMER, CARET_BLINK_MS, None);
    }
    // An East Asian input method draws its own composition and candidate
    // windows; without this they appear wherever the last application left
    // them instead of at the caret the user is typing into.
    place_ime_windows(window);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

/// Flips the caret between drawn and hidden, the way an edit control blinks.
///
/// Only the flag changes, so the page is not laid out again on every blink.
/// The rectangle the caret occupies, which is where an input method anchors.
fn caret_rect(field: &TextInputRegion, caret_index: usize) -> LayerRect {
    let offset = text_offset_for_index(field, caret_index);
    let caret_height = (field.height as f32 * 0.7).round().max(1.0) as i32;
    LayerRect {
        left: (field.left + offset).min(field.left + field.width - 1),
        top: field.top + (field.height - caret_height) / 2,
        width: 1,
        height: caret_height,
    }
}

/// Tells the active input method where the caret is.
///
/// An IME is not part of the window it types into: it keeps its own composition
/// and candidate windows and only asks the window for a position. Moving those
/// windows to the caret is what makes a Chinese, Japanese, or Korean candidate
/// list appear next to the text field the user is typing in.
///
/// Nothing here reports a failure. A machine with no input method, or a field
/// scrolled out of view, simply keeps the default IME placement, which is what
/// the installer did before.
unsafe fn place_ime_windows(window: HWND) {
    let caret = UI
        .get()
        .and_then(|state| state.lock().ok())
        .and_then(|state| state.ui.caret_rect);
    let Some(caret) = caret else {
        return;
    };
    let context = ImmGetContext(window);
    if context.is_invalid() {
        return;
    }
    // The composition text starts at the caret, and the candidate list sits
    // just below it, which is where a user looks for it.
    let point = POINT {
        x: caret.left,
        y: caret.top,
    };
    let composition = COMPOSITIONFORM {
        dwStyle: CFS_POINT,
        ptCurrentPos: point,
        rcArea: RECT::default(),
    };
    let _ = ImmSetCompositionWindow(context, &composition);
    let candidate = CANDIDATEFORM {
        dwIndex: 0,
        dwStyle: CFS_CANDIDATEPOS,
        ptCurrentPos: POINT {
            x: point.x,
            y: point.y + caret.height,
        },
        rcArea: RECT::default(),
    };
    let _ = ImmSetCandidateWindow(context, &candidate);
    let _ = ImmReleaseContext(window, context);
}

unsafe fn toggle_caret_blink() -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    if state.interaction.focused_text_input.is_none() {
        return Ok(());
    }
    state.interaction.caret_visible = !state.interaction.caret_visible;
    state.ui.caret_drawn = state.interaction.caret_visible;
    let window = state.window;
    drop(state);
    if window != 0 {
        let _ = InvalidateRect(HWND(window as *mut _), None, false);
    }
    Ok(())
}

/// Applies a typed character to the focused field.
unsafe fn handle_text_input_char(window: HWND, character: u32) {
    // Control characters arrive as their own `WM_CHAR` values; editing keys are
    // handled in `WM_KEYDOWN`, so they are dropped here.
    if character < 0x20 || character == 0x7F {
        return;
    }
    let Some(character) = char::from_u32(character) else {
        return;
    };
    let result = edit_focused_text(true, |text, caret| {
        let index = byte_index(text, caret);
        text.insert(index, character);
        caret + 1
    });
    if let Err(error) = result {
        show_runtime_error(&error);
    } else {
        let _ = InvalidateRect(window, None, false);
        let _ = UpdateWindow(window);
    }
}

/// Applies an editing key to the focused field.
///
/// Shift extends the selection, Ctrl drives the clipboard and undo instead of
/// moving the caret, and Escape hands the keyboard back to the window.
unsafe fn handle_text_input_key(window: HWND, key: u32) {
    let control = key_down(VK_CONTROL);
    let shift = key_down(VK_SHIFT);
    let result = match key {
        value if value == VK_BACK.0 as u32 => delete_before_caret(control),
        value if value == VK_DELETE.0 as u32 => delete_after_caret(control),
        value if value == VK_LEFT.0 as u32 => move_caret(-1, shift, control),
        value if value == VK_RIGHT.0 as u32 => move_caret(1, shift, control),
        value if value == VK_HOME.0 as u32 => move_caret_to(0, shift),
        value if value == VK_END.0 as u32 => move_caret_to(usize::MAX, shift),
        value if control && value == VK_A.0 as u32 => select_all_focused_text(),
        value if control && value == VK_C.0 as u32 => copy_focused_selection(false),
        value if control && value == VK_X.0 as u32 => copy_focused_selection(true),
        value if control && value == VK_V.0 as u32 => paste_into_focused_text(),
        value if control && value == VK_Z.0 as u32 => undo_focused_text(),
        value if control && value == VK_Y.0 as u32 => redo_focused_text(),
        // Escape gives the keyboard back to the window, so the next one closes
        // the wizard the way it does everywhere else.
        0x1B => focus_text_input(window, None, 0),
        _ => Ok(()),
    };
    if let Err(error) = result {
        show_runtime_error(&error);
    } else {
        let _ = InvalidateRect(window, None, false);
        let _ = UpdateWindow(window);
    }
}

/// Whether `key` is held down right now, which is how a modifier arrives.
fn key_down(key: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(key.0 as i32) as u16 & 0x8000 != 0 }
}

/// Runs `edit` against the focused field's text and caret, then republishes it.
///
/// A selection is removed first and the caret moves to its start, so whatever
/// the caller inserts replaces the selected text the way any edit control
/// behaves. `typing` marks a keystroke that inserts a character, which lets a
/// run of them undo together.
fn edit_focused_text(typing: bool, edit: impl FnOnce(&mut String, usize) -> usize) -> Result<()> {
    update_runtime(|state| {
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        state.interaction.remember_for_undo(typing);
        state.interaction.remove_selection();
        let caret = state.interaction.caret_index;
        let text = state.interaction.text_input_values.entry(id).or_default();
        let caret = edit(text, caret);
        state.interaction.caret_index = caret;
        // Typing restarts the blink so the caret is visible while keys arrive.
        state.interaction.caret_visible = true;
        Ok(())
    })
}

/// Removes the selection, or the text before the caret when none is set.
///
/// A selection is removed on its own: backspacing into it must not also eat the
/// character that was sitting in front of the range. With `whole_word` the key
/// takes the word before the caret, which is what Ctrl+Backspace does.
fn delete_before_caret(whole_word: bool) -> Result<()> {
    update_runtime(|state| {
        state.interaction.remember_for_undo(false);
        if state.interaction.remove_selection() {
            state.interaction.caret_visible = true;
            return Ok(());
        }
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        let caret = state.interaction.caret_index;
        if caret == 0 {
            return Ok(());
        }
        let text = state.interaction.text_input_values.entry(id).or_default();
        let index = byte_index(text, caret);
        let start_at = if whole_word {
            word_start_before(text, caret)
        } else {
            caret - 1
        };
        let start = byte_index(text, start_at);
        text.replace_range(start..index, "");
        state.interaction.caret_index = start_at;
        state.interaction.caret_visible = true;
        Ok(())
    })
}

/// Where Ctrl+Backspace starts removing: the whitespace in front of the caret
/// first, and then the run before it.
///
/// A separator stands alone the same way it does for `word_range`, so the key
/// removes one path separator rather than the punctuation around it.
fn word_start_before(text: &str, caret: usize) -> usize {
    let characters: Vec<char> = text.chars().collect();
    let mut index = caret.min(characters.len());
    while index > 0 && characters[index - 1].is_whitespace() {
        index -= 1;
    }
    if index == 0 {
        return 0;
    }
    if !is_word_character(characters[index - 1]) {
        return index - 1;
    }
    while index > 0 && is_word_character(characters[index - 1]) {
        index -= 1;
    }
    index
}

/// Where Ctrl+Delete stops removing, mirroring `word_start_before`.
fn word_end_after(text: &str, caret: usize) -> usize {
    let characters: Vec<char> = text.chars().collect();
    let count = characters.len();
    let mut index = caret.min(count);
    while index < count && characters[index].is_whitespace() {
        index += 1;
    }
    if index == count {
        return count;
    }
    if !is_word_character(characters[index]) {
        return index + 1;
    }
    while index < count && is_word_character(characters[index]) {
        index += 1;
    }
    index
}

/// Removes the selection, or the text after the caret when none is set.
///
/// With `whole_word` the key takes the word after the caret and the whitespace
/// in front of it, which is what Ctrl+Delete does.
fn delete_after_caret(whole_word: bool) -> Result<()> {
    update_runtime(|state| {
        state.interaction.remember_for_undo(false);
        if state.interaction.remove_selection() {
            state.interaction.caret_visible = true;
            return Ok(());
        }
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        let caret = state.interaction.caret_index;
        let text = state.interaction.text_input_values.entry(id).or_default();
        let count = text.chars().count();
        if caret >= count {
            return Ok(());
        }
        let end_at = if whole_word {
            word_end_after(text, caret)
        } else {
            caret + 1
        };
        let (start, end) = (byte_index(text, caret), byte_index(text, end_at));
        text.replace_range(start..end, "");
        state.interaction.caret_visible = true;
        Ok(())
    })
}

/// Selects the whole focused field, the way Ctrl+A does everywhere else.
fn select_all_focused_text() -> Result<()> {
    update_runtime(|state| {
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        let count = state
            .interaction
            .text_input_values
            .get(&id)
            .map(|text| text.chars().count())
            .unwrap_or(0);
        state.interaction.selection_anchor = Some(0);
        state.interaction.caret_index = count;
        state.interaction.caret_visible = true;
        state.interaction.typing_run = false;
        Ok(())
    })
}

/// Moves the caret, extending the selection when asked.
///
/// A plain arrow key that meets an existing selection collapses it to the end
/// the key points at instead of moving past it, which is what a native edit
/// control does. With `whole_word` the caret jumps over a word instead of one
/// character, which is what Ctrl+Left and Ctrl+Right do.
fn move_caret(delta: i32, extend: bool, whole_word: bool) -> Result<()> {
    update_runtime(|state| {
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        let text = state
            .interaction
            .text_input_values
            .get(&id)
            .cloned()
            .unwrap_or_default();
        let count = text.chars().count();
        if !extend {
            if let Some((start, end)) = state.interaction.selection_range() {
                state.interaction.clear_selection();
                state.interaction.caret_index = if delta < 0 { start } else { end };
                state.interaction.caret_visible = true;
                state.interaction.typing_run = false;
                return Ok(());
            }
        }
        let caret = state.interaction.caret_index;
        let moved = if whole_word {
            if delta < 0 {
                word_start_before(&text, caret)
            } else {
                word_end_after(&text, caret)
            }
        } else {
            (caret as i32 + delta).clamp(0, count as i32) as usize
        };
        extend_selection_to(state, moved, extend);
        Ok(())
    })
}

/// Moves the caret to `index`, extending the selection when asked.
fn move_caret_to(index: usize, extend: bool) -> Result<()> {
    update_runtime(|state| {
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        let count = state
            .interaction
            .text_input_values
            .get(&id)
            .map(|text| text.chars().count())
            .unwrap_or(0);
        if !extend {
            if let Some((start, end)) = state.interaction.selection_range() {
                state.interaction.clear_selection();
                state.interaction.caret_index = if index == 0 { start } else { end };
                state.interaction.caret_visible = true;
                state.interaction.typing_run = false;
                return Ok(());
            }
        }
        extend_selection_to(state, index.min(count), extend);
        Ok(())
    })
}

/// Moves the caret, remembering where the selection started when extending.
fn extend_selection_to(state: &mut RuntimeState, caret: usize, extend: bool) {
    if extend {
        state
            .interaction
            .selection_anchor
            .get_or_insert(state.interaction.caret_index);
    } else {
        state.interaction.clear_selection();
    }
    state.interaction.caret_index = caret;
    state.interaction.caret_visible = true;
    state.interaction.typing_run = false;
}

/// Byte offset of character `index`, so multi-byte text edits in the right place.
fn byte_index(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map(|(offset, _)| offset)
        .unwrap_or(text.len())
}

/// The selected text of the focused field, if there is a selection.
fn focused_selection() -> Option<String> {
    let state = UI.get()?.lock().ok()?;
    let id = state.interaction.focused_text_input.as_deref()?;
    let (start, end) = state.interaction.selection_range()?;
    let text = state.interaction.text_input_values.get(id)?;
    Some(
        text.chars()
            .skip(start)
            .take(end - start)
            .collect::<String>(),
    )
}

/// Puts the selected text on the clipboard, and removes it for a cut.
fn copy_focused_selection(cut: bool) -> Result<()> {
    let Some(selected) = focused_selection() else {
        return Ok(());
    };
    if selected.is_empty() {
        return Ok(());
    }
    // The clipboard is written first: a cut that cannot reach the clipboard
    // must not lose the text it was supposed to copy.
    unsafe { write_clipboard_text(&selected)? };
    if cut {
        edit_focused_text(false, |_, caret| caret)?;
    }
    Ok(())
}

/// Appends the clipboard text to the focused field, if the clipboard holds text.
fn paste_into_focused_text() -> Result<()> {
    let Some(pasted) = (unsafe { read_clipboard_text() }) else {
        return Ok(());
    };
    if pasted.is_empty() {
        return Ok(());
    }
    edit_focused_text(false, |text, caret| {
        let index = byte_index(text, caret);
        text.insert_str(index, &pasted);
        caret + pasted.chars().count()
    })
}

/// Steps back through the edits of this run, newest first.
fn undo_focused_text() -> Result<()> {
    update_runtime(|state| {
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        let Some(previous) = state.interaction.undo_stack.pop() else {
            return Ok(());
        };
        let current = state
            .interaction
            .text_input_values
            .get(&id)
            .cloned()
            .unwrap_or_default();
        state.interaction.redo_stack.push(TextSnapshot {
            id: id.clone(),
            text: current,
            caret: state.interaction.caret_index,
        });
        restore_snapshot(&mut state.interaction, &previous);
        Ok(())
    })
}

/// Replays an edit that Ctrl+Z took back.
fn redo_focused_text() -> Result<()> {
    update_runtime(|state| {
        let Some(id) = state.interaction.focused_text_input.clone() else {
            return Ok(());
        };
        let Some(next) = state.interaction.redo_stack.pop() else {
            return Ok(());
        };
        let current = state
            .interaction
            .text_input_values
            .get(&id)
            .cloned()
            .unwrap_or_default();
        state.interaction.undo_stack.push(TextSnapshot {
            id: id.clone(),
            text: current,
            caret: state.interaction.caret_index,
        });
        restore_snapshot(&mut state.interaction, &next);
        Ok(())
    })
}

/// Puts a remembered value and caret back into the focused field.
fn restore_snapshot(interaction: &mut InteractionState, snapshot: &TextSnapshot) {
    interaction
        .text_input_values
        .insert(snapshot.id.clone(), snapshot.text.clone());
    let count = snapshot.text.chars().count();
    // The remembered caret is clamped, not trusted: the field may have been
    // shortened by an edit that happened after the snapshot was taken.
    interaction.caret_index = snapshot.caret.min(count);
    interaction.clear_selection();
    interaction.caret_visible = true;
    interaction.typing_run = false;
}

unsafe fn read_clipboard_text() -> Option<String> {
    let window = UI
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| state.window)
        .unwrap_or(0);
    OpenClipboard(HWND(window as *mut _)).ok()?;
    let text = (|| {
        let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
        if handle.is_invalid() {
            return None;
        }
        // The clipboard owns this memory until it is closed, so the handle is
        // locked and read but never freed here.
        let pointer = GlobalLock(HGLOBAL(handle.0)) as *const u16;
        if pointer.is_null() {
            return None;
        }
        let mut length = 0usize;
        while *pointer.add(length) != 0 {
            length += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length));
        let _ = GlobalUnlock(HGLOBAL(handle.0));
        Some(text)
    })();
    let _ = CloseClipboard();
    text
}

/// Replaces the clipboard with `text`.
///
/// The buffer is handed to the clipboard, which owns it from then on; the
/// handle is only freed here when the handover fails.
unsafe fn write_clipboard_text(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    let window = UI
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| state.window)
        .unwrap_or(0);
    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    OpenClipboard(HWND(window as *mut _)).context("failed to open the clipboard")?;
    let result = (|| -> Result<()> {
        EmptyClipboard()?;
        let handle = GlobalAlloc(GMEM_MOVEABLE, utf16.len() * std::mem::size_of::<u16>())?;
        let pointer = GlobalLock(handle);
        if pointer.is_null() {
            let _ = GlobalFree(handle);
            bail!("failed to lock the clipboard buffer");
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), pointer.cast::<u16>(), utf16.len());
        let _ = GlobalUnlock(handle);
        if SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(handle.0)).is_err() {
            let _ = GlobalFree(handle);
            bail!("failed to place text on the clipboard");
        }
        Ok(())
    })();
    let _ = CloseClipboard();
    result
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
        WindowAction::CloseConfirm => match confirm_close() {
            Ok(true) => {
                if !install::busy() {
                    let _ = DestroyWindow(window);
                }
            }
            Ok(false) => {}
            Err(error) => show_runtime_error(&error),
        },
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
        WindowAction::LaunchApp => {
            if let Err(error) = launch_installed_app() {
                show_runtime_error(&error);
            }
        }
        WindowAction::OpenLink(target) => {
            if let Err(error) = open_link(&target) {
                show_runtime_error(&error);
            }
        }
        WindowAction::PickDirectory { id } => match pick_directory() {
            Ok(Some(directory)) => {
                if let Err(error) = set_text_input_value(window, id, directory) {
                    show_runtime_error(&error);
                }
            }
            Ok(None) => {}
            Err(error) => show_runtime_error(&error),
        },
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

fn language_menu_is_open() -> bool {
    UI.get()
        .and_then(|state| state.lock().ok())
        .is_some_and(|state| state.language_menu_open)
}

/// Moves the highlight, confirms a choice, or dismisses the open language menu.
unsafe fn handle_language_menu_key(window: HWND, key: u32) {
    // Virtual-key codes, spelled here so the message handler stays free of
    // another namespace-wide import.
    const VK_RETURN: u32 = 0x0D;
    const VK_ESCAPE: u32 = 0x1B;
    const VK_UP: u32 = 0x26;
    const VK_DOWN: u32 = 0x28;

    match key {
        VK_ESCAPE => {
            let _ = set_language_menu_open(window, false);
        }
        VK_UP | VK_DOWN => {
            let _ = move_language_highlight(window, key == VK_DOWN);
        }
        VK_RETURN => {
            let locale = highlighted_language();
            if let Some(locale) = locale {
                let _ = select_language(window, locale);
            }
        }
        _ => {}
    }
}

fn highlighted_language() -> Option<String> {
    let state = UI.get()?.lock().ok()?;
    let index = state.interaction.highlighted_option?;
    state.ui.language_options.get(index).cloned()
}

/// Walks the highlight one row, wrapping at both ends.
unsafe fn move_language_highlight(window: HWND, forward: bool) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    let count = state.ui.language_options.len();
    if count == 0 {
        return Ok(());
    }
    let current = state.interaction.highlighted_option.unwrap_or(0);
    let next = if forward {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    };
    state.interaction.highlighted_option = Some(next);
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
    // Opening starts on the language in use, so the first arrow key moves one
    // step from what the user is reading.
    state.interaction.highlighted_option = open.then(|| {
        state
            .ui
            .language_options
            .iter()
            .position(|option| *option == state.locale)
            .unwrap_or(0)
    });
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
    state.interaction.highlighted_option = None;
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

unsafe fn set_text_input_value(window: HWND, id: String, value: String) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    state.interaction.text_input_values.insert(id, value);
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

/// Asks the localized close question a `close_confirm` control declares.
unsafe fn confirm_close() -> Result<bool> {
    let (question, title) = UI
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| {
            (
                state.ui.close_confirm_message.clone(),
                state.ui.product_name.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                "Exit the installer?".to_string(),
                "nano-installer".to_string(),
            )
        });
    let message = HSTRING::from(question);
    let title = HSTRING::from(title);
    let answer = unsafe { MessageBoxW(None, &message, &title, MB_YESNO | MB_ICONQUESTION) };
    Ok(answer == IDYES)
}

/// Opens a resolved URL in the user's default browser.
fn open_link(target: &str) -> Result<()> {
    let operation = w!("open");
    let file = HSTRING::from(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            operation,
            &file,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW reports values above 32 on success; anything at or below is
    // an error code rather than a handle.
    if result.0 as usize <= 32 {
        bail!("failed to open {target}");
    }
    Ok(())
}

/// Shows the shell folder picker and returns the chosen directory.
fn pick_directory() -> Result<Option<String>> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let _guard = ComGuard;
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .context("failed to create the folder picker")?;
        let options = dialog.GetOptions().unwrap_or_default();
        dialog
            .SetOptions(options | FOS_PICKFOLDERS)
            .context("failed to configure the folder picker")?;
        // A cancelled dialog is a normal outcome, not an error.
        if dialog.Show(None).is_err() {
            return Ok(None);
        }
        let item: IShellItem = dialog
            .GetResult()
            .context("the folder picker returned no folder")?;
        let raw: PWSTR = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .context("the chosen folder has no filesystem path")?;
        let path = raw
            .to_string()
            .context("the chosen folder path is not Unicode");
        CoTaskMemFree(Some(raw.as_ptr().cast()));
        Ok(Some(path?))
    }
}

/// Starts the application an install deployed, from its own directory.
fn launch_installed_app() -> Result<()> {
    let app = {
        let state = UI
            .get()
            .context("native UI state is missing")?
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        state
            .installed_app
            .clone()
            .context("no installed application is recorded")?
    };
    let directory = app
        .parent()
        .context("installed application has no parent")?;
    std::process::Command::new(&app)
        .current_dir(directory)
        .spawn()
        .with_context(|| format!("failed to launch {}", app.display()))?;
    Ok(())
}

/// Switches the wizard to `index` and repaints.
pub(crate) fn show_page(index: usize) -> Result<()> {
    update_runtime(|state| {
        state.interaction.page_index = index;
        Ok(())
    })
}

/// Publishes task progress so the progress pages can draw it.
pub(crate) fn report_progress(percent: u8, status_key: &str) -> Result<()> {
    update_runtime(|state| {
        state.interaction.begin_step(percent, status_key);
        Ok(())
    })
}

/// Publishes task progress without changing the published status text.
pub(crate) fn publish_progress(percent: u8) -> Result<()> {
    update_runtime(|state| {
        state.interaction.progress = Some(percent.min(100));
        Ok(())
    })
}

/// Publishes literal status text written by a project script.
///
/// The text is already localized by the script itself, so it is shown as
/// written instead of being looked up in the locale table.
pub(crate) fn publish_status_text(text: &str) -> Result<()> {
    update_runtime(|state| {
        state.interaction.status_text = Some(text.to_string());
        Ok(())
    })
}

/// Publishes a locale key a project script chose for the running step.
pub(crate) fn publish_status_key(key: &str) -> Result<()> {
    update_runtime(|state| {
        state.interaction.status_key = Some(key.to_string());
        state.interaction.status_text = None;
        Ok(())
    })
}

/// Records the deployed application so the finish page can launch it.
pub(crate) fn record_installed_app(app: PathBuf) -> Result<()> {
    update_runtime(|state| {
        state.installed_app = Some(app.clone());
        Ok(())
    })
}

/// The page count of the mode the runtime is currently in.
pub(crate) fn page_count() -> usize {
    let Some(runtime) = UI.get() else {
        return 0;
    };
    let Ok(state) = runtime.lock() else {
        return 0;
    };
    let Ok(config) = serde_json::from_slice::<serde_json::Value>(
        state
            .files
            .get("installer_config.json")
            .map(Vec::as_slice)
            .unwrap_or_default(),
    ) else {
        return 0;
    };
    runtime_page_count(&config, state.mode)
}

/// Applies `change` to the runtime state, rebuilds the layout, and repaints.
///
/// Worker threads call this; painting itself stays on the UI thread because the
/// repaint is requested through a posted message.
fn update_runtime(change: impl FnOnce(&mut RuntimeState) -> Result<()>) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let window = {
        let mut state = runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        change(&mut state)?;
        rebuild_runtime_ui(&mut state)?;
        state.window
    };
    if window != 0 {
        unsafe {
            let _ = PostMessageW(HWND(window as *mut _), WM_APP_REFRESH, None, None);
        }
    }
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

/// A run of glyphs measured but not yet placed on a wrapped line: colour, link
/// target, encoded text, and advance width.
type MeasuredRun = (COLORREF, Option<String>, Vec<u16>, i32);

/// One run positioned on screen. Drawing and hit testing both consume this, so
/// a click always lands on the run the user sees.
struct PositionedRun {
    color: COLORREF,
    text: Vec<u16>,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    link: Option<String>,
}

/// Lays out the runs of a text layer into screen positions.
///
/// A layer that wraps breaks at the layer width, one character at a time, which
/// is what mixed CJK and Latin text needs. A single-line layer keeps its runs on
/// one row and aligns the row inside the layer.
unsafe fn layout_text_runs(destination: HDC, layer: &TextLayer) -> Vec<PositionedRun> {
    if layer.wrap {
        layout_wrapped_runs(destination, layer)
    } else {
        layout_single_line_runs(destination, layer)
    }
}

unsafe fn layout_single_line_runs(destination: HDC, layer: &TextLayer) -> Vec<PositionedRun> {
    let mut measured = Vec::with_capacity(layer.runs.len());
    let mut total_width = 0;
    for run in &layer.runs {
        let text: Vec<u16> = run.text.encode_utf16().collect();
        let width = measure_text_width(destination, &text);
        total_width += width;
        measured.push((run, text, width));
    }
    let mut left = match layer.alignment {
        TextAlignment::Left => layer.left,
        TextAlignment::Center => layer.left + (layer.width - total_width) / 2,
        TextAlignment::Right => layer.left + layer.width - total_width,
    };
    let mut positioned = Vec::with_capacity(measured.len());
    for (run, text, width) in measured {
        if left >= layer.left + layer.width {
            break;
        }
        let right = (left + width).min(layer.left + layer.width);
        positioned.push(PositionedRun {
            color: run.color,
            text,
            left,
            top: layer.top,
            right,
            bottom: layer.top + layer.height,
            link: run.link.clone(),
        });
        left += width;
    }
    positioned
}

unsafe fn layout_wrapped_runs(destination: HDC, layer: &TextLayer) -> Vec<PositionedRun> {
    let mut lines: Vec<Vec<MeasuredRun>> = vec![Vec::new()];
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
            lines[line_index].push((run.color, run.link.clone(), text, width));
        }
    }
    let line_height = (layer.font_size * 3 / 2).max(layer.font_size);
    let total_height = line_height * i32::try_from(lines.len()).unwrap_or(1);
    let mut top = layer.top + (layer.height - total_height).max(0) / 2;
    let mut positioned = Vec::new();
    for (line, line_width) in lines.into_iter().zip(line_widths) {
        if top >= layer.top + layer.height {
            break;
        }
        let mut left = match layer.alignment {
            TextAlignment::Left => layer.left,
            TextAlignment::Center => layer.left + (layer.width - line_width) / 2,
            TextAlignment::Right => layer.left + layer.width - line_width,
        };
        for (color, link, text, width) in line {
            positioned.push(PositionedRun {
                color,
                text,
                left,
                top,
                right: left + width,
                bottom: (top + line_height).min(layer.top + layer.height),
                link,
            });
            left += width;
        }
        top += line_height;
    }
    positioned
}

/// Clickable spans of a text layer, in layer-rectangle coordinates.
///
/// Wrapped text lays a link out one glyph at a time, so neighbouring glyphs of
/// the same target are merged back into one region. That keeps the highlight
/// and the hit test aligned with the words the user sees.
unsafe fn text_layer_hits(layer: &TextLayer) -> Vec<TextHit> {
    if !layer.runs.iter().any(|run| run.link.is_some()) {
        return Vec::new();
    }
    let dc = GetDC(None);
    if dc.is_invalid() {
        return Vec::new();
    }
    let font = create_ui_font(layer.font_size, layer.bold);
    let hits = if font.is_invalid() {
        Vec::new()
    } else {
        let previous = SelectObject(dc, font);
        let mut hits: Vec<TextHit> = Vec::new();
        for run in layout_text_runs(dc, layer) {
            let Some(target) = run.link else {
                continue;
            };
            match hits.last_mut() {
                // Same target, same line, touching boxes: extend the region.
                Some(previous)
                    if previous.right == run.left
                        && previous.bottom == run.bottom
                        && matches!(&previous.action, WindowAction::OpenLink(previous_target) if *previous_target == target) =>
                {
                    previous.right = run.right;
                }
                _ => hits.push(TextHit {
                    action: WindowAction::OpenLink(target),
                    left: run.left,
                    top: run.top,
                    right: run.right,
                    bottom: run.bottom,
                }),
            }
        }
        let _ = SelectObject(dc, previous);
        hits
    };
    if !font.is_invalid() {
        let _ = DeleteObject(font);
    }
    let _ = ReleaseDC(None, dc);
    hits
}

unsafe fn draw_single_line_text(destination: HDC, layer: &TextLayer) {
    for run in layout_text_runs(destination, layer) {
        draw_positioned_run(destination, layer, &run);
    }
}

unsafe fn draw_wrapped_text(destination: HDC, layer: &TextLayer) {
    for run in layout_text_runs(destination, layer) {
        draw_positioned_run(destination, layer, &run);
    }
}

unsafe fn draw_positioned_run(destination: HDC, layer: &TextLayer, run: &PositionedRun) {
    if run.left >= layer.left + layer.width {
        return;
    }
    let _ = SetTextColor(destination, run.color);
    let mut text = run.text.clone();
    let mut bounds = RECT {
        left: run.left,
        top: run.top,
        right: layer.left + layer.width,
        bottom: run.bottom,
    };
    let _ = DrawTextW(
        destination,
        &mut text,
        &mut bounds,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
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
    let (source_x, source_y, source_width, source_height) = match layer.source {
        Some(region) => (region.left, region.top, region.width, region.height),
        None => (0, 0, layer.image.width as i32, layer.image.height as i32),
    };
    let _ = GdiAlphaBlend(
        destination,
        layer.left,
        layer.top,
        layer.width,
        layer.height,
        source,
        source_x,
        source_y,
        source_width,
        source_height,
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
        button_image, byte_index, caret_layer, disk_root, flow_axis, flow_widths,
        format_size_bytes, initial_interaction, inspect_project, installer_version_info,
        load_layout, measure_layout_text_width, pack_project, pack_project_with_progress,
        parse_bundle, parse_color, parse_image_style, parse_text_runs, pick_directory_target,
        push_action, push_border_layer, push_node_border, query_disk_free_bytes, render_flow,
        render_flow_item, render_progress_bar, resolve_asset_path, resolve_link_target,
        resolved_text_for_node, restore_snapshot, runtime_layout_path, runtime_layout_path_at,
        runtime_page_count, scale_value, selection_layers, size_attribute,
        uninstaller_version_info, validate_output_filename, word_end_after, word_range,
        word_start_before, wrap_lines, wraps, BundleIndex, DpiContext, FlowAxis, FlowItem,
        InteractionState, LayerRect, LayoutContext, LayoutOutput, PayloadFormat, RuntimeMode,
        RuntimeUi, TextAlignment, TextHit, TextInputRegion, TextSnapshot, WindowAction, COLORREF,
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
                    flex_basis: 0,
                },
                FlowItem {
                    fixed_width: None,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: 0,
                    flex_basis: 0,
                },
                FlowItem {
                    fixed_width: Some(184),
                    flex_grow: 0.0,
                    flex_shrink: 0.0,
                    min_width: 0,
                    flex_basis: 0,
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
                    flex_basis: 0,
                },
                FlowItem {
                    fixed_width: None,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: 0,
                    flex_basis: 0,
                },
                FlowItem {
                    fixed_width: Some(184),
                    flex_grow: 0.0,
                    flex_shrink: 0.0,
                    min_width: 0,
                    flex_basis: 0,
                },
            ],
            640,
            32,
        );
        assert_eq!(widths, [392, 0, 184]);
    }

    #[test]
    fn a_border_layer_paints_a_ring_and_leaves_the_center_empty() {
        let mut layers = Vec::new();
        push_border_layer(
            &mut layers,
            LayerRect {
                left: 10,
                top: 20,
                width: 12,
                height: 12,
            },
            "#FF00C4B2",
            2,
            0,
        )
        .expect("border draws");
        assert_eq!(layers.len(), 1);
        let layer = &layers[0];
        assert_eq!(
            (layer.left, layer.top, layer.width, layer.height),
            (10, 20, 12, 12)
        );
        let alpha_at = |x: usize, y: usize| layer.image.pixels[(y * 12 + x) * 4 + 3];
        // Edge pixels are painted, the interior is untouched.
        assert_eq!(alpha_at(0, 0), 255);
        assert_eq!(alpha_at(11, 11), 255);
        assert_eq!(alpha_at(6, 6), 0);
        assert_eq!(alpha_at(2, 5), 0);

        // A control with no `border-color` must not add any layer at all.
        let document = roxmltree::Document::parse(r#"<Page><Box width="10" height="10" /></Page>"#)
            .expect("layout parses");
        let box_node = document
            .descendants()
            .find(|node| node.has_tag_name("Box"))
            .expect("box exists");
        let files = HashMap::new();
        let translations = HashMap::new();
        let config = serde_json::json!({});
        let interaction = InteractionState::default();
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &translations,
            interaction: &interaction,
            language_menu_open: false,
        };
        let mut layers = Vec::new();
        push_node_border(
            box_node,
            LayerRect {
                left: 0,
                top: 0,
                width: 10,
                height: 10,
            },
            &context,
            &mut layers,
        )
        .expect("no border is not an error");
        assert!(layers.is_empty());
    }

    #[test]
    fn action_attributes_map_to_window_actions() {
        let document = roxmltree::Document::parse(
            r#"<Page>
                 <Button id="close" action="close_confirm" />
                 <Button id="terms" action="open_url:terms_of_service" />
                 <Button id="direct" action="open_url:https://example.test/help" />
                 <Button id="missing" action="open_url:not_configured" />
                 <Button id="browse" action="pick_directory" target="editDir" />
               </Page>"#,
        )
        .expect("layout parses");
        let config = serde_json::json!({
            "links": { "terms_of_service": "https://example.test/terms" }
        });
        let files = HashMap::new();
        let translations = HashMap::new();
        let interaction = InteractionState::default();
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &translations,
            interaction: &interaction,
            language_menu_open: false,
        };
        let action_for = |id: &str| {
            let node = document
                .descendants()
                .find(|node| node.attribute("id") == Some(id))
                .expect("control exists");
            let mut actions = Vec::new();
            push_action(
                node,
                LayerRect {
                    left: 0,
                    top: 0,
                    width: 10,
                    height: 10,
                },
                &mut actions,
                &context,
            );
            actions.into_iter().next().map(|region| region.action)
        };
        assert!(matches!(
            action_for("close"),
            Some(WindowAction::CloseConfirm)
        ));
        assert!(matches!(
            action_for("terms"),
            Some(WindowAction::OpenLink(ref target)) if target == "https://example.test/terms"
        ));
        assert!(matches!(
            action_for("direct"),
            Some(WindowAction::OpenLink(ref target)) if target == "https://example.test/help"
        ));
        // A key that is not configured stays inert instead of opening nothing.
        assert!(action_for("missing").is_none());
        assert!(matches!(
            action_for("browse"),
            Some(WindowAction::PickDirectory { ref id }) if id == "editDir"
        ));
    }

    #[test]
    fn agreement_links_resolve_through_the_project_links_table() {
        let config: serde_json::Value = serde_json::from_str(
            r#"{"links":{"terms_of_service":"https://example.test/terms","privacy_policy":"https://example.test/privacy"}}"#,
        )
        .expect("config parses");
        let files = HashMap::new();
        let translations = HashMap::new();
        let interaction = InteractionState::default();
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &translations,
            interaction: &interaction,
            language_menu_open: false,
        };
        // The example locales name their links `agreement` and `policy`, so both
        // spellings must reach the configured URLs.
        assert_eq!(
            resolve_link_target("agreement", &context).as_deref(),
            Some("https://example.test/terms")
        );
        assert_eq!(
            resolve_link_target("policy", &context).as_deref(),
            Some("https://example.test/privacy")
        );
        assert_eq!(
            resolve_link_target("terms_of_service", &context).as_deref(),
            Some("https://example.test/terms")
        );
        // Absolute targets bypass the table, and unknown names stay inert.
        assert_eq!(
            resolve_link_target("https://example.test/help", &context).as_deref(),
            Some("https://example.test/help")
        );
        assert!(resolve_link_target("missing", &context).is_none());
    }

    #[test]
    fn pick_directory_falls_back_to_the_layout_text_input() {
        let interaction = InteractionState::default();
        let document = roxmltree::Document::parse(
            r#"<Page>
                 <TextInput id="readOnly" readonly="true" />
                 <TextInput id="editDir" />
                 <Button id="browse" action="pick_directory" />
               </Page>"#,
        )
        .expect("layout parses");
        let button = document
            .descendants()
            .find(|node| node.has_tag_name("Button"))
            .expect("button exists");
        assert_eq!(
            pick_directory_target(button, &interaction).as_deref(),
            Some("editDir")
        );
    }

    #[test]
    fn link_runs_carry_their_target_and_plain_runs_do_not() {
        let runs = parse_text_runs(
            "agree to [Terms](agreement) now",
            COLORREF(0),
            Some(COLORREF(1)),
        );
        assert_eq!(runs.len(), 3);
        assert!(runs[0].link.is_none());
        assert_eq!(runs[1].link.as_deref(), Some("agreement"));
        assert!(runs[2].link.is_none());
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
        // In order: the page fill, the page background image, the page outline,
        // then each control as the document lists it.
        assert_eq!(ui.layers.len(), 13);
        let page_fill = &ui.layers[0];
        assert_eq!(
            (
                page_fill.left,
                page_fill.top,
                page_fill.width,
                page_fill.height
            ),
            (0, 0, 1440, 900)
        );
        let select_border = &ui.layers[3];
        assert_eq!(
            (
                select_border.left,
                select_border.top,
                select_border.width,
                select_border.height
            ),
            (1056, 32, 192, 52)
        );
        let minimize = &ui.layers[5];
        assert_eq!(
            (minimize.left, minimize.top, minimize.width, minimize.height),
            (1294, 38, 40, 40)
        );
        assert_eq!(minimize.alpha, 160);
        let close = &ui.layers[6];
        assert_eq!(
            (close.left, close.top, close.width, close.height),
            (1362, 38, 40, 40)
        );
        assert_eq!(close.alpha, 160);
        let checkbox = &ui.layers[10];
        assert_eq!(
            (checkbox.left, checkbox.top, checkbox.width, checkbox.height),
            (80, 796, 32, 32)
        );
        let badge = &ui.layers[11];
        assert_eq!(
            (badge.left, badge.top, badge.width, badge.height),
            (1312, 788, 48, 48)
        );
        let arrow = &ui.layers[12];
        assert_eq!(
            (arrow.left, arrow.top, arrow.width, arrow.height),
            (1324, 800, 24, 24)
        );
        assert_eq!(ui.texts.len(), 5);
        assert!(ui.texts[3].wrap);
        // The agreement control grows and measures to its own text, so it keeps
        // the whole sentence on one line while the spacer absorbs what is left.
        assert!(ui.texts[3].width > 450);
        assert!(matches!(ui.texts[4].alignment, TextAlignment::Right));
        // The agreement label renders two links, and each one registers a click
        // region carrying the URL its `links` key resolves to.
        let link_hits: Vec<&TextHit> = ui
            .text_hits
            .iter()
            .filter(|hit| matches!(hit.action, WindowAction::OpenLink(_)))
            .collect();
        assert_eq!(link_hits.len(), 2);
        let targets: Vec<&str> = link_hits
            .iter()
            .filter_map(|hit| match &hit.action {
                WindowAction::OpenLink(target) => Some(target.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            targets,
            [
                "https://www.taptap.cn/doc/terms/",
                "https://www.taptap.cn/doc/privacy-policy/"
            ]
        );
        // A hit is the link run's own box, inside the label and smaller than it.
        let label = &ui.texts[3];
        for hit in &link_hits {
            assert!(hit.left >= label.left && hit.right <= label.left + label.width);
            assert!(hit.top >= label.top && hit.bottom <= label.top + label.height);
            assert!(hit.right - hit.left < label.width);
        }
        // The two links do not overlap and read left to right.
        assert!(link_hits[0].right <= link_hits[1].left);
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

    /// Loads one wizard page of the TapTap example at 96 DPI.
    fn taptap_page(
        mode: RuntimeMode,
        page_index: usize,
        interaction: &InteractionState,
    ) -> anyhow::Result<RuntimeUi> {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap");
        let bundle = pack_project_with_progress(&project, None, false, |_| {})?;
        let files = parse_bundle(&bundle)?;
        let mut interaction = interaction.clone();
        interaction.page_index = page_index;
        load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            false,
            &interaction,
            mode,
        )
    }

    #[test]
    fn editable_text_fields_are_recorded_and_readonly_ones_are_not() -> anyhow::Result<()> {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({ "resources": { "locales_dir": "locales" } });
        let document = roxmltree::Document::parse(
            r##"<Page width="400" height="300">
                  <TextInput id="path" value="AppsDir" />
                  <TextInput id="hint" value="read only" readonly="true" />
                </Page>"##,
        )?;
        let page = document
            .descendants()
            .find(|node| node.has_tag_name("Page"))
            .context("page missing")?;
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &HashMap::new(),
            interaction: &InteractionState::default(),
            language_menu_open: false,
        };
        let mut output = LayoutOutput::default();
        for node in page.children().filter(|node| node.is_element()) {
            render_flow_item(
                node,
                LayerRect {
                    left: 10,
                    top: 10,
                    width: 200,
                    height: 20,
                },
                &context,
                &mut output,
            )?;
        }

        // Only the writable field takes typed text, so only it is recorded.
        assert_eq!(output.text_inputs.len(), 1);
        assert_eq!(output.text_inputs[0].id, "path");
        assert_eq!(output.text_inputs[0].text, "AppsDir");
        Ok(())
    }

    /// An interaction state with a populated, focused text field.
    fn focused_field(id: &str, text: &str, caret: usize) -> InteractionState {
        let mut values = HashMap::new();
        values.insert(id.to_string(), text.to_string());
        InteractionState {
            focused_text_input: Some(id.to_string()),
            text_input_values: values,
            caret_index: caret,
            ..Default::default()
        }
    }

    #[test]
    fn a_selection_is_ordered_from_whichever_end_the_caret_is_at() {
        let mut interaction = InteractionState {
            caret_index: 2,
            selection_anchor: Some(6),
            ..Default::default()
        };
        assert_eq!(interaction.selection_range(), Some((2, 6)));

        // Dragging the other way selects the same range, not an inverted one.
        interaction.caret_index = 6;
        interaction.selection_anchor = Some(2);
        assert_eq!(interaction.selection_range(), Some((2, 6)));

        // A caret sitting on its anchor is not a selection at all.
        interaction.selection_anchor = Some(6);
        assert_eq!(interaction.selection_range(), None);

        interaction.clear_selection();
        assert_eq!(interaction.selection_range(), None);
    }

    #[test]
    fn typing_coalesces_into_one_undo_step() {
        let mut interaction = focused_field("path", "C:", 2);

        // The first keystroke of a run is remembered.
        interaction.remember_for_undo(true);
        assert_eq!(interaction.undo_stack.len(), 1);
        assert_eq!(interaction.undo_stack[0].text, "C:");
        assert_eq!(interaction.undo_stack[0].caret, 2);

        // Later keystrokes of the same run share that snapshot, so Ctrl+Z comes
        // back to the value the run started from.
        interaction
            .text_input_values
            .insert("path".to_string(), "C:\\Apps".to_string());
        interaction.remember_for_undo(true);
        assert_eq!(interaction.undo_stack.len(), 1);

        // A different kind of edit starts its own step.
        interaction.remember_for_undo(false);
        assert_eq!(interaction.undo_stack.len(), 2);
    }

    #[test]
    fn undo_remembers_the_caret_that_belongs_to_the_value() {
        let mut interaction = focused_field("path", "C:\\Apps", 7);
        interaction.remember_for_undo(false);
        let snapshot = interaction.undo_stack.remove(0);

        // The caret of a later value does not survive the restore, and a caret
        // past the end of the restored text is pulled back into it.
        interaction.caret_index = 99;
        interaction.selection_anchor = Some(0);
        restore_snapshot(&mut interaction, &snapshot);
        assert_eq!(
            interaction
                .text_input_values
                .get("path")
                .map(String::as_str),
            Some("C:\\Apps")
        );
        assert_eq!(interaction.caret_index, 7);
        assert_eq!(interaction.selection_range(), None);

        // A snapshot whose caret is past a shortened value comes back in range.
        let shortened = TextSnapshot {
            id: "path".to_string(),
            text: "C:".to_string(),
            caret: 7,
        };
        restore_snapshot(&mut interaction, &shortened);
        assert_eq!(interaction.caret_index, 2);
    }

    #[test]
    fn removing_a_selection_keeps_the_text_around_it() {
        let mut interaction = focused_field("path", "C:\\Apps\\Game", 0);
        // Nothing is selected yet, so there is nothing to remove.
        assert!(!interaction.remove_selection());
        assert_eq!(
            interaction
                .text_input_values
                .get("path")
                .map(String::as_str),
            Some("C:\\Apps\\Game")
        );

        // Selecting the "Apps" in `C:\\Apps\\Game` leaves the separators.
        interaction.selection_anchor = Some(3);
        interaction.caret_index = 7;
        assert!(interaction.remove_selection());
        assert_eq!(
            interaction
                .text_input_values
                .get("path")
                .map(String::as_str),
            Some("C:\\\\Game")
        );
        assert_eq!(interaction.caret_index, 3);
        assert_eq!(interaction.selection_range(), None);
    }

    #[test]
    fn a_selection_band_covers_the_characters_it_selects() {
        let field = TextInputRegion {
            id: "path".to_string(),
            text: "C:\\Apps".to_string(),
            color: COLORREF(0x00FF_FFFF),
            font_size: 12,
            bold: false,
            left: 100,
            top: 50,
            width: 200,
            height: 20,
        };
        // A collapsed range draws nothing, so a plain click leaves no band.
        assert!(selection_layers(&field, 3, 3).is_empty());
        assert!(selection_layers(&field, 5, 2).is_empty());

        let bands = selection_layers(&field, 0, 7);
        assert_eq!(bands.len(), 1);
        let band = &bands[0];
        assert_eq!(band.left, 100);
        // The band stops inside the field even when the range runs past its end
        // and spans fewer pixels than the text does.
        assert!(band.left + band.width <= 300);
        assert!(band.width > 0);
        assert!(band.top > field.top);
        assert!(band.top + band.height <= field.top + field.height);
    }

    #[test]
    fn a_double_click_selects_the_word_under_the_pointer() {
        let words = "C:\\Program Files\\TapTap";
        // A word: letters and digits run together, so the directory name is one
        // selection rather than one character.
        assert_eq!(word_range(words, 3), (3, 10));
        assert_eq!(word_range(words, 12), (11, 16));
        // A path separator stands alone, which is what a user clicking it means.
        assert_eq!(word_range(words, 2), (2, 3));
        assert_eq!(word_range(words, 10), (10, 11));
        // Whitespace groups with whitespace, so the gap between names is one run.
        assert_eq!(word_range("a  b", 1), (1, 3));
        // An underscore is part of the word it joins.
        assert_eq!(word_range("install_dir", 4), (0, 11));
        // Presses outside the text have nothing to select.
        assert_eq!(word_range(words, words.chars().count()), (23, 23));
        assert_eq!(word_range("", 0), (0, 0));
    }

    #[test]
    fn word_keys_stop_at_the_boundaries_they_delete() {
        let text = "C:\\Program Files\\TapTap";
        // Ctrl+Backspace from the end takes the last name and stops at the
        // separator, leaving the separator in place.
        assert_eq!(word_start_before(text, 23), 17);
        // A second one takes the separator on its own.
        assert_eq!(word_start_before(text, 17), 16);
        // Repeated presses walk the whole path down to its start and stop there.
        assert_eq!(word_start_before(text, 16), 11);
        assert_eq!(word_start_before("abc", 0), 0);
        // Trailing whitespace is skipped before the word itself is taken.
        assert_eq!(word_start_before("one two ", 8), 4);

        // Ctrl+Delete removes the word after the caret, keeping the separator.
        assert_eq!(word_end_after(text, 3), 10);
        // A caret sitting in front of a name first takes the gap, then the name.
        assert_eq!(word_end_after(text, 10), 16);
        // Whitespace alone is consumed when the caret sits in front of it.
        assert_eq!(word_end_after("one   two", 3), 9);
        assert_eq!(word_end_after("abc", 3), 3);
    }

    #[test]
    fn a_caret_sits_after_the_characters_before_it() {
        let field = TextInputRegion {
            id: "path".to_string(),
            text: "C:\\Apps".to_string(),
            color: COLORREF(0x00FF_FFFF),
            font_size: 12,
            bold: false,
            left: 100,
            top: 50,
            width: 200,
            height: 20,
        };
        let at_start = caret_layer(&field, 0);
        let at_end = caret_layer(&field, 7);

        assert_eq!(at_start.left, 100);
        assert!(at_end.left > at_start.left);
        // The caret stays inside the field even when the index runs past the
        // end of the text.
        assert!(caret_layer(&field, 99).left <= 300);
        assert_eq!(at_start.height, 14);
    }

    #[test]
    fn byte_index_walks_characters_not_bytes() {
        // Editing a multi-byte field has to split on character boundaries or the
        // inserted text would corrupt the surrounding characters.
        let text = "C:\\安装";
        assert_eq!(byte_index(text, 0), 0);
        assert_eq!(byte_index(text, 3), 3);
        assert_eq!(byte_index(text, 4), 3 + "安".len());
        assert_eq!(byte_index(text, 99), text.len());
    }

    #[test]
    fn a_wrapping_row_starts_a_new_line_when_the_next_item_does_not_fit() {
        let item = |width: i32| FlowItem {
            fixed_width: Some(width),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            min_width: 0,
            flex_basis: 0,
        };
        // Three 40px cards and an 8px gap fit two per 100px line.
        let items = [item(40), item(40), item(40), item(40)];
        assert_eq!(wrap_lines(&items, 100, 8), vec![vec![0, 1], vec![2, 3]]);

        // Everything fits, so nothing wraps.
        assert_eq!(wrap_lines(&items, 400, 8), vec![vec![0, 1, 2, 3]]);

        // An item wider than the line gets a line of its own instead of being
        // dropped, so an oversized control is still drawn.
        let wide = [item(400), item(10)];
        assert_eq!(wrap_lines(&wide, 100, 8), vec![vec![0], vec![1]]);

        // An empty container has no lines at all.
        assert!(wrap_lines(&[], 100, 8).is_empty());
    }

    #[test]
    fn flex_wrap_is_opt_in_per_container() {
        let document = roxmltree::Document::parse(
            r##"<Page>
                  <HBox id="wrapped" flex-wrap="true" />
                  <HBox id="plain" />
                  <HBox id="spelled" flex-wrap="wrap" />
                  <HBox id="off" flex-wrap="false" />
                </Page>"##,
        )
        .unwrap();
        let named = |id: &str| {
            document
                .descendants()
                .find(|node| node.attribute("id") == Some(id))
                .expect("node missing")
        };
        assert!(wraps(named("wrapped")));
        assert!(wraps(named("spelled")));
        assert!(!wraps(named("plain")));
        assert!(!wraps(named("off")));
    }

    #[test]
    fn flow_items_honour_align_self_basis_and_anchored_edges() -> anyhow::Result<()> {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({ "resources": { "locales_dir": "locales" } });
        let document = roxmltree::Document::parse(
            r##"<Page width="400" height="300">
                  <HBox position="absolute" left="0" top="0" width="400" height="100"
                        align-items="center" item-spacing="10">
                    <Button id="tall" text="a" action="minimize" width="80" height="60" />
                    <Button id="short" text="b" action="close" width="80" height="20"
                            align-self="start" />
                    <Spacer flex-grow="1" />
                  </HBox>
                  <Button id="pinned" text="c" action="finish" right="10" bottom="20"
                          width="60" height="30" />
                </Page>"##,
        )?;
        let page = document
            .descendants()
            .find(|node| node.has_tag_name("Page"))
            .context("page missing")?;
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &HashMap::new(),
            interaction: &InteractionState::default(),
            language_menu_open: false,
        };
        let mut output = LayoutOutput::default();
        for node in page.children().filter(|node| node.is_element()) {
            let rect = LayerRect {
                left: 0,
                top: 0,
                width: 400,
                height: 100,
            };
            match flow_axis(node) {
                Some(axis) => render_flow(node, rect, axis, &context, &mut output)?,
                None => {
                    let rect = LayerRect {
                        height: 300,
                        ..rect
                    };
                    // `render_flow_item` applies the anchoring rules itself.
                    let left = super::anchored_left(node, rect.left, 60, rect.width, &context);
                    let top = super::anchored_top(node, rect.top, 30, rect.height, &context);
                    let placed = LayerRect {
                        left,
                        top,
                        width: 60,
                        height: 30,
                    };
                    render_flow_item(node, placed, &context, &mut output)?;
                }
            }
        }

        // `align-items="center"` centres the 60px button in the 100px row,
        // while `align-self="start"` pins the second one to the top edge.
        let centred = output
            .actions
            .iter()
            .find(|region| region.bottom - region.top == 60)
            .context("centred button missing")?;
        assert_eq!(centred.top, 20);
        let to_start = output
            .actions
            .iter()
            .find(|region| region.bottom - region.top == 20)
            .context("start-aligned button missing")?;
        assert_eq!(to_start.top, 0);
        // `right` and `bottom` measure from the far edges of the page.
        let pinned = output
            .actions
            .iter()
            .find(|region| region.right - region.left == 60)
            .context("pinned button missing")?;
        assert_eq!((pinned.left, pinned.top), (330, 250));
        Ok(())
    }

    #[test]
    fn vbox_stacks_children_vertically_with_padding_and_margins() -> anyhow::Result<()> {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({
            "resources": { "locales_dir": "locales" }
        });
        let document = roxmltree::Document::parse(
            r##"<Page width="400" height="300">
                  <VBox position="absolute" left="0" top="0" width="100%" height="300"
                        padding="10 20" align-items="center">
                    <Spacer height="60" />
                    <Label text="hello" font-size="14" color="#FFFFFFFF" />
                    <Button id="ok" action="install" text="ok" width="120" height="30"
                            margin-top="16" />
                  </VBox>
                </Page>"##,
        )?;
        let page = document
            .descendants()
            .find(|node| node.has_tag_name("Page"))
            .context("page missing")?;
        let vbox = page
            .descendants()
            .find(|node| node.has_tag_name("VBox"))
            .context("vbox missing")?;
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &HashMap::new(),
            interaction: &InteractionState::default(),
            language_menu_open: false,
        };
        let mut output = LayoutOutput::default();
        render_flow(
            vbox,
            LayerRect {
                left: 0,
                top: 0,
                width: 400,
                height: 300,
            },
            FlowAxis::Vertical,
            &context,
            &mut output,
        )?;

        // Padding insets the content box: 20 left/right, 10 top/bottom. The
        // label follows the 60px spacer, keeping only its own line height.
        let label = output.texts.first().context("label text missing")?;
        assert_eq!(label.top, 70);
        assert_eq!(label.height, 20);
        // `align-items="center"` centres the cross axis, which a VBox measures
        // horizontally, so the label sits in the middle of the content box.
        assert_eq!(label.left, 20 + (360 - label.width) / 2);
        // The button is centred the same way, and `margin-top` pushes it below
        // the label instead of overlapping it. A button without artwork still
        // registers its hit region.
        let button = output
            .actions
            .first()
            .context("button action region missing")?;
        assert_eq!(button.left, 20 + (360 - 120) / 2);
        assert_eq!(button.right - button.left, 120);
        assert_eq!(button.top, 10 + 60 + label.height + 16);
        assert_eq!(button.bottom - button.top, 30);
        Ok(())
    }

    #[test]
    fn control_padding_insets_what_the_control_draws() -> anyhow::Result<()> {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        // The pause button pairs a fixed width with `padding="25 0 0 0"`, so its
        // label must start 25px below the top of its own flow slot.
        let document = roxmltree::Document::parse(
            r##"<Page width="400" height="200">
                  <HBox position="absolute" left="0" top="0" width="400" height="40">
                    <Label text="hello" padding="25 0 0 0" color="#FFFFFFFF" />
                  </HBox>
                </Page>"##,
        )?;
        let node = document
            .descendants()
            .find(|node| node.has_tag_name("Label"))
            .context("label missing")?;
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &HashMap::new(),
            interaction: &InteractionState::default(),
            language_menu_open: false,
        };
        let mut output = LayoutOutput::default();
        render_flow_item(
            node,
            LayerRect {
                left: 10,
                top: 0,
                width: 100,
                height: 40,
            },
            &context,
            &mut output,
        )?;
        let label = output.texts.first().context("label text missing")?;
        assert_eq!((label.left, label.top), (10, 25));
        // The padding also caps the height the text may occupy.
        assert_eq!(label.height, 15);
        Ok(())
    }

    #[test]
    fn percentage_and_pixel_extents_scale_with_the_layout() -> anyhow::Result<()> {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let document = roxmltree::Document::parse(
            r#"<Page width="200" height="100">
                 <Box position="absolute" left="0" top="0" width="50%" height="20" />
               </Page>"#,
        )?;
        let page = document
            .descendants()
            .find(|node| node.has_tag_name("Page"))
            .context("page missing")?;
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &HashMap::new(),
            interaction: &InteractionState::default(),
            language_menu_open: false,
        };
        let node = page
            .descendants()
            .find(|node| node.has_tag_name("Box"))
            .context("box missing")?;
        // A percentage resolves against the parent extent; the pixel value is
        // unaffected by the percentage handling.
        assert_eq!(size_attribute(node, "width", 200, &context), Some(100));
        assert_eq!(size_attribute(node, "height", 100, &context), Some(20));
        // At 2x the absolute extent doubles while the percentage stays relative.
        let scaled = LayoutContext {
            dpi: DpiContext {
                scale: 2.0,
                use_2x: true,
            },
            ..context
        };
        assert_eq!(size_attribute(node, "width", 400, &scaled), Some(200));
        assert_eq!(size_attribute(node, "height", 200, &scaled), Some(40));
        Ok(())
    }

    #[test]
    fn progress_bar_clips_its_sprite_to_the_completed_share() -> anyhow::Result<()> {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "assets/bar.png".to_string(),
            include_bytes!("../../../examples/TapTap/assets/bar_installing.png").to_vec(),
        );
        let config = serde_json::json!({});
        let document = roxmltree::Document::parse(
            r##"<Page width="600" height="100">
                  <ProgressBar id="bar" position="absolute" left="10" top="20"
                               width="600" height="10" progress="0"
                               background="#FF4C5868"
                               bar-image="assets/bar.png" />
                </Page>"##,
        )?;
        let page = document
            .descendants()
            .find(|node| node.has_tag_name("Page"))
            .context("page missing")?;
        let node = page
            .descendants()
            .find(|node| node.has_tag_name("ProgressBar"))
            .context("progress bar missing")?;
        let interaction = InteractionState {
            progress: Some(25),
            ..Default::default()
        };
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &HashMap::new(),
            interaction: &interaction,
            language_menu_open: false,
        };
        let mut output = LayoutOutput::default();
        // The track paints first, then the clipped sprite on top of it.
        let rect = LayerRect {
            left: 10,
            top: 20,
            width: 600,
            height: 10,
        };
        render_progress_bar(node, rect, &context, &mut output)?;
        assert_eq!(output.layers.len(), 2);
        let track = &output.layers[0];
        assert_eq!(
            (track.left, track.top, track.width, track.height),
            (10, 20, 600, 10)
        );
        let fill = &output.layers[1];
        // A quarter of the control, drawn from the leading quarter of a 60px sprite.
        assert_eq!((fill.left, fill.width), (10, 150));
        let source = fill.source.context("fill must clip the sprite")?;
        assert_eq!((source.left, source.width), (0, 15));
        Ok(())
    }

    #[test]
    fn out_of_range_pages_fall_back_to_the_first_layout() -> anyhow::Result<()> {
        let config: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../examples/TapTap/installer_config.json"
        ))?;
        assert_eq!(
            runtime_layout_path_at(&config, RuntimeMode::Installer, 1)?,
            "layouts/installingpage.xml"
        );
        assert_eq!(
            runtime_layout_path_at(&config, RuntimeMode::Installer, 2)?,
            "layouts/finishpage.xml"
        );
        // Past the end of the list, the first page is the safe fallback.
        assert_eq!(
            runtime_layout_path_at(&config, RuntimeMode::Installer, 9)?,
            "layouts/configpage.xml"
        );
        assert_eq!(
            runtime_layout_path_at(&config, RuntimeMode::Uninstaller, 1)?,
            "layouts/uninstallingpage.xml"
        );
        assert_eq!(
            runtime_layout_path_at(&config, RuntimeMode::Uninstaller, 7)?,
            "layouts/uninstallpage.xml"
        );
        assert_eq!(runtime_page_count(&config, RuntimeMode::Installer), 3);
        assert_eq!(runtime_page_count(&config, RuntimeMode::Uninstaller), 3);
        Ok(())
    }

    #[test]
    fn progress_pages_render_every_control_they_declare() -> anyhow::Result<()> {
        let interaction = {
            let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap");
            let bundle = pack_project_with_progress(&project, None, false, |_| {})?;
            initial_interaction(&parse_bundle(&bundle)?, RuntimeMode::Installer)?
        };
        let ui = taptap_page(RuntimeMode::Installer, 1, &interaction)?;
        // The installing page is fully absolute: a background, two window
        // buttons, the progress fill and its track.
        assert!(
            ui.layers
                .iter()
                .any(|layer| layer.top == 326 && layer.height == 10),
            "progress track is missing: {:?}",
            ui.layers
                .iter()
                .map(|layer| (layer.left, layer.top, layer.width, layer.height))
                .collect::<Vec<_>>()
        );
        assert!(
            ui.texts.iter().any(|text| text.top == 359),
            "progress label is missing"
        );

        // The uninstalling page uses a VBox, which has to stack its children.
        let uninstall = taptap_page(RuntimeMode::Uninstaller, 1, &interaction)?;
        assert!(
            uninstall.layers.iter().any(|layer| layer.height == 6),
            "uninstall progress bar is missing: {:?}",
            uninstall
                .layers
                .iter()
                .map(|layer| (layer.left, layer.top, layer.width, layer.height))
                .collect::<Vec<_>>()
        );
        assert!(
            uninstall.texts.iter().any(|text| text.top > 260),
            "uninstall progress label is missing"
        );
        Ok(())
    }

    #[test]
    fn status_source_replaces_placeholder_text_with_the_published_step() -> anyhow::Result<()> {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let document = roxmltree::Document::parse(
            r#"<Page width="200" height="100">
                 <Label text="@installing_text" value-source="status" />
               </Page>"#,
        )?;
        let page = document
            .descendants()
            .find(|node| node.has_tag_name("Page"))
            .context("page missing")?;
        let node = page
            .descendants()
            .find(|node| node.has_tag_name("Label"))
            .context("label missing")?;
        let mut translations = HashMap::new();
        translations.insert("installing_text".to_string(), "正在安装 0%".to_string());
        translations.insert(
            "status.extracting".to_string(),
            "正在解压文件...".to_string(),
        );
        let interaction = InteractionState {
            status_key: Some("status.extracting".to_string()),
            ..Default::default()
        };
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &translations,
            interaction: &interaction,
            language_menu_open: false,
        };
        let (text, _) = resolved_text_for_node(node, &context).context("label text missing")?;
        assert_eq!(text, "正在解压文件...");

        // Without a published step the authored placeholder stays in place.
        let idle = LayoutContext {
            interaction: &InteractionState::default(),
            ..context
        };
        let (text, _) = resolved_text_for_node(node, &idle).context("label text missing")?;
        assert_eq!(text, "正在安装 0%");
        Ok(())
    }

    #[test]
    fn a_script_step_text_wins_over_the_locale_key() -> anyhow::Result<()> {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let document = roxmltree::Document::parse(
            r#"<Page width="200" height="100">
                 <Label text="@installing_text" value-source="status" />
               </Page>"#,
        )?;
        let page = document
            .descendants()
            .find(|node| node.has_tag_name("Page"))
            .context("page missing")?;
        let node = page
            .descendants()
            .find(|node| node.has_tag_name("Label"))
            .context("label missing")?;
        let mut translations = HashMap::new();
        translations.insert("installing_text".to_string(), "正在安装 0%".to_string());
        translations.insert(
            "status.extracting".to_string(),
            "正在解压文件...".to_string(),
        );
        // A script that publishes literal text keeps it on screen even while a
        // locale key from an earlier step is still recorded.
        let interaction = InteractionState {
            status_key: Some("status.extracting".to_string()),
            status_text: Some("Extracting archive files".to_string()),
            ..Default::default()
        };
        let context = LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files: &files,
            config: &config,
            locale: "zh-CN",
            translations: &translations,
            interaction: &interaction,
            language_menu_open: false,
        };
        let (text, _) = resolved_text_for_node(node, &context).context("label text missing")?;
        assert_eq!(text, "Extracting archive files");
        Ok(())
    }

    #[test]
    fn a_built_in_step_clears_a_script_step_text() {
        // The built-in steps name themselves from the locale table, so text a
        // project script published must not survive into them.
        let interaction = InteractionState {
            status_text: Some("Extracting archive files".to_string()),
            status_key: Some("status.checking_processes".to_string()),
            progress: Some(2),
            ..Default::default()
        };
        let mut interaction = interaction;
        interaction.begin_step(150, "status.deploying");
        assert_eq!(interaction.status_text, None);
        assert_eq!(interaction.status_key.as_deref(), Some("status.deploying"));
        // Progress is clamped, so a script cannot push the bar past its end.
        assert_eq!(interaction.progress, Some(100));
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
