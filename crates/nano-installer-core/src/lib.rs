#[cfg(not(target_arch = "x86_64"))]
compile_error!("nano-installer-native-x64 must be built for x86_64");

mod config;
mod contrast;
mod delta;
mod dependency;
mod icon;
mod install;
mod install_log;
mod manifest;
mod msi;
mod net;
mod script;
mod service;
mod shell;
mod version;

pub use delta::UpdateSummary;

use anyhow::{bail, Context, Result};
use std::borrow::Cow;
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
    BeginPaint, BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, CreateRectRgn, CreateRoundRectRgn, DeleteDC, DeleteObject,
    DrawTextW, EndPaint, GdiAlphaBlend, GetDC, GetDeviceCaps, GetMonitorInfoW,
    GetTextExtentPoint32W, IntersectClipRect, InvalidateRect, MonitorFromWindow, ReleaseDC,
    RestoreDC, SaveDC, ScreenToClient, SelectObject, SetBkMode, SetTextColor, SetWindowRgn,
    UpdateWindow, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS,
    DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_BOLD, FW_NORMAL, HDC, HFONT, LOGPIXELSX,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
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
    TRACKMOUSEEVENT, VIRTUAL_KEY, VK_A, VK_BACK, VK_C, VK_CONTROL, VK_DELETE, VK_END, VK_ESCAPE,
    VK_HOME, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_V, VK_X, VK_Y, VK_Z,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, IShellItem, ShellExecuteW, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos,
    GetMessageW, KillTimer, LoadCursorW, LoadIconW, MessageBoxW, PostMessageW, PostQuitMessage,
    RegisterClassExW, SendMessageW, SetProcessDPIAware, SetTimer, SetWindowPos, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HTCAPTION, HTCLIENT, HWND_TOP, ICON_BIG, ICON_SMALL,
    IDC_ARROW, IDC_HAND, MB_ICONERROR, MB_OK, MSG, SWP_NOACTIVATE, SWP_NOZORDER, SW_MINIMIZE,
    SW_SHOW, SW_SHOWNORMAL, WM_CHAR, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND,
    WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCLBUTTONDOWN,
    WM_PAINT, WM_SETCURSOR, WM_SETICON, WM_SETTINGCHANGE, WM_SYSCOLORCHANGE, WM_THEMECHANGED,
    WM_TIMER, WNDCLASSEXW, WS_EX_APPWINDOW, WS_POPUP,
};
use windows::Win32::UI::WindowsAndMessaging::{SetCursor, IDC_IBEAM};

const BUNDLE_MAGIC: &[u8; 8] = b"NATVRS01";
const FOOTER_MAGIC: &[u8; 8] = b"NATVEND1";
/// Version 2 carries a SHA-256 beside every entry, so a setup whose bundle was
/// damaged after the build refuses the damaged entry instead of deploying it.
/// Version 1 entries have no digest and are refused as unsupported.
const BUNDLE_VERSION: u16 = 2;
/// Length of the digest an index entry records for its bytes.
const BUNDLE_DIGEST: usize = 32;
const BASE_DPI: u32 = 96;
const DEFAULT_DPI_THRESHOLD: u32 = 144;
const WM_MOUSELEAVE: u32 = 0x02A3;
/// Timer that blinks the caret of the focused text field.
const CARET_TIMER: usize = 1;
/// Caret blink period in milliseconds, the usual Windows cadence.
const CARET_BLINK_MS: u32 = 530;
/// Posted by a worker thread when it changed runtime progress or page state.
const WM_APP_REFRESH: u32 = 0x8001;
/// Layout a project gets for its questions when it declares none of its own.
const DEFAULT_DIALOG_LAYOUT: &str = "layouts/msgBox.xml";
/// Dimming colour laid over the page while a dialog waits for an answer.
const DIALOG_SCRIM: &str = "66000000";
static UI: OnceLock<Mutex<RuntimeState>> = OnceLock::new();
static TEMP_EXE_ID: AtomicU64 = AtomicU64::new(0);

/// Answer the script that is waiting on the dialog now on screen, if any.
///
/// A script asks on the worker thread and waits here; the click arrives on the
/// window's thread, so the answer travels back through this channel.
static DIALOG_REPLY: Mutex<Option<std::sync::mpsc::Sender<bool>>> = Mutex::new(None);

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
    /// View the layer is cut to, for content inside a scrollable container.
    /// `None` draws the layer wherever it was placed.
    clip: Option<LayerRect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayerRect {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

impl LayerRect {
    fn right(self) -> i32 {
        self.left + self.width
    }

    fn bottom(self) -> i32 {
        self.top + self.height
    }

    /// Whether a point falls inside, with the trailing edges excluded the way a
    /// hit region is.
    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right() && y >= self.top && y < self.bottom()
    }

    /// The part two rectangles share, or `None` when they do not meet.
    fn intersect(self, other: LayerRect) -> Option<LayerRect> {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > left && bottom > top).then(|| LayerRect {
            left,
            top,
            width: right - left,
            height: bottom - top,
        })
    }
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
    /// Controls the keyboard can reach, in the order the page lays them out.
    focus_regions: Vec<FocusRegion>,
    /// Containers the page lets the user scroll, with what the window needs to
    /// move them.
    scroll_views: Vec<ScrollView>,
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
    /// Menu a `Select` has open, if any: the rows it draws, and the row the
    /// keyboard and the current choice point at.
    menu: Option<MenuUi>,
    /// Localized question a `close_confirm` button asks before closing.
    close_confirm_message: String,
    /// Localized label for a dialog's confirm button.
    dialog_accept_label: String,
    /// Localized label for a dialog's secondary button.
    dialog_dismiss_label: String,
    /// Localized label for the answer that agrees, when a script asks.
    dialog_yes_label: String,
    /// Localized label for the answer that refuses, when a script asks.
    dialog_no_label: String,
    /// Controls of the dialog that is currently open. They are kept apart from
    /// the page's own so a modal dialog is the only thing a click can reach.
    dialog: Option<DialogUi>,
    /// Title for runtime dialogs, taken from `project.name`.
    product_name: String,
}

/// The option list a `Select` draws while its menu is open.
///
/// Rows are values rather than nodes: keyboard navigation walks this list,
/// and confirming a row needs the value it stands for without parsing the
/// layout again.
/// One scrollable container the page drew.
///
/// The layout keeps this rather than the window working it out again: only the
/// layout knows how far a container's children reach past its own edge, and the
/// offset the user left behind has to be clamped against exactly that.
struct ScrollView {
    /// Id the container declares, which is the name its position is kept under.
    id: String,
    /// Axis the container stacks its children along, and therefore scrolls.
    axis: FlowAxis,
    /// The container's own rectangle, which is where a wheel reaches it.
    area: LayerRect,
    /// Rectangle the children are laid out in, inside its padding, which is what
    /// they are cut to.
    viewport: LayerRect,
    /// Extent the children reach along that axis.
    content: i32,
    /// How far the container is scrolled, after clamping.
    offset: i32,
}

impl ScrollView {
    /// How far this container can be scrolled before its end comes into view.
    fn max_offset(&self) -> i32 {
        (self.content - self.axis.main(self.viewport)).max(0)
    }
}

struct MenuUi {
    /// Id of the control that opened the menu.
    select: String,
    /// Whether the rows are locales, which is what confirming one does.
    language: bool,
    /// Value of every row, in the order the layout declares them.
    values: Vec<String>,
    /// Row the current choice marks, when the menu has one.
    chosen: Option<usize>,
}

struct RuntimeState {
    files: HashMap<String, Vec<u8>>,
    /// The project's page hook, when it ships one: the runtime asks it where
    /// the wizard goes every time the user leaves a page forward.
    page_hook: Option<String>,
    /// Scale the current page was measured at. Replaced when the window moves
    /// to a display with a different scaling factor.
    dpi: DpiContext,
    /// The scaling a project asked for, kept so a new context can be derived.
    dpi_settings: DpiSettings,
    /// The colours the current frame was painted with, which is how a change to
    /// the machine's contrast setting is told from a setting that stayed put.
    contrast: Option<contrast::Palette>,
    locale: String,
    /// `Select` whose menu is open, if any. A page shows one menu at a time,
    /// and the id names the control that drew it.
    open_select: Option<String>,
    interaction: InteractionState,
    mode: RuntimeMode,
    ui: RuntimeUi,
    /// The window the runtime paints into, so worker threads can report back.
    window: isize,
    /// Size the window was last placed at. A page change that brings a different
    /// layout re-centres the wizard, while a size the user already has is left
    /// alone so a moved window stays where it was put.
    window_size: (i32, i32),
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
    /// Value the user picked from a `Select` or a radio group, kept by the id
    /// of the control that owns the choice: a select's own id, or the group a
    /// radio button belongs to.
    choices: HashMap<String, String>,
    /// How far each scrollable container is scrolled, under the id it declares.
    scroll_offsets: HashMap<String, i32>,
    panel_visibility: HashMap<String, bool>,
    hovered_control: Option<String>,
    pressed_control: Option<String>,
    text_input_values: HashMap<String, String>,
    /// Wizard page currently rendered, as an index into the mode's page list.
    page_index: usize,
    /// The pages the user walked through to reach the one on screen, oldest
    /// first. A page hook may send the wizard past a page the project declares,
    /// so going back follows the way the user actually came rather than the
    /// declared order, which would show a page the hook decided against.
    page_history: Vec<usize>,
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
    /// Control the keyboard is on, if any. It is the control Tab walked to, or
    /// the one the last click landed on, and it is what a keystroke acts on: the
    /// ring around it is the only thing that says where the next key goes. A
    /// text field is a control like any other, so focusing one also puts the
    /// caret in it.
    focused_control: Option<String>,
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
    /// Question the runtime is showing over the page, if any.
    dialog: Option<DialogState>,
}

/// A question the runtime draws itself instead of handing to Windows.
///
/// The system message box the runtime used before could not take a product's
/// skin, was not owned by the installer window, and so could end up behind
/// another application while the wizard kept waiting for an answer. A dialog is
/// now built from a project layout and drawn inside the installer window, which
/// is what makes it look like part of the product and share its z-order.
#[derive(Clone)]
struct DialogState {
    /// What the question means, which decides what confirming it does.
    kind: DialogKind,
    /// Text for the layout's message label.
    message: String,
    /// Label for the layout's confirm button.
    accept_label: String,
    /// Label for the layout's secondary button.
    dismiss_label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DialogKind {
    /// Closing the window was confirmed; confirming again closes it.
    CloseConfirm,
    /// A notice; confirming dismisses it and leaves the wizard as it was.
    Notice,
    /// A question a project script asked. Both answers go back to the script
    /// waiting for one, and neither closes the wizard.
    Question,
}

impl DialogState {
    /// Whether this dialog offers a second answer besides confirming.
    ///
    /// A question a script asked does, because its two answers are the whole
    /// point of it. A notice does not, and that is how one layout serves all
    /// three.
    fn offers_dismiss(&self) -> bool {
        matches!(self.kind, DialogKind::CloseConfirm | DialogKind::Question)
    }
}

/// The controls a dialog adds on top of the page.
///
/// Its regions are held separately from the page's own, because a click while a
/// dialog is open must not reach the buttons the dialog is covering.
#[derive(Default)]
struct DialogUi {
    actions: Vec<ActionRegion>,
    text_hits: Vec<TextHit>,
    hover_regions: Vec<HoverRegion>,
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
    /// View the text is cut to, for content inside a scrollable container.
    clip: Option<LayerRect>,
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

/// A control the keyboard can land on, in the order the page lays it out.
///
/// The order is the page's own: the regions are recorded while the layout walks
/// the tree, so Tab follows what the reader sees on a page of any shape, and a
/// control the page hid is not in the list at all.
struct FocusRegion {
    id: String,
    /// What activating this control does. A text field has none: a keystroke
    /// there is typing, which the caret already covers.
    action: Option<WindowAction>,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Clone)]
enum WindowAction {
    Close,
    CloseConfirm,
    /// Stops the task that is running, or leaves the wizard when none is.
    Cancel,
    Minimize,
    /// Opens or closes the menu of a `Select`.
    ToggleSelectMenu {
        id: String,
    },
    SelectLanguage(String),
    /// Records the value the user picked from a `Select` or a radio group.
    ChooseOption {
        id: String,
        value: String,
    },
    /// Moves a scrollable container by one of its own pages, from a click on the
    /// track of its scrollbar.
    ScrollPage {
        id: String,
        forward: bool,
    },
    /// Moves to the next page the project declares.
    NextPage,
    /// Moves back to the previous page the project declares.
    PreviousPage,
    Install,
    Uninstall,
    LaunchApp,
    OpenLink(String),
    PickDirectory {
        id: String,
    },
    ToggleCheckbox {
        id: String,
        checked: bool,
    },
    SetPanelVisibility {
        id: String,
        visible: bool,
    },
    /// Confirms the open dialog; a close question then closes the window.
    DialogOk,
    /// Dismisses the open dialog and returns to the page under it.
    DialogCancel,
}

#[derive(Clone, Copy)]
struct DpiContext {
    scale: f32,
    use_2x: bool,
}

impl DpiContext {
    /// The layout scale and image variant a given display DPI resolves to.
    ///
    /// A project that turns awareness off keeps the 96 DPI baseline, so the
    /// shell does the scaling instead.
    fn for_dpi(dpi: u32, settings: DpiSettings) -> Self {
        let dpi = if settings.aware { dpi } else { BASE_DPI };
        Self {
            scale: dpi as f32 / BASE_DPI as f32,
            use_2x: settings.aware && dpi >= settings.threshold,
        }
    }
}

/// What a project asked for when it comes to display scaling.
///
/// Kept beside the resolved [`DpiContext`] so the runtime can work out a new
/// context when Windows moves the window to a differently scaled display.
#[derive(Clone, Copy)]
struct DpiSettings {
    aware: bool,
    threshold: u32,
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
    /// Controls the keyboard can reach, in the order they are laid out.
    focus_regions: Vec<FocusRegion>,
    /// Containers this render found scrollable, with the view each one shows.
    scroll_views: Vec<ScrollView>,
    /// Menu a `Select` drew open while this layout was rendered.
    menu: Option<MenuUi>,
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
    /// Part of the field a scrollable container leaves in view. The field keeps
    /// the rectangle it was laid out with, because its caret and its selection
    /// are measured against that; the visible part is what a click has to land
    /// inside, so a field scrolled out of sight is not editable where it is
    /// hidden.
    clip: Option<LayerRect>,
}

struct LayoutContext<'a> {
    dpi: DpiContext,
    files: &'a HashMap<String, Vec<u8>>,
    config: &'a serde_json::Value,
    locale: &'a str,
    translations: &'a HashMap<String, String>,
    interaction: &'a InteractionState,
    /// What each text field on the page accepts, read once for this render.
    fields: HashMap<String, FieldState>,
    /// `Select` whose menu this render draws open, if any.
    open_select: Option<&'a str>,
    /// What the machine asks for while the user has high contrast on, read
    /// before the page was laid out. `None` paints the page as it was written.
    contrast: Option<contrast::Palette>,
}

impl LayoutContext<'_> {
    /// The colour to paint `role` with.
    ///
    /// What the layout declares, unless the user has high contrast on, where the
    /// scheme names the colour instead. The declared colour is borrowed as it
    /// was written while no scheme is asked for, so a page that says nothing
    /// about the setting is rendered exactly as it always was.
    fn colour<'declared>(
        &self,
        role: contrast::Role,
        declared: &'declared str,
    ) -> Cow<'declared, str> {
        match &self.contrast {
            Some(palette) => Cow::Owned(palette.colour(role)),
            None => Cow::Borrowed(declared),
        }
    }
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
    show_notice(&format!("Native runtime failed:\n{error:#}"));
}

/// Tells the user something, in the product's own skin once a window exists.
///
/// A failure before the window exists — a broken bundle, a display that cannot
/// be opened — still has to reach the user, so that case keeps a system box, now
/// owned by the window when there is one. Owning it is what stops the box from
/// sinking behind the installer that opened it.
pub(crate) fn show_notice(message: &str) {
    // Nothing is on screen to own a box, and a modal box would wait for a click
    // that an unattended run never sends.
    if silent_mode() {
        return;
    }
    if let Ok(window) = runtime_window() {
        if !window.0.is_null() && open_notice_dialog(message.to_string()).is_ok() {
            return;
        }
    }
    let message = HSTRING::from(message.to_string());
    let title = UI
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| HSTRING::from(state.ui.product_name.clone()))
        .unwrap_or_else(|| HSTRING::from("nano-installer"));
    unsafe {
        let _ = MessageBoxW(
            runtime_window().unwrap_or_default(),
            &message,
            &title,
            MB_OK | MB_ICONERROR,
        );
    }
}

/// The window the runtime paints into, for anything that needs to reach it.
pub(crate) fn runtime_window() -> Result<HWND> {
    let state = UI
        .get()
        .context("native UI state is missing")?
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    Ok(HWND(state.window as *mut _))
}

/// The argument that runs a task with no window at all.
///
/// A project opts in through `advanced.silent_mode_support` and
/// `advanced.uninstall_mode_support`; a project that did not is refused rather
/// than installed unattended. This is also the entry point an end-to-end test
/// drives, because it is the only one that needs no desktop session.
pub const SILENT_FLAG: &str = "--silent";

/// Whether this process is running a task with no window.
///
/// A silent run must not open anything: a modal box would wait for a click that
/// never comes, which hangs an unattended install and any test driving one.
/// Everything that would draw therefore asks here first.
static SILENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn silent_mode() -> bool {
    SILENT.load(std::sync::atomic::Ordering::Acquire)
}

fn mark_silent(flag: bool) {
    SILENT.store(flag, std::sync::atomic::Ordering::Release);
}

pub fn run_installer_runtime() -> Result<()> {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if arguments.first().map(std::ffi::OsString::as_os_str)
        == Some(std::ffi::OsStr::new(SILENT_FLAG))
    {
        mark_silent(true);
        return install::run_silent_install(&arguments[1..]);
    }
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
    if arguments.first().map(std::ffi::OsString::as_os_str)
        == Some(std::ffi::OsStr::new(SILENT_FLAG))
    {
        mark_silent(true);
        return install::run_silent_uninstall(&arguments[1..]);
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
    /// The payload archive of an earlier release, when this build is an update
    /// package rather than a full setup.
    ///
    /// An update package carries only the files whose bytes differ from that
    /// release and installs over exactly it, so it is a fraction of the full
    /// setup. The project's own payload stays the source of the new version.
    pub delta_from: Option<PathBuf>,
    /// Where to write the installer package that wraps the finished setup, when
    /// this build is asked for one.
    ///
    /// The package is written after the project's own command has had the setup
    /// -- signing is what that command is for -- so the image it carries is the
    /// one a machine will install.
    pub msi: Option<PathBuf>,
}

impl BuildRequest {
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
            output: None,
            stub_directory: None,
            delta_from: None,
            msi: None,
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
    /// What the setup carries, which for an update package is the archive this
    /// build wrote rather than the project's own payload.
    pub payload_size: u64,
    /// The format of the payload this setup carries: the project's own payload,
    /// or the update archive a `delta_from` build writes.
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
    /// Wrapping the finished setup in the package an administrator deploys.
    Packaging,
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
    /// What building an update package came to, and `None` for a full setup.
    pub update: Option<UpdateSummary>,
    /// What wrapping the setup in an installer package came to, and `None` for
    /// a build that was not asked for one.
    pub msi: Option<MsiSummary>,
}

/// What an installer package around a finished setup came to.
#[derive(Clone, Debug)]
pub struct MsiSummary {
    pub output_path: PathBuf,
    pub output_size: u64,
    /// The code that names this version of the product to Windows Installer.
    pub product_code: String,
    /// The code that names the product across versions, which is what makes a
    /// newer package upgrade an older one.
    pub upgrade_code: String,
    /// The directory the package installs into unless a caller names another.
    pub install_directory: String,
    /// Whether the package installs for the machine or for one user.
    pub per_machine: bool,
}

pub fn inspect_project(project: impl AsRef<Path>) -> Result<ProjectSummary> {
    let project = project.as_ref();
    let config = read_project_config(project)?;
    config::audit(&config)?;
    let output_path = default_output_for_project(project, &config)?;
    let payload_path = project.join(
        config["resources"]["payload_file"]
            .as_str()
            .context("resources.payload_file is required")?,
    );
    let declared_format = payload_format(&payload_path)?;
    // One stub unpacks every payload a setup carries, so a component in another
    // format is a build error rather than an install that fails on the machine the
    // product is being installed on.
    for item in config["components"]["items"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let (Some(id), Some(payload)) = (item["id"].as_str(), item["payload"].as_str()) else {
            continue;
        };
        let format = payload_format(&project.join(payload))?;
        if format != declared_format {
            bail!(
                "component {id} carries a {format:?} payload while resources.payload_file is {declared_format:?}; a setup unpacks every payload with one runtime"
            );
        }
    }
    // A dependency's program is part of the setup, so a project that names one
    // it does not ship is a build error rather than an install that fails on
    // the machine that needed it.
    for item in config["dependencies"]["items"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let (Some(id), Some(payload)) = (item["id"].as_str(), item["payload"].as_str()) else {
            continue;
        };
        let path = project.join(payload);
        if !path.is_file() {
            bail!("dependency {id} ships no program at {}", path.display());
        }
    }
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
        payload_format: declared_format,
        require_admin: manifest.require_admin,
        dpi_aware: manifest.dpi_aware,
        warnings,
    })
}

/// A directory a build writes into and removes when it ends.
///
/// An update package is assembled outside the project -- the archive is this
/// build's own work rather than a file the author wrote -- so it lives in the
/// temporary directory and goes away with the build that made it.
struct ScratchDirectory(PathBuf);

impl ScratchDirectory {
    fn create() -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "nano-installer-build-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
    // An update package carries only what changed since the release it is built
    // from, so the archive the setup embeds is one this build writes rather than
    // the project's own payload.
    let mut update: Option<UpdateSummary> = None;
    let mut update_plan: Option<Vec<u8>> = None;
    let mut update_archive: Option<PathBuf> = None;
    let mut update_scratch: Option<ScratchDirectory> = None;
    if let Some(from) = request.delta_from.as_deref() {
        if config["components"]["items"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
        {
            bail!(
                "{}: an update package is built from a project whose content is one payload, and \
                 this project cuts its content into components; ship the full setup instead",
                from.display()
            );
        }
        let scratch = ScratchDirectory::create()?;
        let archive = scratch.path().join("update.zip");
        let (plan, built) = delta::build_update(
            from,
            &summary.payload_path,
            config["project"]["version"].as_str().unwrap_or_default(),
            request.stub_directory.as_deref(),
            scratch.path(),
            &archive,
        )?;
        progress(BuildEvent {
            stage: BuildStage::Packing,
            message: format!(
                "Update package from {}: {} file(s) kept on the machine, {} carried ({})",
                from.display(),
                built.kept,
                built.changed,
                format_build_size(built.archive_size)
            ),
        });
        update = Some(built);
        update_plan = Some(plan.to_json_bytes()?);
        update_archive = Some(archive);
        update_scratch = Some(scratch);
        // What this setup carries is the update archive, and the runtime it
        // embeds is the one that unpacks that archive.
        summary.payload_format = PayloadFormat::Zip;
        summary.payload_size = built.archive_size;
    }
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
    // A package stores its text in this machine's code page, which is worth
    // knowing before the build spends anything on a setup nobody can package.
    if request.msi.is_some() {
        msi::ensure_text_is_storable(
            &summary.project_name,
            config["project"]["publisher"]
                .as_str()
                .unwrap_or("nano-installer"),
            uninstaller_name,
        )?;
    }
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
    let mut extras: Vec<(String, Vec<u8>)> = vec![(embedded_uninstaller_name.clone(), uninstaller)];
    if let Some(plan) = update_plan {
        extras.push((delta::UPDATE_PLAN.to_string(), plan));
    }
    let bundle = pack_project_with_progress(
        &project,
        extras,
        update_archive.as_deref(),
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
    drop(file);
    // The setup is complete and in place, which is when a project's own command
    // gets it -- NSIS's `!finalize`. Signing appends a certificate table behind
    // the footer, which is why a signed setup still installs: the runtime
    // searches the end of the file for the footer rather than reading it there.
    if let Some(command) = finalize_command(&config, "installer") {
        let outcome = run_finalize("finalize.installer", command, output, |message| {
            progress(BuildEvent {
                stage: BuildStage::WritingResources,
                message,
            })
        });
        if outcome.is_err() {
            // A setup the project's own command refused is not one to leave
            // lying about: a later step of the same pipeline would pick it up
            // and ship a file nobody signed.
            let _ = std::fs::remove_file(output);
        }
        outcome?;
    }
    let output_size = std::fs::metadata(output)?.len();
    // An installer package wraps the setup the project's own command has just
    // had, so what it carries is the file a machine will install.
    let mut msi = None;
    if let Some(msi_output) = request.msi {
        for key in ["silent_mode_support", "uninstall_mode_support"] {
            if config["advanced"][key].as_bool() != Some(true) {
                bail!(
                    "an installer package drives the setup with no window, which this project does \
                     not support: set advanced.{key} to true in installer_config.json"
                );
            }
        }
        progress(BuildEvent {
            stage: BuildStage::Packaging,
            message: format!("Wrapping the setup in {}", msi_output.display()),
        });
        let wrapped = msi::write_wrapper(&msi::Wrapper {
            setup: output,
            output: &msi_output,
            product_name: &summary.project_name,
            product_version: &summary.project_version,
            manufacturer: config["project"]["publisher"]
                .as_str()
                .unwrap_or("nano-installer"),
            locale: &summary.default_locale,
            uninstaller_name,
            require_admin: summary.require_admin,
        })?;
        // The package is a file this build finished, so a project's own command
        // gets it the same way the setup does: an estate signs the package it
        // deploys, and the package is what a machine really sees. Signing an MSI
        // rewrites it, so what the build reports about the package is read from
        // disk after the command rather than from the wrapper that was written.
        if let Some(command) = finalize_command(&config, "package") {
            let outcome = run_finalize("finalize.package", command, &wrapped.output, |message| {
                progress(BuildEvent {
                    stage: BuildStage::Packaging,
                    message,
                })
            });
            if outcome.is_err() {
                // A package the project's own command refused is not one to leave
                // lying about: the next step of a pipeline would deploy a package
                // nobody signed.
                let _ = std::fs::remove_file(&wrapped.output);
            }
            outcome?;
        }
        let package_size = std::fs::metadata(&wrapped.output)?.len();
        progress(BuildEvent {
            stage: BuildStage::Packaging,
            message: format!(
                "Installer package: {} ({}, {}, product code {})",
                wrapped.output.display(),
                format_build_size(package_size),
                if wrapped.per_machine {
                    "for the machine"
                } else {
                    "for one user"
                },
                wrapped.product_code
            ),
        });
        progress(BuildEvent {
            stage: BuildStage::Packaging,
            message: format!(
                "The package installs into {} unless a caller names another directory",
                wrapped.install_directory
            ),
        });
        msi = Some(MsiSummary {
            output_path: wrapped.output,
            output_size: package_size,
            product_code: wrapped.product_code,
            upgrade_code: wrapped.upgrade_code,
            install_directory: wrapped.install_directory,
            per_machine: wrapped.per_machine,
        });
    }
    progress(BuildEvent {
        stage: BuildStage::Complete,
        message: format!("Created {}", output.display()),
    });
    Ok(BuildResult {
        summary,
        stub_path: stub,
        bundle_size: bundle.len() as u64,
        output_size,
        update,
        msi,
    })
}

/// The command a project wants run on a finished file, if it named one.
fn finalize_command<'a>(config: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    config["finalize"][key]
        .as_str()
        .map(str::trim)
        .filter(|command| !command.is_empty())
}

/// Runs a project's finalize command on a file this build has finished.
///
/// The builder's version of NSIS's `!finalize` and `!uninstfinalize`: the
/// project names a command, the builder hands it the file it has just written
/// and waits for the answer. Signing is what that is usually for, and the
/// builder signs nothing itself -- the certificate, and the pipeline that owns
/// it, stay outside. `%1` in the command stands for the file's path, and a
/// command that exits non-zero stops the build: a setup whose signature failed
/// must not be the one that ships.
fn run_finalize(
    setting: &str,
    command: &str,
    file: &Path,
    mut progress: impl FnMut(String),
) -> Result<()> {
    let command = command.replace("%1", &file.display().to_string());
    progress(format!("Running {setting}: {command}"));
    // The command goes into a script of its own rather than straight onto
    // `cmd.exe`'s command line: a command line that names one quoted path and
    // one quoted argument is the shape cmd.exe re-parses wrongly once it holds
    // more than two quote characters, and both the program and the file it is
    // handed may well live under a name with a space in it.
    let scratch = ScratchDirectory::create()?;
    let script = scratch.path().join("finalize.cmd");
    std::fs::write(&script, format!("@echo off\r\n{command}\r\n"))
        .with_context(|| format!("cannot write {setting} to {}", script.display()))?;
    let output = shell::run_captured(
        "cmd.exe",
        &[
            "/d".to_string(),
            "/c".to_string(),
            script.display().to_string(),
        ],
        &|| false,
    )
    .with_context(|| format!("{setting} could not be started: {command}"))?;
    // Whatever the command printed belongs in the build log: a signer that
    // refused the file says why there, and that reason is the whole answer.
    for line in output.stdout.lines().chain(output.stderr.lines()) {
        if !line.trim().is_empty() {
            progress(line.trim_end().to_string());
        }
    }
    if output.code != 0 {
        let code = if output.code < 0 {
            "no exit code".to_string()
        } else {
            format!("exit code {}", output.code)
        };
        bail!("{setting} failed with {code}: {command}");
    }
    Ok(())
}

fn build_uninstaller_executable(
    project: &Path,
    config: &serde_json::Value,
    stub: &Path,
    output_name: &str,
    mut progress: impl FnMut(String),
) -> Result<Vec<u8>> {
    let bundle = pack_project_with_progress(project, Vec::new(), None, false, |message| {
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
    // The uninstaller is finished here, and this is the last moment at which it
    // exists as a file of its own: whatever the project runs on it -- signing it
    // is the usual reason -- is baked into the bytes the setup embeds.
    if let Some(command) = finalize_command(config, "uninstaller") {
        run_finalize(
            "finalize.uninstaller",
            command,
            &temporary.path,
            &mut progress,
        )?;
    }
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
fn pack_project(project: &Path, extra: Option<(&str, Vec<u8>)>) -> Result<Vec<u8>> {
    let extras = extra
        .into_iter()
        .map(|(name, data)| (name.to_string(), data))
        .collect();
    pack_project_with_progress(project, extras, None, true, |_| {})
}

fn pack_project_with_progress(
    project: &Path,
    extras: Vec<(String, Vec<u8>)>,
    payload_override: Option<&Path>,
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
    // Tools are opt-in: a project that names a directory ships it whole, and a
    // project that names none bundles nothing, however its tree looks on disk.
    if let Some(directory) = config["resources"]["tools_dir"].as_str() {
        let file_count_before = files.len();
        let size_before = collected_size(&files);
        collect_directory(project, &project.join(directory), &mut files)?;
        progress(format!(
            "Collected {directory}/: {} files ({})",
            files.len() - file_count_before,
            format_build_size(collected_size(&files) - size_before)
        ));
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
        match payload_override {
            // An update package carries the archive this build wrote, under the
            // name the project gives its payload, so the runtime finds it the
            // same way in either kind of setup.
            Some(path) => {
                if !path.is_file() {
                    bail!("missing update archive: {}", path.display());
                }
                files.push((payload.to_string(), std::fs::read(path)?));
            }
            None => collect_file(project, &project.join(payload), &mut files)?,
        }
        let payload_size = files.last().map(|(_, data)| data.len()).unwrap_or_default();
        progress(format!(
            "Added payload {payload} ({}, already compressed)",
            format_build_size(payload_size as u64)
        ));
        // Every component's payload travels with the setup as well: which of them
        // an install unfolds is decided on the page.
        for item in config["components"]["items"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let (Some(id), Some(component)) = (item["id"].as_str(), item["payload"].as_str())
            else {
                continue;
            };
            collect_file(project, &project.join(component), &mut files)?;
            let size = files.last().map(|(_, data)| data.len()).unwrap_or_default();
            progress(format!(
                "Added payload {component} of component {id} ({}, already compressed)",
                format_build_size(size as u64)
            ));
        }
        // A dependency's program travels with the setup too: the machine that
        // runs it is the one that turned out not to have what the product needs.
        for item in config["dependencies"]["items"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let (Some(id), Some(dependency)) = (item["id"].as_str(), item["payload"].as_str())
            else {
                continue;
            };
            collect_file(project, &project.join(dependency), &mut files)?;
            let size = files.last().map(|(_, data)| data.len()).unwrap_or_default();
            progress(format!(
                "Added dependency {id} ({})",
                format_build_size(size as u64)
            ));
        }
    }
    for (name, data) in extras {
        progress(format!(
            "Embedded {name} ({})",
            format_build_size(data.len() as u64)
        ));
        files.push((name, data));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    progress(format!(
        "Hashing and indexing {} entries ({} content, each with its SHA-256)",
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
        // The digest sits in front of the bytes it describes, so the runtime
        // knows what a payload should hash to without reading it once more.
        bundle.extend_from_slice(&net::sha256_bytes(&data)?);
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
/// Only the offset, length, and digest are kept; contents are read from the
/// executable on demand so a large payload never lands in the process address
/// space, and every read is checked against the digest the build recorded.
struct BundleEntry {
    offset: u64,
    size: u64,
    digest: [u8; BUNDLE_DIGEST],
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
        let Some((start, size)) = read_bundle_footer(&mut file, length)? else {
            return Ok(None);
        };
        let files = parse_bundle_index(&mut file, start, size)?;
        Ok(Some(Self {
            exe: exe.to_path_buf(),
            files,
        }))
    }

    fn contains(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }

    /// A bundle with nothing in it, for a caller that never reads a file from
    /// it: the page hook decides where the wizard goes, and the primitives that
    /// would read the bundle are not registered against it.
    fn empty() -> Self {
        Self {
            exe: PathBuf::new(),
            files: HashMap::new(),
        }
    }

    fn read_file(&self, name: &str) -> Result<Vec<u8>> {
        let entry = self.entry(name)?;
        let mut file = std::fs::File::open(&self.exe)?;
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut contents = vec![0u8; entry.size as usize];
        file.read_exact(&mut contents)?;
        check_entry_digest(name, &entry.digest, &net::sha256_bytes(&contents)?)?;
        Ok(contents)
    }

    /// The index entry for `name`.
    fn entry(&self, name: &str) -> Result<&BundleEntry> {
        self.files
            .get(name)
            .with_context(|| format!("missing from native bundle: {name}"))
    }

    /// Every entry stored under `directory`, in path order.
    ///
    /// A project ships its tools as a tree rather than a list, so a script gets
    /// them the same way: it names the directory once instead of every file
    /// inside it, which would break the moment the tools change.
    fn read_directory(&self, directory: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let prefix = format!("{}/", directory.trim_end_matches(['/', '\\']));
        let mut names: Vec<&String> = self
            .files
            .keys()
            .filter(|name| name.starts_with(&prefix))
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|name| Ok((name.clone(), self.read_file(name)?)))
            .collect()
    }

    /// Streams one bundle entry to `destination`, checking its digest.
    ///
    /// A payload is far larger than the address space this runs in, so the
    /// digest is taken while the bytes go past rather than over a buffer that
    /// holds them all. A copy that does not check out is removed: the step that
    /// would have unpacked it must not be able to run against damaged bytes.
    fn copy_file_to(&self, name: &str, destination: &Path) -> Result<u64> {
        let outcome = self.stream_file_to(name, destination);
        if outcome.is_err() {
            let _ = std::fs::remove_file(destination);
        }
        outcome
    }

    fn stream_file_to(&self, name: &str, destination: &Path) -> Result<u64> {
        let entry = self.entry(name)?;
        let mut source = std::fs::File::open(&self.exe)?;
        source.seek(SeekFrom::Start(entry.offset))?;
        let mut target = std::fs::File::create(destination)
            .with_context(|| format!("failed to create {}", destination.display()))?;
        let mut hasher = net::Sha256::new()?;
        let mut buffer = vec![0u8; 1024 * 1024];
        let mut remaining = entry.size;
        while remaining > 0 {
            let chunk = remaining.min(buffer.len() as u64) as usize;
            source.read_exact(&mut buffer[..chunk])?;
            hasher.update(&buffer[..chunk])?;
            target.write_all(&buffer[..chunk])?;
            remaining -= chunk as u64;
        }
        target.flush()?;
        drop(target);
        check_entry_digest(name, &entry.digest, &hasher.finish()?)?;
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

/// How far back from the end of the file the bundle footer is looked for.
///
/// The build writes the footer last, but it is not the last thing in a setup
/// that has been signed: Authenticode appends its certificate table behind the
/// whole file, a few kilobytes of signature and timestamp. Nothing else follows
/// a build, so this window covers a signature with room to spare.
const FOOTER_SEARCH_WINDOW: u64 = 1 << 20;

/// The offset and length of the appended bundle, read from the footer that ends
/// it. Returns `None` when the file carries no footer.
fn read_bundle_footer(file: &mut std::fs::File, length: u64) -> Result<Option<(u64, u64)>> {
    let window = length.min(FOOTER_SEARCH_WINDOW);
    if window < 16 {
        return Ok(None);
    }
    let window_start = length - window;
    let mut tail = vec![0u8; window as usize];
    file.seek(SeekFrom::Start(window_start))?;
    file.read_exact(&mut tail)?;
    // Backwards, because the footer that describes this bundle is the last one
    // in the file, whatever an integrator has appended behind it.
    for magic_at in (8..=tail.len() - 8).rev() {
        if &tail[magic_at..magic_at + 8] != FOOTER_MAGIC {
            continue;
        }
        let size = u64::from_le_bytes(tail[magic_at - 8..magic_at].try_into()?);
        // checked arithmetic: a corrupt footer can carry a size that runs past
        // the start of the file.
        let start = (window_start + magic_at as u64 - 8)
            .checked_sub(size)
            .context("invalid native bundle size")?;
        return Ok(Some((start, size)));
    }
    Ok(None)
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
    let version = u16::from_le_bytes(header[8..10].try_into()?);
    if version != BUNDLE_VERSION {
        bail!(
            "this setup carries native bundle version {version}, and this runtime understands \
             version {BUNDLE_VERSION}; setup and runtime must come from the same build"
        );
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
        let mut digest = [0u8; BUNDLE_DIGEST];
        file.seek(SeekFrom::Start(cursor))?;
        file.read_exact(&mut digest)?;
        cursor += BUNDLE_DIGEST as u64;
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
                digest,
            },
        );
    }
    Ok(files)
}

/// Refuses bytes that are not the ones the build recorded for `name`.
///
/// A bundle travels inside the setup, so damage to it is damage to the product:
/// naming the entry and both digests says which file arrived wrong, which a
/// failed extraction of anonymous bytes does not.
fn check_entry_digest(
    name: &str,
    expected: &[u8; BUNDLE_DIGEST],
    actual: &[u8; BUNDLE_DIGEST],
) -> Result<()> {
    if expected == actual {
        return Ok(());
    }
    bail!(
        "{name} is damaged: this setup records {}, the bytes inside it hash to {}; \
         copy the setup again or download it once more",
        net::hex_digest(expected),
        net::hex_digest(actual)
    );
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
        let mut digest = [0u8; BUNDLE_DIGEST];
        cursor.read_exact(&mut digest)?;
        let mut contents = vec![0u8; u64::from_le_bytes(length) as usize];
        cursor.read_exact(&mut contents)?;
        let name = String::from_utf8(name)?;
        // The same check the runtime makes, so a test that reads a packed
        // bundle back also says the digests in it are the right ones.
        check_entry_digest(&name, &digest, &net::sha256_bytes(&contents)?)?;
        files.insert(name, contents);
    }
    Ok(files)
}

fn run_embedded(bundle: BundleIndex, mode: RuntimeMode) -> Result<()> {
    // Only the configuration, layout, asset, and locale entries are read; the
    // payload stays on disk and is streamed when the install action runs.
    let files = bundle.read_ui_files()?;
    // The page hook is carried for as long as the wizard lives, because a click
    // asks it for an answer rather than the task that runs later.
    let page_hook = bundle
        .contains(script::page::PAGE_SCRIPT)
        .then(|| bundle.read_file(script::page::PAGE_SCRIPT))
        .transpose()?
        .and_then(|source| String::from_utf8(source).ok());
    let (dpi_settings, dpi) = configure_dpi(&files)?;
    let locale = initial_locale(&files)?;
    let interaction = initial_interaction(&files, mode)?;
    // The machine is asked once here, and again whenever it says the setting
    // changed: a page is a picture, so it only follows a new scheme when it is
    // laid out again.
    let palette = contrast::palette();
    let ui = load_layout(&files, dpi, &locale, None, &interaction, mode, palette)?;
    let (width, height) = (ui.width, ui.height);
    UI.set(Mutex::new(RuntimeState {
        files,
        page_hook,
        dpi,
        dpi_settings,
        contrast: palette,
        locale,
        open_select: None,
        interaction,
        mode,
        ui,
        window: 0,
        window_size: (0, 0),
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

/// The values the wizard holds before the user has touched anything.
///
/// Every page a project declares is read, not just the one the wizard opens
/// on: what a control holds until the user changes it is the default its own
/// layout gives it, and both a script and the page hook may ask about a page
/// the user has not reached yet.
fn initial_interaction(
    files: &HashMap<String, Vec<u8>>,
    mode: RuntimeMode,
) -> Result<InteractionState> {
    let config: serde_json::Value = serde_json::from_slice(
        files
            .get("installer_config.json")
            .context("installer_config.json missing from native bundle")?,
    )?;
    let mut interaction = InteractionState::default();
    for index in 0..runtime_page_count(&config, mode) {
        let layout_path = runtime_layout_path_at(&config, mode, index)?;
        let Some(xml) = files.get(layout_path) else {
            continue;
        };
        let document = roxmltree::Document::parse(std::str::from_utf8(xml)?)?;
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
    }
    Ok(interaction)
}

/// Reads the scaling a project asked for and measures the current display.
///
/// The application manifest normally settled awareness before `main` ran. The
/// call below only covers a raw stub executed without a manifest: Windows then
/// starts the process unaware, and this is the only chance to ask for scaling
/// before the first window exists. It is a no-op when the manifest already
/// declared it, and it never fights the manifest because awareness can only be
/// set once per process.
fn configure_dpi(files: &HashMap<String, Vec<u8>>) -> Result<(DpiSettings, DpiContext)> {
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
    let threshold = config["ui"]["dpi_threshold"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_DPI_THRESHOLD);
    let settings = DpiSettings { aware, threshold };
    // The first window is created on the primary display, so the primary
    // display is what its first page has to be measured against. Windows sends
    // WM_DPICHANGED if the window then lands anywhere else.
    Ok((settings, DpiContext::for_dpi(system_dpi(), settings)))
}

/// A verification hook for display scaling.
///
/// Windows drives per-monitor scaling when the user moves the window onto a
/// display with a different factor, which a machine with a single display
/// cannot produce. Setting `NANO_INSTALLER_TEST_DPI_CHANGE` makes the window
/// deliver the same message to itself once it is on screen, so the real handler
/// can be exercised end to end. It does nothing unless the variable is set, and
/// it runs in-process, which is what lets the message carry a rectangle: a
/// pointer in a message from another process would not be readable.
fn test_dpi_change() -> Option<(u32, *const RECT)> {
    let dpi = std::env::var("NANO_INSTALLER_TEST_DPI_CHANGE")
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    if dpi == 0 {
        return None;
    }
    // Without a suggestion the window is re-centred, which is what Windows
    // offers for a window it moved itself.
    Some((dpi, test_dpi_change_rect().unwrap_or(std::ptr::null())))
}

/// The suggested window position for the scaling hook, if one was given.
///
/// The value is stored once and never moved, so the pointer handed to the
/// message stays valid for as long as the window can receive it.
fn test_dpi_change_rect() -> Option<*const RECT> {
    static SUGGESTION: OnceLock<Option<RECT>> = OnceLock::new();
    SUGGESTION
        .get_or_init(|| {
            let raw = std::env::var("NANO_INSTALLER_TEST_DPI_RECT").ok()?;
            let mut parts = raw.split(',').map(|part| part.trim().parse::<i32>().ok());
            let left = parts.next().flatten()?;
            let top = parts.next().flatten()?;
            Some(RECT {
                left,
                top,
                ..Default::default()
            })
        })
        .as_ref()
        .map(|rect| rect as *const RECT)
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

/// Lays one page of the wizard out into the layers the window paints.
///
/// Everything the page is measured against is handed in rather than read here:
/// the display it is measured for, the language it is read in, and the colours
/// the machine asks for. A render is therefore the same wherever it runs, which
/// is what lets a case state a scheme of its own.
fn load_layout(
    files: &HashMap<String, Vec<u8>>,
    dpi: DpiContext,
    locale: &str,
    open_select: Option<&str>,
    interaction: &InteractionState,
    mode: RuntimeMode,
    contrast: Option<contrast::Palette>,
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
        fields: collect_field_states(page, interaction, &config),
        open_select,
        contrast,
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

    render_layout_content(
        page,
        width,
        height,
        corner_radius,
        active_panel,
        &context,
        &mut output,
    )?;
    // The ring says which control the keyboard is on. A dialog owns the keyboard
    // while it is open, so it covers the page without a ring behind it.
    if interaction.dialog.is_none() {
        push_focus_ring(&mut output, page, &context)?;
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
    let dialog = render_dialog_overlay(
        files,
        &config,
        dpi,
        &translations,
        interaction,
        width,
        height,
        contrast,
        &mut output,
    )?;
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
        focus_regions: output.focus_regions,
        scroll_views: output.scroll_views,
        text_inputs: output.text_inputs,
        caret,
        caret_rect,
        selection,
        caret_drawn: interaction.caret_visible,
        menu: output.menu,
        close_confirm_message: translations
            .get("close_confirm_message")
            .cloned()
            .unwrap_or_else(|| "Exit the installer?".to_string()),
        dialog_accept_label: translations
            .get("ok")
            .cloned()
            .unwrap_or_else(|| "OK".to_string()),
        dialog_dismiss_label: translations
            .get("cancel")
            .cloned()
            .unwrap_or_else(|| "Cancel".to_string()),
        dialog_yes_label: translations
            .get("yes")
            .cloned()
            .unwrap_or_else(|| "Yes".to_string()),
        dialog_no_label: translations
            .get("no")
            .cloned()
            .unwrap_or_else(|| "No".to_string()),
        dialog,
        product_name: product_name(files),
    })
}

/// Paints one layout into `output`.
///
/// The page fill sits under its background image, so a layout can paint a base
/// colour and still lay artwork over it. Both a wizard page and a dialog use
/// this, which is what keeps a dialog looking like the page it covers.
// The parameter list mirrors the layout being rendered rather than a struct
// that would be rebuilt at every call site.
#[allow(clippy::too_many_arguments)]
fn render_layout_content(
    page: roxmltree::Node<'_, '_>,
    width: i32,
    height: i32,
    corner_radius: i32,
    active_panel: Option<(usize, LayerRect)>,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    let files = context.files;
    let dpi = context.dpi;
    if let Some(background) = page.attribute("background") {
        let background = context.colour(contrast::Role::Surface, background);
        push_solid_layer(
            &mut output.layers,
            LayerRect {
                left: 0,
                top: 0,
                width,
                height,
            },
            &background,
            corner_radius,
        )?;
    }
    if let Some(path) = page.attribute("background-image") {
        // The artwork keeps the colours it was drawn with: a picture is not a
        // colour the scheme has a name for, so what a page lays over its fill
        // stays as the project painted it.
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
        context,
        &mut output.layers,
    )?;
    for node in page.descendants().filter(|node| node.is_element()) {
        if is_hidden(node, context.interaction) || is_inside_render_container(node) {
            continue;
        }
        let (left, top) = absolute_position(node, dpi);
        let layer_width = size_attribute(node, "width", width, context).unwrap_or(0);
        let layer_height = size_attribute(node, "height", height, context).unwrap_or(0);
        let own_left = scale_value(int_attribute(node, "left").unwrap_or(0), dpi.scale);
        let own_top = scale_value(int_attribute(node, "top").unwrap_or(0), dpi.scale);
        let rect = LayerRect {
            left: anchored_left(node, left - own_left, layer_width, width, context),
            top: anchored_top(node, top - own_top, layer_height, height, context),
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
            push_action(
                node,
                rect,
                &mut output.actions,
                &mut output.focus_regions,
                context,
            );
            push_hover_region(node, rect, context, &mut output.hover_regions);
        }
        if node.has_tag_name("ProgressBar") && layer_width > 0 && layer_height > 0 {
            render_progress_bar(node, rect, context, output)?;
            continue;
        }
        // Containers own their subtree. `is_inside_render_container` already
        // skipped their children, so drawing here cannot double-place them.
        if let Some(axis) = flow_axis(node).filter(|_| layer_width > 0 && layer_height > 0) {
            render_flow(node, rect, axis, context, output)?;
            continue;
        }
        let has_layer = layer_width > 0 && layer_height > 0;
        // A checkbox draws its state image, its click region, and text placed
        // beside that image, so it is not part of the plain text pass.
        if has_layer && !node.has_tag_name("Checkbox") && !node.has_tag_name("RadioButton") {
            push_node_text(node, rect, context, output);
            push_text_input(node, rect, context, output);
        }
        if node.has_tag_name("Select") && has_layer {
            render_select(node, rect, context, output)?;
        }
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
                if let Some(style) =
                    button_image(node, context.interaction, &context.fields).map(parse_image_style)
                {
                    push_styled_layer(files, &mut output.layers, style, rect, dpi)?;
                }
                if node.attribute("normal-image").is_none() {
                    push_node_border(node, rect, context, &mut output.layers)?;
                }
            }
            "Checkbox" | "RadioButton" if has_layer => {
                render_toggle(node, rect, context, output)?;
            }
            "Box" | "Divider" if has_layer => {
                render_box_contents(node, rect, context, output)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Draws the open dialog over the page and reports the controls it added.
///
/// The dialog is a project layout like any page, which is what lets a product
/// skin its own questions instead of showing a system message box. It is centred
/// in the page, and its controls are kept apart from the page's own so a click
/// while it is open cannot reach a button it is covering.
///
/// Returns `None` when no dialog is open, or when the project declares no dialog
/// layout, in which case the page is drawn on its own.
#[allow(clippy::too_many_arguments)]
fn render_dialog_overlay(
    files: &HashMap<String, Vec<u8>>,
    config: &serde_json::Value,
    dpi: DpiContext,
    translations: &HashMap<String, String>,
    interaction: &InteractionState,
    page_width: i32,
    page_height: i32,
    contrast: Option<contrast::Palette>,
    output: &mut LayoutOutput,
) -> Result<Option<DialogUi>> {
    if interaction.dialog.is_none() {
        return Ok(None);
    }
    let path = config["ui"]["dialog_layout"]
        .as_str()
        .unwrap_or(DEFAULT_DIALOG_LAYOUT);
    let Some(bytes) = files.get(path) else {
        // A project that declares no dialog layout keeps working: a close
        // question then closes without asking, as it did before the wizard
        // asked at all.
        return Ok(None);
    };
    let xml = std::str::from_utf8(bytes)?;
    let document = roxmltree::Document::parse(xml)
        .with_context(|| format!("invalid dialog layout: {path}"))?;
    let page = document
        .descendants()
        .find(|node| node.has_tag_name("Page"))
        .with_context(|| format!("dialog layout has no Page element: {path}"))?;
    let dialog_width = scale_value(int_attribute(page, "width").unwrap_or(400), dpi.scale);
    let declared_height = scale_value(int_attribute(page, "height").unwrap_or(230), dpi.scale);
    let corner_radius = scale_value(
        int_attribute(page, "border-radius").unwrap_or(16),
        dpi.scale,
    );

    let context = LayoutContext {
        dpi,
        files,
        config,
        locale: "",
        translations,
        interaction,
        fields: HashMap::new(),
        open_select: None,
        contrast,
    };
    // The declared height is a minimum: the question comes from the product's
    // own translations, so a longer sentence needs a taller card rather than a
    // row of buttons pushed out through the bottom of one.
    let dialog_height = dialog_intrinsic_height(page, declared_height, &context);
    // The dialog is laid out at the origin and then moved to the middle of the
    // page, so its own layout can use the ordinary coordinate system.
    let mut dialog_output = LayoutOutput::default();
    render_layout_content(
        page,
        dialog_width,
        dialog_height,
        corner_radius,
        None,
        &context,
        &mut dialog_output,
    )?;
    translate_layout(
        &mut dialog_output,
        ((page_width - dialog_width) / 2).max(0),
        ((page_height - dialog_height) / 2).max(0),
    );

    // A scrim separates the question from the page behind it, so the page does
    // not look live while the wizard waits for an answer.
    push_solid_layer(
        &mut output.overlay_layers,
        LayerRect {
            left: 0,
            top: 0,
            width: page_width,
            height: page_height,
        },
        DIALOG_SCRIM,
        0,
    )?;
    output.overlay_layers.append(&mut dialog_output.layers);
    output.overlay_texts.append(&mut dialog_output.texts);
    output
        .overlay_layers
        .append(&mut dialog_output.overlay_layers);
    output
        .overlay_texts
        .append(&mut dialog_output.overlay_texts);

    Ok(Some(DialogUi {
        actions: dialog_output.actions,
        text_hits: dialog_output.text_hits,
        hover_regions: dialog_output.hover_regions,
    }))
}

/// Moves every region of a layout by `(left, top)`.
///
/// A dialog is authored on its own and then placed in the middle of the page, so
/// its art and its hit regions have to move together or the two disagree.
fn translate_layout(output: &mut LayoutOutput, left: i32, top: i32) {
    for layer in output
        .layers
        .iter_mut()
        .chain(output.overlay_layers.iter_mut())
    {
        layer.left += left;
        layer.top += top;
    }
    for text in output
        .texts
        .iter_mut()
        .chain(output.overlay_texts.iter_mut())
    {
        text.left += left;
        text.top += top;
    }
    for region in output.actions.iter_mut() {
        region.left += left;
        region.right += left;
        region.top += top;
        region.bottom += top;
    }
    for hit in output.text_hits.iter_mut() {
        hit.left += left;
        hit.right += left;
        hit.top += top;
        hit.bottom += top;
    }
    for region in output.hover_regions.iter_mut() {
        region.left += left;
        region.right += left;
        region.top += top;
        region.bottom += top;
    }
}

fn runtime_page_count(config: &serde_json::Value, mode: RuntimeMode) -> usize {
    let pages = match mode {
        RuntimeMode::Installer => &config["wizard"]["pages"],
        RuntimeMode::Uninstaller => &config["wizard"]["uninstall_pages"],
    };
    pages.as_array().map(Vec::len).unwrap_or(0)
}

/// The page a role names, or the position that role takes by default.
///
/// A task reports on the progress page and ends on the finish page. A
/// project that orders its pages differently marks those two with
/// `"role": "progress"` and `"role": "finish"`; without a role the
/// second page reports and the last one finishes, which is what a project
/// that declares the three usual pages gets.
fn runtime_page_index_for_role(
    config: &serde_json::Value,
    mode: RuntimeMode,
    role: &str,
) -> Option<usize> {
    let pages = match mode {
        RuntimeMode::Installer => &config["wizard"]["pages"],
        RuntimeMode::Uninstaller => &config["wizard"]["uninstall_pages"],
    };
    let entries = pages.as_array()?;
    if let Some(index) = entries
        .iter()
        .position(|entry| entry["role"].as_str() == Some(role))
    {
        return Some(index);
    }
    // One page has nowhere to report to and nothing to finish.
    if entries.len() < 2 {
        return None;
    }
    match role {
        "progress" => Some(1),
        "finish" => Some(entries.len() - 1),
        _ => None,
    }
}

/// The page a role names in the running wizard.
pub(crate) fn page_index_for_role(role: &str) -> Option<usize> {
    let runtime = UI.get()?;
    let state = runtime.lock().ok()?;
    let config = runtime_config(&state).ok()?;
    runtime_page_index_for_role(&config, state.mode, role)
}

/// The configuration the running wizard was built from.
fn runtime_config(state: &RuntimeState) -> Result<serde_json::Value> {
    Ok(serde_json::from_slice::<serde_json::Value>(
        state
            .files
            .get("installer_config.json")
            .map(Vec::as_slice)
            .unwrap_or_default(),
    )?)
}

/// The pages the running wizard declares, in the order the project lists them.
fn runtime_pages(state: &RuntimeState) -> Result<Vec<serde_json::Value>> {
    let config = runtime_config(state)?;
    let pages = match state.mode {
        RuntimeMode::Installer => &config["wizard"]["pages"],
        RuntimeMode::Uninstaller => &config["wizard"]["uninstall_pages"],
    };
    Ok(pages.as_array().cloned().unwrap_or_default())
}

/// The id of the page at `index`, empty for a page the project did not name.
fn page_id(pages: &[serde_json::Value], index: usize) -> &str {
    pages
        .get(index)
        .and_then(|page| page["id"].as_str())
        .unwrap_or_default()
}

/// The page a project declares under `id`.
fn page_index_of_id(pages: &[serde_json::Value], id: &str) -> Option<usize> {
    pages
        .iter()
        .position(|page| page["id"].as_str() == Some(id))
}

/// What stopped the wizard from walking the way the project asked.
enum MoveTrouble {
    /// The project's page hook could not name a page, so the wizard keeps the
    /// order the project declares.
    Hook(anyhow::Error),
    /// There is nowhere to go: the declared order ends here and the hook said
    /// nothing about this move.
    End(anyhow::Error),
}

/// Where the wizard goes when the user leaves page `from` by pressing next.
///
/// The project's hook decides when it names a page for this move; otherwise the
/// declared order does, which is what a project without a hook always gets. A
/// hook that names a page the project does not declare, or the page the wizard
/// is already on, is a mistake the caller reports -- and the declared order
/// still moves the wizard on, because an installer a user cannot walk is worse
/// than a page order an author got wrong.
fn forward_page(
    pages: &[serde_json::Value],
    from: usize,
    chosen: Result<Option<String>>,
) -> (Option<usize>, Option<MoveTrouble>) {
    let declared = (from + 1 < pages.len()).then_some(from + 1);
    let beyond = || anyhow::anyhow!("this wizard has no page 1 step(s) from the one on screen");
    let chosen = match chosen {
        Ok(chosen) => chosen,
        // Nowhere to go either way: the hook's failure is the more useful
        // thing to say, because the declared order has nothing to offer.
        Err(error) => return (declared, Some(MoveTrouble::Hook(error))),
    };
    let Some(id) = chosen else {
        return match declared {
            Some(index) => (Some(index), None),
            None => (None, Some(MoveTrouble::End(beyond()))),
        };
    };
    match page_index_of_id(pages, &id) {
        Some(index) if index != from => (Some(index), None),
        Some(_) => (
            declared,
            Some(MoveTrouble::Hook(anyhow::anyhow!(
                "the page hook named the page the wizard is on, {id}"
            ))),
        ),
        None => (
            declared,
            Some(MoveTrouble::Hook(anyhow::anyhow!(
                "the page hook named a page this wizard does not declare: {id}"
            ))),
        ),
    }
}

/// Asks the project's page hook where the wizard goes when it leaves `from`.
///
/// Nothing but the values the hook may read is gathered here: the script itself
/// runs once the wizard's state is no longer locked, because a hook that is
/// evaluated while the state is held would deadlock the moment one of its
/// primitives asked the wizard something.
fn page_hook_request(
    state: &RuntimeState,
    from: usize,
) -> Result<Option<script::page::PageRequest>> {
    let Some(hook) = state.page_hook.clone() else {
        return Ok(None);
    };
    let config = runtime_config(state)?;
    let pages = runtime_pages(state)?;
    let checked = |id: &str, fallback: bool| {
        state
            .interaction
            .checkbox_states
            .get(id)
            .copied()
            .unwrap_or(fallback)
    };
    let mode = match state.mode {
        RuntimeMode::Installer => script::Mode::Install,
        RuntimeMode::Uninstaller => script::Mode::Uninstall,
    };
    Ok(Some(script::page::PageRequest {
        environment: script::PageEnvironment {
            mode,
            components: install::selected_components(&config, checked),
            config,
            install_path: PathBuf::from(
                state
                    .interaction
                    .text_input_values
                    .get("editDir")
                    .cloned()
                    .unwrap_or_default(),
            ),
            checkboxes: state.interaction.checkbox_states.clone(),
            texts: state.interaction.text_input_values.clone(),
            choices: state.interaction.choices.clone(),
        },
        from: page_id(&pages, from).to_string(),
        hook,
    }))
}

/// Moves the wizard one page forward, along the order the project declares or
/// the page its hook names.
fn navigate_forward() -> Result<()> {
    let (pages, from, request) = {
        let runtime = UI.get().context("native UI state is missing")?;
        let state = runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        (
            runtime_pages(&state)?,
            state.interaction.page_index,
            page_hook_request(&state, state.interaction.page_index)?,
        )
    };
    let chosen = match request {
        Some(request) => script::page::next_page(request),
        None => Ok(None),
    };
    let (target, trouble) = forward_page(&pages, from, chosen);
    match trouble {
        Some(MoveTrouble::Hook(error)) => show_notice(&format!(
            "The page hook could not say where the wizard goes, so the order the project \
             declares is used instead:\n{error:#}"
        )),
        Some(MoveTrouble::End(error)) => show_runtime_error(&error),
        None => {}
    }
    match target {
        Some(index) => walk_to_page(index),
        None => Ok(()),
    }
}

/// Returns the wizard to the page the user came from.
///
/// The way back is the way the user actually walked: a page a hook sent the
/// wizard past is never shown by going back, and a page a task took the wizard
/// to has no way back into it at all, because the task owns the window in
/// between and starts the wizard afresh when it is done.
fn navigate_back() -> Result<()> {
    update_runtime(|state| {
        let from = state.interaction.page_index;
        let target = match state.interaction.page_history.pop() {
            Some(previous) => previous,
            None if from > 0 => from - 1,
            None => bail!("this wizard has no page before the one on screen"),
        };
        state.interaction.page_index = target;
        Ok(())
    })
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
    // A dialog layout serves both a notice and a question, so a control says
    // which kind of dialog it belongs to instead of hard-coding `visible`.
    if let Some(role) = node.attribute("visible-with") {
        return interaction
            .dialog
            .as_ref()
            .is_some_and(|dialog| match role {
                "dismiss" => dialog.offers_dismiss(),
                _ => true,
            });
    }
    node.attribute("visible") != Some("false")
}

/// Positions a node along the horizontal axis.
///
/// `left` measures from the near edge, `right` from the far one, and `inset`
/// is the shorthand that sets all four edges. Either edge can also be given on
/// its own, as `inset-left` or `inset-right`, which measures the same way as
/// the edge attribute it names. A near edge wins when both are declared,
/// because a declared width already fixes the extent.
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
    if let Some(right) = int_attribute(node, "inset-right") {
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
    if let Some(bottom) = int_attribute(node, "inset-bottom") {
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
    focus_regions: &mut Vec<FocusRegion>,
    context: &LayoutContext<'_>,
) {
    let interaction = context.interaction;
    if node.has_tag_name("Button") && !button_enabled(node, interaction, &context.fields) {
        return;
    }
    let action = if node.has_tag_name("Checkbox") {
        node.attribute("id").map(|id| WindowAction::ToggleCheckbox {
            id: id.to_string(),
            checked: checkbox_checked(node, interaction),
        })
    } else if node.has_tag_name("RadioButton") {
        // A radio answers for its group: `group` names the choice, and
        // `value` is the row this button stands for.
        node.attribute("group")
            .zip(node.attribute("value"))
            .map(|(group, value)| WindowAction::ChooseOption {
                id: group.to_string(),
                value: value.to_string(),
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
            Some("cancel") => Some(WindowAction::Cancel),
            Some("pick_directory") => pick_directory_target(node, interaction)
                .map(|id| WindowAction::PickDirectory { id }),
            // `open_url:` takes either a `links` key or a URL written out in
            // full, so a layout can link somewhere the config does not name.
            Some(action) if action.starts_with("open_url:") => {
                let target = action.trim_start_matches("open_url:");
                resolve_link_target(target, context).map(WindowAction::OpenLink)
            }
            Some("switch_language") => Some(WindowAction::ToggleSelectMenu {
                id: select_id(node),
            }),
            // A project that declares more than one page walks them with
            // these, so a licence page or an options page needs no script.
            Some("next") => Some(WindowAction::NextPage),
            Some("back") => Some(WindowAction::PreviousPage),
            // Dialog buttons belong to a dialog layout, so they only ever appear
            // while that dialog is open.
            Some("dialog_ok") => Some(WindowAction::DialogOk),
            Some("dialog_cancel") => Some(WindowAction::DialogCancel),
            Some("install") => Some(WindowAction::Install),
            Some("uninstall") => Some(WindowAction::Uninstall),
            Some("launch_app") => Some(WindowAction::LaunchApp),
            // The finish page closes the wizard; `finish` is that same action
            // under the name the example layouts use.
            Some("finish") => Some(WindowAction::Close),
            // A select that declares no action is still a select: the options
            // it lists are a choice the page offers.
            None if node.has_tag_name("Select") => Some(WindowAction::ToggleSelectMenu {
                id: select_id(node),
            }),
            _ => None,
        }
    };
    if let Some(action) = action {
        let left = rect.left;
        let top = rect.top;
        let right = rect.left + rect.width;
        let bottom = rect.top + rect.height;
        // What carries an action is what the keyboard can land on, and it is
        // recorded here because this is where the layout knows both the control
        // and where it put it -- in the order the page itself is walked.
        if let Some(id) = node.attribute("id") {
            focus_regions.push(FocusRegion {
                id: id.to_string(),
                action: Some(action.clone()),
                left,
                top,
                right,
                bottom,
            });
        }
        actions.push(ActionRegion {
            action,
            left,
            top,
            right,
            bottom,
        });
    }
}

/// Whether a text field holds a value the project accepts, and what to say
/// when it does not.
#[derive(Clone)]
struct FieldState {
    /// True while the value breaks none of the rules the field declares.
    valid: bool,
    /// The locale key of the rule the value breaks first, as the layout wrote
    /// it (`@key`), so the label showing the hint can look it up.
    message: Option<String>,
}

/// Reads the rules every text field on the page declares, against the value it
/// holds right now.
///
/// The rules live in the layout and the value in the interaction, so this runs
/// once per render: a keystroke rebuilds the page, and a button that waits for
/// the field sees the value it was waiting for.
fn collect_field_states(
    page: roxmltree::Node<'_, '_>,
    interaction: &InteractionState,
    config: &serde_json::Value,
) -> HashMap<String, FieldState> {
    page.descendants()
        .filter(|node| node.has_tag_name("TextInput"))
        .filter_map(|node| {
            let id = node.attribute("id")?;
            let value = text_input_value(node, interaction, config).unwrap_or_default();
            Some((id.to_string(), field_state(node, &value)))
        })
        .collect()
}

/// Checks one field's value against the rules its attributes declare.
///
/// A field that declares nothing is always valid. So is an optional field that
/// is empty: `min-length` and `pattern` speak about a value the user chose to
/// give, and `required` is how a project asks for one at all.
fn field_state(node: roxmltree::Node<'_, '_>, value: &str) -> FieldState {
    let required = node
        .attribute("required")
        .is_some_and(|flag| flag != "false");
    let characters = value.chars().count();
    let broken = if value.is_empty() {
        required.then_some("required-message")
    } else if node
        .attribute("min-length")
        .and_then(|minimum| minimum.parse().ok())
        .is_some_and(|minimum| characters < minimum)
    {
        Some("min-length-message")
    } else if node
        .attribute("max-length")
        .and_then(|maximum| maximum.parse().ok())
        .is_some_and(|maximum| characters > maximum)
    {
        Some("max-length-message")
    } else if node
        .attribute("pattern")
        .is_some_and(|mask| !mask_matches(mask, value))
    {
        Some("pattern-message")
    } else {
        None
    };
    FieldState {
        valid: broken.is_none(),
        message: broken
            .and_then(|attribute| node.attribute(attribute))
            .map(str::to_string),
    }
}

/// Whether a value fits a mask, character by character.
///
/// The mask is not a regular expression: `*` stands for any run of characters,
/// including none, `?` for exactly one, and every other character for itself.
/// The whole value has to match, so `?.exe` accepts a two-character name with
/// that extension and nothing else.
fn mask_matches(mask: &str, value: &str) -> bool {
    let mask: Vec<char> = mask.chars().collect();
    let value: Vec<char> = value.chars().collect();
    let (mut m, mut v) = (0usize, 0usize);
    // Where the last `*` stood and how much of the value it had swallowed, so
    // a mismatch hands it one more character instead of starting over.
    let mut star: Option<(usize, usize)> = None;
    while v < value.len() {
        if m < mask.len() && (mask[m] == '?' || mask[m] == value[v]) {
            m += 1;
            v += 1;
        } else if m < mask.len() && mask[m] == '*' {
            star = Some((m, v));
            m += 1;
        } else if let Some((star_mask, star_value)) = star {
            m = star_mask + 1;
            v = star_value + 1;
            star = Some((star_mask, star_value + 1));
        } else {
            return false;
        }
    }
    mask[m..].iter().all(|character| *character == '*')
}

fn button_enabled(
    node: roxmltree::Node<'_, '_>,
    interaction: &InteractionState,
    fields: &HashMap<String, FieldState>,
) -> bool {
    if node.attribute("enabled") == Some("false") {
        return false;
    }
    if let Some(condition) = node.attribute("enabled-when") {
        return evaluate_ui_condition(condition, interaction, fields);
    }
    true
}

fn evaluate_ui_condition(
    condition: &str,
    interaction: &InteractionState,
    fields: &HashMap<String, FieldState>,
) -> bool {
    // A control may wait for more than one thing: the conditions are listed one
    // after another, separated by commas, and every one of them has to hold. A
    // stray comma leaves an empty condition behind, which holds the control
    // back rather than passing for a rule that was met.
    condition
        .split(',')
        .all(|part| condition_holds(part.trim(), interaction, fields))
}

/// Whether one `id:state` condition holds.
fn condition_holds(
    condition: &str,
    interaction: &InteractionState,
    fields: &HashMap<String, FieldState>,
) -> bool {
    let Some((id, expected)) = condition.rsplit_once(':') else {
        return false;
    };
    // A field the page does not declare is one nothing keeps valid, so it is
    // invalid rather than valid: a condition naming the wrong control holds the
    // button back instead of letting a click through, as it does for the other
    // states above.
    let field_valid = fields.get(id).is_some_and(|field| field.valid);
    // A select and a radio group are named by the value that has to hold,
    // which is read before the fixed states below: those name what a checkbox
    // or a panel answers, and a choice that matches none of them falls through
    // to them as it always has.
    if interaction
        .choices
        .get(id)
        .is_some_and(|chosen| chosen == expected)
    {
        return true;
    }
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
        "valid" => field_valid,
        "invalid" => !field_valid,
        _ => false,
    }
}

fn button_image<'a>(
    node: roxmltree::Node<'a, 'a>,
    interaction: &InteractionState,
    fields: &HashMap<String, FieldState>,
) -> Option<&'a str> {
    let enabled = button_enabled(node, interaction, fields);
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
    if !node.has_tag_name("Button") || !button_enabled(node, context.interaction, &context.fields) {
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

/// Whether a radio button is the one its group holds.
///
/// A radio answers for its group rather than for itself: the layout may mark
/// one `checked="true"` as the default, and the first click replaces that for
/// the whole group.
fn radio_checked(node: roxmltree::Node<'_, '_>, interaction: &InteractionState) -> bool {
    let (Some(group), Some(value)) = (node.attribute("group"), node.attribute("value")) else {
        return false;
    };
    match interaction.choices.get(group) {
        Some(chosen) => chosen == value,
        None => node.attribute("checked") == Some("true"),
    }
}

/// Whether a toggle control is on: a checkbox answers for itself, a radio
/// button for the value its group holds.
fn toggle_checked(node: roxmltree::Node<'_, '_>, interaction: &InteractionState) -> bool {
    if node.has_tag_name("RadioButton") {
        radio_checked(node, interaction)
    } else {
        checkbox_checked(node, interaction)
    }
}

/// Name of the control whose menu is open.
///
/// A `switch_language` select may be written without an id, and its menu still
/// has to be told apart from a project's own, so the language control has a
/// name of its own.
const LANGUAGE_SELECT: &str = "@language";

fn select_id(node: roxmltree::Node<'_, '_>) -> String {
    node.attribute("id").unwrap_or(LANGUAGE_SELECT).to_string()
}

/// Whether a select's rows are the languages the project ships.
fn select_language_menu(node: roxmltree::Node<'_, '_>) -> bool {
    node.attribute("action") == Some("switch_language")
}

/// Draws a select: its own background, outline and arrow, and the menu of
/// options it offers while that menu is open.
///
/// The language control and a project's own select share this code and differ
/// in two places: what a row is worth -- a locale to switch to, or a value to
/// record -- and which row counts as the one in use.
fn render_select(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    let id = select_id(node);
    let language = select_language_menu(node);
    let menu_open = context.open_select == Some(id.as_str());
    // Background first, then the outline, so the ring sits on top of the fill
    // and grows inwards from the control edge.
    let radius = scale_value(
        int_attribute(node, "border-radius").unwrap_or(0),
        context.dpi.scale,
    );
    if let Some(background) = node.attribute("background") {
        let background = context.colour(contrast::Role::Control, background);
        push_solid_layer(&mut output.layers, rect, &background, radius)?;
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
    let values: Vec<String> = options
        .iter()
        .map(|option| option.attribute("value").unwrap_or_default().to_string())
        .collect();
    // The row the menu marks as in use: the locale the runtime is reading for
    // the language control, and the value the user picked for a project's own.
    let chosen = if language {
        Some(context.locale.to_string())
    } else {
        context.interaction.choices.get(&id).cloned()
    };
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
        &context.colour(
            contrast::Role::Surface,
            node.attribute("popup-background").unwrap_or("#FF303F4B"),
        ),
        scale_value(4, context.dpi.scale),
    )?;
    for (index, option) in options.into_iter().enumerate() {
        let row = LayerRect {
            left: popup.left + padding,
            top: popup.top + padding + row_height * index as i32,
            width: popup.width - padding * 2,
            height: row_height,
        };
        let value = option.attribute("value").unwrap_or_default();
        let highlighted = context.interaction.highlighted_option == Some(index);
        // A keyboard highlight wins over the current choice, so arrow keys stay
        // visible while they walk past the selected entry.
        //
        // A scheme names one colour for both, so the two are told apart by what
        // they are: the row the keyboard is on is where the user is, and takes
        // the highlight, while the row in use is a state of the control and
        // takes the colour of a control face.
        let (background, role) = if highlighted {
            (
                node.attribute("popup-highlight-background")
                    .or_else(|| node.attribute("popup-selected-background"))
                    .unwrap_or("#FF495A68"),
                contrast::Role::Highlight,
            )
        } else if chosen.as_deref() == Some(value) {
            (
                node.attribute("popup-selected-background")
                    .unwrap_or("#FF42515E"),
                contrast::Role::Control,
            )
        } else {
            ("", contrast::Role::Control)
        };
        if !background.is_empty() {
            let background = context.colour(role, background);
            push_solid_layer(
                &mut output.overlay_layers,
                row,
                &background,
                scale_value(3, context.dpi.scale),
            )?;
        }
        // Text on the row the keyboard is on stands on the highlight, so it
        // takes the colour the scheme keeps for that, or it would be drawn in
        // the background's own colour and disappear into the band.
        let text_role = if highlighted {
            contrast::Role::HighlightText
        } else {
            contrast::Role::Text
        };
        output.overlay_texts.push(TextLayer {
            runs: vec![TextRun {
                text: option
                    .attribute("text")
                    .map(|text| resolve_text(text, context.translations))
                    .unwrap_or_else(|| value.to_string()),
                color: parse_color(
                    &context.colour(text_role, node.attribute("color").unwrap_or("#FFFFFFFF")),
                ),
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
            clip: None,
        });
        let action = if language {
            WindowAction::SelectLanguage(value.to_string())
        } else {
            WindowAction::ChooseOption {
                id: id.clone(),
                value: value.to_string(),
            }
        };
        output.actions.push(ActionRegion {
            action,
            left: row.left,
            top: row.top,
            right: row.left + row.width,
            bottom: row.top + row.height,
        });
    }
    // A menu is drawn over the page rather than inside the control, and Windows
    // draws a line around a list of its own. A layout names no line for a popup,
    // so under a scheme the line is drawn here: without it the menu would sit
    // unseen on a page the scheme paints in the same colour.
    if let Some(palette) = &context.contrast {
        push_border_layer(
            &mut output.overlay_layers,
            popup,
            &palette.colour(contrast::Role::Line),
            scale_value(1, context.dpi.scale),
            scale_value(4, context.dpi.scale),
        )?;
    }
    // What the window needs to walk these rows with the keyboard, kept beside
    // the pixels the menu just drew.
    let chosen_row = chosen
        .as_deref()
        .and_then(|chosen| values.iter().position(|value| value == chosen));
    output.menu = Some(MenuUi {
        select: id,
        language,
        values,
        chosen: chosen_row,
    });
    Ok(())
}

/// Which colour the words of a node belong to.
///
/// A label stands on the page itself, while the words of a button or a select
/// stand on the control's own face, and a scheme is free to name a colour for
/// each: the high contrast themes that ship with Windows draw a button's label
/// in a colour of its own.
fn text_role(node: roxmltree::Node<'_, '_>) -> contrast::Role {
    if node.has_tag_name("Button") || node.has_tag_name("Select") {
        contrast::Role::ControlText
    } else {
        contrast::Role::Text
    }
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
    let color = parse_color(&context.colour(
        text_role(node),
        node.attribute("color").unwrap_or("#FFFFFFFF"),
    ));
    // A link is the one run of text that has to stand out from the words around
    // it, so it takes the highlight colour the scheme keeps for the one thing
    // the user is meant to pick out.
    let link_color = node
        .attribute("linkcolor")
        .map(|declared| parse_color(&context.colour(contrast::Role::Highlight, declared)));
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
        wrap: node.has_tag_name("Checkbox")
            || node.has_tag_name("RadioButton")
            || node.attribute("wrap") == Some("true"),
        clip: None,
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
}

/// Records a writable text field, so a click can put the caret in it.
///
/// A field is recorded even while it holds nothing. An empty field is exactly
/// the one a user has to click into to fill in, and the rules a page declares
/// about the value are what a button waits for; a field that could not be
/// reached until it already had text would never get any.
fn push_text_input(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) {
    if !node.has_tag_name("TextInput") || is_readonly_text_input(node) {
        return;
    }
    let id = node.attribute("id").unwrap_or_default().to_string();
    // A field the user can type into holds a place in the Tab order like any
    // other control. It carries no action, because what a key does there is
    // type: the caret is what answers it.
    if !id.is_empty() {
        output.focus_regions.push(FocusRegion {
            id: id.clone(),
            action: None,
            left: rect.left,
            top: rect.top,
            right: rect.left + rect.width,
            bottom: rect.top + rect.height,
        });
    }
    output.text_inputs.push(TextInputRegion {
        id,
        text: text_input_value(node, context.interaction, context.config).unwrap_or_default(),
        // What a user types is read on the page like any other words, and the
        // caret is drawn in the same colour so it is never lost in the text.
        color: parse_color(&context.colour(
            contrast::Role::Text,
            node.attribute("color").unwrap_or("#FFFFFFFF"),
        )),
        font_size: scale_value(
            int_attribute(node, "font-size").unwrap_or(12),
            context.dpi.scale,
        )
        .max(1),
        bold: node.attribute("font-weight") == Some("bold"),
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
        clip: None,
    });
}

/// A `readonly` field shows a value the user cannot type into.
fn is_readonly_text_input(node: roxmltree::Node<'_, '_>) -> bool {
    node.attribute("readonly")
        .is_some_and(|value| value != "false")
}

/// The text a `Select` shows while its menu is closed.
///
/// The language control shows the locale in use; a project's own select shows
/// the value the user picked, or the first option it declares while nobody has
/// picked one. An option the layout gives no `text` reads as its value, which
/// is how a language list written without one still shows.
fn select_text(node: roxmltree::Node<'_, '_>, context: &LayoutContext<'_>) -> Option<String> {
    let id = select_id(node);
    let wanted = if select_language_menu(node) {
        Some(context.locale.to_string())
    } else {
        context.interaction.choices.get(&id).cloned()
    };
    let options: Vec<_> = node
        .children()
        .filter(|child| child.has_tag_name("Option") && !is_hidden(*child, context.interaction))
        .collect();
    let option = wanted
        .as_deref()
        .and_then(|value| {
            options
                .iter()
                .find(|option| option.attribute("value") == Some(value))
                .copied()
        })
        .or_else(|| options.first().copied())?;
    Some(match option.attribute("text") {
        Some(text) => resolve_text(text, context.translations),
        None => option.attribute("value").unwrap_or_default().to_string(),
    })
}

fn resolved_text_for_node(
    node: roxmltree::Node<'_, '_>,
    context: &LayoutContext<'_>,
) -> Option<(String, TextAlignment)> {
    if node.has_tag_name("TextInput") {
        return text_input_value(node, context.interaction, context.config)
            .map(|value| (value, TextAlignment::Left));
    }
    if node.has_tag_name("Select") {
        return select_text(node, context).map(|text| (text, declared_text_alignment(node)));
    }
    // A hint names the field it explains and draws the words of the rule the
    // value breaks first -- nothing at all while the value is acceptable.
    if let Some(id) = node
        .attribute("value-source")
        .and_then(|source| source.strip_prefix("field-error:"))
    {
        let key = context
            .fields
            .get(id)
            .and_then(|field| field.message.clone())?;
        let message = key
            .strip_prefix('@')
            .and_then(|key| context.translations.get(key))
            .cloned()
            .unwrap_or(key);
        return Some((message, declared_text_alignment(node)));
    }
    let (mut text, alignment) = text_for_node(node, context.translations)?;
    if let Some(source) = node.attribute("value-source") {
        // `dialog:...` comes from the dialog that is open. A dialog layout is
        // authored once and used for every question, so its text comes from the
        // dialog rather than from the layout or the locale table.
        if let Some(role) = source.strip_prefix("dialog:") {
            let resolved = context
                .interaction
                .dialog
                .as_ref()
                .and_then(|dialog| match role {
                    "message" => Some(dialog.message.clone()),
                    "accept" => Some(dialog.accept_label.clone()),
                    "dismiss" => Some(dialog.dismiss_label.clone()),
                    _ => None,
                });
            if let Some(resolved) = resolved {
                return Some((resolved, alignment));
            }
            return None;
        }
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

fn text_input_value(
    node: roxmltree::Node<'_, '_>,
    interaction: &InteractionState,
    config: &serde_json::Value,
) -> Option<String> {
    node.attribute("id")
        .and_then(|id| interaction.text_input_values.get(id))
        .cloned()
        .or_else(|| node.attribute("value").map(str::to_string))
        .or_else(|| {
            node.attribute("value-source")
                .and_then(|source| source.strip_prefix("config:"))
                .and_then(|path| config_value_as_string(config, path))
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
/// Width of the scrollbar a scrollable container draws, at 96 DPI.
const SCROLLBAR_WIDTH: i32 = 8;
/// Shortest a scrollbar thumb may get, at 96 DPI, so it stays usable in a list
/// that holds far more than it shows.
const SCROLLBAR_MIN_THUMB: i32 = 24;
/// Colours of a scrollbar's track and of the thumb inside it, which a container
/// may override with `scrollbar-background` and `scrollbar-thumb-background`.
const SCROLLBAR_TRACK_COLOR: &str = "#40FFFFFF";
const SCROLLBAR_THUMB_COLOR: &str = "#A8FFFFFF";
/// How far one wheel notch moves a scrollable container, at 96 DPI.
const SCROLL_STEP: i32 = 48;
/// Wheel delta Windows counts as one notch.
const WHEEL_DELTA: i32 = 120;

/// The id a container scrolls under, when the layout asks it to.
///
/// A container that asks to scroll without an id has no name to keep a position
/// under, so it stays a plain container rather than quietly sharing one
/// position with every other nameless list on the page.
fn scrollable_id(node: roxmltree::Node<'_, '_>) -> Option<String> {
    if !matches!(node.attribute("scrollable"), Some("true" | "yes" | "1")) {
        return None;
    }
    node.attribute("id").map(str::to_string)
}

/// How long each list of drawn output was, so a container can cut what its own
/// children added without touching what was there before them.
#[derive(Clone, Copy)]
struct OutputMarks {
    layers: usize,
    texts: usize,
    overlay_layers: usize,
    overlay_texts: usize,
    actions: usize,
    text_hits: usize,
    text_inputs: usize,
    hover_regions: usize,
}

impl LayoutOutput {
    fn marks(&self) -> OutputMarks {
        OutputMarks {
            layers: self.layers.len(),
            texts: self.texts.len(),
            overlay_layers: self.overlay_layers.len(),
            overlay_texts: self.overlay_texts.len(),
            actions: self.actions.len(),
            text_hits: self.text_hits.len(),
            text_inputs: self.text_inputs.len(),
            hover_regions: self.hover_regions.len(),
        }
    }

    /// Cuts everything drawn since `marks` to `viewport`.
    ///
    /// A container that scrolls shows one window onto its children, so a row it
    /// has moved past its own edge is neither drawn nor clicked: this cut is
    /// what keeps a button scrolled out of sight from answering a click aimed
    /// at the page behind it.
    fn clip_since(&mut self, marks: OutputMarks, viewport: LayerRect) {
        clip_layers(&mut self.layers, marks.layers, viewport);
        clip_layers(&mut self.overlay_layers, marks.overlay_layers, viewport);
        clip_texts(&mut self.texts, marks.texts, viewport);
        clip_texts(&mut self.overlay_texts, marks.overlay_texts, viewport);
        clip_actions(&mut self.actions, marks.actions, viewport);
        clip_hover_regions(&mut self.hover_regions, marks.hover_regions, viewport);
        clip_text_hits(&mut self.text_hits, marks.text_hits, viewport);
        clip_text_inputs(&mut self.text_inputs, marks.text_inputs, viewport);
    }
}

fn clip_layers(layers: &mut Vec<ImageLayer>, from: usize, viewport: LayerRect) {
    let mut kept = Vec::with_capacity(layers.len() - from);
    for mut layer in layers.drain(from..) {
        let rect = LayerRect {
            left: layer.left,
            top: layer.top,
            width: layer.width,
            height: layer.height,
        };
        let Some(cut) = layer.clip.unwrap_or(rect).intersect(viewport) else {
            continue;
        };
        layer.clip = Some(cut);
        kept.push(layer);
    }
    layers.extend(kept);
}

fn clip_texts(texts: &mut Vec<TextLayer>, from: usize, viewport: LayerRect) {
    let mut kept = Vec::with_capacity(texts.len() - from);
    for mut text in texts.drain(from..) {
        let rect = LayerRect {
            left: text.left,
            top: text.top,
            width: text.width,
            height: text.height,
        };
        let Some(cut) = text.clip.unwrap_or(rect).intersect(viewport) else {
            continue;
        };
        text.clip = Some(cut);
        kept.push(text);
    }
    texts.extend(kept);
}

fn clip_actions(actions: &mut Vec<ActionRegion>, from: usize, viewport: LayerRect) {
    let mut kept = Vec::with_capacity(actions.len() - from);
    for region in actions.drain(from..) {
        let rect = LayerRect {
            left: region.left,
            top: region.top,
            width: region.right - region.left,
            height: region.bottom - region.top,
        };
        let Some(cut) = rect.intersect(viewport) else {
            continue;
        };
        kept.push(ActionRegion {
            action: region.action,
            left: cut.left,
            top: cut.top,
            right: cut.right(),
            bottom: cut.bottom(),
        });
    }
    actions.extend(kept);
}

fn clip_hover_regions(regions: &mut Vec<HoverRegion>, from: usize, viewport: LayerRect) {
    let mut kept = Vec::with_capacity(regions.len() - from);
    for region in regions.drain(from..) {
        let rect = LayerRect {
            left: region.left,
            top: region.top,
            width: region.right - region.left,
            height: region.bottom - region.top,
        };
        let Some(cut) = rect.intersect(viewport) else {
            continue;
        };
        kept.push(HoverRegion {
            id: region.id,
            left: cut.left,
            top: cut.top,
            right: cut.right(),
            bottom: cut.bottom(),
        });
    }
    regions.extend(kept);
}

fn clip_text_hits(hits: &mut Vec<TextHit>, from: usize, viewport: LayerRect) {
    let mut kept = Vec::with_capacity(hits.len() - from);
    for hit in hits.drain(from..) {
        let rect = LayerRect {
            left: hit.left,
            top: hit.top,
            width: hit.right - hit.left,
            height: hit.bottom - hit.top,
        };
        let Some(cut) = rect.intersect(viewport) else {
            continue;
        };
        kept.push(TextHit {
            action: hit.action,
            left: cut.left,
            top: cut.top,
            right: cut.right(),
            bottom: cut.bottom(),
        });
    }
    hits.extend(kept);
}

fn clip_text_inputs(fields: &mut Vec<TextInputRegion>, from: usize, viewport: LayerRect) {
    let mut kept = Vec::with_capacity(fields.len() - from);
    for mut field in fields.drain(from..) {
        let rect = LayerRect {
            left: field.left,
            top: field.top,
            width: field.width,
            height: field.height,
        };
        let Some(cut) = field.clip.unwrap_or(rect).intersect(viewport) else {
            continue;
        };
        field.clip = Some(cut);
        kept.push(field);
    }
    fields.extend(kept);
}

/// The strip a scrollable container draws its scrollbar in, at the trailing
/// edge of the view the container shows.
fn scrollbar_track(axis: FlowAxis, viewport: LayerRect, scale: f32) -> LayerRect {
    let width = scale_value(SCROLLBAR_WIDTH, scale);
    match axis {
        FlowAxis::Vertical => LayerRect {
            left: viewport.right() - width,
            top: viewport.top,
            width,
            height: viewport.height,
        },
        FlowAxis::Horizontal => LayerRect {
            left: viewport.left,
            top: viewport.bottom() - width,
            width: viewport.width,
            height: width,
        },
    }
}

/// Where the thumb stands inside its track for a given offset.
///
/// The thumb is as long as the share of the list the view shows, and it travels
/// the rest of the track as the offset covers the rest of the list, so the bar
/// says where the window is rather than how much there is.
fn scrollbar_thumb(
    axis: FlowAxis,
    track: LayerRect,
    viewport_main: i32,
    content: i32,
    offset: i32,
    scale: f32,
) -> LayerRect {
    let track_main = axis.main(track);
    let shortest = scale_value(SCROLLBAR_MIN_THUMB, scale).min(track_main);
    let share = if content > 0 {
        (viewport_main as f32 / content as f32) * track_main as f32
    } else {
        track_main as f32
    };
    let thumb_main = (share.round() as i32).clamp(shortest, track_main);
    let travel = track_main - thumb_main;
    let overflow = (content - viewport_main).max(0);
    let travelled = if overflow > 0 {
        (travel as f32 * offset as f32 / overflow as f32).round() as i32
    } else {
        0
    }
    .clamp(0, travel);
    match axis {
        FlowAxis::Vertical => LayerRect {
            left: track.left,
            top: track.top + travelled,
            width: track.width,
            height: thumb_main,
        },
        FlowAxis::Horizontal => LayerRect {
            left: track.left + travelled,
            top: track.top,
            width: thumb_main,
            height: track.height,
        },
    }
}

/// The two parts of a track a click pages the view through: the part before the
/// thumb moves back, the part after it moves on. Either is absent when the thumb
/// reaches that end.
fn scrollbar_pages(
    axis: FlowAxis,
    track: LayerRect,
    thumb: LayerRect,
) -> (Option<LayerRect>, Option<LayerRect>) {
    let part = |start: i32, end: i32| {
        (end > start).then(|| match axis {
            FlowAxis::Vertical => LayerRect {
                top: start,
                height: end - start,
                ..track
            },
            FlowAxis::Horizontal => LayerRect {
                left: start,
                width: end - start,
                ..track
            },
        })
    };
    match axis {
        FlowAxis::Vertical => (
            part(track.top, thumb.top),
            part(thumb.bottom(), track.bottom()),
        ),
        FlowAxis::Horizontal => (
            part(track.left, thumb.left),
            part(thumb.right(), track.right()),
        ),
    }
}

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
    // The room the container itself has. A container that scrolls lays its
    // children out beside that room rather than inside it, so the two are told
    // apart from here on.
    let room = axis.main(content);
    let items: Vec<_> = children
        .iter()
        .map(|child| flow_item_for_node(*child, axis, room, context))
        .collect();
    let cross_sizes: Vec<i32> = children
        .iter()
        .map(|child| cross_size_for_node(*child, axis, axis.cross(content), context))
        .collect();
    // A container that does not wrap keeps every item on one line, which is what
    // free space is shared across.
    let lines = if wraps(node) {
        wrap_lines(&items, room, gap)
    } else {
        vec![(0..items.len()).collect()]
    };
    let single_line = lines.len() == 1 && !wraps(node);
    // A container that scrolls shows one window onto its children: it lays them
    // out at the size each one claims on its own and lets the run grow past the
    // room it has, which is what there is to scroll through. A wrapping
    // container keeps deciding its own line breaks, so it stays a plain one.
    let scrolling = scrollable_id(node).filter(|_| single_line);
    let available_main = match scrolling {
        Some(_) => {
            items.iter().map(|item| item.basis_size()).sum::<i32>()
                + gap * i32::try_from(items.len().saturating_sub(1)).unwrap_or(0)
        }
        None => room,
    };
    // What each line holds is worked out before anything is drawn, because a
    // container that scrolls has to know how much it holds before it can say how
    // far it is scrolled.
    let planned: Vec<(Vec<usize>, Vec<i32>)> = lines
        .into_iter()
        .map(|line| {
            let line_items: Vec<FlowItem> = line.iter().map(|index| items[*index]).collect();
            let sizes = flow_widths(&line_items, available_main, gap);
            (line, sizes)
        })
        .collect();
    let held = planned
        .iter()
        .map(|(line, sizes)| {
            sizes.iter().sum::<i32>()
                + gap * i32::try_from(line.len().saturating_sub(1)).unwrap_or(0)
        })
        .sum::<i32>();
    // How far down its run the container is scrolled, never past the end of it.
    let scroll = scrolling.map(|id| {
        let offset = context
            .interaction
            .scroll_offsets
            .get(&id)
            .copied()
            .unwrap_or(0)
            .clamp(0, (held - room).max(0));
        (id, offset)
    });
    let marks = output.marks();
    let mut cross_cursor = 0;
    for (line, sizes) in planned {
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
                let placed = match &scroll {
                    Some((_, offset)) => axis.shift(placed, -offset),
                    None => placed,
                };
                render_flow_item(child, placed, context, output)?;
            }
            cursor += outer_main + gap;
        }
        cross_cursor += line_cross + gap;
    }
    if let Some((id, offset)) = scroll {
        // Everything the children drew is cut to the view the container shows,
        // so a row the user scrolled past is neither drawn nor clicked.
        output.clip_since(marks, content);
        // A container may name the colours of its own bar; Windows keeps a
        // colour for a scrollbar's track as well, which is what a scheme names
        // in place of either.
        let track_colour = node
            .attribute("scrollbar-background")
            .unwrap_or(SCROLLBAR_TRACK_COLOR);
        let thumb_colour = node
            .attribute("scrollbar-thumb-background")
            .unwrap_or(SCROLLBAR_THUMB_COLOR);
        let track = (held > room).then(|| scrollbar_track(axis, content, context.dpi.scale));
        let thumb =
            track.map(|track| scrollbar_thumb(axis, track, room, held, offset, context.dpi.scale));
        if let (Some(track), Some(thumb)) = (track, thumb) {
            push_solid_layer(
                &mut output.overlay_layers,
                track,
                &context.colour(contrast::Role::Scrollbar, track_colour),
                scale_value(3, context.dpi.scale),
            )?;
            push_solid_layer(
                &mut output.overlay_layers,
                thumb,
                &context.colour(contrast::Role::Control, thumb_colour),
                scale_value(3, context.dpi.scale),
            )?;
            // A click on the track outside the thumb moves the view by one of
            // its own pages, which is what makes the bar usable without dragging
            // the thumb.
            let (before, after) = scrollbar_pages(axis, track, thumb);
            for (forward, page) in [(false, before), (true, after)] {
                let Some(page) = page else { continue };
                output.actions.push(ActionRegion {
                    action: WindowAction::ScrollPage {
                        id: id.clone(),
                        forward,
                    },
                    left: page.left,
                    top: page.top,
                    right: page.right(),
                    bottom: page.bottom(),
                });
            }
        }
        output.scroll_views.push(ScrollView {
            id,
            axis,
            area: rect,
            viewport: content,
            content: held,
            offset,
        });
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
            let image_and_gap = if node.has_tag_name("Checkbox") || node.has_tag_name("RadioButton")
            {
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
    (container_intrinsic_content(node, axis, context) + padding.along(axis)).max(0)
}

/// The extent a container's children need along `axis`, without its own padding.
///
/// Keeping the two apart is what lets a container be measured as a child of
/// another container without counting its padding twice.
fn container_intrinsic_content(
    node: roxmltree::Node<'_, '_>,
    axis: FlowAxis,
    context: &LayoutContext<'_>,
) -> i32 {
    let children: Vec<_> = node
        .children()
        .filter(|child| child.is_element() && !is_hidden(*child, context.interaction))
        .collect();
    if children.is_empty() {
        return 0;
    }
    // The extent the container declares for itself on the axis being measured,
    // which is the base a percentage child resolves against.
    let declared_along_axis =
        size_attribute(node, main_axis_attribute(axis), 0, context).unwrap_or(0);
    if flow_axis(node) == Some(axis) {
        let gap = scale_value(
            int_attribute(node, "item-spacing")
                .or_else(|| int_attribute(node, "gap"))
                .unwrap_or(0),
            context.dpi.scale,
        );
        let items: Vec<FlowItem> = children
            .iter()
            .map(|child| flow_item_for_node(*child, axis, declared_along_axis, context))
            .collect();
        if wraps(node) && declared_along_axis > 0 {
            // A wrapping container is only as wide as its widest line, which is
            // what an outer flow needs to place it.
            wrap_lines(&items, declared_along_axis, gap)
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
        // The container flows across the axis being measured, so its extent on
        // that axis is the largest extent its children ask for along it. Asking
        // for a child's size across the container's flow instead would measure
        // the wrong edge, which is what used to collapse a dialog row.
        children
            .iter()
            .map(|child| outer_extent_along(*child, axis, declared_along_axis, context))
            .max()
            .unwrap_or(0)
    }
}

/// The extent a child claims along `axis`, padding and margin included.
///
/// A child that declares an extent keeps it, a child that does not falls back to
/// the extent its own content needs — a label's line box, a nested container's
/// children — and a child with neither contributes nothing. Nothing here
/// stretches to its container, because the container's extent is exactly what
/// this is measuring.
fn outer_extent_along(
    child: roxmltree::Node<'_, '_>,
    axis: FlowAxis,
    base: i32,
    context: &LayoutContext<'_>,
) -> i32 {
    let content = size_attribute(child, main_axis_attribute(axis), base, context).or_else(|| {
        if renders_own_children(child) {
            // A container reports its children, not its own padding, which is
            // added below so it is counted exactly once.
            Some(container_intrinsic_content(child, axis, context))
        } else {
            intrinsic_size(child, axis, context)
        }
    });
    let Some(content) = content else {
        return 0;
    };
    let insets = insets_for_node(child, "padding", context).along(axis)
        + insets_for_node(child, "margin", context).along(axis);
    (content + insets).max(0)
}

/// The height a dialog needs for the content it is about to show.
///
/// A dialog declares a height, but the text it holds is chosen at runtime from
/// the product's translations: a longer question wraps onto another line, and
/// the answers underneath would then be pushed past the card's bottom edge.
/// The declared height is therefore a minimum, and a layout whose content asks
/// for more gets it, so no translation can push a button out of the frame.
///
/// Children that are placed absolutely, or hidden for this question, take no
/// room and are not measured.
fn dialog_intrinsic_height(
    page: roxmltree::Node<'_, '_>,
    declared: i32,
    context: &LayoutContext<'_>,
) -> i32 {
    page.children()
        .filter(|child| {
            child.is_element()
                && !is_hidden(*child, context.interaction)
                && child.attribute("position") != Some("absolute")
        })
        .map(|child| match child.attribute("height") {
            // A child that fills the page would only repeat the page's own
            // extent when asked for its height, so its content is measured
            // instead. That is exactly the case a long question overflows.
            Some(raw) if raw.trim().ends_with('%') => {
                if renders_own_children(child) {
                    container_intrinsic_size(child, FlowAxis::Vertical, context)
                } else {
                    intrinsic_size(child, FlowAxis::Vertical, context).unwrap_or(0)
                }
            }
            _ => outer_extent_along(child, FlowAxis::Vertical, declared, context),
        })
        .max()
        .unwrap_or(0)
        .max(declared)
}

fn has_intrinsic_text(node: roxmltree::Node<'_, '_>) -> bool {
    node.tag_name().name() == "Checkbox"
        || node.tag_name().name() == "RadioButton"
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

/// Draws a checkbox or a radio button: its state image, its click region, and
/// the text the layout places beside that image.
///
/// The flow and the absolute path both need this. A checkbox that was given
/// coordinates used to reach neither the image nor the click region, so it drew
/// as a caption that could not be toggled, which is what the shipped uninstall
/// page's keep-data box had become. A radio button shares the path and differs
/// only in which image its state picks.
fn render_toggle(
    node: roxmltree::Node<'_, '_>,
    rect: LayerRect,
    context: &LayoutContext<'_>,
    output: &mut LayoutOutput,
) -> Result<()> {
    let checked = toggle_checked(node, context.interaction);
    let image = if checked {
        node.attribute("checked-image")
    } else {
        node.attribute("unchecked-image")
    };
    let mut text_left = rect.left;
    if let Some(value) = image {
        let mut style = parse_image_style(value);
        if let Some(destination) = style.destination {
            text_left += scale_value(destination.left + destination.width + 8, context.dpi.scale);
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
    push_action(
        node,
        rect,
        &mut output.actions,
        &mut output.focus_regions,
        context,
    );
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
    Ok(())
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
        "Checkbox" | "RadioButton" => render_toggle(node, rect, context, output)?,
        "Button" => {
            push_action(
                node,
                rect,
                &mut output.actions,
                &mut output.focus_regions,
                context,
            );
            push_hover_region(node, rect, context, &mut output.hover_regions);
            if let Some(value) = button_image(node, context.interaction, &context.fields) {
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
        "Label" => {
            push_action(
                node,
                rect,
                &mut output.actions,
                &mut output.focus_regions,
                context,
            );
            push_hover_region(node, rect, context, &mut output.hover_regions);
            push_node_text(node, rect, context, output);
        }
        "Select" => {
            push_action(
                node,
                rect,
                &mut output.actions,
                &mut output.focus_regions,
                context,
            );
            push_hover_region(node, rect, context, &mut output.hover_regions);
            push_node_text(node, rect, context, output);
            render_select(node, rect, context, output)?;
        }
        "TextInput" => {
            push_node_text(node, rect, context, output);
            push_text_input(node, rect, context, output);
        }
        "ProgressBar" => render_progress_bar(node, rect, context, output)?,
        "Image" | "Icon" => {
            push_action(
                node,
                rect,
                &mut output.actions,
                &mut output.focus_regions,
                context,
            );
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
        // A box is a surface a page lays a control or a card onto, so it is
        // painted with the colour a scheme keeps for a control face.
        let background = context.colour(contrast::Role::Control, background);
        push_solid_layer(
            &mut output.layers,
            rect,
            &background,
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

/// Text a node draws from the layout itself.
///
/// A select is not read here: it shows one of its options, and which one
/// depends on what the user has picked, so `select_text` answers for it.
fn text_for_node(
    node: roxmltree::Node<'_, '_>,
    translations: &HashMap<String, String>,
) -> Option<(String, TextAlignment)> {
    let raw = match node.tag_name().name() {
        "Button" | "Checkbox" | "RadioButton" | "Label" => {
            node.attribute("text").or_else(|| node.attribute("value"))?
        }
        _ => return None,
    };
    Some((
        resolve_text(raw, translations),
        declared_text_alignment(node),
    ))
}

/// Text a layout writes, with an `@key` looked up in the locale table.
///
/// A key no language file holds stays as it was written, which is what makes a
/// missing translation visible instead of blank.
fn resolve_text(raw: &str, translations: &HashMap<String, String>) -> String {
    raw.strip_prefix('@')
        .and_then(|key| translations.get(key))
        .cloned()
        .unwrap_or_else(|| raw.to_string())
}

/// The alignment a text element asks for: `textalign` (or `text-align`), with a
/// button centring its own text unless it says otherwise.
fn declared_text_alignment(node: roxmltree::Node<'_, '_>) -> TextAlignment {
    match node
        .attribute("textalign")
        .or_else(|| node.attribute("text-align"))
    {
        Some("center") => TextAlignment::Center,
        Some("right") => TextAlignment::Right,
        _ if node.has_tag_name("Button") => TextAlignment::Center,
        _ => TextAlignment::Left,
    }
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
        clip: field.clip,
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
        clip: field.clip,
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
        clip: None,
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
    let color = context.colour(contrast::Role::Line, color);
    push_border_layer(
        layers,
        rect,
        &color,
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
        clip: None,
    });
    Ok(())
}

/// Paints the ring around the control the keyboard is on, if it has one.
///
/// The ring is drawn over the control's own rectangle, so it follows the layout
/// at any scale and needs no room of its own. It is dotted rather than solid
/// because a control can draw a border of its own, and a second solid line laid
/// over the first says nothing about where the keyboard is. Under a scheme it
/// takes the highlight colour, which is the one a user is sure to have picked to
/// be seen against their own background.
fn push_focus_ring(
    output: &mut LayoutOutput,
    page: roxmltree::Node<'_, '_>,
    context: &LayoutContext<'_>,
) -> Result<()> {
    let Some(focused) = context.interaction.focused_control.as_deref() else {
        return Ok(());
    };
    // The page is laid out again whenever the ring moves, so the regions on
    // screen are the ones this render just recorded: a control the new page does
    // not have leaves nothing to draw, which is what makes Tab a no-op there.
    let rect = {
        let Some(region) = output
            .focus_regions
            .iter()
            .find(|region| region.id == focused)
        else {
            return Ok(());
        };
        LayerRect {
            left: region.left,
            top: region.top,
            width: region.right - region.left,
            height: region.bottom - region.top,
        }
    };
    push_focus_ring_layer(
        &mut output.layers,
        rect,
        &context.colour(contrast::Role::Highlight, &focus_ring_color(page, focused)),
    )
}

/// The colour of the ring: what the focused control asks for, then what the page
/// it sits on asks for, then the accent the wizard draws its focus with.
fn focus_ring_color(page: roxmltree::Node<'_, '_>, id: &str) -> String {
    // The accent a project gets when it names no colour of its own.
    const FOCUS_COLOR: &str = "#FF1F6FEB";

    page.document()
        .descendants()
        .find(|node| node.is_element() && node.attribute("id") == Some(id))
        .and_then(|control| control.attribute("focus-color"))
        .or_else(|| page.attribute("focus-color"))
        .unwrap_or(FOCUS_COLOR)
        .to_string()
}

/// Draws the dotted outline of the focus ring as one layer.
///
/// Every other pixel of every edge is left transparent, which is what makes a
/// one-pixel line read as a ring at any scale: the pixels are layout pixels, so
/// a display that scales the page scales the dots with it.
fn push_focus_ring_layer(layers: &mut Vec<ImageLayer>, rect: LayerRect, color: &str) -> Result<()> {
    if rect.width <= 0 || rect.height <= 0 {
        return Ok(());
    }
    let pixel_width = u32::try_from(rect.width).context("focus ring width must be positive")?;
    let pixel_height = u32::try_from(rect.height).context("focus ring height must be positive")?;
    let pixel_count = (pixel_width as usize)
        .checked_mul(pixel_height as usize)
        .context("focus ring size overflow")?;
    let mut pixels = vec![
        0u8;
        pixel_count
            .checked_mul(4)
            .context("focus ring size overflow")?
    ];
    let (alpha, red, green, blue) = parse_argb(color);
    let premultiply = |component: u8| ((component as u16 * alpha as u16) / 255) as u8;
    for y in 0..rect.height {
        for x in 0..rect.width {
            let dotted = if y == 0 || y == rect.height - 1 {
                x % 2 == 0
            } else if x == 0 || x == rect.width - 1 {
                y % 2 == 0
            } else {
                continue;
            };
            if !dotted {
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
        clip: None,
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
        // The track is the control's own face; the part that is filled is the
        // project's artwork, which keeps the colours it was drawn with.
        let background = context.colour(contrast::Role::Control, background);
        push_solid_layer(&mut output.layers, rect, &background, radius)?;
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
        clip: None,
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
        clip: None,
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
        // The window is placed once it exists, so it can be centred on the
        // monitor the user is actually looking at and clamped to that monitor's
        // work area. Primary-screen metrics ignore a second display and, on a
        // display scaled past 100%, a layout can measure wider than the desktop,
        // in which case the old arithmetic resolved to the top-left corner.
        let window = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name,
            &title,
            style,
            0,
            0,
            client_width,
            client_height,
            None,
            None,
            instance,
            None,
        )?;
        center_window(window);
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
        // Worker threads post progress updates back to this window.
        if let Some(state) = UI.get() {
            if let Ok(mut state) = state.lock() {
                state.window = window.0 as isize;
            }
        }
        let _ = ShowWindow(window, SW_SHOW);
        let _ = UpdateWindow(window);
        if let Some((dpi, suggestion)) = test_dpi_change() {
            // Both axes carry the same factor, which is what Windows reports
            // for every display it drives.
            let packed = ((dpi as usize) << 16) | dpi as usize;
            // Windows delivers this message synchronously, and the rectangle
            // it carries means it has to: a pointer cannot cross a posted
            // message, which Windows rejects outright.
            SendMessageW(
                window,
                WM_DPICHANGED,
                WPARAM(packed),
                LPARAM(suggestion as isize),
            );
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

/// Puts the window in the middle of its monitor, sized to the current page.
///
/// The work area is used rather than the full screen, so the wizard never hides
/// under the taskbar, and the size is clamped to it so a layout that is larger
/// than the desktop stays fully reachable.
///
/// Nothing here reports a failure. A window that cannot be moved keeps the
/// position Windows gave it, which is what the installer did before.
unsafe fn center_window(window: HWND) {
    let (width, height) = {
        let Some(runtime) = UI.get() else {
            return;
        };
        let Ok(mut state) = runtime.lock() else {
            return;
        };
        let size = (state.ui.width, state.ui.height);
        if size == state.window_size && size.0 > 0 {
            // The page still has the size the window already has, so a window the
            // user moved stays where they put it.
            return;
        }
        state.window_size = size;
        size
    };
    let Some(work) = monitor_work_area(window) else {
        return;
    };
    let (left, top, width, height) = centered_bounds(work, width, height);
    place_window(window, left, top, width, height);
}

/// Moves and resizes the window without touching z-order or focus.
///
/// The rounded region has to be rebuilt with the window: a region outlives the
/// size it was cut for, so a page that grows would keep its old corners.
unsafe fn place_window(window: HWND, left: i32, top: i32, width: i32, height: i32) {
    let _ = SetWindowPos(
        window,
        HWND_TOP,
        left,
        top,
        width,
        height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    let radius = UI
        .get()
        .and_then(|state| state.lock().ok())
        .map(|state| state.ui.corner_radius)
        .unwrap_or(0);
    apply_window_shape(window, width, height, radius);
}

/// Cuts the window into its rounded rectangle, or clears the region when the
/// layout asks for square corners.
unsafe fn apply_window_shape(window: HWND, width: i32, height: i32, radius: i32) {
    let region = if radius > 0 {
        CreateRoundRectRgn(0, 0, width + 1, height + 1, radius * 2, radius * 2)
    } else {
        CreateRectRgn(0, 0, width + 1, height + 1)
    };
    if !region.is_invalid() {
        // The window owns the region from here, so it must not be deleted.
        let _ = SetWindowRgn(window, region, true);
    }
}

/// Re-measures the current page for a display with a different scaling factor.
///
/// Windows sends the new DPI and a rectangle it suggests for the window. The
/// suggestion is honoured as a position so a window the user dragged between
/// displays does not jump, but the size comes from the re-measured layout: the
/// layout engine knows how much room the page needs at the new scale.
unsafe fn handle_dpi_changed(window: HWND, dpi: u32, suggested: *const RECT) {
    let Some(runtime) = UI.get() else {
        return;
    };
    let Ok(mut state) = runtime.lock() else {
        return;
    };
    let context = DpiContext::for_dpi(dpi, state.dpi_settings);
    if context.scale == state.dpi.scale {
        // Windows reports the DPI again for events that do not change the
        // scale, and re-measuring the page for those would be wasted work.
        return;
    }
    state.dpi = context;
    if rebuild_runtime_ui(&mut state).is_err() {
        return;
    }
    let (width, height) = (state.ui.width, state.ui.height);
    state.window_size = (width, height);
    drop(state);

    let (left, top) = if suggested.is_null() {
        (i32::MIN, i32::MIN)
    } else {
        let suggestion = &*suggested;
        (suggestion.left, suggestion.top)
    };
    let Some(work) = monitor_work_area(window) else {
        return;
    };
    let (left, top, width, height) = if left == i32::MIN {
        centered_bounds(work, width, height)
    } else {
        clamped_bounds(work, left, top, width, height)
    };
    place_window(window, left, top, width, height);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
}

/// Lays the page out again when the machine asks for different colours.
///
/// Windows tells every window when the user turns high contrast on or off, or
/// picks another scheme, and a page this runtime paints follows that only when
/// it is laid out again. The rebuild is skipped while the machine asks for what
/// the current frame was already painted with, which is what makes it cheap to
/// answer the messages that are not about colours at all.
unsafe fn refresh_contrast(window: HWND) {
    let Some(runtime) = UI.get() else {
        return;
    };
    let Ok(mut state) = runtime.lock() else {
        return;
    };
    let palette = contrast::palette();
    if palette == state.contrast {
        return;
    }
    state.contrast = palette;
    if rebuild_runtime_ui(&mut state).is_err() {
        return;
    }
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
}

/// The `(left, top, right, bottom)` of the work area of the monitor a window is
/// on. A window that does not exist yet falls back to the primary monitor.
unsafe fn monitor_work_area(window: HWND) -> Option<(i32, i32, i32, i32)> {
    let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
    if monitor.is_invalid() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return None;
    }
    Some((
        info.rcWork.left,
        info.rcWork.top,
        info.rcWork.right,
        info.rcWork.bottom,
    ))
}

/// Centres a `width` by `height` window inside a work area, clamped to it.
///
/// Split out from the Win32 call so the arithmetic that decides whether a window
/// lands in the middle or in a corner is testable on its own.
fn centered_bounds(work: (i32, i32, i32, i32), width: i32, height: i32) -> (i32, i32, i32, i32) {
    let (left, top, right, bottom) = work;
    let work_width = (right - left).max(1);
    let work_height = (bottom - top).max(1);
    let width = width.clamp(1, work_width);
    let height = height.clamp(1, work_height);
    clamped_bounds(
        work,
        left + (work_width - width) / 2,
        top + (work_height - height) / 2,
        width,
        height,
    )
}

/// Keeps a window of `width` by `height` inside a work area.
///
/// A window the user has placed keeps that place as far as the work area allows;
/// one that would hang off an edge is pulled back in, and a window larger than
/// the work area is shrunk to it so every edge stays reachable.
fn clamped_bounds(
    work: (i32, i32, i32, i32),
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> (i32, i32, i32, i32) {
    let (work_left, work_top, work_right, work_bottom) = work;
    let work_width = (work_right - work_left).max(1);
    let work_height = (work_bottom - work_top).max(1);
    let width = width.clamp(1, work_width);
    let height = height.clamp(1, work_height);
    (
        left.clamp(work_left, work_right - width),
        top.clamp(work_top, work_bottom - height),
        width,
        height,
    )
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
                let _ = set_open_select(window, None);
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
                // A click on a field puts the ring on it as well as the caret in
                // it, so the keyboard carries on from where the click left off.
                let _ = set_focused_control(window, Some(id.clone()));
                let _ = begin_text_selection_drag(window, id, index, extend);
                let _ = SetCapture(window);
                return LRESULT(0);
            }
            if let Some(id) = hover_control_at(x, y) {
                let _ = set_pressed_control(window, Some(id.clone()));
                if text_input_is_focused() {
                    // Clicking away from a field takes the caret out of it.
                    let _ = focus_text_input(window, None, 0);
                }
                let _ = set_focused_control(window, Some(id));
                return LRESULT(0);
            }
            let over_action = window_action_at(x, y).is_some();
            let over_caption = UI
                .get()
                .and_then(|state| state.lock().ok())
                .is_some_and(|state| y < state.ui.caption_height && state.ui.dialog.is_none());
            if over_caption && !over_action {
                // `WM_LBUTTONDOWN` carries client coordinates, but this message
                // defers to the same fields as a real caption press, where they
                // are screen coordinates. Passing the client point through
                // unconverted made the window jump to a position derived from
                // where the pointer sat inside it.
                let mut point = POINT { x, y };
                if ClientToScreen(window, &mut point).as_bool() {
                    let _ = ReleaseCapture();
                    let _ = SendMessageW(
                        window,
                        WM_NCLBUTTONDOWN,
                        WPARAM(HTCAPTION as usize),
                        LPARAM(((point.y << 16) | (point.x & 0xFFFF)) as isize),
                    );
                }
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // A wheel message carries screen coordinates even though the mouse
            // messages beside it carry client ones, so the container the pointer
            // is over is the one its point lands in once it is converted.
            let (x, y) = client_point(lparam);
            let mut point = POINT { x, y };
            if ScreenToClient(window, &mut point).as_bool() {
                let delta = (wparam.0 >> 16) as u16 as i16;
                if let Err(error) = scroll_container_at(window, point.x, point.y, delta) {
                    show_runtime_error(&error);
                }
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
        // Tab walks the controls the page lays out, and Shift+Tab walks them the
        // other way. It comes before the field that owns the keyboard, because a
        // field has to be leavable by the same key that entered it. A menu or a
        // dialog owns the keyboard while it is open, so neither walks the page
        // under it: the ring stays where it was.
        WM_KEYDOWN
            if wparam.0 as u32 == VK_TAB.0 as u32 && !menu_is_open() && !dialog_is_open() =>
        {
            if let Err(error) = move_page_focus(window, key_down(VK_SHIFT)) {
                show_runtime_error(&error);
            }
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
        WM_KEYDOWN if menu_is_open() => {
            handle_menu_key(window, wparam.0 as u32);
            LRESULT(0)
        }
        // A dialog answers to the keyboard the way a message box does, and it
        // takes these keys before the page can treat Escape as "close the
        // window".
        WM_KEYDOWN if dialog_is_open() => {
            match VIRTUAL_KEY(wparam.0 as u16) {
                VK_RETURN => handle_window_action(window, WindowAction::DialogOk),
                VK_ESCAPE => handle_window_action(window, WindowAction::DialogCancel),
                _ => {}
            }
            LRESULT(0)
        }
        // The keys a button answers to do what the control the ring is on does.
        // They come after the menu and the dialog, which answer them themselves,
        // and after the focused field, where a space is a character.
        WM_KEYDOWN
            if (wparam.0 as u32 == VK_RETURN.0 as u32 || wparam.0 as u32 == VK_SPACE.0 as u32)
                && !install::busy() =>
        {
            if let Err(error) = activate_focused_control(window) {
                show_runtime_error(&error);
            }
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u32 == VK_ESCAPE.0 as u32 && !install::busy() => {
            let _ = DestroyWindow(window);
            LRESULT(0)
        }
        // Windows moved the window to a display that scales differently. The
        // page is measured in layout units and painted at the scale the display
        // asks for, so it has to be laid out again rather than stretched.
        WM_DPICHANGED => {
            // The X-axis DPI is in the low word of wParam; the high word holds
            // the Y axis, which is the same value on every display Windows
            // reports, and the scale is derived from one number.
            let dpi = u32::from(wparam.0 as u16);
            handle_dpi_changed(window, dpi, lparam.0 as *const RECT);
            LRESULT(0)
        }
        // The user turned high contrast on or off, or picked another scheme.
        // Windows says so to every window, and a page is a picture, so it only
        // follows the new colours when it is laid out again. The other two
        // messages say the same thing in a different way, and the rebuild they
        // ask for is skipped unless the colours really changed.
        WM_SETTINGCHANGE | WM_SYSCOLORCHANGE | WM_THEMECHANGED => {
            refresh_contrast(window);
            LRESULT(0)
        }
        WM_APP_REFRESH => {
            // The state is already rebuilt by whoever posted this message. A
            // page change can bring a differently sized layout, and the window
            // follows it so the wizard is never left off-centre.
            center_window(window);
            let _ = InvalidateRect(window, None, false);
            // A worker may have moved the caret, and an input method is only
            // repositioned from the thread that owns the window.
            place_ime_windows(window);
            LRESULT(0)
        }
        WM_CLOSE if install::busy() => LRESULT(0),
        WM_DESTROY => {
            // The window is gone before the answer was given, so a script
            // waiting on a question stops waiting instead of hanging the worker
            // thread that runs it.
            answer_pending_dialog(false);
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

/// Runs `draw` with the device context cut to `clip`.
///
/// A scrollable container shows one window onto its children, and that window is
/// drawn rather than approximated: this cut is what keeps the part a user
/// scrolled past out of the frame.
unsafe fn with_clip(destination: HDC, clip: LayerRect, draw: impl FnOnce()) {
    let saved = SaveDC(destination);
    let _ = IntersectClipRect(
        destination,
        clip.left,
        clip.top,
        clip.right(),
        clip.bottom(),
    );
    draw();
    if saved != 0 {
        let _ = RestoreDC(destination, saved);
    }
}

unsafe fn draw_layers(destination: HDC, layers: &[ImageLayer]) {
    for layer in layers {
        match layer.clip {
            Some(clip) => with_clip(destination, clip, || draw_layer(destination, layer)),
            None => draw_layer(destination, layer),
        }
    }
}

unsafe fn draw_texts(destination: HDC, texts: &[TextLayer]) {
    for text in texts {
        match text.clip {
            Some(clip) => with_clip(destination, clip, || draw_text(destination, text)),
            None => draw_text(destination, text),
        }
    }
}

unsafe fn draw_ui_frame(destination: HDC, ui: &RuntimeUi) {
    draw_layers(destination, &ui.layers);
    // Selection bands sit above the page art but under the glyphs they cover.
    draw_layers(destination, &ui.selection);
    draw_texts(destination, &ui.texts);
    draw_layers(destination, &ui.overlay_layers);
    draw_texts(destination, &ui.overlay_texts);
    if ui.caret_drawn {
        if let Some(caret) = &ui.caret {
            draw_layers(destination, std::slice::from_ref(caret));
        }
    }
}

fn window_action_at(x: i32, y: i32) -> Option<WindowAction> {
    let state = UI.get()?.lock().ok()?;
    // A dialog is modal: while one is open only its own controls respond, so a
    // click can never reach a page button it happens to be covering.
    let regions = match state.ui.dialog.as_ref() {
        Some(dialog) => (&dialog.text_hits, &dialog.actions),
        None => (&state.ui.text_hits, &state.ui.actions),
    };
    let (text_hits, actions) = regions;
    // Text hits are checked first: a link sits inside a label that may itself
    // overlap a panel, and the innermost target is the one the user aimed at.
    if let Some(hit) = text_hits
        .iter()
        .rev()
        .find(|hit| x >= hit.left && x < hit.right && y >= hit.top && y < hit.bottom)
    {
        return Some(hit.action.clone());
    }
    actions
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
    // A dialog covers the page, so the fields under it are not reachable while
    // it is open. A dialog layout declares no editable field of its own.
    if state.ui.dialog.is_some() {
        return None;
    }
    let field = state.ui.text_inputs.iter().rev().find(|field| {
        let rect = LayerRect {
            left: field.left,
            top: field.top,
            width: field.width,
            height: field.height,
        };
        field.clip.unwrap_or(rect).contains(x, y)
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

/// Where an input method puts the two windows it draws for a caret.
///
/// The composition text starts at the caret, which is where the user is typing,
/// and the candidate list sits one caret's height below it, which is where a
/// user looks for it. Both are read off the caret the page drew, so a field
/// that moved, or a line that scrolled sideways, takes them along.
fn composition_points(caret: LayerRect) -> (POINT, POINT) {
    (
        POINT {
            x: caret.left,
            y: caret.top,
        },
        POINT {
            x: caret.left,
            y: caret.top + caret.height,
        },
    )
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
    let (composition_at, candidate_at) = composition_points(caret);
    let composition = COMPOSITIONFORM {
        dwStyle: CFS_POINT,
        ptCurrentPos: composition_at,
        rcArea: RECT::default(),
    };
    let _ = ImmSetCompositionWindow(context, &composition);
    let candidate = CANDIDATEFORM {
        dwIndex: 0,
        dwStyle: CFS_CANDIDATEPOS,
        ptCurrentPos: candidate_at,
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
    // A dialog is modal, so nothing behind it highlights while it is open.
    let regions = match state.ui.dialog.as_ref() {
        Some(dialog) => &dialog.hover_regions,
        None => &state.ui.hover_regions,
    };
    regions
        .iter()
        .rev()
        .find(|region| x >= region.left && x < region.right && y >= region.top && y < region.bottom)
        .map(|region| region.id.clone())
}

/// What the dialog on screen means, or `None` when none is open.
fn open_dialog_kind() -> Option<DialogKind> {
    UI.get()
        .and_then(|state| state.lock().ok())
        .and_then(|state| state.interaction.dialog.as_ref().map(|dialog| dialog.kind))
}

unsafe fn handle_window_action(window: HWND, action: WindowAction) {
    match action {
        WindowAction::Close => {
            if !install::busy() {
                let _ = DestroyWindow(window);
            }
        }
        WindowAction::CloseConfirm => {
            // The question is drawn inside the window rather than handed to
            // Windows, so it carries the product's skin and cannot end up behind
            // another application.
            if let Err(error) = open_close_confirm_dialog(window) {
                show_runtime_error(&error);
            }
        }
        WindowAction::Cancel => {
            // A cancel button means what it says while a task runs. With
            // nothing running there is nothing to stop, so it leaves the
            // wizard, which is what the same button means on any other page.
            if install::busy() {
                install::request_cancel();
            } else {
                let _ = DestroyWindow(window);
            }
        }
        WindowAction::DialogOk => {
            let kind = open_dialog_kind();
            if let Err(error) = set_dialog(window, None) {
                show_runtime_error(&error);
            } else if kind == Some(DialogKind::CloseConfirm) {
                if install::busy() {
                    // Confirming the question while a task runs is how a project
                    // lets its user stop it: the task gives up at its next
                    // checkpoint and undoes what it wrote.
                    install::request_cancel();
                } else {
                    let _ = DestroyWindow(window);
                }
            } else if kind.is_some() {
                // Confirming a notice, or answering a script's question with
                // yes: the wizard itself stays as it was either way.
                answer_pending_dialog(true);
            }
        }
        WindowAction::DialogCancel => {
            let kind = open_dialog_kind();
            if let Err(error) = set_dialog(window, None) {
                show_runtime_error(&error);
            } else if kind.is_some() {
                // Escape and the dialog's own second button both mean no.
                answer_pending_dialog(false);
            }
        }
        WindowAction::Minimize => {
            let _ = ShowWindow(window, SW_MINIMIZE);
        }
        WindowAction::ToggleSelectMenu { id } => {
            // Clicking the control whose menu is open closes it again.
            let open = UI
                .get()
                .and_then(|state| state.lock().ok())
                .is_some_and(|state| state.open_select.as_deref() != Some(id.as_str()));
            if let Err(error) = set_open_select(window, open.then_some(id)) {
                show_runtime_error(&error);
            }
        }
        WindowAction::SelectLanguage(locale) => {
            if let Err(error) = select_language(window, locale) {
                show_runtime_error(&error);
            }
        }
        WindowAction::ChooseOption { id, value } => {
            if let Err(error) = set_choice(window, id, value) {
                show_runtime_error(&error);
            }
        }
        WindowAction::ScrollPage { id, forward } => {
            if let Err(error) = scroll_container_page(window, &id, forward) {
                show_runtime_error(&error);
            }
        }
        WindowAction::NextPage => {
            if let Err(error) = navigate_forward() {
                show_runtime_error(&error);
            }
        }
        WindowAction::PreviousPage => {
            if let Err(error) = navigate_back() {
                show_runtime_error(&error);
            }
        }
        WindowAction::Install => install::start_install(),
        WindowAction::Uninstall => install::start_uninstall(),
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

/// Moves the keyboard onto a control, and repaints so the ring follows.
unsafe fn set_focused_control(window: HWND, id: Option<String>) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    if state.interaction.focused_control == id {
        return Ok(());
    }
    state.interaction.focused_control = id;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

/// Moves the keyboard to the next control the page lays out.
///
/// Tab walks the order the page itself lays them out in and Shift+Tab walks it
/// backwards; either end wraps, so the keyboard never falls off the ring and a
/// key that does nothing is one the user pressed on an empty page. A text field
/// takes the caret while the ring is on it, which is what lets a user reach a
/// field and type without a mouse, and gives the caret back when the ring
/// leaves.
unsafe fn move_page_focus(window: HWND, backward: bool) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let (id, editable, caret_index) = {
        let state = runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        let regions = &state.ui.focus_regions;
        if regions.is_empty() {
            return Ok(());
        }
        let current = state
            .interaction
            .focused_control
            .as_deref()
            .and_then(|focused| regions.iter().position(|region| region.id == focused));
        // A page whose focus went away -- it was walked to another page, or the
        // control it was on is hidden now -- starts the walk over.
        let index = next_focus_index(regions.len(), current, backward);
        let region = &regions[index];
        // A region with no action is a text field: what a key does there is type.
        let editable = region.action.is_none();
        // Tabbing into a field puts the caret after what it already holds, which
        // is where the next character would go.
        let caret_index = if editable {
            state
                .ui
                .text_inputs
                .iter()
                .find(|field| field.id == region.id)
                .map_or(0, |field| field.text.chars().count())
        } else {
            0
        };
        (region.id.clone(), editable, caret_index)
    };
    focus_text_input(window, editable.then(|| id.clone()), caret_index)?;
    set_focused_control(window, Some(id))
}

/// Runs what the control the keyboard is on does.
///
/// A text field carries no action, so Enter and Space over one are left to the
/// field: there, they are characters like any other.
unsafe fn activate_focused_control(window: HWND) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let action = {
        let state = runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        let focused = state.interaction.focused_control.as_deref();
        state
            .ui
            .focus_regions
            .iter()
            .find(|region| Some(region.id.as_str()) == focused)
            .and_then(|region| region.action.clone())
    };
    if let Some(action) = action {
        handle_window_action(window, action);
    }
    Ok(())
}

/// The place the keyboard moves to in the order a page lays its controls out.
///
/// `current` is where the ring is now, which is `None` while the keyboard has
/// not landed on anything yet: a walk that starts there begins at the end it is
/// walking from. Either end wraps, so the keyboard never falls off the page and
/// a key that does nothing is one the user pressed on a page with nothing on it
/// (`len` is never zero, which is what the caller checks first).
fn next_focus_index(len: usize, current: Option<usize>, backward: bool) -> usize {
    match (current, backward) {
        (Some(index), true) => (index + len - 1) % len,
        (Some(index), false) => (index + 1) % len,
        (None, true) => len - 1,
        (None, false) => 0,
    }
}

/// Whether any select has its menu open.
fn menu_is_open() -> bool {
    UI.get()
        .and_then(|state| state.lock().ok())
        .is_some_and(|state| state.open_select.is_some())
}

/// Moves the highlight, confirms a row, or dismisses the open menu.
unsafe fn handle_menu_key(window: HWND, key: u32) {
    // Virtual-key codes, spelled here so the message handler stays free of
    // another namespace-wide import.
    const VK_RETURN: u32 = 0x0D;
    const VK_ESCAPE: u32 = 0x1B;
    const VK_UP: u32 = 0x26;
    const VK_DOWN: u32 = 0x28;

    match key {
        VK_ESCAPE => {
            let _ = set_open_select(window, None);
        }
        VK_UP | VK_DOWN => {
            let _ = move_menu_highlight(window, key == VK_DOWN);
        }
        VK_RETURN => {
            let Some((select, language, value)) = highlighted_row() else {
                return;
            };
            let result = if language {
                select_language(window, value)
            } else {
                set_choice(window, select, value)
            };
            if let Err(error) = result {
                show_runtime_error(&error);
            }
        }
        _ => {}
    }
}

/// The row the keyboard rests on: the control it belongs to, whether that
/// control lists locales, and the value the row stands for.
fn highlighted_row() -> Option<(String, bool, String)> {
    let state = UI.get()?.lock().ok()?;
    let menu = state.ui.menu.as_ref()?;
    let index = state.interaction.highlighted_option?;
    Some((
        menu.select.clone(),
        menu.language,
        menu.values.get(index)?.clone(),
    ))
}

/// Walks the highlight one row of the open menu, wrapping at both ends.
///
/// The first arrow key starts where the current choice sits, so a menu opens
/// on the value in use and the keyboard moves away from it.
unsafe fn move_menu_highlight(window: HWND, forward: bool) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    let (count, selected) = match state.ui.menu.as_ref() {
        Some(menu) => (menu.values.len(), menu.chosen),
        None => (0, None),
    };
    if count == 0 {
        return Ok(());
    }
    let current = state
        .interaction
        .highlighted_option
        .or(selected)
        .unwrap_or(0);
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

/// Opens the menu of one select, or closes whatever menu is open.
///
/// The keyboard highlight starts empty, which leaves the row the current
/// choice marks standing until an arrow key moves: a menu opens on the value
/// in use rather than on its first row.
unsafe fn set_open_select(window: HWND, select: Option<String>) -> Result<()> {
    let Some(runtime) = UI.get() else {
        return Ok(());
    };
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    if state.open_select == select {
        return Ok(());
    }
    state.open_select = select;
    state.interaction.highlighted_option = None;
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
    state.open_select = None;
    state.interaction.highlighted_option = None;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

/// Records the value the user picked.
///
/// A select's row and a radio button both come here: the id names the choice
/// -- a select of its own, or the group a radio belongs to -- and the value is
/// what that row stands for. Picking a row is also what closes the menu it came
/// from; a radio button has none open, and closing a closed menu changes
/// nothing.
unsafe fn set_choice(window: HWND, id: String, value: String) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    state.interaction.choices.insert(id, value);
    state.open_select = None;
    state.interaction.highlighted_option = None;
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

/// Scrolls a container to the offset a wheel or a click on its scrollbar asked
/// for.
///
/// The offset is stored rather than the movement, so a page that is redrawn for
/// a new display scale, a language change or a dialog keeps showing the part of
/// the list the user left it at.
unsafe fn set_scroll_offset(window: HWND, id: String, offset: i32) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    let mut state = runtime
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    if state.interaction.scroll_offsets.get(&id).copied() == Some(offset) {
        return Ok(());
    }
    state.interaction.scroll_offsets.insert(id, offset);
    rebuild_runtime_ui(&mut state)?;
    drop(state);
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

/// Where the container under a wheel message should end up, and which one it is.
///
/// The pointer decides: a page may hold several scrollable containers, and the
/// one the wheel is over is the one the user aimed at.
unsafe fn scroll_container_at(window: HWND, x: i32, y: i32, delta: i16) -> Result<()> {
    let target = {
        let runtime = UI.get().context("native UI state is missing")?;
        let state = runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        // A dialog owns the window while it is open, so the page behind it does
        // not move under the pointer.
        if state.interaction.dialog.is_some() {
            return Ok(());
        }
        let Some(view) = state
            .ui
            .scroll_views
            .iter()
            .find(|view| view.area.contains(x, y))
        else {
            return Ok(());
        };
        let step = scale_value(SCROLL_STEP, state.dpi.scale);
        let offset = (view.offset - wheel_notches(delta) * step).clamp(0, view.max_offset());
        (view.id.clone(), offset)
    };
    set_scroll_offset(window, target.0, target.1)
}

/// Moves a container by one of its own pages, from a click on its scrollbar.
///
/// A page is the view's own extent, so the row that was at the edge stays in
/// sight at the other one instead of being stepped over.
unsafe fn scroll_container_page(window: HWND, id: &str, forward: bool) -> Result<()> {
    let target = {
        let runtime = UI.get().context("native UI state is missing")?;
        let state = runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        let Some(view) = state.ui.scroll_views.iter().find(|view| view.id == id) else {
            return Ok(());
        };
        let page = view.axis.main(view.viewport);
        let moved = if forward {
            view.offset + page
        } else {
            view.offset - page
        };
        (view.id.clone(), moved.clamp(0, view.max_offset()))
    };
    set_scroll_offset(window, target.0, target.1)
}

/// How many steps one wheel message asks for.
///
/// A wheel that reports part of a notch still moves the list one step: a
/// high-resolution wheel sends small deltas, and a list that ignored them would
/// not move at all under such a mouse.
fn wheel_notches(delta: i16) -> i32 {
    let notches = i32::from(delta) / WHEEL_DELTA;
    if notches == 0 {
        i32::from(delta).signum()
    } else {
        notches
    }
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
    state.open_select = None;
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
        state.open_select.as_deref(),
        &state.interaction,
        state.mode,
        state.contrast,
    )?;
    Ok(())
}

/// Asks the localized close question, drawn inside the installer window.
///
/// Returns as soon as the dialog opens. What confirming it does is decided when
/// the answer arrives, which is what keeps the window's message loop free to
/// keep painting while the question is up.
unsafe fn open_close_confirm_dialog(window: HWND) -> Result<()> {
    let (question, accept, dismiss) = dialog_labels()?;
    // The close question replaces whatever was on screen, and a script's
    // question cannot be answered once it is gone.
    answer_pending_dialog(false);
    set_dialog(
        window,
        Some(DialogState {
            kind: DialogKind::CloseConfirm,
            message: question,
            accept_label: accept,
            dismiss_label: dismiss,
        }),
    )
}

/// The localized question and button labels a dialog uses.
fn dialog_labels() -> Result<(String, String, String)> {
    let state = UI
        .get()
        .context("native UI state is missing")?
        .lock()
        .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
    Ok((
        state.ui.close_confirm_message.clone(),
        state.ui.dialog_accept_label.clone(),
        state.ui.dialog_dismiss_label.clone(),
    ))
}

/// Shows a notice that waits to be acknowledged.
///
/// The runtime uses this instead of a system message box so that a message the
/// installer has to deliver carries the product's skin and stays with the
/// installer window.
pub(crate) fn open_notice_dialog(message: String) -> Result<()> {
    let window = runtime_window()?;
    if window.0.is_null() {
        bail!("the installer window is not open yet");
    }
    let (_, accept, _) = dialog_labels()?;
    // A notice from the runtime replaces what is on screen, and a script's
    // question cannot be answered once it is gone.
    answer_pending_dialog(false);
    unsafe {
        set_dialog(
            window,
            Some(DialogState {
                kind: DialogKind::Notice,
                message,
                accept_label: accept,
                dismiss_label: String::new(),
            }),
        )
    }
}

/// Shows a message the way the product shows its own questions and waits until
/// it is acknowledged.
///
/// Returns whether the message reached the screen: `false` means there is
/// nothing to draw a dialog in, which leaves the caller the system box it used
/// before.
pub(crate) fn show_script_message(title: &str, message: &str) -> bool {
    script_dialog(DialogKind::Notice, title, message).is_some()
}

/// Asks the question `ask_yes_no` puts to the user, in the product's own skin.
///
/// Returns `None` when there is no dialog to ask in -- no window, or a project
/// that ships no dialog layout -- which leaves the caller its system box. The
/// answer is `false` for everything the click does not confirm, including the
/// dialog being replaced by another one or the window going away.
pub(crate) fn ask_script_question(title: &str, message: &str) -> Option<bool> {
    script_dialog(DialogKind::Question, title, message)
}

/// Draws a script's message or question in the installer window and blocks the
/// worker thread until it is answered.
///
/// The script runs on the worker thread, so waiting here blocks the task and not
/// the window: the card keeps painting and its buttons keep taking clicks while
/// the task it belongs to stands still.
fn script_dialog(kind: DialogKind, title: &str, message: &str) -> Option<bool> {
    let window = runtime_window().ok().filter(|window| !window.0.is_null())?;
    // A project that ships no layout for a dialog draws no card, and waiting for
    // one that never appears would hang the task, so the caller keeps the system
    // box it had before this dialog existed.
    if !dialog_layout_present() {
        return None;
    }
    let labels = script_dialog_labels(kind)?;
    // The layout carries one message label, so a title becomes its first line
    // rather than a control the project never declared.
    let message = match title.trim() {
        "" => message.to_string(),
        title => format!("{title}\n\n{message}"),
    };

    // A question that is on screen when this one opens can no longer be
    // answered, so whoever asked it stops waiting rather than waiting forever.
    answer_pending_dialog(false);
    let (sender, receiver) = std::sync::mpsc::channel();
    *DIALOG_REPLY.lock().ok()? = Some(sender);
    let opened = unsafe {
        set_dialog(
            window,
            Some(DialogState {
                kind,
                message,
                accept_label: labels.0,
                dismiss_label: labels.1,
            }),
        )
    };
    if let Err(error) = opened {
        // The caller falls back to the system box, which is what the user sees
        // of a dialog the runtime could not draw.
        eprintln!("cannot draw the dialog a script asked for: {error:#}");
        answer_pending_dialog(false);
        return None;
    }
    // A click that lands while the card is opening answers it before this line
    // is reached, and the answer is waiting in the channel all the same.
    receiver.recv().ok()
}

/// Whether the project ships the layout a script's dialog is drawn from.
///
/// A project names the layout it wants or leaves the default to the runtime, so
/// this reads the same setting the overlay renderer does: when the file is not
/// in the bundle there is no card, and only then does a script keep the system
/// box this dialog replaced.
fn dialog_layout_present() -> bool {
    let Some(state) = UI.get().and_then(|state| state.lock().ok()) else {
        return false;
    };
    let path = state
        .files
        .get("installer_config.json")
        .and_then(|encoded| serde_json::from_slice::<serde_json::Value>(encoded).ok())
        .and_then(|config| config["ui"]["dialog_layout"].as_str().map(str::to_string))
        .unwrap_or_else(|| DEFAULT_DIALOG_LAYOUT.to_string());
    state.files.contains_key(&path)
}

/// The labels a script's dialog asks for.
///
/// A question offers both answers; a message offers only the one that takes it
/// off the screen.
fn script_dialog_labels(kind: DialogKind) -> Option<(String, String)> {
    let state = UI.get()?.lock().ok()?;
    Some(match kind {
        DialogKind::Question => (
            state.ui.dialog_yes_label.clone(),
            state.ui.dialog_no_label.clone(),
        ),
        _ => (state.ui.dialog_accept_label.clone(), String::new()),
    })
}

/// Hands `answer` to the script waiting on the open dialog, if any.
fn answer_pending_dialog(answer: bool) {
    let Ok(mut slot) = DIALOG_REPLY.lock() else {
        return;
    };
    if let Some(sender) = slot.take() {
        let _ = sender.send(answer);
    }
}

/// Replaces the open dialog, if any, and brings the window up to date.
unsafe fn set_dialog(window: HWND, dialog: Option<DialogState>) -> Result<()> {
    let runtime = UI.get().context("native UI state is missing")?;
    {
        let mut state = runtime
            .lock()
            .map_err(|_| anyhow::anyhow!("native UI state lock was poisoned"))?;
        if state.interaction.dialog.is_none() && dialog.is_none() {
            return Ok(());
        }
        state.interaction.dialog = dialog;
        // The dialog takes any text focus the page had, so the caret goes away
        // while a question is up.
        state.interaction.focused_text_input = None;
        rebuild_runtime_ui(&mut state)?;
    }
    let _ = InvalidateRect(window, None, false);
    let _ = UpdateWindow(window);
    Ok(())
}

/// Whether a dialog is waiting for an answer.
fn dialog_is_open() -> bool {
    UI.get()
        .and_then(|state| state.lock().ok())
        .is_some_and(|state| state.interaction.dialog.is_some())
}

/// Opens a resolved URL in the user's default browser./// Opens a resolved URL in the user's default browser.
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
///
/// A direct move -- the page a task reports on, or the first page a failed task
/// returns to -- forgets where the wizard has been. The task owns the window in
/// between, so what follows is a new walk through the pages rather than a
/// continuation of the one that led into it.
pub(crate) fn show_page(index: usize) -> Result<()> {
    update_runtime(|state| {
        state.interaction.page_history.clear();
        state.interaction.page_index = index;
        Ok(())
    })
}

/// Walks the wizard to `index`, remembering the page it leaves behind.
fn walk_to_page(index: usize) -> Result<()> {
    update_runtime(|state| {
        let from = state.interaction.page_index;
        if from != index {
            state.interaction.page_history.push(from);
        }
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

/// Applies `change` to the runtime state, rebuilds the layout, and repaints.
///
/// Worker threads call this; painting itself stays on the UI thread because the
/// repaint is requested through a posted message.
///
/// A task that runs without a window has no runtime state, and the wizard is
/// only ever an observer of one: progress, step text, and the page index exist
/// so a window can draw them. Reporting them is therefore a no-op rather than a
/// failure when nothing is on screen, which is what lets the same deployment
/// code serve a silent run and a wizard alike.
fn update_runtime(change: impl FnOnce(&mut RuntimeState) -> Result<()>) -> Result<()> {
    let Some(runtime) = UI.get() else {
        return Ok(());
    };
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
        anchored_left, anchored_top, button_enabled, button_image, byte_index, caret_layer,
        centered_bounds, clamped_bounds, composition_points, container_intrinsic_size, contrast,
        cross_alignment, cross_alignment_for_item, disk_free_bytes, disk_root, field_state,
        flow_axis, flow_item_for_node, flow_widths, format_size_bytes, forward_page,
        initial_interaction, insets_for_node, inspect_project, installer_version_info, load_layout,
        main_alignment, mask_matches, measure_layout_text_width, next_focus_index, pack_project,
        pack_project_with_progress, page_id, page_index_of_id, parse_bundle, parse_color,
        parse_image_style, parse_text_runs, pick_directory_target, push_action, push_border_layer,
        push_hover_region, push_node_border, query_disk_free_bytes, render_flow, render_flow_item,
        render_progress_bar, resolve_asset_path, resolve_link_target, resolve_value_source,
        resolved_text_for_node, restore_snapshot, runtime_layout_path_at, runtime_page_count,
        runtime_page_index_for_role, scale_value, selection_layers, size_attribute,
        uninstaller_version_info, validate_output_filename, word_end_after, word_range,
        word_start_before, wrap_lines, wraps, BundleIndex, DialogKind, DialogState, DpiContext,
        DpiSettings, FlowAxis, FlowItem, ImageLayer, Insets, InteractionState, LayerRect,
        LayoutContext, LayoutOutput, MoveTrouble, PayloadFormat, RuntimeMode, RuntimeUi,
        TextAlignment, TextHit, TextInputRegion, TextSnapshot, WindowAction, BUNDLE_MAGIC,
        BUNDLE_VERSION, COLORREF, FOOTER_MAGIC, POINT,
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

    /// A project that names a tools directory ships the whole tree inside its
    /// setup, subdirectories and their relative paths included.
    #[test]
    fn a_project_bundles_the_tools_directory_it_names() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path();
        for directory in ["layouts", "assets", "locales", "payload", "tools/bin"] {
            std::fs::create_dir_all(project.join(directory))?;
        }
        std::fs::write(
            project.join("installer_config.json"),
            br#"{"resources":{"payload_file":"payload/app.7z","tools_dir":"tools"}}"#,
        )?;
        std::fs::write(project.join("tools/7za.exe"), b"seven zip")?;
        std::fs::write(project.join("tools/bin/helper.dll"), b"helper")?;
        std::fs::write(project.join("payload/app.7z"), b"payload")?;

        let packed = pack_project(project, None)?;
        let files = parse_bundle(&packed)?;

        // The whole tree travels, subdirectories included, under the paths the
        // project's own directory gives it: that is what a script unpacks and
        // runs.
        assert_eq!(files.get("tools/7za.exe").unwrap(), b"seven zip");
        assert_eq!(files.get("tools/bin/helper.dll").unwrap(), b"helper");
        Ok(())
    }

    /// A project that names no tools directory bundles none, however that
    /// directory looks in its own tree: a setup carries what the project asks
    /// for, not what happens to sit beside it.
    #[test]
    fn a_project_that_names_no_tools_bundles_none() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path();
        for directory in ["layouts", "assets", "locales", "payload", "tools"] {
            std::fs::create_dir_all(project.join(directory))?;
        }
        std::fs::write(
            project.join("installer_config.json"),
            br#"{"resources":{"payload_file":"payload/app.7z"}}"#,
        )?;
        // The directory sits in the project whatever the configuration says;
        // only the setting puts it in the bundle.
        std::fs::write(project.join("tools/7za.exe"), b"seven zip")?;
        std::fs::write(project.join("payload/app.7z"), b"payload")?;

        let packed = pack_project(project, None)?;
        let files = parse_bundle(&packed)?;

        assert!(
            !files.keys().any(|name| name.starts_with("tools/")),
            "a tools directory the project never named ended up in the bundle"
        );
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

    /// An entry whose bytes changed after the build is refused by name, and a
    /// streamed copy of it leaves no file behind: the step that would have
    /// unpacked it must not run against bytes nobody vouched for.
    #[test]
    fn a_damaged_bundle_entry_is_refused_and_leaves_no_copy() -> anyhow::Result<()> {
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
        let payload = b"the payload as the build stored it";
        std::fs::write(project.join("payload/app.7z"), payload)?;

        let image = temp.path().join("setup.exe");
        write_setup_image(&image, &pack_project(&project, None)?)?;

        // One byte flipped, the way a bad sector or a truncated transfer leaves
        // a setup: the index still describes the payload exactly as the build
        // recorded it, so the entry is found and its own bytes are what fails.
        let mut bytes = std::fs::read(&image)?;
        let at = bytes
            .windows(payload.len())
            .position(|window| window == &payload[..])
            .context("the payload is in the image")?;
        bytes[at] ^= 0x20;
        std::fs::write(&image, &bytes)?;

        let index =
            BundleIndex::read(&image)?.context("the damaged setup still carries a bundle")?;
        assert!(index.contains("payload/app.7z"));
        let error = index
            .read_file("payload/app.7z")
            .expect_err("damaged bytes are not handed to a caller")
            .to_string();
        assert!(
            error.contains("payload/app.7z") && error.contains("is damaged"),
            "the failure should name the entry and what is wrong with it: {error}"
        );
        // The entries that arrived intact still read, so one damaged file does
        // not put the rest of the bundle out of reach.
        assert_eq!(index.read_file("layouts/config.xml")?, b"layout");

        let streamed = temp.path().join("streamed.7z");
        assert!(index.copy_file_to("payload/app.7z", &streamed).is_err());
        assert!(
            !streamed.exists(),
            "a damaged copy stayed at {}",
            streamed.display()
        );
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

    /// A signed setup is still a setup. Authenticode appends its certificate
    /// table behind everything the build wrote, and a footer read from the last
    /// bytes of the file reported exactly that as "no bundle at all".
    #[test]
    fn bundle_index_reads_a_bundle_that_a_signature_follows() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let image = temp.path().join("setup.exe");
        let mut bundle = BUNDLE_MAGIC.to_vec();
        bundle.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
        bundle.extend_from_slice(&0u32.to_le_bytes());
        write_setup_image(&image, &bundle)?;

        let mut signed = std::fs::read(&image)?;
        signed.extend_from_slice(&[0x5Au8; 1024]);
        std::fs::write(&image, signed)?;

        let index = BundleIndex::read(&image)?.context("the bundle is behind the signature")?;
        assert!(index.files.is_empty());
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
            vec![(
                "runtime/uninst-stub-native.exe".to_string(),
                b"uninstaller".to_vec(),
            )],
            None,
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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

    /// A layout and the translation table a dialog test needs.
    fn dialog_fixture() -> (HashMap<String, Vec<u8>>, serde_json::Value) {
        let config = serde_json::json!({
            "ui": { "dialog_layout": "layouts/msgBox.xml" },
            "resources": { "locales_dir": "locales" },
            "wizard": { "pages": [{ "layout": "layouts/page.xml" }] }
        });
        let mut files = HashMap::new();
        files.insert(
            "installer_config.json".to_string(),
            serde_json::to_vec(&config).expect("config serialises"),
        );
        files.insert(
            "layouts/page.xml".to_string(),
            br##"<Page width="720" height="450" background="#FF000000" />"##.to_vec(),
        );
        files.insert(
            "layouts/msgBox.xml".to_string(),
            br##"<Page width="400" height="230" background="#FF2A3844" border-radius="16">
                 <Label id="lblMsg" text="placeholder" value-source="dialog:message" width="336" height="24" />
                 <Button id="btnCancel" action="dialog_cancel" visible-with="dismiss"
                         text="@cancel" value-source="dialog:dismiss" width="160" height="40" />
                 <Button id="btnOK" action="dialog_ok" text="@ok" value-source="dialog:accept"
                         width="160" height="40" />
               </Page>"##
                .to_vec(),
        );
        files.insert(
            "locales/zh-CN.json".to_string(),
            br##"{"ok":"Exit","cancel":"Keep going","close_confirm_message":"Leave the wizard?"}"##
                .to_vec(),
        );
        (files, config)
    }

    /// A dialog card whose two buttons are placed where a case can click them.
    ///
    /// The shipped layout is a flow of spacers, which is right for a product and
    /// unusable for a test that has to know where a button is: the two buttons
    /// here sit on one line at fixed coordinates.
    const CARD: &str = r##"<Page width="400" height="180" background="#FF2A3844">
  <Label id="lblMsg" text="@ok" value-source="dialog:message"
         position="absolute" left="20" top="20" width="360" height="60" wrap="true" />
  <Button id="btnNo" action="dialog_cancel" visible-with="dismiss"
          text="@cancel" value-source="dialog:dismiss"
          position="absolute" left="40" top="120" width="140" height="36" />
  <Button id="btnYes" action="dialog_ok"
          text="@ok" value-source="dialog:accept"
          position="absolute" left="220" top="120" width="140" height="36" />
</Page>"##;

    /// The interaction state with a dialog open, which is what makes a layout
    /// render its dialog branch.
    fn interaction_with_dialog(dialog: DialogState) -> InteractionState {
        InteractionState {
            dialog: Some(dialog),
            ..Default::default()
        }
    }

    fn notice() -> DialogState {
        DialogState {
            kind: DialogKind::Notice,
            message: "Everything finished".to_string(),
            accept_label: "OK".to_string(),
            dismiss_label: String::new(),
        }
    }

    fn close_question() -> DialogState {
        DialogState {
            kind: DialogKind::CloseConfirm,
            message: "Leave the wizard?".to_string(),
            accept_label: "Exit".to_string(),
            dismiss_label: "Keep going".to_string(),
        }
    }

    #[test]
    fn a_dialog_is_drawn_over_the_page_and_centred() -> anyhow::Result<()> {
        let (files, _) = dialog_fixture();
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(close_question()),
            RuntimeMode::Installer,
            None,
        )?;
        let dialog = ui.dialog.expect("the layout reported a dialog");

        // The question is drawn with the dialog's own text, not the placeholder
        // the layout was authored with.
        let question = ui
            .overlay_texts
            .iter()
            .map(visible_text)
            .any(|text| text == "Leave the wizard?");
        assert!(question, "the dialog question was not drawn");

        // A dialog smaller than its page is centred inside it, so its own layout
        // can use the ordinary coordinate system.
        let expected_left = (720 - 400) / 2;
        let expected_top = (450 - 230) / 2;
        assert!(dialog
            .actions
            .iter()
            .any(|region| region.left >= expected_left && region.top >= expected_top));
        for region in &dialog.actions {
            assert!(region.left >= expected_left && region.right <= 720 - expected_left);
            assert!(region.top >= expected_top && region.bottom <= 450 - expected_top);
        }
        Ok(())
    }

    /// A question a project script asks is drawn with both of its answers, in
    /// the words the script and the locale give it.
    ///
    /// `ask_yes_no` waits for the answer, so the card it waits on has to carry
    /// both buttons: a question whose second answer was not drawn would leave
    /// the script with one answer and no way to give the other.
    #[test]
    fn a_question_a_script_asks_is_drawn_with_both_of_its_answers() -> anyhow::Result<()> {
        let (mut files, _) = dialog_fixture();
        files.insert("layouts/msgBox.xml".to_string(), CARD.as_bytes().to_vec());
        let question = DialogState {
            kind: DialogKind::Question,
            message: "Install this product?".to_string(),
            accept_label: "Yes".to_string(),
            dismiss_label: "No".to_string(),
        };
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(question),
            RuntimeMode::Installer,
            None,
        )?;
        let dialog = ui.dialog.expect("the layout reported a dialog");
        let yes = dialog
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::DialogOk))
            .context("the question drew no button that agrees")?;
        let no = dialog
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::DialogCancel))
            .context("the question drew no button that refuses")?;
        // The card is centred on the 720x450 page, which puts the button the
        // layout places at 220,120 inside the card at 220 + 160, 120 + 135.
        assert_eq!((yes.left, yes.top), (380, 255));
        assert_eq!((no.left, no.top), (200, 255));
        let words: Vec<String> = ui.overlay_texts.iter().map(visible_text).collect();
        assert!(
            words.contains(&"Install this product?".to_string()),
            "the question the script asked is not what the card shows: {words:?}"
        );
        assert!(
            words.contains(&"Yes".to_string()) && words.contains(&"No".to_string()),
            "the two answers are not the words the script gave them: {words:?}"
        );
        Ok(())
    }

    #[test]
    fn a_dialog_button_answers_with_its_own_action() -> anyhow::Result<()> {
        let (files, _) = dialog_fixture();
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(close_question()),
            RuntimeMode::Installer,
            None,
        )?;
        let dialog = ui.dialog.expect("the layout reported a dialog");
        assert!(dialog
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::DialogOk)));
        assert!(dialog
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::DialogCancel)));
        Ok(())
    }

    #[test]
    fn a_notice_hides_the_secondary_button() -> anyhow::Result<()> {
        // A notice has only one answer, so the layout's cancel button is not
        // drawn and cannot be clicked. That is how one dialog layout serves both
        // a question and a notice.
        let (files, _) = dialog_fixture();
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(notice()),
            RuntimeMode::Installer,
            None,
        )?;
        let dialog = ui.dialog.expect("the layout reported a dialog");
        assert!(!dialog
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::DialogCancel)));
        assert!(dialog
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::DialogOk)));

        // The close question draws it again.
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(close_question()),
            RuntimeMode::Installer,
            None,
        )?;
        let dialog = ui.dialog.expect("the layout reported a dialog");
        assert!(dialog
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::DialogCancel)));
        Ok(())
    }

    #[test]
    fn a_page_without_a_dialog_draws_no_overlay() -> anyhow::Result<()> {
        let (files, _) = dialog_fixture();
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &InteractionState::default(),
            RuntimeMode::Installer,
            None,
        )?;
        assert!(ui.dialog.is_none());
        assert!(ui.overlay_layers.is_empty());
        assert!(ui.overlay_texts.is_empty());
        Ok(())
    }

    #[test]
    fn a_project_without_a_dialog_layout_still_opens() -> anyhow::Result<()> {
        // A project that ships no dialog layout keeps working: the page draws on
        // its own and the caller falls back to closing without asking.
        let (mut files, _) = dialog_fixture();
        files.remove("layouts/msgBox.xml");
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(close_question()),
            RuntimeMode::Installer,
            None,
        )?;
        assert!(ui.dialog.is_none());
        assert!(ui.overlay_layers.is_empty());
        Ok(())
    }

    #[test]
    fn the_example_dialog_places_its_message_and_both_buttons() -> anyhow::Result<()> {
        // The example's own dialog layout, so a change to either the layout or
        // the flow measurement that drops a percentage-sized row shows up here
        // rather than only on screen.
        let (mut files, _) = dialog_fixture();
        files.insert(
            "layouts/msgBox.xml".to_string(),
            include_bytes!("../../../examples/TapTap/layouts/msgBox.xml").to_vec(),
        );
        files.insert(
            "assets/btn_dialog.png".to_string(),
            include_bytes!("../../../examples/TapTap/assets/btn_dialog.png").to_vec(),
        );
        files.insert(
            "assets/btn_dialog_primary.png".to_string(),
            include_bytes!("../../../examples/TapTap/assets/btn_dialog_primary.png").to_vec(),
        );
        // The example's own translations, read the way the runtime reads them
        // when it opens the question.
        let locale: HashMap<String, String> = serde_json::from_slice(include_bytes!(
            "../../../examples/TapTap/locales/zh-CN.json"
        ))?;
        let question = locale
            .get("close_confirm_message")
            .context("the locale has no close question")?
            .clone();
        let dialog = DialogState {
            kind: DialogKind::CloseConfirm,
            message: question.clone(),
            accept_label: locale.get("ok").cloned().unwrap_or_default(),
            dismiss_label: locale.get("cancel").cloned().unwrap_or_default(),
        };
        files.insert(
            "locales/zh-CN.json".to_string(),
            include_bytes!("../../../examples/TapTap/locales/zh-CN.json").to_vec(),
        );
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(dialog),
            RuntimeMode::Installer,
            None,
        )?;

        // The question is one wrapped line with real height, not a collapsed
        // row, and it sits above the buttons.
        let message = ui
            .overlay_texts
            .iter()
            .find(|layer| visible_text(layer) == question)
            .with_context(|| format!("the dialog question {question:?} was not drawn"))?;
        assert!(
            message.height > 0,
            "the question was laid out with no height"
        );
        assert!(
            message.width > 0 && message.left > 0,
            "the question was not placed inside the dialog"
        );

        // Both answers are drawn, and they are side by side rather than stacked
        // on top of each other.
        let dialog = ui.dialog.expect("the layout reported a dialog");
        let ok = dialog
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::DialogOk))
            .context("no accept button")?;
        let cancel = dialog
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::DialogCancel))
            .context("no dismiss button")?;
        assert!(ok.right > ok.left && ok.bottom > ok.top);
        assert!(cancel.right > cancel.left && cancel.bottom > cancel.top);
        assert!(
            cancel.right <= ok.left || ok.right <= cancel.left,
            "the dialog buttons overlap: cancel={}..{} and accept={}..{}",
            cancel.left,
            cancel.right,
            ok.left,
            ok.right
        );
        // The question is not covered by its own answers.
        assert!(
            message.top + message.height <= ok.top.max(cancel.top),
            "the question overlaps the buttons"
        );
        Ok(())
    }

    /// The dialog answers have to stay inside the card around them.
    ///
    /// The question a dialog shows comes from the product's own translations,
    /// so a longer sentence wraps onto another line. The card used to be a
    /// fixed height with the answers flush against its bottom edge, which a
    /// two-line question pushed out through the frame. Every shipped locale is
    /// checked here, because the ones with the longest sentences are exactly
    /// the ones that only break on a user's machine.
    #[test]
    fn every_shipped_question_keeps_its_answers_inside_the_card() -> anyhow::Result<()> {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap");
        let layout = std::fs::read(project.join("layouts/msgBox.xml"))?;
        let mut checked = 0;
        for entry in std::fs::read_dir(project.join("locales"))? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let locale_name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .context("locale file name is not Unicode")?
                .to_string();
            // The example's own translations, read the way the runtime reads
            // them when it opens the question.
            let locale: HashMap<String, String> = serde_json::from_slice(&std::fs::read(&path)?)?;
            let (mut files, _) = dialog_fixture();
            files.insert("layouts/msgBox.xml".to_string(), layout.clone());
            files.insert(
                "assets/btn_dialog.png".to_string(),
                std::fs::read(project.join("assets/btn_dialog.png"))?,
            );
            files.insert(
                "assets/btn_dialog_primary.png".to_string(),
                std::fs::read(project.join("assets/btn_dialog_primary.png"))?,
            );
            files.insert(format!("locales/{locale_name}.json"), std::fs::read(&path)?);
            let dialog = DialogState {
                kind: DialogKind::CloseConfirm,
                message: locale
                    .get("close_confirm_message")
                    .cloned()
                    .unwrap_or_default(),
                accept_label: locale.get("ok").cloned().unwrap_or_default(),
                dismiss_label: locale.get("cancel").cloned().unwrap_or_default(),
            };
            let ui = load_layout(
                &files,
                DpiContext {
                    scale: 1.0,
                    use_2x: false,
                },
                &locale_name,
                None,
                &interaction_with_dialog(dialog),
                RuntimeMode::Installer,
                None,
            )?;

            let question = ui
                .overlay_texts
                .iter()
                .find(|layer| layer.wrap)
                .with_context(|| format!("{locale_name}: the question was not drawn"))?;
            assert!(
                question.height > 0,
                "{locale_name}: the question was laid out with no height"
            );

            // The card is the fill the layout paints behind its controls.
            let card = ui
                .overlay_layers
                .iter()
                .find(|layer| layer.width == 400)
                .with_context(|| format!("{locale_name}: the dialog card was not drawn"))?;
            let regions = ui.dialog.expect("the layout reported a dialog");
            let ok = regions
                .actions
                .iter()
                .find(|region| matches!(region.action, WindowAction::DialogOk))
                .with_context(|| format!("{locale_name}: no accept button"))?;
            let cancel = regions
                .actions
                .iter()
                .find(|region| matches!(region.action, WindowAction::DialogCancel))
                .with_context(|| format!("{locale_name}: no dismiss button"))?;
            let answers_top = ok.top.min(cancel.top);
            let answers_bottom = ok.bottom.max(cancel.bottom);

            // The reported bug: the answers sat on the card's bottom edge, so
            // the space below them was the border rather than a margin.
            assert!(
                card.top + card.height - answers_bottom >= 16,
                "{locale_name}: the answers are flush with the card bottom \
                 (card ends at {}, answers end at {answers_bottom})",
                card.top + card.height
            );
            // Both answers are inside the card horizontally too.
            for region in &regions.actions {
                assert!(
                    region.left >= card.left && region.right <= card.left + card.width,
                    "{locale_name}: an answer is outside the card horizontally"
                );
            }
            // A wrapped question never runs into the answers below it, and a
            // taller question makes the card grow instead of moving them down.
            assert!(
                question.top + question.height <= answers_top,
                "{locale_name}: the question overlaps the answers"
            );
            assert!(
                card.height >= 180,
                "{locale_name}: the card dropped below its declared height"
            );
            // The card stays centred on the page it covers.
            let page = (ui.width, ui.height);
            assert_eq!(
                card.left,
                (page.0 - card.width) / 2,
                "{locale_name}: the card is not horizontally centred"
            );
            assert_eq!(
                card.top,
                (page.1 - card.height) / 2,
                "{locale_name}: the card is not vertically centred"
            );
            checked += 1;
        }
        assert!(checked >= 11, "only {checked} locales were checked");
        Ok(())
    }

    #[test]
    fn a_display_scales_the_layout_by_its_own_dpi() {
        let aware = DpiSettings {
            aware: true,
            threshold: 144,
        };
        let base = DpiContext::for_dpi(96, aware);
        assert_eq!(base.scale, 1.0);
        assert!(!base.use_2x);

        // 150% and 200% displays, the two the runtime has to look right on.
        let middle = DpiContext::for_dpi(144, aware);
        assert_eq!(middle.scale, 1.5);
        assert!(middle.use_2x);
        let large = DpiContext::for_dpi(192, aware);
        assert_eq!(large.scale, 2.0);
        assert!(large.use_2x);

        // Below the threshold the 1x artwork is still the sharper choice.
        let just_under = DpiContext::for_dpi(120, aware);
        assert_eq!(just_under.scale, 1.25);
        assert!(!just_under.use_2x);

        // A project that turned scaling off stays on the 96 DPI baseline
        // whichever display it lands on, so the shell does the scaling.
        let unaware = DpiSettings {
            aware: false,
            threshold: 144,
        };
        let off = DpiContext::for_dpi(384, unaware);
        assert_eq!(off.scale, 1.0);
        assert!(!off.use_2x);
    }

    #[test]
    fn a_placed_window_is_pulled_back_inside_its_work_area() {
        let work = (0, 0, 1920, 1080);
        // A suggestion that already fits is left where it is, so a window the
        // user dragged to a second display does not jump back to a corner.
        assert_eq!(
            clamped_bounds(work, 400, 200, 720, 450),
            (400, 200, 720, 450)
        );
        // A window hanging off the right and bottom edges is pulled in.
        assert_eq!(
            clamped_bounds(work, 1800, 1000, 720, 450),
            (1200, 630, 720, 450)
        );
        // A negative suggestion, as a monitor left of the primary produces.
        let left_monitor = (-1920, 0, 0, 1080);
        assert_eq!(
            clamped_bounds(left_monitor, -2000, -50, 720, 450),
            (-1920, 0, 720, 450)
        );
        // A window larger than the work area is shrunk to it rather than
        // sticking out of an edge.
        assert_eq!(
            clamped_bounds(work, 100, 100, 4000, 3000),
            (0, 0, 1920, 1080)
        );
    }

    #[test]
    fn a_window_is_centred_and_clamped_to_its_work_area() {
        // An ordinary desktop: the window sits in the middle.
        assert_eq!(
            centered_bounds((0, 0, 1920, 1080), 720, 450),
            (600, 315, 720, 450)
        );
        // A work area that does not start at the origin, as a second monitor or
        // a taskbar on the left produces, keeps its own offset.
        assert_eq!(
            centered_bounds((-1920, 0, 0, 1040), 720, 450),
            (-1320, 295, 720, 450)
        );
        // A layout larger than the desktop is clamped instead of hanging off an
        // edge. This is the case that used to resolve to the top-left corner.
        assert_eq!(
            centered_bounds((0, 0, 1440, 900), 2880, 1800),
            (0, 0, 1440, 900)
        );
        // Only one axis overflowing still centres the other.
        assert_eq!(
            centered_bounds((0, 0, 1440, 1000), 2000, 400),
            (0, 300, 1440, 400)
        );
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
                &mut Vec::new(),
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
    fn a_container_measures_the_edge_its_children_are_asked_for() -> anyhow::Result<()> {
        // An HBox stretches its children vertically, so asking it for a height
        // has to report the tallest child height. Reading the children's own
        // cross sizes instead reported the width, which collapsed every row in
        // a dialog: the row was laid out at zero height and its text was lost.
        let files: HashMap<String, Vec<u8>> = HashMap::new();
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
        };
        let document = roxmltree::Document::parse(
            r##"<Page width="400" height="200">
                  <HBox width="336" padding="0 32">
                    <Label id="lblMsg" text="hi" wrap="true" width="272" height="24" />
                  </HBox>
                </Page>"##,
        )?;
        let hbox = document
            .descendants()
            .find(|node| node.has_tag_name("HBox"))
            .context("hbox missing")?;

        // Height: the row is as tall as its content plus its vertical padding.
        assert_eq!(
            container_intrinsic_size(hbox, FlowAxis::Vertical, &context),
            24
        );
        // Width: the row reports what its children ask for along the width.
        assert_eq!(
            container_intrinsic_size(hbox, FlowAxis::Horizontal, &context),
            272 + 64
        );

        // A container nested in the row is measured through its own children, so
        // the outer row is not resized by a percentage-sized inner wrapper.
        let document = roxmltree::Document::parse(
            r##"<Page width="400" height="200">
                  <HBox width="336" padding="0 32">
                    <VBox width="100%">
                      <Label id="lblMsg" text="hi" height="24" />
                    </VBox>
                  </HBox>
                </Page>"##,
        )?;
        let hbox = document
            .descendants()
            .find(|node| node.has_tag_name("HBox"))
            .context("hbox missing")?;
        assert_eq!(
            container_intrinsic_size(hbox, FlowAxis::Vertical, &context),
            24
        );
        Ok(())
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
            None,
            &default_interaction,
            RuntimeMode::Installer,
            None,
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
            Some("langSelect"),
            &default_interaction,
            RuntimeMode::Installer,
            None,
        )?;
        assert_eq!(open_menu.overlay_layers.len(), 2);
        // The rows the keyboard walks are the layout's own option order.
        let menu = open_menu.menu.as_ref().expect("the menu was open");
        assert_eq!(menu.values, ["zh-CN", "en-US", "ru"]);
        assert!(menu.language);
        assert_eq!(menu.chosen, Some(0));
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
            None,
            &default_interaction,
            RuntimeMode::Installer,
            None,
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
            None,
            &interaction,
            RuntimeMode::Installer,
            None,
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
            None,
            &interaction,
            RuntimeMode::Installer,
            None,
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
        assert_eq!(
            button_image(button, &interaction, &HashMap::new()),
            Some("disabled.png")
        );

        interaction
            .checkbox_states
            .insert("terms".to_string(), true);
        assert_eq!(
            button_image(button, &interaction, &HashMap::new()),
            Some("normal.png")
        );
        interaction.hovered_control = Some("btnInstall".to_string());
        assert_eq!(
            button_image(button, &interaction, &HashMap::new()),
            Some("hover.png")
        );
        interaction.pressed_control = Some("btnInstall".to_string());
        assert_eq!(
            button_image(button, &interaction, &HashMap::new()),
            Some("pressed.png")
        );

        let independent_document = roxmltree::Document::parse(
            r#"<Button id="standalone" action="install" normal-image="normal.png"
                       disabled-image="disabled.png" />"#,
        )?;
        assert_eq!(
            button_image(
                independent_document.root_element(),
                &InteractionState::default(),
                &HashMap::new()
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
            runtime_layout_path_at(&config, RuntimeMode::Installer, 0)?,
            "layouts/configpage.xml"
        );
        assert_eq!(
            runtime_layout_path_at(&config, RuntimeMode::Uninstaller, 0)?,
            "layouts/uninstallpage.xml"
        );
        Ok(())
    }

    #[test]
    fn a_page_role_finds_the_page_that_holds_it() {
        let config = serde_json::json!({
            "wizard": {
                "pages": [
                    {"id": "licence", "layout": "layouts/licence.xml"},
                    {"id": "options", "layout": "layouts/options.xml"},
                    {"id": "task", "layout": "layouts/installingpage.xml", "role": "progress"},
                    {"id": "done", "layout": "layouts/finishpage.xml", "role": "finish"}
                ]
            }
        });
        // The task reports on the page that says so, not on the second one.
        assert_eq!(
            runtime_page_index_for_role(&config, RuntimeMode::Installer, "progress"),
            Some(2)
        );
        assert_eq!(
            runtime_page_index_for_role(&config, RuntimeMode::Installer, "finish"),
            Some(3)
        );
    }

    #[test]
    fn a_page_list_without_roles_keeps_its_positions() {
        let config = serde_json::json!({
            "wizard": {
                "pages": [
                    {"layout": "layouts/configpage.xml"},
                    {"layout": "layouts/installingpage.xml"},
                    {"layout": "layouts/finishpage.xml"}
                ]
            }
        });
        assert_eq!(
            runtime_page_index_for_role(&config, RuntimeMode::Installer, "progress"),
            Some(1)
        );
        assert_eq!(
            runtime_page_index_for_role(&config, RuntimeMode::Installer, "finish"),
            Some(2)
        );
        // One page has nowhere to report to and nothing to finish.
        let single = serde_json::json!({
            "wizard": { "pages": [{"layout": "layouts/configpage.xml"}] }
        });
        assert_eq!(
            runtime_page_index_for_role(&single, RuntimeMode::Installer, "progress"),
            None
        );
        assert_eq!(
            runtime_page_index_for_role(&single, RuntimeMode::Installer, "finish"),
            None
        );
    }

    /// The pages the cases below walk, with the ids a page hook addresses.
    fn hook_pages() -> Vec<serde_json::Value> {
        serde_json::json!([
            {"id": "welcome", "layout": "layouts/welcome.xml"},
            {"id": "licence", "layout": "layouts/licence.xml"},
            {"id": "options", "layout": "layouts/options.xml"},
            {"id": "tasks", "layout": "layouts/tasks.xml", "role": "progress"}
        ])
        .as_array()
        .cloned()
        .unwrap_or_default()
    }

    /// What a page hook answered: a page it named, nothing, or a failure.
    fn named(id: &str) -> anyhow::Result<Option<String>> {
        Ok(Some(id.to_string()))
    }

    fn silent() -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    fn failed() -> anyhow::Result<Option<String>> {
        Err(anyhow::anyhow!("scripts/pages.rhai failed: no page"))
    }

    #[test]
    fn a_page_hook_sends_the_wizard_to_the_page_it_names() {
        // The hook skips the licence page from the welcome page.
        let (target, trouble) = forward_page(&hook_pages(), 0, named("options"));
        assert_eq!(target, Some(2));
        assert!(trouble.is_none());
        // A page the hook says nothing about keeps the declared order.
        let (target, trouble) = forward_page(&hook_pages(), 2, silent());
        assert_eq!(target, Some(3));
        assert!(trouble.is_none());
    }

    /// A hook that names a page the project does not declare, or the page the
    /// wizard is already on, is a mistake: the wizard says so and walks on, so
    /// that a project's bug never traps the person running it.
    #[test]
    fn a_page_hook_that_names_no_page_keeps_the_declared_order() {
        for chosen in [named("nothing"), named("welcome"), failed()] {
            let (target, trouble) = forward_page(&hook_pages(), 0, chosen);
            assert_eq!(
                target,
                Some(1),
                "the wizard did not fall back to the next page"
            );
            assert!(
                matches!(trouble, Some(MoveTrouble::Hook(_))),
                "the hook's mistake was not reported"
            );
        }
    }

    /// The last declared page is the end of the walk, and the wizard says so
    /// rather than moving somewhere the project never declared.
    #[test]
    fn the_end_of_the_declared_order_is_an_end() {
        let (target, trouble) = forward_page(&hook_pages(), 3, silent());
        assert_eq!(target, None);
        assert!(matches!(trouble, Some(MoveTrouble::End(_))));
        // A hook that names a page still moves the wizard from the last one.
        let (target, trouble) = forward_page(&hook_pages(), 3, named("licence"));
        assert_eq!(target, Some(1));
        assert!(trouble.is_none());
    }

    #[test]
    fn a_page_id_finds_the_page_that_carries_it() {
        assert_eq!(page_index_of_id(&hook_pages(), "options"), Some(2));
        assert_eq!(page_index_of_id(&hook_pages(), "tasks"), Some(3));
        assert_eq!(page_index_of_id(&hook_pages(), "nothing"), None);
        assert_eq!(page_id(&hook_pages(), 1), "licence");
        // A page a project gave no id is not one a hook can name.
        let anonymous = serde_json::json!([{"layout": "layouts/a.xml"}]);
        assert_eq!(page_id(anonymous.as_array().unwrap(), 0), "");
    }

    #[test]
    fn taptap_uninstaller_buttons_have_distinct_hit_regions() -> anyhow::Result<()> {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/TapTap");
        let bundle = pack_project_with_progress(&project, Vec::new(), None, false, |_| {})?;
        let files = parse_bundle(&bundle)?;
        let interaction = initial_interaction(&files, RuntimeMode::Uninstaller)?;
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction,
            RuntimeMode::Uninstaller,
            None,
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
        let bundle = pack_project_with_progress(&project, Vec::new(), None, false, |_| {})?;
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
            None,
            &interaction,
            mode,
            None,
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
            clip: None,
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
            clip: None,
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
    fn an_input_method_anchors_at_the_caret_the_page_drew() -> anyhow::Result<()> {
        // An East Asian input method keeps windows of its own and only asks the
        // runtime where the caret is. The composition starts at that caret and
        // the candidate list sits one caret's height below it, which is where a
        // user looks for the candidates while typing.
        let files = one_page_project(
            r##"<Page width="400" height="200">
                  <TextInput id="path" position="absolute" left="40" top="60" width="300" height="26" />
                </Page>"##,
            "{}",
        );
        let empty = drawn_at_96(&files, &focused_field("path", "", 0))?;
        let at_start = empty.caret_rect.context("a focused field has a caret")?;
        assert_eq!(at_start.left, 40);
        assert_eq!(at_start.height, 18);
        assert_eq!(at_start.top, 64);
        assert_eq!(
            composition_points(at_start),
            (POINT { x: 40, y: 64 }, POINT { x: 40, y: 82 })
        );

        // A caret further along the line takes both windows with it, so the
        // candidate list stays under the character being composed rather than
        // under the first one.
        let typed = drawn_at_96(&files, &focused_field("path", "C:\\Apps", 7))?;
        let at_end = typed.caret_rect.context("a focused field has a caret")?;
        assert!(at_end.left > at_start.left);
        let (composition, candidate) = composition_points(at_end);
        assert_eq!(
            composition,
            POINT {
                x: at_end.left,
                y: at_end.top
            }
        );
        assert_eq!(
            candidate,
            POINT {
                x: at_end.left,
                y: at_end.top + at_end.height
            }
        );
        Ok(())
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
            let bundle = pack_project_with_progress(&project, Vec::new(), None, false, |_| {})?;
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
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

    // Behaviour the layout guide documents, tested attribute by attribute.
    //
    // The guide is a contract with the projects that write these layouts, so a
    // row it marks as supported has to keep working: these cases pin each claim
    // down rather than leaving it to the example layouts, which only cover the
    // combinations that particular product chose.

    /// One PNG from the example project, which is what the image cases draw.
    ///
    /// The example ships both densities for every file it uses, which is what
    /// lets the same artwork prove both sides of the density choice.
    fn example_asset(name: &str) -> Vec<u8> {
        let bytes: &[u8] = match name {
            "logo.png" => include_bytes!("../../../examples/TapTap/assets/logo.png"),
            "logo@2x.png" => include_bytes!("../../../examples/TapTap/assets/logo@2x.png"),
            "bg_main.png" => include_bytes!("../../../examples/TapTap/assets/bg_main.png"),
            "checkbox-0.png" => include_bytes!("../../../examples/TapTap/assets/checkbox-0.png"),
            "checkbox-2.png" => include_bytes!("../../../examples/TapTap/assets/checkbox-2.png"),
            "bar_installing.png" => {
                include_bytes!("../../../examples/TapTap/assets/bar_installing.png")
            }
            "select-arrow.png" => {
                include_bytes!("../../../examples/TapTap/assets/select-arrow.png")
            }
            "select-arrow-up.png" => {
                include_bytes!("../../../examples/TapTap/assets/select-arrow-up.png")
            }
            other => panic!("the example project ships no asset called {other}"),
        };
        bytes.to_vec()
    }

    /// An in-memory project with one page and one locale file, which is the
    /// smallest thing a layout can be loaded from.
    fn one_page_project_with(
        config: serde_json::Value,
        layout: &str,
        locale: &str,
    ) -> HashMap<String, Vec<u8>> {
        let mut files = HashMap::new();
        files.insert(
            "installer_config.json".to_string(),
            serde_json::to_vec(&config).expect("the config serialises"),
        );
        files.insert("layouts/page.xml".to_string(), layout.as_bytes().to_vec());
        files.insert("locales/zh-CN.json".to_string(), locale.as_bytes().to_vec());
        files
    }

    /// The project file every layout case that does not care about it starts
    /// from: one page, and the locale directory the runtime reads.
    fn one_page_config() -> serde_json::Value {
        serde_json::json!({
            "resources": { "locales_dir": "locales" },
            "wizard": { "pages": [{ "layout": "layouts/page.xml" }] }
        })
    }

    fn one_page_project(layout: &str, locale: &str) -> HashMap<String, Vec<u8>> {
        one_page_project_with(one_page_config(), layout, locale)
    }

    /// Draws a project on a display of the given scale, which is what the
    /// density and scaling claims are about.
    fn drawn_for_display(
        files: &HashMap<String, Vec<u8>>,
        interaction: &InteractionState,
        scale: f32,
        use_2x: bool,
    ) -> anyhow::Result<RuntimeUi> {
        load_layout(
            files,
            DpiContext { scale, use_2x },
            "zh-CN",
            None,
            interaction,
            RuntimeMode::Installer,
            None,
        )
    }

    /// Draws a project at 96 DPI, the scale a layout is authored at.
    fn drawn_at_96(
        files: &HashMap<String, Vec<u8>>,
        interaction: &InteractionState,
    ) -> anyhow::Result<RuntimeUi> {
        drawn_for_display(files, interaction, 1.0, false)
    }

    /// Draws a project with one select's menu open, which is how a click on
    /// that control reaches the runtime.
    fn drawn_with_menu(
        files: &HashMap<String, Vec<u8>>,
        interaction: &InteractionState,
        select: &str,
    ) -> anyhow::Result<RuntimeUi> {
        load_layout(
            files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            Some(select),
            interaction,
            RuntimeMode::Installer,
            None,
        )
    }

    /// A layout context over an in-memory project, for the measuring helpers
    /// that are called with one node instead of a whole page.
    fn context_for<'a>(
        files: &'a HashMap<String, Vec<u8>>,
        config: &'a serde_json::Value,
        translations: &'a HashMap<String, String>,
        interaction: &'a InteractionState,
    ) -> LayoutContext<'a> {
        LayoutContext {
            dpi: DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            files,
            config,
            locale: "zh-CN",
            translations,
            interaction,
            fields: HashMap::new(),
            open_select: None,
            contrast: None,
        }
    }

    /// The `(top, right, bottom, left)` of an inset set, in the order the
    /// shorthand is written and read.
    fn inset_edges(insets: Insets) -> (i32, i32, i32, i32) {
        (insets.top, insets.right, insets.bottom, insets.left)
    }

    /// Parses a small layout and hands back its document, so a case can reach
    /// the node it is about without a temporary being dropped under it.
    fn parsed_layout(markup: &str) -> roxmltree::Document<'_> {
        roxmltree::Document::parse(markup).expect("layout parses")
    }

    /// The one node of a small layout that carries `id`, so a case can hand the
    /// measuring helpers the element it is about.
    fn node_with_id<'a, 'input>(
        document: &'a roxmltree::Document<'input>,
        id: &str,
    ) -> roxmltree::Node<'a, 'input> {
        document
            .descendants()
            .find(|node| node.attribute("id") == Some(id))
            .unwrap_or_else(|| panic!("no element with id {id}"))
    }

    #[test]
    fn a_page_paints_its_fill_under_its_image_and_its_outline_over_them() -> anyhow::Result<()> {
        // The page decoration in the order the guide lists it: the base colour,
        // the background image stretched over it, then the outline grown inwards
        // from the edges. `border-radius` becomes the window region, so it is
        // reported as a radius rather than drawn into a layer.
        let mut files = one_page_project(
            r##"<Page width="400" height="200" background="#FF181B22"
                       background-image="assets/bg_main.png"
                       border-color="#FF00FF00" border-width="2" border-radius="12" />"##,
            "{}",
        );
        files.insert(
            "assets/bg_main.png".to_string(),
            example_asset("bg_main.png"),
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;

        assert_eq!((ui.width, ui.height), (400, 200));
        assert_eq!(ui.corner_radius, 12);
        assert_eq!(ui.layers.len(), 3);
        let fill = &ui.layers[0];
        assert_eq!(
            (fill.left, fill.top, fill.width, fill.height),
            (0, 0, 400, 200)
        );

        // The image keeps its own decoded size and is stretched over the client
        // area, which is wider than the 400px page.
        let background = &ui.layers[1];
        assert_eq!(
            (
                background.left,
                background.top,
                background.width,
                background.height
            ),
            (0, 0, 400, 200)
        );
        assert_eq!(
            (background.image.width, background.image.height),
            (720, 450)
        );

        // The outline is a ring: two device pixels at the edge are painted and
        // the interior is left clear, so the fill below it still shows through.
        let outline = &ui.layers[2];
        assert_eq!((outline.width, outline.height), (400, 200));
        let alpha = |x: usize, y: usize| outline.image.pixels[(y * 400 + x) * 4 + 3];
        assert_eq!(alpha(200, 0), 255);
        assert_eq!(alpha(200, 1), 255);
        assert_eq!(alpha(200, 2), 0);
        assert_eq!(alpha(200, 100), 0);
        // The corner is cut away, which is what the window region does with the
        // same radius.
        assert_eq!(alpha(0, 0), 0);

        // A scaled display gets a scaled radius, so the region and the page art
        // are rounded by the same amount.
        let scaled = drawn_for_display(&files, &InteractionState::default(), 2.0, true)?;
        assert_eq!(scaled.corner_radius, 24);
        assert_eq!((scaled.width, scaled.height), (800, 400));

        // A page that declares no extent at all falls back to the documented
        // client area rather than to a window of nothing.
        let default_size = drawn_at_96(
            &one_page_project("<Page />", "{}"),
            &InteractionState::default(),
        )?;
        assert_eq!((default_size.width, default_size.height), (720, 450));
        Ok(())
    }

    #[test]
    fn an_absolute_image_and_icon_draw_at_the_rectangle_they_declare() -> anyhow::Result<()> {
        // `Image` and `Icon` are the tags that draw artwork of their own. The
        // guide's claim is that an absolute position with an extent is what makes
        // them appear at all, so a control without one has nothing to draw.
        let mut files = one_page_project(
            r##"<Page width="400" height="200">
                  <Image id="logo" src="assets/logo.png" position="absolute"
                         left="10" top="20" width="100" height="30" />
                  <Icon id="badge" src="assets/checkbox-2.png" position="absolute"
                        left="200" top="120" width="24" height="24" />
                  <Image id="unplaced" src="assets/logo.png" position="absolute" left="0" top="0" />
                </Page>"##,
            "{}",
        );
        files.insert("assets/logo.png".to_string(), example_asset("logo.png"));
        files.insert(
            "assets/checkbox-2.png".to_string(),
            example_asset("checkbox-2.png"),
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;

        assert_eq!(ui.layers.len(), 2);
        let logo = &ui.layers[0];
        assert_eq!(
            (logo.left, logo.top, logo.width, logo.height),
            (10, 20, 100, 30)
        );
        // The PNG is decoded at its own size and drawn into the declared extent.
        assert_eq!((logo.image.width, logo.image.height), (200, 58));
        let badge = &ui.layers[1];
        assert_eq!(
            (badge.left, badge.top, badge.width, badge.height),
            (200, 120, 24, 24)
        );
        Ok(())
    }

    #[test]
    fn a_layout_picks_the_image_density_the_display_asks_for() -> anyhow::Result<()> {
        // A project names one file and the runtime chooses the variant: the 1x
        // artwork below the threshold and the `@2x` one at or above it, in both
        // directions and with a fallback when only one of the two shipped.
        let layout = r##"<Page width="400" height="200">
                  <Image id="logo" src="assets/logo.png" position="absolute"
                         left="0" top="0" width="200" height="58" />
                </Page>"##;
        let spelled_out = r##"<Page width="400" height="200">
                  <Image id="logo" src="assets/logo@2x.png" position="absolute"
                         left="0" top="0" width="200" height="58" />
                </Page>"##;
        let both = |layout: &str| {
            let mut files = one_page_project(layout, "{}");
            files.insert("assets/logo.png".to_string(), example_asset("logo.png"));
            files.insert(
                "assets/logo@2x.png".to_string(),
                example_asset("logo@2x.png"),
            );
            files
        };

        // Both variants shipped: the display decides which one is decoded.
        let one_x = drawn_at_96(&both(layout), &InteractionState::default())?;
        assert_eq!(one_x.layers[0].image.width, 200);
        let two_x = drawn_for_display(&both(layout), &InteractionState::default(), 2.0, true)?;
        assert_eq!(two_x.layers[0].image.width, 400);

        // A layout that names the dense file is normalized the same way, so a
        // project never maintains two copies of one layout.
        let named_dense = drawn_at_96(&both(spelled_out), &InteractionState::default())?;
        assert_eq!(named_dense.layers[0].image.width, 200);
        let named_dense_at_2x =
            drawn_for_display(&both(spelled_out), &InteractionState::default(), 2.0, true)?;
        assert_eq!(named_dense_at_2x.layers[0].image.width, 400);

        // Only the 1x artwork shipped, and only the dense one shipped.
        let mut base_only = one_page_project(layout, "{}");
        base_only.insert("assets/logo.png".to_string(), example_asset("logo.png"));
        let dense_display = drawn_for_display(&base_only, &InteractionState::default(), 2.0, true)?;
        assert_eq!(dense_display.layers[0].image.width, 200);
        let mut dense_only = one_page_project(layout, "{}");
        dense_only.insert(
            "assets/logo@2x.png".to_string(),
            example_asset("logo@2x.png"),
        );
        let plain_display = drawn_at_96(&dense_only, &InteractionState::default())?;
        assert_eq!(plain_display.layers[0].image.width, 400);
        Ok(())
    }

    #[test]
    fn a_button_state_image_falls_back_to_the_normal_one() {
        // A layout that paints only some states still has to draw the button: a
        // missing `hover-image`, `pressed-image` or `disabled-image` falls back to
        // `normal-image` instead of leaving a hole in the page.
        let document = roxmltree::Document::parse(
            r#"<Button id="next" action="install" normal-image="normal.png" hover-image="hover.png" />"#,
        )
        .expect("layout parses");
        let button = document.root_element();
        let mut interaction = InteractionState::default();
        assert_eq!(
            button_image(button, &interaction, &HashMap::new()),
            Some("normal.png")
        );

        // Hovering has artwork of its own; pressing does not, so the press is
        // drawn with the normal image rather than with the hover one.
        interaction.hovered_control = Some("next".to_string());
        assert_eq!(
            button_image(button, &interaction, &HashMap::new()),
            Some("hover.png")
        );
        interaction.pressed_control = Some("next".to_string());
        assert_eq!(
            button_image(button, &interaction, &HashMap::new()),
            Some("normal.png")
        );

        // A button held back by its condition is drawn from `disabled-image` when
        // it declares one, whatever the pointer is doing.
        let document = roxmltree::Document::parse(
            r#"<Button id="gated" action="install" enabled-when="terms:checked"
                       normal-image="normal.png" hover-image="hover.png"
                       disabled-image="disabled.png" />"#,
        )
        .expect("layout parses");
        let gated = document.root_element();
        assert_eq!(
            button_image(gated, &InteractionState::default(), &HashMap::new()),
            Some("disabled.png")
        );
        let gated_interaction = InteractionState {
            hovered_control: Some("gated".to_string()),
            ..Default::default()
        };
        assert_eq!(
            button_image(gated, &gated_interaction, &HashMap::new()),
            Some("disabled.png")
        );

        // A control with no id cannot be hovered or pressed at all, which is how
        // an unbound button behaves.
        let document = roxmltree::Document::parse(
            r#"<Button action="install" normal-image="normal.png" hover-image="hover.png" />"#,
        )
        .expect("layout parses");
        let unbound = InteractionState {
            hovered_control: Some("next".to_string()),
            pressed_control: Some("next".to_string()),
            ..Default::default()
        };
        assert_eq!(
            button_image(document.root_element(), &unbound, &HashMap::new()),
            Some("normal.png")
        );
    }

    #[test]
    fn a_button_waits_for_each_state_its_condition_can_name() {
        // `enabled-when` is what makes one control depend on another. The four
        // states the guide lists decide on the checkbox or the panel they name,
        // and a condition the runtime does not understand holds the button back
        // rather than letting a click through.
        let document = roxmltree::Document::parse(
            r#"<Page>
                 <Button id="checked" enabled-when="terms:checked" />
                 <Button id="unchecked" enabled-when="terms:unchecked" />
                 <Button id="visible" enabled-when="panel:visible" />
                 <Button id="hidden" enabled-when="panel:hidden" />
                 <Button id="elsewhere" enabled-when="anything:checked" />
                 <Button id="unknown_state" enabled-when="terms:whenever" />
                 <Button id="no_state" enabled-when="terms" />
                 <Button id="both" enabled-when="terms:checked, panel:visible" />
                 <Button id="spaced" enabled-when="terms:checked , panel:visible" />
                 <Button id="trailing" enabled-when="terms:checked," />
                 <Button id="unconditional" />
               </Page>"#,
        )
        .expect("layout parses");

        let fields = HashMap::new();
        let mut interaction = InteractionState::default();
        interaction
            .checkbox_states
            .insert("terms".to_string(), false);
        assert!(button_enabled(
            node_with_id(&document, "unchecked"),
            &interaction,
            &fields
        ));
        assert!(!button_enabled(
            node_with_id(&document, "checked"),
            &interaction,
            &fields
        ));
        assert!(button_enabled(
            node_with_id(&document, "hidden"),
            &interaction,
            &fields
        ));
        assert!(!button_enabled(
            node_with_id(&document, "visible"),
            &interaction,
            &fields
        ));

        interaction
            .checkbox_states
            .insert("terms".to_string(), true);
        assert!(button_enabled(
            node_with_id(&document, "checked"),
            &interaction,
            &fields
        ));
        assert!(!button_enabled(
            node_with_id(&document, "unchecked"),
            &interaction,
            &fields
        ));
        interaction
            .panel_visibility
            .insert("panel".to_string(), true);
        assert!(button_enabled(
            node_with_id(&document, "visible"),
            &interaction,
            &fields
        ));
        assert!(!button_enabled(
            node_with_id(&document, "hidden"),
            &interaction,
            &fields
        ));

        // The runtime carries no rules about which control a condition names, so
        // any id works; only the state has to be one it knows.
        interaction
            .checkbox_states
            .insert("anything".to_string(), true);
        assert!(button_enabled(
            node_with_id(&document, "elsewhere"),
            &interaction,
            &fields
        ));
        assert!(!button_enabled(
            node_with_id(&document, "unknown_state"),
            &interaction,
            &fields
        ));
        assert!(!button_enabled(
            node_with_id(&document, "no_state"),
            &interaction,
            &fields
        ));
        // One control may wait for several others: a condition that lists
        // more than one `id:state` holds only when every one of them does.
        interaction
            .panel_visibility
            .insert("panel".to_string(), true);
        assert!(button_enabled(
            node_with_id(&document, "both"),
            &interaction,
            &fields
        ));
        assert!(button_enabled(
            node_with_id(&document, "spaced"),
            &interaction,
            &fields
        ));
        interaction
            .panel_visibility
            .insert("panel".to_string(), false);
        assert!(!button_enabled(
            node_with_id(&document, "both"),
            &interaction,
            &fields
        ));
        // A comma with nothing after it is not a condition that holds.
        assert!(!button_enabled(
            node_with_id(&document, "trailing"),
            &interaction,
            &fields
        ));

        // A button without a condition is enabled, which is what makes the
        // attribute opt-in.
        assert!(button_enabled(
            node_with_id(&document, "unconditional"),
            &interaction,
            &fields
        ));
    }

    #[test]
    fn a_field_checks_the_value_the_project_asks_it_to() {
        // The rules a TextInput declares about the value the user gives it: it
        // may be required, it may have to be this long, and it may have to fit a
        // mask. A field with no rules, and an optional field left empty, are
        // valid; a broken rule names the message the layout wrote for it.
        let document = parsed_layout(
            r#"<Page>
                 <TextInput id="plain" />
                 <TextInput id="needed" required="true" required-message="@need_it" />
                 <TextInput id="shortest" min-length="3" min-length-message="@longer" />
                 <TextInput id="longest" max-length="3" max-length-message="@shorter" />
                 <TextInput id="named" pattern="*.exe" pattern-message="@exe_only" />
                 <TextInput id="letter" pattern="?.exe" />
                 <TextInput id="optional" required="false" pattern="*.exe" />
               </Page>"#,
        );
        let state = |id: &str, value: &str| field_state(node_with_id(&document, id), value);

        assert!(state("plain", "").valid);
        assert!(state("plain", "anything").valid);
        assert!(!state("needed", "").valid);
        assert_eq!(state("needed", "").message.as_deref(), Some("@need_it"));
        assert!(state("needed", "x").valid);
        assert!(state("needed", "x").message.is_none());

        // Lengths count characters, so a Chinese name is not measured in bytes.
        assert!(!state("shortest", "安装").valid);
        assert!(state("shortest", "安装包").valid);
        assert_eq!(
            state("shortest", "安装").message.as_deref(),
            Some("@longer")
        );
        assert!(!state("longest", "abcd").valid);
        assert!(state("longest", "abc").valid);
        assert_eq!(
            state("longest", "abcd").message.as_deref(),
            Some("@shorter")
        );

        // The mask is not a regular expression: `*` is any run of characters,
        // `?` exactly one, and the whole value has to match.
        assert!(state("named", "setup.exe").valid);
        assert!(!state("named", "setup").valid);
        assert_eq!(
            state("named", "setup").message.as_deref(),
            Some("@exe_only")
        );
        assert!(state("letter", "a.exe").valid);
        assert!(!state("letter", "ab.exe").valid);
        // A rule without a message leaves the field invalid without words: the
        // control that waits for it is the whole answer.
        assert!(state("letter", "ab.exe").message.is_none());
        assert!(mask_matches("*.exe", ".exe"));
        // `?:*` is what a project writes for "a drive letter and whatever
        // follows it", which is how an install directory is asked for.
        assert!(mask_matches("?:*", "C:\\Program Files\\Demo"));
        assert!(!mask_matches("?:*", "Program Files"));
        assert!(!mask_matches("?:*", ""));

        // An optional field the user left empty passes the rules that speak
        // about the value it would hold.
        assert!(state("optional", "").valid);
    }

    #[test]
    fn a_button_waits_for_the_field_its_condition_names() -> anyhow::Result<()> {
        // A condition names a text field the way it names a checkbox, so a
        // button that starts an install waits for a value the project accepts.
        let files = one_page_project(
            r#"<Page width="200" height="100">
                 <TextInput id="dir" required="true" pattern="?:*" required-message="@dir_needed"
                            position="absolute" left="0" top="0" width="120" height="20" />
                 <Button id="go" action="install" enabled-when="dir:valid" text="Go"
                         position="absolute" left="0" top="40" width="60" height="20" />
                 <Button id="explain" action="install" enabled-when="dir:invalid" text="?"
                         position="absolute" left="80" top="40" width="60" height="20" />
               </Page>"#,
            r#"{"dir_needed": "请填写安装目录"}"#,
        );
        let clickable = |ui: &RuntimeUi, left: i32, top: i32| {
            ui.actions
                .iter()
                .any(|region| region.left == left && region.top == top)
        };
        let holding = |value: &str| {
            let mut interaction = InteractionState::default();
            interaction
                .text_input_values
                .insert("dir".to_string(), value.to_string());
            interaction
        };

        // An empty field is what `required` speaks about, so the button that
        // starts the install is inert and carries no hover either.
        let empty = drawn_at_96(&files, &InteractionState::default())?;
        assert!(
            !clickable(&empty, 0, 40),
            "a click reached a waiting button"
        );
        assert!(clickable(&empty, 80, 40));
        assert!(!empty
            .hover_regions
            .iter()
            .any(|region| region.left == 0 && region.top == 40));

        // A path the project accepts hands the button back.
        let filled = drawn_at_96(&files, &holding("C:\\Program Files\\Demo"))?;
        assert!(clickable(&filled, 0, 40));
        assert!(!clickable(&filled, 80, 40));

        // Clearing the field takes the click away again, which is what the
        // keystroke path does when it rebuilds the page.
        let cleared = drawn_at_96(&files, &holding(""))?;
        assert!(!clickable(&cleared, 0, 40));
        Ok(())
    }

    #[test]
    fn a_hint_shows_the_rule_the_value_breaks() -> anyhow::Result<()> {
        // The label that names a field draws the words of the rule its value
        // breaks first, in the language the project ships, and nothing at all
        // while the value is one the project accepts.
        let files = one_page_project(
            r##"<Page width="200" height="100">
                 <TextInput id="dir" required="true" pattern="?:*"
                            required-message="@dir_needed" pattern-message="@dir_absolute"
                            position="absolute" left="0" top="0" width="120" height="20" />
                 <Label id="hint" value-source="field-error:dir"
                        position="absolute" left="0" top="30" width="200" height="16"
                        color="#FFFFFFFF" />
                 <Label id="unknown" value-source="field-error:nowhere"
                        position="absolute" left="0" top="50" width="200" height="16"
                        color="#FFFFFFFF" />
               </Page>"##,
            r#"{"dir_needed": "请填写安装目录", "dir_absolute": "安装目录要写完整"}"#,
        );
        let hint = |ui: &RuntimeUi| {
            ui.texts
                .iter()
                .find(|layer| layer.top == 30)
                .map(visible_text)
        };
        let holding = |value: &str| {
            let mut interaction = InteractionState::default();
            interaction
                .text_input_values
                .insert("dir".to_string(), value.to_string());
            interaction
        };

        let empty = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(hint(&empty).as_deref(), Some("请填写安装目录"));
        // A label naming a field the page does not declare has nothing to say.
        assert!(!empty.texts.iter().any(|layer| layer.top == 50));

        let partial = drawn_at_96(&files, &holding("Program Files"))?;
        assert_eq!(hint(&partial).as_deref(), Some("安装目录要写完整"));

        let complete = drawn_at_96(&files, &holding("C:\\Program Files\\Demo"))?;
        assert_eq!(hint(&complete), None);
        Ok(())
    }

    #[test]
    fn a_select_offers_the_options_the_page_declares() -> anyhow::Result<()> {
        // A select is a choice the page offers, not only the language control:
        // its rows are the `<Option>` children the layout declares, the closed
        // control reads the value in use, and a click records the row the user
        // picks. An option the layout hides is not offered.
        let files = one_page_project(
            r##"<Page width="400" height="200">
                 <Select id="mode" position="absolute" left="10" top="10" width="140" height="24">
                   <Option value="quick" text="@mode_quick" />
                   <Option value="full" text="@mode_full" />
                   <Option value="secret" text="Hidden" visible="false" />
                 </Select>
                 <Button id="go" action="install" enabled-when="mode:full" text="Go"
                         position="absolute" left="10" top="60" width="80" height="24" />
               </Page>"##,
            r#"{"mode_quick": "快速安装", "mode_full": "完整安装"}"#,
        );
        let holding = |value: &str| {
            let mut interaction = InteractionState::default();
            interaction
                .choices
                .insert("mode".to_string(), value.to_string());
            interaction
        };
        let started = |ui: &RuntimeUi| {
            ui.actions
                .iter()
                .any(|region| matches!(region.action, WindowAction::Install))
        };

        // Closed, the control reads the option in use: the first one until the
        // user picks another, the picked one afterwards. Clicking it asks the
        // runtime for its own menu rather than the language list.
        let closed = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(visible_text(&closed.texts[0]), "快速安装");
        assert!(closed.menu.is_none(), "a closed select recorded a menu");
        assert!(closed.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::ToggleSelectMenu { ref id } if id == "mode"
        )));

        // The value the button beside it waits for is the one the user picked,
        // which is what makes a select worth more than its own text.
        assert!(!started(&closed), "the install started on the first option");
        let picked = drawn_at_96(&files, &holding("full"))?;
        assert_eq!(visible_text(&picked.texts[0]), "完整安装");
        assert!(
            started(&picked),
            "the picked value never reached the button"
        );

        // Open, it lists the options in the order the layout declares them,
        // each in its own words, with the row in use marked.
        let open = drawn_with_menu(&files, &holding("full"), "mode")?;
        let rows = open.menu.as_ref().expect("the menu was open");
        assert_eq!(
            rows.values,
            ["quick", "full"],
            "a hidden option was offered"
        );
        assert!(!rows.language, "a project's own select listed locales");
        assert_eq!(rows.chosen, Some(1));
        assert_eq!(
            open.overlay_texts
                .iter()
                .map(visible_text)
                .collect::<Vec<_>>(),
            ["快速安装", "完整安装"]
        );
        assert!(open.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::ChooseOption { ref id, ref value } if id == "mode" && value == "quick"
        )));
        Ok(())
    }

    #[test]
    fn a_radio_group_holds_one_value_at_a_time() -> anyhow::Result<()> {
        // A radio button answers for its group rather than for itself: the
        // layout may mark one as the default, a click on either names the group
        // and the value it picks, and the rest of the page reads that choice the
        // way it reads a select's.
        let project = |checked: &str| {
            let mut files = one_page_project(
                &format!(
                    r##"<Page width="400" height="200">
                          <RadioButton id="quick" group="mode" value="quick" text="@mode_quick"
                                       checked="{checked}"
                                       position="absolute" left="10" top="10" width="140" height="20"
                                       unchecked-image="file='assets/checkbox-0.png' dest='0,1,16,17'"
                                       checked-image="file='assets/checkbox-2.png' dest='0,1,16,17'" />
                          <RadioButton id="full" group="mode" value="full" text="@mode_full"
                                       position="absolute" left="10" top="40" width="140" height="20"
                                       unchecked-image="file='assets/checkbox-0.png' dest='0,1,16,17'"
                                       checked-image="file='assets/checkbox-2.png' dest='0,1,16,17'" />
                          <Button id="go" action="install" enabled-when="mode:full" text="Go"
                                  position="absolute" left="10" top="80" width="80" height="24" />
                        </Page>"##
                ),
                r#"{"mode_quick": "快速安装", "mode_full": "完整安装"}"#,
            );
            for name in ["checkbox-0.png", "checkbox-2.png"] {
                files.insert(format!("assets/{name}"), example_asset(name));
            }
            files
        };
        let picked = |value: &str| {
            let mut interaction = InteractionState::default();
            interaction
                .choices
                .insert("mode".to_string(), value.to_string());
            interaction
        };
        let artwork = |ui: &RuntimeUi| {
            ui.layers
                .iter()
                .map(|layer| layer.image.pixels.clone())
                .collect::<Vec<_>>()
        };
        let on = super::decode_image(&example_asset("checkbox-2.png"))?.pixels;
        let off = super::decode_image(&example_asset("checkbox-0.png"))?.pixels;
        let started = |ui: &RuntimeUi| {
            ui.actions
                .iter()
                .any(|region| matches!(region.action, WindowAction::Install))
        };

        // The layout's own default stands until the user picks another row,
        // and only one radio of the group is on at a time.
        let fresh = drawn_at_96(&project("true"), &InteractionState::default())?;
        assert_eq!(visible_text(&fresh.texts[0]), "快速安装");
        assert_eq!(visible_text(&fresh.texts[1]), "完整安装");
        assert_eq!(artwork(&fresh), [on.clone(), off.clone()]);
        assert!(
            !started(&fresh),
            "the install started on the row the layout defaults to"
        );
        // A click on either radio names the group and the value it stands for.
        assert!(fresh.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::ChooseOption { ref id, ref value } if id == "mode" && value == "full"
        )));

        // A group the layout marks nowhere holds no value at all: no row of it
        // carries a mark, and a condition naming one of its values stays unmet.
        let unmarked = drawn_at_96(&project("false"), &InteractionState::default())?;
        assert_eq!(artwork(&unmarked), [off.clone(), off.clone()]);
        assert!(!started(&unmarked), "an unset group let the install start");

        // The choice replaces the layout's default for the whole group, which
        // is what hands the button beside them the value it waits for.
        let chosen = drawn_at_96(&project("true"), &picked("full"))?;
        assert_eq!(artwork(&chosen), [off.clone(), on.clone()]);
        assert!(
            started(&chosen),
            "the picked value never reached the button"
        );

        // Picking the other row moves the mark back and takes the click away
        // again, so the group always holds exactly one value.
        let flipped = drawn_at_96(&project("false"), &picked("quick"))?;
        assert_eq!(artwork(&flipped), [on.clone(), off.clone()]);
        assert!(
            !started(&flipped),
            "a button stayed live after the group moved off its value"
        );
        Ok(())
    }

    /// A rectangle to compare a hit region or a layer against.
    fn area(left: i32, top: i32, width: i32, height: i32) -> LayerRect {
        LayerRect {
            left,
            top,
            width,
            height,
        }
    }

    /// A page holding one scrollable list, with a button that is not part of it.
    ///
    /// Every row is tall enough that six of them need more room than the list
    /// has, which is what makes it scrollable at all.
    fn scrollable_project(rows: usize) -> HashMap<String, Vec<u8>> {
        let mut layout = String::from(
            r##"<Page width="400" height="300">
                 <VBox id="list" scrollable="true" position="absolute" left="10" top="10"
                       width="200" height="100">"##,
        );
        for index in 0..rows {
            layout.push_str(&format!(
                r#"<Button id="row{index}" action="install" text="Row {index}"
                           width="200" height="30" />"#
            ));
        }
        layout.push_str(
            r##"</VBox>
                 <Button id="outside" action="close" text="Outside"
                         position="absolute" left="10" top="200" width="100" height="20" />
               </Page>"##,
        );
        one_page_project(&layout, "{}")
    }

    /// The same page, with the list declared without an id.
    fn nameless_list_project() -> HashMap<String, Vec<u8>> {
        one_page_project(
            r##"<Page width="400" height="300">
                 <VBox scrollable="true" position="absolute" left="10" top="10"
                       width="200" height="100">
                   <Button id="row0" action="install" text="Row 0" width="200" height="30" />
                   <Button id="row1" action="install" text="Row 1" width="200" height="30" />
                   <Button id="row2" action="install" text="Row 2" width="200" height="30" />
                   <Button id="row3" action="install" text="Row 3" width="200" height="30" />
                 </VBox>
               </Page>"##,
            "{}",
        )
    }

    /// A page whose list is scrolled to one offset, which is the state a wheel
    /// or a click on the scrollbar leaves behind.
    fn scrolled(offset: i32) -> InteractionState {
        let mut interaction = InteractionState::default();
        interaction
            .scroll_offsets
            .insert("list".to_string(), offset);
        interaction
    }

    /// Every click region that starts an install, in the order they were laid
    /// out.
    fn install_regions(ui: &RuntimeUi) -> Vec<LayerRect> {
        ui.actions
            .iter()
            .filter(|region| matches!(region.action, WindowAction::Install))
            .map(|region| {
                area(
                    region.left,
                    region.top,
                    region.right - region.left,
                    region.bottom - region.top,
                )
            })
            .collect()
    }

    /// Every region that pages the list through its scrollbar, tagged with the
    /// direction it moves in.
    fn page_regions(ui: &RuntimeUi) -> Vec<(bool, LayerRect)> {
        ui.actions
            .iter()
            .filter_map(|region| match region.action {
                WindowAction::ScrollPage { ref id, forward } if id == "list" => Some((
                    forward,
                    area(
                        region.left,
                        region.top,
                        region.right - region.left,
                        region.bottom - region.top,
                    ),
                )),
                _ => None,
            })
            .collect()
    }

    /// Whether the page drew a layer exactly filling `rect`.
    fn drew_layer(ui: &RuntimeUi, rect: LayerRect) -> bool {
        ui.layers
            .iter()
            .chain(ui.overlay_layers.iter())
            .any(|layer| {
                layer.left == rect.left
                    && layer.top == rect.top
                    && layer.width == rect.width
                    && layer.height == rect.height
            })
    }

    /// The words every text layer on the page draws, in the order they were laid
    /// out.
    fn drawn_words(ui: &RuntimeUi) -> Vec<String> {
        ui.texts.iter().map(visible_text).collect()
    }

    #[test]
    fn a_scrollable_container_shows_the_part_it_is_scrolled_to() -> anyhow::Result<()> {
        // A list taller than the room it has shows one window onto its rows: the
        // offset moves them, a row that moved past the container's edge is
        // neither drawn nor clicked, and an offset past the end of the list
        // stops there instead of showing blank space.
        let files = scrollable_project(6);

        let top = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(top.scroll_views.len(), 1, "the page kept no scroll view");
        let view = &top.scroll_views[0];
        assert_eq!(view.id, "list");
        assert_eq!(view.area, area(10, 10, 200, 100));
        assert_eq!(view.viewport, area(10, 10, 200, 100));
        assert_eq!(view.content, 180, "the list did not say what it holds");
        assert_eq!(view.offset, 0);
        assert_eq!(view.max_offset(), 80);
        // Rows four and five lie past the edge, and the row that only half fits
        // is drawn and clicked only in the part that shows.
        assert_eq!(
            install_regions(&top),
            [
                area(10, 10, 200, 30),
                area(10, 40, 200, 30),
                area(10, 70, 200, 30),
                area(10, 100, 200, 10),
            ],
            "the rows past the edge stayed clickable"
        );
        assert_eq!(
            drawn_words(&top),
            ["Row 0", "Row 1", "Row 2", "Row 3", "Outside"]
        );

        // Scrolled by sixty pixels the rows move up together, and what left the
        // view through the top is gone the same way.
        let middle = drawn_at_96(&files, &scrolled(60))?;
        assert_eq!(middle.scroll_views[0].offset, 60);
        assert_eq!(
            install_regions(&middle),
            [
                area(10, 10, 200, 30),
                area(10, 40, 200, 30),
                area(10, 70, 200, 30),
                area(10, 100, 200, 10),
            ],
        );
        assert_eq!(
            drawn_words(&middle),
            ["Row 2", "Row 3", "Row 4", "Row 5", "Outside"]
        );

        // An offset past the end of the list is clamped to the end, so the last
        // row comes to rest against the container's bottom edge.
        let end = drawn_at_96(&files, &scrolled(1000))?;
        assert_eq!(end.scroll_views[0].offset, 80);
        let end_rows = install_regions(&end);
        assert_eq!(end_rows.first(), Some(&area(10, 10, 200, 10)));
        assert_eq!(
            end_rows.last(),
            Some(&area(10, 80, 200, 30)),
            "the last row did not come to rest at the edge"
        );
        assert_eq!(end_rows.last().map(|row| row.bottom()), Some(110));
        Ok(())
    }

    #[test]
    fn a_scrollbar_says_where_the_list_stands() -> anyhow::Result<()> {
        // The bar is drawn where the view ends and as long as the share of the
        // list the view shows, and a click on either part of the track moves the
        // view by one of its own pages -- which is what makes the bar usable
        // without dragging the thumb.
        let files = scrollable_project(6);

        let top = drawn_at_96(&files, &InteractionState::default())?;
        // A hundred of the list's hundred and eighty pixels fit the view, so the
        // thumb covers a little over half the track and starts at its top.
        assert!(
            drew_layer(&top, area(202, 10, 8, 100)),
            "no track was drawn"
        );
        assert!(
            drew_layer(&top, area(202, 10, 8, 56)),
            "the thumb did not say how much of the list is in view"
        );
        assert_eq!(page_regions(&top), [(true, area(202, 66, 8, 44))]);

        // At the end of the list the thumb reaches the other end of the track,
        // and the part that pages back is the one that is left.
        let end = drawn_at_96(&files, &scrolled(80))?;
        assert!(drew_layer(&end, area(202, 54, 8, 56)));
        assert_eq!(page_regions(&end), [(false, area(202, 10, 8, 44))]);

        // A list whose rows fit its view gets no bar at all, and nothing to page
        // through: there is nowhere to scroll to.
        let short = drawn_at_96(&scrollable_project(2), &InteractionState::default())?;
        assert_eq!(short.scroll_views[0].max_offset(), 0);
        assert!(
            page_regions(&short).is_empty(),
            "a short list offered a bar"
        );
        assert!(!drew_layer(&short, area(202, 10, 8, 100)));

        // A container that asks to scroll without an id has no name to keep a
        // position under, so it stays a plain container rather than sharing one
        // with every other nameless list on the page.
        let nameless = drawn_at_96(&nameless_list_project(), &InteractionState::default())?;
        assert!(nameless.scroll_views.is_empty());
        assert_eq!(
            install_regions(&nameless).len(),
            4,
            "a nameless container hid rows it should have shown"
        );
        Ok(())
    }

    #[test]
    fn a_disabled_button_registers_no_click_and_no_hover() {
        // While the condition is unmet the button is inert: no action region, so
        // the click that would start an install cannot land, and no hover region,
        // so it does not light up either.
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let translations = HashMap::new();
        let document = roxmltree::Document::parse(
            r#"<Page>
                 <Button id="install" action="install" enabled-when="terms:checked"
                         normal-image="normal.png" hover-image="hover.png"
                         disabled-image="disabled.png" />
               </Page>"#,
        )
        .expect("layout parses");
        let button = node_with_id(&document, "install");
        let rect = LayerRect {
            left: 10,
            top: 20,
            width: 100,
            height: 30,
        };

        let mut interaction = InteractionState::default();
        interaction
            .checkbox_states
            .insert("terms".to_string(), false);
        let context = context_for(&files, &config, &translations, &interaction);
        let mut actions = Vec::new();
        let mut hovers = Vec::new();
        push_action(button, rect, &mut actions, &mut Vec::new(), &context);
        push_hover_region(button, rect, &context, &mut hovers);
        assert!(actions.is_empty());
        assert!(hovers.is_empty());

        // Once the condition holds it answers with the action it declares, over
        // the rectangle it was placed at.
        let mut interaction = InteractionState::default();
        interaction
            .checkbox_states
            .insert("terms".to_string(), true);
        let context = context_for(&files, &config, &translations, &interaction);
        let mut actions = Vec::new();
        let mut hovers = Vec::new();
        push_action(button, rect, &mut actions, &mut Vec::new(), &context);
        push_hover_region(button, rect, &context, &mut hovers);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action, WindowAction::Install));
        assert_eq!(
            (
                actions[0].left,
                actions[0].top,
                actions[0].right,
                actions[0].bottom
            ),
            (10, 20, 110, 50)
        );
        assert_eq!(hovers.len(), 1);
        assert_eq!(hovers[0].id, "install");
    }

    #[test]
    fn a_styled_image_draws_into_a_sub_rectangle_at_the_opacity_it_declares() -> anyhow::Result<()>
    {
        // `file='...' dest='...' fade='...'` is the form the example's window
        // buttons use: artwork inset inside a larger hit area, faded into the
        // page. The destination is relative to the control, not to the page.
        let mut files = one_page_project(
            r##"<Page width="400" height="200">
                  <Button id="close" action="close" position="absolute" left="100" top="50"
                          width="34" height="34"
                          normal-image="file='assets/checkbox-2.png' dest='7,7,27,27' fade='160'" />
                </Page>"##,
            "{}",
        );
        files.insert(
            "assets/checkbox-2.png".to_string(),
            example_asset("checkbox-2.png"),
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(ui.layers.len(), 1);
        let layer = &ui.layers[0];
        assert_eq!(
            (layer.left, layer.top, layer.width, layer.height),
            (107, 57, 20, 20)
        );
        assert_eq!(layer.alpha, 160);

        // The destination follows the control onto a scaled display.
        let scaled = drawn_for_display(&files, &InteractionState::default(), 2.0, true)?;
        let layer = &scaled.layers[0];
        assert_eq!(
            (layer.left, layer.top, layer.width, layer.height),
            (214, 114, 40, 40)
        );
        assert_eq!(layer.alpha, 160);
        Ok(())
    }

    #[test]
    fn flow_attributes_become_the_item_a_container_shares_space_with() {
        // The container reads its children through these fields, so an attribute
        // that stopped being read would silently drop the sizing a layout asked
        // for, and the row would be laid out at the wrong widths.
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let translations = HashMap::new();
        let interaction = InteractionState::default();
        let context = context_for(&files, &config, &translations, &interaction);
        let document = roxmltree::Document::parse(
            r##"<Page>
                  <Button id="sized" width="100" padding="0 10" margin-left="5"
                          flex-grow="1" flex-shrink="0.5" min-width="20" flex-basis="30" />
                  <Button id="fixed" width="80" padding="4" margin="2" />
                  <Button id="tall" height="60" padding="2" min-height="30" />
                  <Spacer id="grow" flex-grow="1" />
                </Page>"##,
        )
        .expect("layout parses");

        // `flex-basis` reserves room before free space is shared out, so an item
        // that declares one keeps growing instead of freezing at that size.
        let sized = flow_item_for_node(
            node_with_id(&document, "sized"),
            FlowAxis::Horizontal,
            400,
            &context,
        );
        assert_eq!(sized.fixed_width, None);
        assert_eq!(sized.flex_basis, 30 + 10 + 10 + 5);
        assert_eq!(sized.flex_grow, 1.0);
        assert_eq!(sized.flex_shrink, 0.5);
        assert_eq!(sized.min_width, 20);

        // Without a basis the item claims its width, padding and margin included.
        let fixed = flow_item_for_node(
            node_with_id(&document, "fixed"),
            FlowAxis::Horizontal,
            400,
            &context,
        );
        assert_eq!(fixed.fixed_width, Some(80 + 4 + 4 + 2 + 2));
        assert_eq!(fixed.flex_basis, 0);

        // `Spacer` declares no size at all: growing is the whole of what it does.
        let grow = flow_item_for_node(
            node_with_id(&document, "grow"),
            FlowAxis::Horizontal,
            400,
            &context,
        );
        assert_eq!(grow.fixed_width, None);
        assert_eq!(grow.flex_grow, 1.0);
        assert_eq!(grow.flex_shrink, 1.0);

        // A vertical container reads the same attributes off the other edge:
        // `height` is its main axis and `min-height` its floor.
        let tall = flow_item_for_node(
            node_with_id(&document, "tall"),
            FlowAxis::Vertical,
            400,
            &context,
        );
        assert_eq!(tall.fixed_width, Some(60 + 2 + 2));
        assert_eq!(tall.min_width, 30);
    }

    #[test]
    fn a_shrinking_row_stops_at_the_minimum_its_items_declare() {
        // `flex-shrink` is how a row fits a fixed button beside text, and
        // `min-width` is what keeps a control readable: a row overflows before it
        // squeezes an item past the floor its layout declared.
        let item = |fixed: i32, shrink: f32, min_width: i32| FlowItem {
            fixed_width: Some(fixed),
            flex_grow: 0.0,
            flex_shrink: shrink,
            min_width,
            flex_basis: 0,
        };

        // 684px of content in a 400px row, and no floor to hold the text back.
        assert_eq!(
            flow_widths(&[item(500, 1.0, 0), item(184, 0.0, 0)], 400, 0),
            [216, 184]
        );
        // The 500px item can only give up 200px, so the row stays overflowing
        // rather than shrinking it past `min-width`.
        assert_eq!(
            flow_widths(&[item(500, 1.0, 300), item(184, 0.0, 0)], 400, 0),
            [300, 184]
        );
        // `flex-shrink="0"` is what keeps a button at its designed width, so the
        // item that may shrink carries the whole overflow.
        assert_eq!(
            flow_widths(&[item(500, 0.0, 0), item(400, 1.0, 0)], 400, 0),
            [500, 0]
        );
    }

    #[test]
    fn item_spacing_and_gap_leave_the_same_distance_between_items() -> anyhow::Result<()> {
        // `gap` is the spelling the dialog layouts use and `item-spacing` the one
        // the example pages use; both have to leave the same distance, and a
        // layout that writes both gets the specific one.
        let layout = |attributes: &str| {
            format!(
                r##"<Page width="400" height="100">
                  <HBox position="absolute" left="0" top="0" width="400" height="40" {attributes}>
                    <Button id="first" action="install" width="100" height="40" />
                    <Button id="second" action="close" width="100" height="40" />
                  </HBox>
                </Page>"##
            )
        };
        let second_left = |attributes: &str| -> anyhow::Result<i32> {
            let files = one_page_project(&layout(attributes), "{}");
            let ui = drawn_at_96(&files, &InteractionState::default())?;
            let second = ui
                .actions
                .iter()
                .find(|region| matches!(region.action, WindowAction::Close))
                .expect("the close button is clickable");
            Ok(second.left)
        };

        assert_eq!(second_left("")?, 100);
        assert_eq!(second_left(r#"gap="16""#)?, 116);
        assert_eq!(second_left(r#"item-spacing="16""#)?, 116);
        assert_eq!(second_left(r#"item-spacing="16" gap="40""#)?, 116);
        Ok(())
    }

    #[test]
    fn padding_and_margin_take_one_to_four_values_and_their_single_side_forms() {
        // Both shorthands promise CSS semantics and the example layouts mix them
        // with the single-side forms, which stand alone and override whatever the
        // shorthand said about that edge.
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let translations = HashMap::new();
        let interaction = InteractionState::default();
        let context = context_for(&files, &config, &translations, &interaction);
        let insets = |markup: &str, name: &str| {
            let document = roxmltree::Document::parse(markup).expect("layout parses");
            insets_for_node(document.root_element(), name, &context)
        };

        assert_eq!(
            inset_edges(insets(r#"<Box padding="8" />"#, "padding")),
            (8, 8, 8, 8)
        );
        assert_eq!(
            inset_edges(insets(r#"<Box padding="10 20" />"#, "padding")),
            (10, 20, 10, 20)
        );
        assert_eq!(
            inset_edges(insets(r#"<Box padding="1 2 3" />"#, "padding")),
            (1, 2, 3, 2)
        );
        assert_eq!(
            inset_edges(insets(r#"<Box padding="1 2 3 4" />"#, "padding")),
            (1, 2, 3, 4)
        );
        // A single side overrides the shorthand it is written beside.
        assert_eq!(
            inset_edges(insets(r#"<Box margin="10 20" margin-top="4" />"#, "margin")),
            (4, 20, 10, 20)
        );
        assert_eq!(
            inset_edges(insets(r#"<Box margin-left="6" />"#, "margin")),
            (0, 0, 0, 6)
        );
        assert_eq!(
            inset_edges(insets(r#"<Box padding-right="7" />"#, "padding")),
            (0, 7, 0, 0)
        );
        // Padding and margin are read apart from each other, so one attribute
        // never moves the other.
        assert_eq!(
            inset_edges(insets(r#"<Box padding="8" />"#, "margin")),
            (0, 0, 0, 0)
        );

        // Every value assumes 96 DPI and scales with the display.
        let scaled = LayoutContext {
            dpi: DpiContext {
                scale: 2.0,
                use_2x: true,
            },
            ..context
        };
        let document =
            roxmltree::Document::parse(r#"<Box margin="1 2 3 4" />"#).expect("layout parses");
        assert_eq!(
            inset_edges(insets_for_node(document.root_element(), "margin", &scaled)),
            (2, 4, 6, 8)
        );
    }

    #[test]
    fn justify_content_places_the_run_inside_the_room_it_has() -> anyhow::Result<()> {
        // Centring a row is how the dialogs and the uninstall page keep their
        // content in the middle whatever a translation does to its width.
        let layout = |attributes: &str| {
            format!(
                r##"<Page width="400" height="100">
                  <HBox position="absolute" left="0" top="0" width="400" height="40" {attributes}>
                    <Button id="first" action="minimize" width="100" height="40" />
                    <Button id="second" action="close" width="100" height="40" />
                  </HBox>
                </Page>"##
            )
        };
        let first_left = |attributes: &str| -> anyhow::Result<i32> {
            let files = one_page_project(&layout(attributes), "{}");
            let ui = drawn_at_96(&files, &InteractionState::default())?;
            let first = ui
                .actions
                .iter()
                .find(|region| matches!(region.action, WindowAction::Minimize))
                .expect("the first button is clickable");
            Ok(first.left)
        };

        // Two 100px items leave 200px, which the run shares by its alignment.
        assert_eq!(first_left("")?, 0);
        assert_eq!(first_left(r#"justify-content="center""#)?, 100);
        assert_eq!(first_left(r#"justify-content="end""#)?, 200);
        // `horizontal-align` is the same idea under the spelling the example uses.
        assert_eq!(first_left(r#"horizontal-align="right""#)?, 200);
        Ok(())
    }

    #[test]
    fn align_self_overrides_the_alignment_of_its_container() -> anyhow::Result<()> {
        // `align-items` centres on the cross axis and one item can step out of
        // that with `align-self`, which is how a row keeps its text centred and
        // one control on the bottom edge.
        let files = one_page_project(
            r##"<Page width="400" height="100">
                  <HBox position="absolute" left="0" top="0" width="400" height="100"
                        align-items="center" item-spacing="10">
                    <Button id="middle" action="minimize" width="80" height="40" />
                    <Button id="bottom" action="close" width="80" height="40" align-self="end" />
                    <Button id="top" action="install" width="80" height="40" align-self="start" />
                  </HBox>
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        let top_of = |action: fn(&WindowAction) -> bool| -> i32 {
            ui.actions
                .iter()
                .find(|region| action(&region.action))
                .expect("the button is clickable")
                .top
        };
        assert_eq!(
            top_of(|action| matches!(action, WindowAction::Minimize)),
            30
        );
        assert_eq!(top_of(|action| matches!(action, WindowAction::Close)), 60);
        assert_eq!(top_of(|action| matches!(action, WindowAction::Install)), 0);

        // A vertical container measures its cross axis across the page instead, so
        // `align-self="end"` pushes the item to the right edge.
        let files = one_page_project(
            r##"<Page width="400" height="100">
                  <VBox position="absolute" left="0" top="0" width="400" height="100">
                    <Button id="left" action="minimize" width="80" height="20" />
                    <Button id="right" action="close" width="80" height="20" align-self="end" />
                  </VBox>
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        let left_of = |action: fn(&WindowAction) -> bool| -> i32 {
            ui.actions
                .iter()
                .find(|region| action(&region.action))
                .expect("the button is clickable")
                .left
        };
        assert_eq!(
            left_of(|action| matches!(action, WindowAction::Minimize)),
            0
        );
        assert_eq!(left_of(|action| matches!(action, WindowAction::Close)), 320);
        Ok(())
    }

    #[test]
    fn each_container_tag_accepts_the_alignment_spelling_it_documents() {
        // The two spellings mean the main and the cross axis of the tag that
        // carries them, not a fixed direction, and `Content` stacks vertically
        // only when it says so.
        let document = roxmltree::Document::parse(
            r##"<Page>
                  <HBox id="row" />
                  <VBox id="column" horizontal-align="center" />
                  <Content id="default" layout="horizontal" horizontal-align="right"
                           vertical-align="center" />
                  <Content id="vertical" layout="vertical" />
                </Page>"##,
        )
        .expect("layout parses");
        assert!(matches!(
            flow_axis(node_with_id(&document, "row")),
            Some(FlowAxis::Horizontal)
        ));
        assert!(matches!(
            flow_axis(node_with_id(&document, "column")),
            Some(FlowAxis::Vertical)
        ));
        assert!(matches!(
            flow_axis(node_with_id(&document, "default")),
            Some(FlowAxis::Horizontal)
        ));
        assert!(matches!(
            flow_axis(node_with_id(&document, "vertical")),
            Some(FlowAxis::Vertical)
        ));

        // A horizontal row reads both spellings: the main axis first.
        let content = node_with_id(&document, "default");
        assert_eq!(main_alignment(content, FlowAxis::Horizontal), Some("right"));
        assert_eq!(
            cross_alignment(content, FlowAxis::Horizontal),
            Some("center")
        );
        // A vertical column has no horizontal main axis, so the same attribute
        // there is its cross alignment instead.
        let column = node_with_id(&document, "column");
        assert_eq!(main_alignment(column, FlowAxis::Vertical), None);
        assert_eq!(cross_alignment(column, FlowAxis::Vertical), Some("center"));
        // One item steps out of whatever the container asked for.
        let document = roxmltree::Document::parse(
            r##"<Page><HBox id="row" align-items="center" /><Box id="item" align-self="end" /></Page>"##,
        )
        .expect("layout parses");
        assert_eq!(
            cross_alignment_for_item(
                node_with_id(&document, "item"),
                node_with_id(&document, "row"),
                FlowAxis::Horizontal
            ),
            Some("end")
        );
        assert_eq!(
            cross_alignment_for_item(
                node_with_id(&document, "row"),
                node_with_id(&document, "row"),
                FlowAxis::Horizontal
            ),
            Some("center")
        );
    }

    #[test]
    fn a_wrapping_row_gives_each_line_the_height_of_its_tallest_item() -> anyhow::Result<()> {
        // A narrow window reflows a wrapping row, and the next line starts below
        // the tallest item of the current one, so cards of different heights do
        // not overlap each other.
        let files = one_page_project(
            r##"<Page width="260" height="200">
                  <HBox position="absolute" left="0" top="0" width="260" height="200"
                        flex-wrap="true" gap="10">
                    <Button id="first" action="minimize" width="120" height="40" />
                    <Button id="taller" action="close" width="120" height="70" />
                    <Button id="next_line" action="install" width="120" height="30" />
                  </HBox>
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        let region = |action: fn(&WindowAction) -> bool| -> (i32, i32) {
            let region = ui
                .actions
                .iter()
                .find(|region| action(&region.action))
                .expect("the button is clickable");
            (region.left, region.top)
        };
        // Two 120px buttons and the gap fill the row, so the third wraps.
        assert_eq!(
            region(|action| matches!(action, WindowAction::Minimize)),
            (0, 0)
        );
        assert_eq!(
            region(|action| matches!(action, WindowAction::Close)),
            (130, 0)
        );
        // The line was 70px tall, the tallest card on it, and the gap follows it.
        assert_eq!(
            region(|action| matches!(action, WindowAction::Install)),
            (0, 80)
        );
        Ok(())
    }

    #[test]
    fn a_spacer_takes_what_the_fixed_items_leave() -> anyhow::Result<()> {
        // `Spacer` draws nothing: what it does is push the items after it to the
        // far end, which is how the example keeps its agreement row and its
        // buttons apart.
        let files = one_page_project(
            r##"<Page width="400" height="60">
                  <HBox position="absolute" left="0" top="0" width="400" height="60"
                        align-items="center">
                    <Button id="near" action="minimize" width="100" height="40" />
                    <Spacer flex-grow="1" />
                    <Button id="far" action="close" width="100" height="40" />
                  </HBox>
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        let near = ui
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::Minimize))
            .expect("the first button is clickable");
        let far = ui
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::Close))
            .expect("the second button is clickable");
        assert_eq!((near.left, near.right), (0, 100));
        // The spacer absorbed the 200px between the two fixed buttons.
        assert_eq!((far.left, far.right), (300, 400));
        Ok(())
    }

    #[test]
    fn a_nested_container_reports_the_extent_its_children_need() {
        // A panel without a declared size is as big as what it holds: along its
        // own axis its children add up with their gaps, across it the largest
        // child wins, and its own padding is counted exactly once. Measuring the
        // wrong edge is what used to collapse a dialog row.
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let translations = HashMap::new();
        let interaction = InteractionState::default();
        let context = context_for(&files, &config, &translations, &interaction);
        let document = roxmltree::Document::parse(
            r##"<Page>
                  <VBox id="panel" padding="2">
                    <Label id="one" text="one" height="20" width="60" />
                    <Label id="two" text="two" height="30" width="90" />
                  </VBox>
                </Page>"##,
        )
        .expect("layout parses");
        let panel = node_with_id(&document, "panel");
        // Stacked: 20 + 30, plus the container's own top and bottom padding.
        assert_eq!(
            container_intrinsic_size(panel, FlowAxis::Vertical, &context),
            50 + 4
        );
        // Across: the tallest child, and the padding is added once and not twice.
        assert_eq!(
            container_intrinsic_size(panel, FlowAxis::Horizontal, &context),
            90 + 4
        );

        // An outer row places that panel with the same measurement, so a layout
        // does not have to declare a size for a wrapper.
        let document = roxmltree::Document::parse(
            r##"<Page>
                  <HBox id="row">
                    <VBox id="wrap" padding="2">
                      <Label id="only" text="one" height="20" width="60" />
                    </VBox>
                  </HBox>
                </Page>"##,
        )
        .expect("layout parses");
        assert_eq!(
            container_intrinsic_size(node_with_id(&document, "row"), FlowAxis::Vertical, &context),
            24
        );
    }

    #[test]
    fn an_element_is_pinned_by_the_edge_attribute_it_carries() {
        // `right` and `bottom` measure from the far edge and `inset` is the
        // shorthand for the near ones. A declared near edge wins over both,
        // because the layout has already said where the element starts.
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let config = serde_json::json!({});
        let translations = HashMap::new();
        let interaction = InteractionState::default();
        let context = context_for(&files, &config, &translations, &interaction);

        let from_the_right = parsed_layout(r#"<Box width="60" right="10" />"#);
        assert_eq!(
            anchored_left(from_the_right.root_element(), 0, 60, 400, &context),
            330
        );
        let from_the_bottom = parsed_layout(r#"<Box height="30" bottom="20" />"#);
        assert_eq!(
            anchored_top(from_the_bottom.root_element(), 0, 30, 200, &context),
            150
        );
        // The shorthand insets every edge at once.
        let shorthand = parsed_layout(r#"<Box width="60" inset="8" />"#);
        assert_eq!(
            anchored_left(shorthand.root_element(), 0, 60, 400, &context),
            8
        );
        assert_eq!(
            anchored_top(shorthand.root_element(), 0, 30, 200, &context),
            8
        );
        // A single side stands on its own, and `left` beats the shorthand.
        let one_side = parsed_layout(r#"<Box width="60" inset-left="12" />"#);
        assert_eq!(
            anchored_left(one_side.root_element(), 0, 60, 400, &context),
            12
        );
        // The far edge has single-side forms too, and they measure the way the
        // edge attribute they name does.
        let far_side = parsed_layout(r#"<Box width="60" inset-right="18" />"#);
        assert_eq!(
            anchored_left(far_side.root_element(), 0, 60, 400, &context),
            322
        );
        let below = parsed_layout(r#"<Box height="30" inset-bottom="26" />"#);
        assert_eq!(
            anchored_top(below.root_element(), 0, 30, 200, &context),
            144
        );
        let both_edges = parsed_layout(r#"<Box width="60" left="25" inset="8" />"#);
        assert_eq!(
            anchored_left(both_edges.root_element(), 0, 60, 400, &context),
            25
        );
        // The base the page placed the element at is added to the inset, not
        // replaced by it.
        assert_eq!(
            anchored_left(shorthand.root_element(), 100, 60, 400, &context),
            108
        );
        // The page itself is the parent an absolutely positioned control is
        // measured against, so the far edge is the page's own width.
        let at_the_far_edge = parsed_layout(r#"<Box width="100" right="0" />"#);
        assert_eq!(
            anchored_left(at_the_far_edge.root_element(), 0, 100, 400, &context),
            300
        );
    }

    #[test]
    fn a_label_takes_its_text_font_and_alignment_from_the_layout() -> anyhow::Result<()> {
        // A Label is the control whose whole content is text, so its attributes
        // are the ones a project tunes per language. `value` is accepted where
        // `text` is, and the alignment is what keeps a centred line centred
        // instead of starting at its left edge.
        let files = one_page_project(
            r##"<Page width="400" height="200">
                  <Label id="version" position="absolute" left="10" top="10" width="380" height="18"
                         value="@version_label" font-size="16" font-weight="bold"
                         color="#CCFFFFFF" textalign="center" />
                  <Label id="plain" position="absolute" left="10" top="40" width="100" height="18"
                         text="plain" />
                </Page>"##,
            r##"{"version_label":"v2026.9.17-r2"}"##,
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;

        assert_eq!(ui.texts.len(), 2);
        let version = &ui.texts[0];
        assert_eq!(visible_text(version), "v2026.9.17-r2");
        assert_eq!(version.font_size, 16);
        assert!(version.bold);
        assert!(matches!(version.alignment, TextAlignment::Center));
        // Text is drawn with the RGB part of the colour, alpha ignored.
        assert_eq!(version.runs[0].color.0, parse_color("#FFFFFF").0);
        // A label without an alignment starts at its own left edge, unlike a
        // button whose text is centred by default.
        assert!(matches!(ui.texts[1].alignment, TextAlignment::Left));

        // The font size follows the display like every other measurement.
        let scaled = drawn_for_display(&files, &InteractionState::default(), 2.0, true)?;
        assert_eq!(scaled.texts[0].font_size, 32);
        Ok(())
    }

    #[test]
    fn value_sources_read_the_config_the_disk_and_the_running_step() -> anyhow::Result<()> {
        // The three sources the guide lists. `config:` walks the project file,
        // `disk-free:` asks Windows about the volume a field points at, and the
        // two formats turn the number they find into text a user reads.
        let config = serde_json::json!({
            "project": { "version": "2026.9.17-r2" },
            "install": { "required_space_mb": 200, "default_path": "C:\\Program Files\\Demo" }
        });
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let translations = HashMap::new();
        let mut interaction = InteractionState::default();
        interaction
            .text_input_values
            .insert("editDir".to_string(), "C:\\Program Files\\Demo".to_string());
        let context = context_for(&files, &config, &translations, &interaction);

        assert_eq!(
            resolve_value_source("config:project.version", None, &context).as_deref(),
            Some("2026.9.17-r2")
        );
        assert_eq!(
            resolve_value_source("config:install.default_path", None, &context).as_deref(),
            Some("C:\\Program Files\\Demo")
        );
        // `size-mb` reads the configured mebibytes; without a format the number
        // is shown as it stands.
        assert_eq!(
            resolve_value_source(
                "config:install.required_space_mb",
                Some("size-mb"),
                &context
            )
            .as_deref(),
            Some("200 MB")
        );
        assert_eq!(
            resolve_value_source("config:install.required_space_mb", None, &context).as_deref(),
            Some("200")
        );
        // A path the project file does not have has nothing to show.
        assert_eq!(
            resolve_value_source("config:install.missing", None, &context),
            None
        );

        // `disk-free:` measures the volume the named field points at, and `size`
        // formats the bytes it reports.
        let free = disk_free_bytes(Path::new("C:\\Program Files\\Demo"))
            .context("the demo drive reports no free space")?;
        let formatted = format_size_bytes(free);
        assert_eq!(
            resolve_value_source("disk-free:editDir", Some("size"), &context).as_deref(),
            Some(formatted.as_str())
        );
        // A field the user has not filled in yet has no path to measure, and a
        // source nothing recognises resolves to nothing.
        assert_eq!(
            resolve_value_source("disk-free:nothing", Some("size"), &context),
            None
        );
        assert_eq!(resolve_value_source("something:else", None, &context), None);
        Ok(())
    }

    #[test]
    fn a_bound_label_shows_its_own_text_beside_the_value_it_reads() -> anyhow::Result<()> {
        // A layout writes a localized label and the runtime appends the value, so
        // the same markup reads in every language: "Required: 200 MB". A source
        // with nothing to show leaves a placeholder rather than an empty line.
        let layout = r##"<Page width="400" height="100">
                  <Label id="space" position="absolute" left="0" top="0" width="300" height="18"
                         text="@required_space" value-source="config:install.required_space_mb"
                         value-format="size-mb" />
                  <Label id="none" position="absolute" left="0" top="20" width="300" height="18"
                         text="@available_space" value-source="config:install.missing" />
                </Page>"##;
        let locale = r##"{"required_space":"Required: ","available_space":"Available: "}"##;
        let files = one_page_project_with(
            serde_json::json!({
                "resources": { "locales_dir": "locales" },
                "install": { "required_space_mb": 200 },
                "wizard": { "pages": [{ "layout": "layouts/page.xml" }] }
            }),
            layout,
            locale,
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(visible_text(&ui.texts[0]), "Required: 200 MB");
        assert_eq!(visible_text(&ui.texts[1]), "Available: --");
        Ok(())
    }

    #[test]
    fn a_hidden_element_takes_its_whole_subtree_with_it() -> anyhow::Result<()> {
        // `visible="false"` on an ancestor hides everything under it, which is how
        // a project keeps an alternative panel out of the window until it is
        // asked for. The rest of the page is unaffected.
        let layout = |visible: &str| {
            format!(
                r##"<Page width="400" height="200">
                  <Box id="panel" position="absolute" left="0" top="0" width="200" height="100"
                       visible="{visible}" background="#FF303F4B">
                    <Button id="inside" action="install" position="absolute" left="10" top="10"
                            width="100" height="30" />
                    <Label id="caption" position="absolute" left="10" top="50" width="100"
                           height="18" text="caption" />
                  </Box>
                  <Button id="outside" action="close" position="absolute" left="10" top="120"
                          width="100" height="30" />
                </Page>"##
            )
        };
        let ui = drawn_at_96(
            &one_page_project(&layout("false"), "{}"),
            &InteractionState::default(),
        )?;
        assert!(!ui
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::Install)));
        assert!(ui
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::Close)));
        assert!(!ui
            .texts
            .iter()
            .map(visible_text)
            .any(|text| text == "caption"));
        // The panel's own fill is not painted either.
        assert!(!ui
            .layers
            .iter()
            .any(|layer| (layer.width, layer.height) == (200, 100)));

        // The same layout with the attribute turned on draws the branch again.
        let shown = drawn_at_96(
            &one_page_project(&layout("true"), "{}"),
            &InteractionState::default(),
        )?;
        assert!(shown
            .actions
            .iter()
            .any(|region| matches!(region.action, WindowAction::Install)));
        assert!(shown
            .layers
            .iter()
            .any(|layer| (layer.width, layer.height) == (200, 100)));
        assert!(shown
            .texts
            .iter()
            .map(visible_text)
            .any(|text| text == "caption"));
        Ok(())
    }

    #[test]
    fn a_panel_pair_shows_the_panel_and_only_the_control_that_fits() -> anyhow::Result<()> {
        // `toggle_panel:<id>:show` and `:hide` are two controls on either side of
        // the same panel: while the panel is closed the show control is the one
        // drawn, and expanding the panel swaps them over.
        let files = one_page_project(
            r##"<Page width="400" height="200">
                  <Box id="panel" position="absolute" left="0" top="40" width="400" height="80"
                       visible="false" background="#FF303F4B" />
                  <Button id="show" action="toggle_panel:panel:show" position="absolute"
                          left="0" top="0" width="100" height="30" />
                  <Button id="hide" action="toggle_panel:panel:hide" position="absolute"
                          left="120" top="0" width="100" height="30" />
                </Page>"##,
            "{}",
        );
        let collapsed = drawn_at_96(&files, &InteractionState::default())?;
        assert!(collapsed.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::SetPanelVisibility { ref id, visible: true } if id == "panel"
        )));
        assert!(!collapsed.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::SetPanelVisibility { visible: false, .. }
        )));
        assert!(!collapsed
            .layers
            .iter()
            .any(|layer| (layer.width, layer.height) == (400, 80)));

        let mut interaction = InteractionState::default();
        interaction
            .panel_visibility
            .insert("panel".to_string(), true);
        let expanded = drawn_at_96(&files, &interaction)?;
        assert!(expanded.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::SetPanelVisibility { ref id, visible: false } if id == "panel"
        )));
        assert!(!expanded.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::SetPanelVisibility { visible: true, .. }
        )));
        assert!(expanded
            .layers
            .iter()
            .any(|layer| (layer.width, layer.height) == (400, 80)));
        Ok(())
    }

    #[test]
    fn a_progress_bar_paints_a_rounded_track_and_follows_the_live_value() -> anyhow::Result<()> {
        // `background` is the track and `border-radius` rounds it, so a thin bar
        // reads as a pill. The authored `progress` is what an idle wizard shows,
        // a running task overrides it, and the authored value comes back.
        let mut files = one_page_project(
            r##"<Page width="400" height="100">
                  <ProgressBar id="bar" position="absolute" left="10" top="20" width="200"
                               height="10" progress="40" border-radius="5"
                               background="#FF4C5868" bar-image="assets/bar_installing.png" />
                </Page>"##,
            "{}",
        );
        files.insert(
            "assets/bar_installing.png".to_string(),
            example_asset("bar_installing.png"),
        );

        let idle = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(idle.layers.len(), 2);
        let track = &idle.layers[0];
        assert_eq!(
            (track.left, track.top, track.width, track.height),
            (10, 20, 200, 10)
        );
        // The rounded corner is cut away and the middle of the track is painted.
        let alpha = |x: usize, y: usize| track.image.pixels[(y * 200 + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(100, 5), 255);
        // 40% of the 200px control.
        assert_eq!(idle.layers[1].width, 80);

        // A task that reports its own progress takes the bar over...
        let running = drawn_at_96(
            &files,
            &InteractionState {
                progress: Some(90),
                ..Default::default()
            },
        )?;
        assert_eq!(running.layers[1].width, 180);
        // ...and a task that has not started yet draws the track alone.
        let waiting = drawn_at_96(
            &files,
            &InteractionState {
                progress: Some(0),
                ..Default::default()
            },
        )?;
        assert_eq!(waiting.layers.len(), 1);

        // With nothing running the authored value is drawn again.
        let idle_again = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(idle_again.layers[1].width, 80);
        Ok(())
    }

    #[test]
    fn every_action_in_the_table_answers_with_its_own_window_action() {
        // The actions table is the contract for what a layout may ask the runtime
        // to do, so each row has to reach the action it names; a row that stopped
        // mapping would leave its button doing nothing at all.
        let document = roxmltree::Document::parse(
            r#"<Page>
                 <Button id="minimize" action="minimize" />
                 <Button id="close" action="close" />
                 <Button id="close_confirm" action="close_confirm" />
                 <Button id="cancel" action="cancel" />
                 <Button id="pick_directory" action="pick_directory" target="editDir" />
                 <Button id="open_key" action="open_url:help" />
                 <Button id="open_url" action="open_url:https://example.test/help" />
                 <Button id="install" action="install" />
                 <Button id="uninstall" action="uninstall" />
                 <Button id="launch_app" action="launch_app" />
                 <Button id="finish" action="finish" />
                 <Button id="switch_language" action="switch_language" />
                 <Button id="next" action="next" />
                 <Button id="back" action="back" />
                 <Button id="show_panel" action="toggle_panel:panel:show" />
                 <Button id="hide_panel" action="toggle_panel:panel:hide" />
                 <Button id="dialog_ok" action="dialog_ok" />
                 <Button id="dialog_cancel" action="dialog_cancel" />
                 <Button id="invented" action="not_a_real_action" />
               </Page>"#,
        )
        .expect("layout parses");
        let config = serde_json::json!({
            "links": { "help": "https://example.test/help" }
        });
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let translations = HashMap::new();
        let interaction = InteractionState::default();
        let context = context_for(&files, &config, &translations, &interaction);
        let action = |id: &str| -> Option<WindowAction> {
            let mut actions = Vec::new();
            push_action(
                node_with_id(&document, id),
                LayerRect {
                    left: 0,
                    top: 0,
                    width: 10,
                    height: 10,
                },
                &mut actions,
                &mut Vec::new(),
                &context,
            );
            actions.into_iter().next().map(|region| region.action)
        };

        assert!(matches!(action("minimize"), Some(WindowAction::Minimize)));
        assert!(matches!(action("close"), Some(WindowAction::Close)));
        assert!(matches!(
            action("close_confirm"),
            Some(WindowAction::CloseConfirm)
        ));
        assert!(matches!(action("cancel"), Some(WindowAction::Cancel)));
        assert!(matches!(
            action("pick_directory"),
            Some(WindowAction::PickDirectory { ref id }) if id == "editDir"
        ));
        assert!(matches!(
            action("open_key"),
            Some(WindowAction::OpenLink(ref target)) if target == "https://example.test/help"
        ));
        assert!(matches!(
            action("open_url"),
            Some(WindowAction::OpenLink(ref target)) if target == "https://example.test/help"
        ));
        assert!(matches!(action("install"), Some(WindowAction::Install)));
        assert!(matches!(action("uninstall"), Some(WindowAction::Uninstall)));
        assert!(matches!(
            action("launch_app"),
            Some(WindowAction::LaunchApp)
        ));
        // The finish page closes the wizard, which is the same action as `close`.
        assert!(matches!(action("finish"), Some(WindowAction::Close)));
        assert!(matches!(
            action("switch_language"),
            Some(WindowAction::ToggleSelectMenu { ref id }) if id == "switch_language"
        ));
        assert!(matches!(action("next"), Some(WindowAction::NextPage)));
        assert!(matches!(action("back"), Some(WindowAction::PreviousPage)));
        assert!(matches!(
            action("show_panel"),
            Some(WindowAction::SetPanelVisibility { ref id, visible: true }) if id == "panel"
        ));
        assert!(matches!(
            action("hide_panel"),
            Some(WindowAction::SetPanelVisibility { ref id, visible: false }) if id == "panel"
        ));
        assert!(matches!(action("dialog_ok"), Some(WindowAction::DialogOk)));
        assert!(matches!(
            action("dialog_cancel"),
            Some(WindowAction::DialogCancel)
        ));
        // A row nothing implements leaves the control inert rather than closing
        // the wizard by accident.
        assert!(action("invented").is_none());
    }

    #[test]
    fn an_element_answers_the_pointer_only_when_it_declares_an_action() -> anyhow::Result<()> {
        // The guide's rule for promoting artwork or a label into a link: without
        // an `action` a control is inert even when it covers a clickable area,
        // which is what stops a decoration from swallowing a click.
        let mut files = one_page_project(
            r##"<Page width="400" height="200">
                  <Label id="link" action="open_url:https://example.test/help"
                         position="absolute" left="0" top="0" width="100" height="20" text="help" />
                  <Label id="labelless" position="absolute" left="0" top="0" width="100"
                         height="20" text="decoration" />
                  <Image id="inert_icon" src="assets/checkbox-0.png" position="absolute"
                         left="0" top="0" width="20" height="20" />
                  <Button id="bare_button" position="absolute" left="0" top="0" width="40"
                          height="20" />
                  <Image id="picker" action="pick_directory" target="editDir" cursor="hand"
                         src="assets/checkbox-0.png" position="absolute" left="120" top="0"
                         width="20" height="20" />
                </Page>"##,
            "{}",
        );
        files.insert(
            "assets/checkbox-0.png".to_string(),
            example_asset("checkbox-0.png"),
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;

        // Only the two controls that declare an action are in the list, even
        // though the others are drawn on top of the same spot.
        assert_eq!(ui.actions.len(), 2);
        assert!(ui.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::OpenLink(ref target) if target == "https://example.test/help"
        )));
        assert!(ui.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::PickDirectory { ref id } if id == "editDir"
        )));
        // The artwork is still drawn where the layout put it.
        assert!(ui
            .layers
            .iter()
            .any(|layer| (layer.width, layer.height) == (20, 20)));

        // A button that is in a flow container is clickable through the action it
        // declares; a select answers with the language menu, which is not an
        // `action` attribute value on a plain control.
        let files = one_page_project(
            r##"<Page width="400" height="200">
                  <Select id="lang" action="switch_language" position="absolute" left="0" top="0"
                          width="96" height="26">
                    <Option value="zh-CN" text="简体中文" />
                    <Option value="en-US" text="English" />
                  </Select>
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        assert!(ui.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::ToggleSelectMenu { ref id } if id == "lang"
        )));
        // A page draws no menu until one is opened, so there is nothing to
        // walk yet.
        assert!(ui.menu.is_none());
        Ok(())
    }

    /// A checkbox that is given coordinates behaves like one in a flow: it draws
    /// the image for its state, and it registers the click that flips that state.
    ///
    /// The shipped uninstall page places its keep-data box absolutely, so a
    /// page-level checkbox that reached neither the image nor the click region
    /// left that box drawn as a caption that nobody could toggle.
    #[test]
    fn an_absolutely_placed_checkbox_draws_its_state_image_and_toggles() -> anyhow::Result<()> {
        let project = |checked: &str| {
            let mut files = one_page_project(
                &format!(
                    r##"<Page width="400" height="200">
                          <Checkbox id="chkKeep" text="Keep my data" checked="{checked}"
                                    position="absolute" left="20" top="30" width="240" height="20"
                                    unchecked-image="file='assets/checkbox-0.png' dest='0,1,16,17'"
                                    checked-image="file='assets/checkbox-2.png' dest='0,1,16,17'" />
                        </Page>"##
                ),
                "{}",
            );
            for name in ["checkbox-0.png", "checkbox-2.png"] {
                files.insert(format!("assets/{name}"), example_asset(name));
            }
            files
        };

        let checked = drawn_at_96(&project("true"), &InteractionState::default())?;
        let unchecked = drawn_at_96(&project("false"), &InteractionState::default())?;

        // Flipping a checkbox is a click, so the region and the state it reports
        // have to be there for a box the layout gave coordinates to.
        let region = checked
            .actions
            .iter()
            .find(|region| matches!(region.action, WindowAction::ToggleCheckbox { .. }))
            .context("an absolutely placed checkbox registers no click")?;
        assert!(matches!(
            &region.action,
            WindowAction::ToggleCheckbox { id, checked } if id == "chkKeep" && *checked
        ));
        assert_eq!(
            (region.left, region.top, region.right, region.bottom),
            (20, 30, 260, 50),
            "the region does not cover the control"
        );

        // The state chooses the artwork, so the two runs cannot draw the same
        // pixels where the box is.
        let artwork = |ui: &RuntimeUi| {
            ui.layers
                .iter()
                .map(|layer| layer.image.pixels.clone())
                .collect::<Vec<_>>()
        };
        assert!(
            !checked.layers.is_empty(),
            "the checkbox drew no state image"
        );
        assert_ne!(
            artwork(&checked),
            artwork(&unchecked),
            "both states drew the same artwork"
        );
        Ok(())
    }

    #[test]
    fn a_closed_language_select_draws_its_arrow_over_its_fill_and_outline() -> anyhow::Result<()> {
        // The select is the one control a project skins with three pieces: a fill,
        // an outline, and the arrow that says the list opens. The arrow sits in
        // from the far edge and is centred, and both move with the display like
        // every other measurement in the layout.
        let markup = r##"<Page width="400" height="200">
                  <Select id="lang" action="switch_language" position="absolute" left="10" top="10"
                          width="96" height="26" background="#FF303F4B" border-color="#1AFFFFFF"
                          border-width="1" border-radius="8" font-size="12"
                          dropdown-image="assets/select-arrow.png"
                          dropdown-open-image="assets/select-arrow-up.png"
                          dropdown-image-width="8" dropdown-image-height="6">
                    <Option value="zh-CN" text="简体中文" />
                    <Option value="en-US" text="English" />
                  </Select>
                </Page>"##;
        let mut files = one_page_project(markup, "{}");
        files.insert(
            "assets/select-arrow.png".to_string(),
            example_asset("select-arrow.png"),
        );
        files.insert(
            "assets/select-arrow-up.png".to_string(),
            example_asset("select-arrow-up.png"),
        );
        // The second locale the layout offers, so the select can be read in it.
        files.insert("locales/en-US.json".to_string(), b"{}".to_vec());

        let ui = drawn_at_96(&files, &InteractionState::default())?;
        // The fill, the outline over it, then the arrow: 10px in from the far
        // edge, centred vertically in the control.
        assert_eq!(ui.layers.len(), 3);
        let fill = &ui.layers[0];
        assert_eq!(
            (fill.left, fill.top, fill.width, fill.height),
            (10, 10, 96, 26)
        );
        assert_eq!((ui.layers[1].width, ui.layers[1].height), (96, 26));
        let arrow = &ui.layers[2];
        assert_eq!(
            (arrow.left, arrow.top, arrow.width, arrow.height),
            (10 + 96 - 8 - 10, 10 + (26 - 6) / 2, 8, 6)
        );
        // The arrow pointing down is the one a closed list shows.
        let down = super::decode_image(&example_asset("select-arrow.png"))?;
        assert_eq!(arrow.image.pixels, down.pixels);
        // The option in use is the text the select reads, not the first one.
        assert_eq!(visible_text(&ui.texts[0]), "简体中文");
        assert_eq!(visible_text(&english_select(&files)?.texts[0]), "English");

        // An open list points the other way, and a scaled display draws the arrow
        // at twice the size, twice the inset from the edge.
        let open = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            Some("lang"),
            &InteractionState::default(),
            RuntimeMode::Installer,
            None,
        )?;
        let arrow = &open.layers[2];
        let up = super::decode_image(&example_asset("select-arrow-up.png"))?;
        assert_eq!(arrow.image.pixels, up.pixels);
        let scaled = drawn_for_display(&files, &InteractionState::default(), 2.0, true)?;
        let arrow = &scaled.layers[2];
        assert_eq!(
            (arrow.left, arrow.top, arrow.width, arrow.height),
            (2 * (10 + 96 - 8 - 10), 2 * (10 + (26 - 6) / 2), 16, 12)
        );
        Ok(())
    }

    /// The same select read in another locale, so the text on it is the language
    /// the wizard is running in rather than the first option in the layout.
    fn english_select(files: &HashMap<String, Vec<u8>>) -> anyhow::Result<RuntimeUi> {
        load_layout(
            files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "en-US",
            None,
            &InteractionState::default(),
            RuntimeMode::Installer,
            None,
        )
    }

    /// The colour a solid layer is filled with, read at its middle.
    ///
    /// A fill is drawn inside a rounded rectangle, so its corners stay clear;
    /// the middle is the one pixel a radius can never clip away.
    fn fill_color(layer: &ImageLayer) -> [u8; 4] {
        let x = layer.image.width as usize / 2;
        let y = layer.image.height as usize / 2;
        let offset = (y * layer.image.width as usize + x) * 4;
        [
            layer.image.pixels[offset],
            layer.image.pixels[offset + 1],
            layer.image.pixels[offset + 2],
            layer.image.pixels[offset + 3],
        ]
    }

    #[test]
    fn a_language_menu_lists_its_options_and_marks_the_one_in_use() -> anyhow::Result<()> {
        // The menu is drawn over the page, one row per option, and the row of the
        // language being read is filled so the list says where the user is. The
        // highlight a user moves with the arrow keys takes over that fill, and
        // falls back to it when the layout reserves no colour of its own.
        let markup = |highlight: &str| {
            format!(
                r##"<Page width="400" height="200">
                  <Select id="lang" action="switch_language" position="absolute" left="10" top="10"
                          width="96" height="26" background="#FF303F4B" font-size="12"
                          popup-width="127" popup-row-height="26" popup-padding="2"
                          popup-background="#FF42515E"
                          popup-selected-background="#FF495A68" {highlight}>
                    <Option value="zh-CN" text="简体中文" />
                    <Option value="en-US" text="English" />
                    <Option value="hidden" text="Hidden" visible="false" />
                  </Select>
                </Page>"##
            )
        };
        let plain = one_page_project(&markup(""), "{}");
        let with_highlight =
            one_page_project(&markup(r##"popup-highlight-background="#FF00C4B2""##), "{}");
        let menu = |files: &HashMap<String, Vec<u8>>, interaction: &InteractionState| {
            load_layout(
                files,
                DpiContext {
                    scale: 1.0,
                    use_2x: false,
                },
                "zh-CN",
                Some("lang"),
                interaction,
                RuntimeMode::Installer,
                None,
            )
        };

        let ui = menu(&plain, &InteractionState::default())?;
        // Two options are offered, and the one hidden in the layout is not.
        let rows = ui.menu.as_ref().expect("the menu was open");
        assert_eq!(rows.values, ["zh-CN", "en-US"]);
        assert!(rows.language);
        assert_eq!(ui.overlay_texts.len(), 2);
        assert_eq!(visible_text(&ui.overlay_texts[0]), "简体中文");
        assert_eq!(visible_text(&ui.overlay_texts[1]), "English");
        // The popup is its own fill plus the row of the current locale, which
        // sits one padding step inside it.
        assert_eq!(ui.overlay_layers.len(), 2);
        let popup = &ui.overlay_layers[0];
        assert_eq!(
            (popup.left, popup.top, popup.width, popup.height),
            (10 + 96 - 127, 10 + 26 + 4, 127, 26 * 2 + 4)
        );
        assert_eq!(fill_color(popup), [0x5E, 0x51, 0x42, 0xFF]);
        let current = &ui.overlay_layers[1];
        assert_eq!(
            (current.left, current.top, current.width, current.height),
            (popup.left + 2, popup.top + 2, 127 - 4, 26)
        );
        assert_eq!(fill_color(current), [0x68, 0x5A, 0x49, 0xFF]);

        // Selecting a row switches to that locale, one region per option.
        assert!(ui.actions.iter().any(|region| matches!(
            region.action,
            WindowAction::SelectLanguage(ref locale) if locale == "en-US"
        )));
        assert_eq!(
            ui.actions
                .iter()
                .filter(|region| matches!(region.action, WindowAction::SelectLanguage(_)))
                .count(),
            2
        );

        // The row the arrow keys moved to is filled with the highlight colour,
        // while the language in use keeps its own.
        let moved = InteractionState {
            highlighted_option: Some(1),
            ..Default::default()
        };
        let highlighted = menu(&with_highlight, &moved)?;
        assert_eq!(highlighted.overlay_layers.len(), 3);
        let row = &highlighted.overlay_layers[2];
        assert_eq!((row.left, row.top), (popup.left + 2, popup.top + 2 + 26));
        assert_eq!(fill_color(row), [0xB2, 0xC4, 0x00, 0xFF]);

        // A layout that reserves no highlight colour gets the selected fill.
        let fallback = menu(&plain, &moved)?;
        assert_eq!(fallback.overlay_layers.len(), 3);
        let row = &fallback.overlay_layers[2];
        assert_eq!(fill_color(row), [0x68, 0x5A, 0x49, 0xFF]);
        Ok(())
    }

    #[test]
    fn a_dialog_button_is_drawn_from_the_question_rather_than_the_layout() -> anyhow::Result<()> {
        // The dialog layout is written once and serves every question, so its
        // words come from the dialog state: the question itself, the confirming
        // label and the dismissing one. A role nothing provides draws nothing.
        let (mut files, _) = dialog_fixture();
        files.insert(
            "layouts/msgBox.xml".to_string(),
            br##"<Page width="400" height="230" background="#FF2A3844">
                 <Label id="lblMsg" text="placeholder" value-source="dialog:message" width="336" height="24" />
                 <Button id="btnCancel" action="dialog_cancel" visible-with="dismiss"
                         text="@cancel" value-source="dialog:dismiss" width="160" height="40" />
                 <Button id="btnOK" action="dialog_ok" text="@ok" value-source="dialog:accept"
                         width="160" height="40" />
                 <Button id="btnOdd" action="dialog_ok" text="@ok" value-source="dialog:something"
                         width="160" height="40" />
                 <Label id="lblOdd" text="placeholder" value-source="dialog:something"
                        width="160" height="24" />
               </Page>"##
                .to_vec(),
        );
        let question = DialogState {
            kind: DialogKind::CloseConfirm,
            message: "Discard the download?".to_string(),
            accept_label: "Continue".to_string(),
            dismiss_label: "Stay here".to_string(),
        };
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            &interaction_with_dialog(question),
            RuntimeMode::Installer,
            None,
        )?;
        let texts: Vec<String> = ui.overlay_texts.iter().map(visible_text).collect();

        // The words come from the dialog, not from the locale table the layout
        // would otherwise read `@ok` and `@cancel` from.
        assert!(texts.iter().any(|text| text == "Discard the download?"));
        assert!(texts.iter().any(|text| text == "Continue"));
        assert!(texts.iter().any(|text| text == "Stay here"));
        assert!(!texts.iter().any(|text| text == "Exit"));
        assert!(!texts.iter().any(|text| text == "Keep going"));
        // A role the dialog does not answer leaves its control with no text, and
        // the authored placeholder is not shown in its place.
        assert!(!texts.iter().any(|text| text == "placeholder"));
        Ok(())
    }

    #[test]
    fn a_link_the_project_does_not_configure_stays_plain_text() -> anyhow::Result<()> {
        // The guide's resolution order ends with a target that resolves to nothing
        // staying plain text: the words are still shown and nothing happens when
        // they are clicked, rather than the label failing to draw.
        let config = serde_json::json!({
            "resources": { "locales_dir": "locales" },
            "links": { "terms_of_service": "https://example.test/terms" },
            "wizard": { "pages": [{ "layout": "layouts/page.xml" }] }
        });
        let files = one_page_project_with(
            config,
            r##"<Page width="400" height="100">
                  <Label id="terms" linkcolor="#00C4B2" color="#CCFFFFFF"
                         position="absolute" left="0" top="0" width="380" height="20"
                         text="read [the terms](agreement) and [the notes](missing)" />
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;

        let runs = &ui.texts[0].runs;
        assert_eq!(runs.len(), 4);
        // The historical alias reaches the configured URL...
        assert_eq!(runs[1].text, "the terms");
        assert_eq!(runs[1].link.as_deref(), Some("https://example.test/terms"));
        // ...while a name the project does not carry stays a plain run: the words
        // are there, and they are not a click target.
        assert_eq!(runs[3].text, "the notes");
        assert_eq!(runs[3].link, None);
        assert_eq!(ui.text_hits.len(), 1);
        assert!(matches!(
            ui.text_hits[0].action,
            WindowAction::OpenLink(ref target) if target == "https://example.test/terms"
        ));
        // The whole sentence is drawn, link markup and all.
        assert_eq!(visible_text(&ui.texts[0]), "read the terms and the notes");
        Ok(())
    }

    #[test]
    fn pick_directory_writes_to_the_field_the_page_offers_it() {
        // `target` names the field; without one the first writable TextInput is
        // used, a readonly field only as a last resort, and a page that declares
        // no field at all falls back to the one the runtime has recorded.
        let mut interaction = InteractionState::default();
        interaction
            .text_input_values
            .insert("editDir".to_string(), "C:\\Program Files\\Demo".to_string());

        // The target wins over the field order.
        let targeted = parsed_layout(
            r#"<Page><TextInput id="first" /><Button id="browse" action="pick_directory" target="editDir" /></Page>"#,
        );
        assert_eq!(
            pick_directory_target(node_with_id(&targeted, "browse"), &interaction).as_deref(),
            Some("editDir")
        );
        // Without one, the first field the user can type into is used.
        let two_fields = parsed_layout(
            r#"<Page><TextInput id="readonly" readonly="true" /><TextInput id="writable" /><Button id="browse" action="pick_directory" /></Page>"#,
        );
        assert_eq!(
            pick_directory_target(node_with_id(&two_fields, "browse"), &interaction).as_deref(),
            Some("writable")
        );
        // A page whose only field is a display field still gets the path, because
        // showing it somewhere is better than dropping the choice.
        let display_only = parsed_layout(
            r#"<Page><TextInput id="only" readonly="true" /><Button id="browse" action="pick_directory" /></Page>"#,
        );
        assert_eq!(
            pick_directory_target(node_with_id(&display_only, "browse"), &interaction).as_deref(),
            Some("only")
        );
        // A page with no field of its own falls back to what the runtime holds.
        let no_field =
            parsed_layout(r#"<Page><Button id="browse" action="pick_directory" /></Page>"#);
        assert_eq!(
            pick_directory_target(node_with_id(&no_field, "browse"), &interaction).as_deref(),
            Some("editDir")
        );
    }

    #[test]
    fn text_colors_read_as_rgb_with_or_without_an_alpha_channel() {
        // The guide says colours accept both spellings and that text drawing
        // ignores alpha, so the two have to resolve to the same colour; anything
        // unreadable draws white rather than nothing.
        assert_eq!(parse_color("#00C4B2").0, parse_color("#FF00C4B2").0);
        assert_eq!(parse_color("#FFFFFF").0, parse_color("#00FFFFFF").0);
        let color = parse_color("#010203");
        assert_eq!(color.0 & 0xFF, 0x01);
        assert_eq!((color.0 >> 8) & 0xFF, 0x02);
        assert_eq!((color.0 >> 16) & 0xFF, 0x03);
        assert_eq!(parse_color("not a color").0, parse_color("#FFFFFF").0);
    }

    #[test]
    fn a_typed_value_wins_over_the_bound_default() -> anyhow::Result<()> {
        // A field bound to the project file starts at the configured path, but a
        // value the user typed or picked has to stay on screen for the rest of
        // the run, and the field a `disk-free:` binding reads is the same one.
        let files = one_page_project_with(
            serde_json::json!({
                "resources": { "locales_dir": "locales" },
                "install": { "default_path": "C:\\Program Files\\Demo" },
                "wizard": { "pages": [{ "layout": "layouts/page.xml" }] }
            }),
            r##"<Page width="400" height="100">
                  <TextInput id="editDir" position="absolute" left="10" top="10" width="200"
                             height="20" value-source="config:install.default_path" />
                </Page>"##,
            "{}",
        );
        let bound = drawn_at_96(&files, &InteractionState::default())?;
        assert_eq!(visible_text(&bound.texts[0]), "C:\\Program Files\\Demo");

        let mut interaction = InteractionState::default();
        interaction
            .text_input_values
            .insert("editDir".to_string(), "D:\\Games\\Demo".to_string());
        let typed = drawn_at_96(&files, &interaction)?;
        assert_eq!(visible_text(&typed.texts[0]), "D:\\Games\\Demo");
        Ok(())
    }

    #[test]
    fn an_empty_field_is_one_a_user_can_click_into() -> anyhow::Result<()> {
        // A field with nothing in it draws no text, but it is still a field: it
        // takes the caret, which is how the value a page asks the user for gets
        // typed in at all. A readonly field stays out of it, as it does with
        // text.
        let files = one_page_project(
            r##"<Page width="400" height="200">
                  <TextInput id="editDir" required="true"
                             position="absolute" left="10" top="10" width="200" height="20"
                             font-size="14" color="#FF00FF00" />
                  <TextInput id="shown" readonly="true"
                             position="absolute" left="10" top="40" width="200" height="20" />
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        assert!(ui.texts.is_empty(), "an empty field drew text");
        assert_eq!(ui.text_inputs.len(), 1);
        assert_eq!(ui.text_inputs[0].id, "editDir");
        assert_eq!(ui.text_inputs[0].text, "");
        // The caret follows the font and colour the field declares.
        assert_eq!(ui.text_inputs[0].font_size, 14);
        assert_eq!(ui.text_inputs[0].color, parse_color("#FF00FF00"));

        // A field inside a flow container is reached the same way.
        let flow = one_page_project(
            r#"<Page width="400" height="200">
                 <VBox position="absolute" left="0" top="0" width="100%" height="100%" padding="8">
                   <TextInput id="editDir" height="20" />
                 </VBox>
               </Page>"#,
            "{}",
        );
        let ui = drawn_at_96(&flow, &InteractionState::default())?;
        assert_eq!(ui.text_inputs.len(), 1);
        assert_eq!(ui.text_inputs[0].id, "editDir");
        assert_eq!(ui.text_inputs[0].top, 8);
        Ok(())
    }

    #[test]
    fn a_readonly_field_shows_its_value_without_taking_edits() -> anyhow::Result<()> {
        // `readonly="true"` is what makes a field display-only: it still draws its
        // text, but it takes no typed characters, so it is not recorded as an
        // editable field. `readonly="false"` says the opposite and stays editable.
        let files = one_page_project(
            r##"<Page width="400" height="200">
                  <TextInput id="shown" value="C:\Program Files\Demo" readonly="true"
                             position="absolute" left="10" top="10" width="200" height="20" />
                  <TextInput id="editable" value="readonly=false" readonly="false"
                             position="absolute" left="10" top="40" width="200" height="20" />
                </Page>"##,
            "{}",
        );
        let ui = drawn_at_96(&files, &InteractionState::default())?;
        // Both fields draw their value...
        assert_eq!(ui.texts.len(), 2);
        assert_eq!(visible_text(&ui.texts[0]), "C:\\Program Files\\Demo");
        // ...but only the one that accepts typing is recorded.
        assert_eq!(ui.text_inputs.len(), 1);
        assert_eq!(ui.text_inputs[0].id, "editable");
        Ok(())
    }

    /// A page whose controls are declared in an order a case can name, with the
    /// ones that must stay out of the keyboard's way beside them.
    ///
    /// The `gone` control is hidden by the panel state, so a case can check that
    /// what the page hides is not reachable by Tab either.
    const FOCUS_ORDER_PAGE: &str = r##"<Page width="400" height="200" background="#FF000000" focus-color="#FF00FF00">
                  <Button id="first" action="next" text="Next"
                          position="absolute" left="20" top="20" width="120" height="30" />
                  <TextInput id="field" position="absolute" left="20" top="60"
                             width="200" height="24" />
                  <TextInput id="sealed" readonly="true" position="absolute" left="20" top="90"
                             width="200" height="24" />
                  <TextInput position="absolute" left="20" top="120" width="200" height="24" />
                  <Checkbox id="terms" position="absolute" left="240" top="20"
                            width="24" height="24" />
                  <Button id="locked" action="next" enabled="false" text="Locked"
                          position="absolute" left="240" top="60" width="120" height="30" />
                  <Button id="gone" action="next" text="Gone"
                          position="absolute" left="240" top="100" width="120" height="30" />
                  <Label id="note" text="Just words"
                         position="absolute" left="240" top="140" width="120" height="20" />
                </Page>"##;

    fn focus_order_project() -> HashMap<String, Vec<u8>> {
        one_page_project(FOCUS_ORDER_PAGE, "{}")
    }

    /// The same page in a project that declares the dialog layout a question is
    /// drawn from, so a case can cover the page with one.
    fn focus_order_project_with_dialog() -> HashMap<String, Vec<u8>> {
        let mut config = one_page_config();
        config["ui"] = serde_json::json!({ "dialog_layout": "layouts/msgBox.xml" });
        let mut files = one_page_project_with(config, FOCUS_ORDER_PAGE, "{}");
        files.insert(
            "layouts/msgBox.xml".to_string(),
            br##"<Page width="400" height="230" background="#FF2A3844">
                  <Label id="lblMsg" text="placeholder" value-source="dialog:message"
                         position="absolute" left="20" top="20" width="360" height="40" />
                  <Button id="btnOK" action="dialog_ok" text="@ok"
                          position="absolute" left="240" top="170" width="140" height="36" />
                </Page>"##
                .to_vec(),
        );
        files
    }

    /// The interaction state of `focus_order_project` with the panel that hides
    /// `gone` closed, which is what keeps that control off the page.
    fn focus_order_interaction() -> InteractionState {
        let mut interaction = InteractionState::default();
        interaction
            .panel_visibility
            .insert("gone".to_string(), false);
        interaction
    }

    /// The corner of a control, at an even coordinate, where a dotted ring draws
    /// its first pixel.
    ///
    /// The ring is dotted from the rectangle's own corner: every other pixel of
    /// every edge, starting with the corner itself.
    fn ring_corner(layer: &ImageLayer) -> [u8; 4] {
        [
            layer.image.pixels[0],
            layer.image.pixels[1],
            layer.image.pixels[2],
            layer.image.pixels[3],
        ]
    }

    /// What one pixel of a layer holds, as blue, green, red and alpha.
    fn layer_pixel(layer: &ImageLayer, x: usize, y: usize) -> [u8; 4] {
        let offset = (y * layer.image.width as usize + x) * 4;
        [
            layer.image.pixels[offset],
            layer.image.pixels[offset + 1],
            layer.image.pixels[offset + 2],
            layer.image.pixels[offset + 3],
        ]
    }

    // The eight colours a case gives the scheme of its own, all different so a
    // claim about which role a colour was painted with can be checked. The
    // themes that ship with Windows paint a page and a control face in the same
    // colour, which would leave a case unable to tell the two apart.
    const SCHEME_SURFACE: u32 = 0x0001_0203;
    const SCHEME_CONTROL: u32 = 0x0004_0506;
    const SCHEME_HIGHLIGHT: u32 = 0x0007_0809;
    const SCHEME_TEXT: u32 = 0x000A_0B0C;
    const SCHEME_CONTROL_TEXT: u32 = 0x000D_0E0F;
    const SCHEME_HIGHLIGHT_TEXT: u32 = 0x0010_1112;
    const SCHEME_LINE: u32 = 0x0013_1415;
    const SCHEME_SCROLLBAR: u32 = 0x0016_1718;

    /// The scheme a case states instead of the machine's own.
    fn scheme() -> contrast::Palette {
        contrast::Palette::named(
            SCHEME_SURFACE,
            SCHEME_CONTROL,
            SCHEME_HIGHLIGHT,
            SCHEME_TEXT,
            SCHEME_CONTROL_TEXT,
            SCHEME_HIGHLIGHT_TEXT,
            SCHEME_LINE,
            SCHEME_SCROLLBAR,
        )
    }

    /// The colour the scheme names for a role, read the way the renderer reads
    /// it, so a case can compare text colours as well as pixels.
    fn scheme_colour(role: contrast::Role) -> COLORREF {
        parse_color(&scheme().colour(role))
    }

    /// The pixel a colour the machine reports paints, in the order a layer holds
    /// its components.
    fn pixel(colour: u32) -> [u8; 4] {
        [
            (colour >> 16) as u8,
            (colour >> 8) as u8,
            colour as u8,
            0xFF,
        ]
    }

    /// Every colour painted over one rectangle, read at one pixel of it, in the
    /// order it was painted, so a case can say what a surface was drawn with
    /// without depending on where in the layer list it landed.
    ///
    /// The pixel is named because a rectangle carries two layers at once: a fill
    /// is read at the middle of the rectangle, where a rounded corner does not
    /// reach, and a line where it runs along an edge.
    fn painted_over(
        layers: &[ImageLayer],
        rect: (i32, i32, i32, i32),
        at: (usize, usize),
    ) -> Vec<[u8; 4]> {
        layers
            .iter()
            .filter(|layer| (layer.left, layer.top, layer.width, layer.height) == rect)
            .map(|layer| layer_pixel(layer, at.0, at.1))
            .collect()
    }

    /// The keyboard walks the page the way the page is written: the controls it
    /// can reach are recorded in the order the layout lays them out, each over
    /// the rectangle it was placed at, so Tab follows the order a reader reads
    /// rather than a numbering the layout would have to keep in step by hand.
    ///
    /// What carries an action carries what activating it does; a text field
    /// carries none, because a keystroke there is typing and the caret is what
    /// answers it.
    #[test]
    fn tab_reaches_every_control_in_the_order_the_page_lays_them_out() -> anyhow::Result<()> {
        // The keyboard walks the page the way the page is written, so the order
        // is the one a reader follows rather than a numbering the layout would
        // have to keep in step by hand.
        let files = focus_order_project();
        let ui = drawn_at_96(&files, &focus_order_interaction())?;
        let reached: Vec<&str> = ui
            .focus_regions
            .iter()
            .map(|region| region.id.as_str())
            .collect();
        assert_eq!(reached, ["first", "field", "terms"]);

        // Each region is the rectangle the control was placed at, which is what
        // the ring is drawn over and what a case can find the control by.
        let first = &ui.focus_regions[0];
        assert_eq!(
            (first.left, first.top, first.right, first.bottom),
            (20, 20, 140, 50)
        );
        // A button carries what activating it does; a text field carries nothing,
        // because a keystroke there is typing.
        assert!(matches!(first.action, Some(WindowAction::NextPage)));
        assert!(ui.focus_regions[1].action.is_none());
        Ok(())
    }

    /// A control the keyboard must not stop on is not in the order: a button a
    /// condition holds back answers nothing, a readonly field takes no typing, a
    /// control the page hides is not on the page, and an element with no action
    /// does nothing when it is activated.
    ///
    /// A field the layout gives no id is left out too, because there would be
    /// nowhere to say which control the ring is on.
    #[test]
    fn a_control_the_page_keeps_out_of_reach_is_not_in_the_tab_order() {
        // A disabled button answers nothing, a readonly field takes no typing,
        // and a control the page hides is not on the page: none of them is
        // somewhere Tab can land, or the keyboard would stop on a control that
        // does not answer.
        let files = focus_order_project();
        let ui = drawn_at_96(&files, &focus_order_interaction()).expect("the page draws");
        let reached: Vec<&str> = ui
            .focus_regions
            .iter()
            .map(|region| region.id.as_str())
            .collect();
        for absent in ["sealed", "locked", "gone", "note"] {
            assert!(
                !reached.contains(&absent),
                "{absent} is in the Tab order: {reached:?}"
            );
        }
        // A field the layout gives no id cannot be named, so it is not reachable
        // either: the wizard would have nowhere to say where the keyboard is.
        assert_eq!(reached.len(), 3, "an unnamed field joined the order");
    }

    /// Tab walks one way and Shift+Tab the other, and either end wraps, so a page
    /// with a control on it always has somewhere for the keyboard to go.
    ///
    /// A keyboard that is nowhere yet starts at the end it is walking from, and
    /// a page whose focus went away -- the control it was on belongs to the page
    /// before this one -- starts the walk over the same way.
    #[test]
    fn the_walk_wraps_at_both_ends_of_the_control_order() {
        assert_eq!(next_focus_index(3, None, false), 0);
        assert_eq!(next_focus_index(3, None, true), 2);
        // Forward: through the order, then from the last one round to the first.
        assert_eq!(next_focus_index(3, Some(1), false), 2);
        assert_eq!(next_focus_index(3, Some(2), false), 0);
        // Backward: through the order, then from the first one round to the last.
        assert_eq!(next_focus_index(3, Some(1), true), 0);
        assert_eq!(next_focus_index(3, Some(0), true), 2);
        // One control is a page the keyboard stays on whichever way it walks.
        assert_eq!(next_focus_index(1, Some(0), false), 0);
        assert_eq!(next_focus_index(1, Some(0), true), 0);
    }

    /// The ring is one layer over the control's own rectangle, so the page is
    /// drawn exactly as it is without it and the ring is the only difference.
    ///
    /// It is dotted rather than solid -- the corner carries a colour and the
    /// pixel beside it does not, and the middle of the control is left to the
    /// control -- because a control may draw a border of its own, and a second
    /// solid line over the first would say nothing about where the keyboard is.
    #[test]
    fn the_focus_ring_is_drawn_over_the_control_the_keyboard_is_on() -> anyhow::Result<()> {
        // The ring is one layer over the control's own rectangle: the page is
        // drawn the same either way, and the only difference is the ring.
        let files = focus_order_project();
        let plain = drawn_at_96(&files, &focus_order_interaction())?;
        let mut interaction = focus_order_interaction();
        interaction.focused_control = Some("field".to_string());
        let focused = drawn_at_96(&files, &interaction)?;

        assert_eq!(focused.layers.len(), plain.layers.len() + 1);
        let ring = focused.layers.last().expect("the ring was drawn");
        let field = focused
            .focus_regions
            .iter()
            .find(|region| region.id == "field")
            .expect("the field is reachable");
        assert_eq!(
            (ring.left, ring.top, ring.width, ring.height),
            (
                field.left,
                field.top,
                field.right - field.left,
                field.bottom - field.top
            )
        );
        // The colour is the one the page declares, and the edge is dotted: the
        // corner carries it and the pixel beside the corner does not.
        assert_eq!(ring_corner(ring), [0x00, 0xFF, 0x00, 0xFF]);
        assert_eq!(layer_pixel(ring, 1, 0), [0x00, 0x00, 0x00, 0x00]);
        assert_eq!(layer_pixel(ring, 2, 0), [0x00, 0xFF, 0x00, 0xFF]);
        // The middle of the control is left to the control itself.
        assert_eq!(layer_pixel(ring, 10, 10), [0x00, 0x00, 0x00, 0x00]);
        Ok(())
    }

    /// A control laid over artwork can name a ring colour the page's would
    /// disappear into, and the page's own colour covers every control that says
    /// nothing.
    #[test]
    fn a_control_names_the_colour_of_its_own_focus_ring() -> anyhow::Result<()> {
        // A control laid over artwork can ask for a ring the page's colour would
        // disappear into, and the page's own colour covers the controls that say
        // nothing.
        let files = one_page_project(
            r##"<Page width="400" height="200" focus-color="#FF00FF00">
                  <Button id="one" action="next" text="One" focus-color="#FFFF0000"
                          position="absolute" left="20" top="20" width="120" height="30" />
                  <Button id="two" action="next" text="Two"
                          position="absolute" left="20" top="80" width="120" height="30" />
                </Page>"##,
            "{}",
        );
        let mut interaction = InteractionState {
            focused_control: Some("one".to_string()),
            ..Default::default()
        };
        let first = drawn_at_96(&files, &interaction)?;
        // Red is the first component the layer holds last, and blue is first.
        assert_eq!(
            ring_corner(first.layers.last().expect("the ring was drawn")),
            [0x00, 0x00, 0xFF, 0xFF]
        );
        interaction.focused_control = Some("two".to_string());
        let second = drawn_at_96(&files, &interaction)?;
        assert_eq!(
            ring_corner(second.layers.last().expect("the ring was drawn")),
            [0x00, 0xFF, 0x00, 0xFF]
        );
        Ok(())
    }

    /// A dialog takes the keyboard while it is open, so the page behind it says
    /// nothing about where a key would go: the ring is the one layer the dialog
    /// takes away, and the page under it is otherwise drawn the same.
    #[test]
    fn a_dialog_covers_the_page_without_a_focus_ring_behind_it() -> anyhow::Result<()> {
        // A dialog takes the keyboard while it is open, so the page behind it
        // says nothing about where a key would go: no ring is drawn there.
        let files = focus_order_project_with_dialog();
        let mut interaction = focus_order_interaction();
        interaction.focused_control = Some("first".to_string());
        let without = drawn_at_96(&files, &interaction)?;
        let with_dialog = drawn_at_96(&files, &interaction_with_dialog(notice()))?;
        assert!(
            without.layers.len() > with_dialog.layers.len(),
            "the page behind the dialog drew no ring to leave out"
        );
        // The ring is the layer the dialog takes away, and nothing else.
        assert_eq!(without.layers.len(), with_dialog.layers.len() + 1);
        Ok(())
    }

    /// A page that declares one colour per role: the page's own fill and the
    /// line around it, a card laid on the page with a line of its own, a label
    /// whose words stand on the page, and a button whose words stand on a
    /// control's own face.
    const CONTRAST_PAGE: &str = r##"<Page width="400" height="200" background="#FF101010"
                                       border-color="#FF202020" focus-color="#FF303030">
                  <Box id="card" position="absolute" left="20" top="20" width="200" height="60"
                       background="#FF404040" border-color="#FF505050">
                    <Label text="Words" color="#FF606060"
                           position="absolute" left="8" top="8" width="180" height="24" />
                  </Box>
                  <Button id="next" action="next" text="Next" color="#FF707070"
                          border-color="#FF808080"
                          position="absolute" left="20" top="120" width="120" height="30" />
                </Page>"##;

    /// Draws a project at 96 DPI with the colours a case states, which is what a
    /// machine whose user turned high contrast on hands the renderer.
    fn drawn_in_contrast(
        files: &HashMap<String, Vec<u8>>,
        interaction: &InteractionState,
        palette: Option<contrast::Palette>,
    ) -> anyhow::Result<RuntimeUi> {
        load_layout(
            files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            None,
            interaction,
            RuntimeMode::Installer,
            palette,
        )
    }

    /// High contrast replaces the colours a page declares with the ones the
    /// scheme keeps for what they paint.
    ///
    /// A page this runtime draws is a picture, so nothing in it follows the
    /// user's scheme on its own: every surface, line, and word has to be painted
    /// with the colour the machine names for it -- and by role, because a theme
    /// may paint a page and a control face alike and still draw the words on a
    /// control in a colour of their own.
    #[test]
    fn the_scheme_replaces_the_colours_a_page_declares() -> anyhow::Result<()> {
        let files = one_page_project(CONTRAST_PAGE, "{}");
        let ui = drawn_in_contrast(&files, &InteractionState::default(), Some(scheme()))?;

        // The page: its fill, and the line grown inwards from its edge.
        assert_eq!(
            painted_over(&ui.layers, (0, 0, 400, 200), (0, 0)),
            [pixel(SCHEME_SURFACE), pixel(SCHEME_LINE)]
        );
        // The card laid on the page, with the line around its own edge.
        assert_eq!(
            painted_over(&ui.layers, (20, 20, 200, 60), (0, 0)),
            [pixel(SCHEME_CONTROL), pixel(SCHEME_LINE)]
        );
        // The words of the label stand on the page and those of the button on
        // the control's face, which a scheme is free to colour apart.
        assert_eq!(ui.texts.len(), 2, "the page drew the words it declares");
        assert_eq!(
            ui.texts[0].runs[0].color,
            scheme_colour(contrast::Role::Text)
        );
        assert_eq!(
            ui.texts[1].runs[0].color,
            scheme_colour(contrast::Role::ControlText)
        );
        Ok(())
    }

    /// A machine that is not in high contrast paints the page it was given.
    ///
    /// The setting belongs to the user, so the colours a project declared have
    /// to come back the moment it is off: nothing about a page may depend on
    /// what the machine happened to be set to.
    #[test]
    fn a_page_keeps_the_colours_it_declares_while_no_scheme_is_asked_for() -> anyhow::Result<()> {
        let files = one_page_project(CONTRAST_PAGE, "{}");
        let ui = drawn_in_contrast(&files, &InteractionState::default(), None)?;
        assert_eq!(
            painted_over(&ui.layers, (0, 0, 400, 200), (0, 0)),
            [pixel(0x0010_1010), pixel(0x0020_2020)]
        );
        assert_eq!(
            painted_over(&ui.layers, (20, 20, 200, 60), (0, 0)),
            [pixel(0x0040_4040), pixel(0x0050_5050)]
        );
        assert_eq!(ui.texts[0].runs[0].color, parse_color("#FF606060"));
        assert_eq!(ui.texts[1].runs[0].color, parse_color("#FF707070"));
        Ok(())
    }

    /// The ring around the control the keyboard is on takes the colour the
    /// scheme keeps for where the user is.
    ///
    /// A project's own ring colour is what it chose against its own artwork, and
    /// under a scheme the page behind the ring is the scheme's own colour, so
    /// the ring has to be the one the user picked to be seen.
    #[test]
    fn the_ring_follows_the_scheme_under_high_contrast() -> anyhow::Result<()> {
        let files = one_page_project(CONTRAST_PAGE, "{}");
        let interaction = InteractionState {
            focused_control: Some("next".to_string()),
            ..Default::default()
        };
        let ui = drawn_in_contrast(&files, &interaction, Some(scheme()))?;
        let ring = ui.layers.last().expect("the ring was drawn");
        assert_eq!((ring.left, ring.top), (20, 120));
        assert_eq!(ring_corner(ring), pixel(SCHEME_HIGHLIGHT));

        // The page without a scheme keeps the ring colour it declared, which is
        // the one the page and the control would each have asked for.
        let plain = drawn_in_contrast(&files, &interaction, None)?;
        assert_eq!(
            ring_corner(plain.layers.last().expect("the ring was drawn")),
            pixel(0x0030_3030)
        );
        Ok(())
    }

    /// An open menu under a scheme says where the keyboard is and what the user
    /// chose, tells the two apart, and keeps a line of its own.
    ///
    /// The popup is drawn over the page rather than inside the control, and a
    /// scheme paints both in its window colour: without a line around the menu
    /// the rows would lie on the page with nothing to say where the list starts.
    #[test]
    fn an_open_menu_marks_the_keyboard_and_the_choice_under_a_scheme() -> anyhow::Result<()> {
        let files = one_page_project(
            r##"<Page width="400" height="200" background="#FF101010">
                  <Select id="edition" position="absolute" left="20" top="20"
                          width="180" height="26" background="#FF303030" color="#FF606060"
                          popup-background="#FF42515E" popup-selected-background="#FF495A68"
                          popup-highlight-background="#FF7050B0"
                          popup-row-height="26" popup-padding="2">
                    <Option value="one" text="One" />
                    <Option value="two" text="Two" />
                  </Select>
                </Page>"##,
            "{}",
        );
        let mut interaction = InteractionState::default();
        interaction
            .choices
            .insert("edition".to_string(), "two".to_string());
        interaction.highlighted_option = Some(0);
        let ui = load_layout(
            &files,
            DpiContext {
                scale: 1.0,
                use_2x: false,
            },
            "zh-CN",
            Some("edition"),
            &interaction,
            RuntimeMode::Installer,
            Some(scheme()),
        )?;

        // The control's own face is a control face, and the menu it drops is a
        // list of its own: the fill of the popup, and the line drawn around it
        // where that line runs along the top edge.
        assert_eq!(
            painted_over(&ui.layers, (20, 20, 180, 26), (0, 0)),
            [pixel(SCHEME_CONTROL)]
        );
        assert_eq!(
            painted_over(&ui.overlay_layers, (20, 50, 180, 56), (90, 0)),
            [pixel(SCHEME_SURFACE), pixel(SCHEME_LINE)]
        );
        // The row the keyboard is on is where the user is, and the row in use is
        // a state of the control, so the two are still told apart.
        assert_eq!(
            painted_over(&ui.overlay_layers, (22, 52, 176, 26), (88, 0)),
            [pixel(SCHEME_HIGHLIGHT)]
        );
        assert_eq!(
            painted_over(&ui.overlay_layers, (22, 78, 176, 26), (88, 0)),
            [pixel(SCHEME_CONTROL)]
        );
        // Words on the highlight take the colour the scheme keeps for that, or
        // they would be drawn in the background's own colour and vanish.
        assert_eq!(ui.overlay_texts.len(), 2);
        assert_eq!(
            ui.overlay_texts[0].runs[0].color,
            scheme_colour(contrast::Role::HighlightText)
        );
        assert_eq!(
            ui.overlay_texts[1].runs[0].color,
            scheme_colour(contrast::Role::Text)
        );
        Ok(())
    }

    /// A scrollbar takes the two colours the scheme keeps for it: the track has
    /// a colour of its own in Windows, and the thumb is a control.
    #[test]
    fn a_scrollbar_takes_the_colours_the_scheme_keeps_for_it() -> anyhow::Result<()> {
        let ui = drawn_in_contrast(
            &scrollable_project(6),
            &InteractionState::default(),
            Some(scheme()),
        )?;
        assert_eq!(
            painted_over(&ui.overlay_layers, (202, 10, 8, 100), (4, 50)),
            [pixel(SCHEME_SCROLLBAR)]
        );
        assert_eq!(
            painted_over(&ui.overlay_layers, (202, 10, 8, 56), (4, 28)),
            [pixel(SCHEME_CONTROL)]
        );
        Ok(())
    }
}
