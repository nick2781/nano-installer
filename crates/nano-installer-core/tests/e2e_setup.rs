//! End-to-end tests that build a real setup and run it.
//!
//! These drive the actual builder and the actual runtime binaries, because the
//! unit tests cannot see the seams between them: a change that packages the
//! wrong layout, drops the payload, or forgets to inject the manifest passes
//! every unit test and still produces a setup that cannot install.
//!
//! Every fixture is generated here, so the suite carries no product payload and
//! no third-party assets. Nothing installs to a shared location either: each
//! case picks a fresh directory below the temporary directory, and each builds
//! a project whose uninstall registration names only that case, so two tests
//! running at once cannot see each other's work.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::Context;
use nano_installer_core::{
    build_project, build_project_with_progress, BuildRequest, BuildResult, PayloadFormat,
};
use windows::core::{Interface, BSTR, GUID, PCWSTR, VARIANT};
use windows::Win32::Foundation::{BOOL, HANDLE, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    GetDIBits, GetSysColor, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    COLOR_BTNFACE, COLOR_HIGHLIGHT, COLOR_WINDOW, COLOR_WINDOWFRAME, COLOR_WINDOWTEXT,
    DIB_RGB_COLORS, HDC, HGDIOBJ, SYS_COLOR_INDEX,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, IDispatch, COINIT_APARTMENTTHREADED, DISPATCH_PROPERTYGET,
    DISPPARAMS,
};
use windows::Win32::UI::Accessibility::{
    AccessibleObjectFromWindow, IAccessible, SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK,
    ROLE_SYSTEM_CHECKBUTTON, ROLE_SYSTEM_COMBOBOX, ROLE_SYSTEM_DIALOG, ROLE_SYSTEM_PROGRESSBAR,
    ROLE_SYSTEM_PUSHBUTTON, ROLE_SYSTEM_RADIOBUTTON, ROLE_SYSTEM_STATICTEXT, ROLE_SYSTEM_TEXT,
    ROLE_SYSTEM_WINDOW, SELFLAG_TAKEFOCUS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_BACK, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Programs, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GetClassNameW, GetClientRect, GetCursorInfo, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId, IsWindow,
    IsWindowVisible, LoadCursorW, PeekMessageW, PostMessageW, SetCursorPos, SetForegroundWindow,
    SetProcessDPIAware, SetWindowPos, TranslateMessage, CURSORINFO, EVENT_OBJECT_FOCUS,
    EVENT_OBJECT_LIVEREGIONCHANGED, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_VALUECHANGE, HCURSOR,
    HTCLIENT, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW, IDC_HAND, IDC_IBEAM, MSG, OBJID_CLIENT,
    PM_REMOVE, SM_CYSCREEN, SPI_SETHIGHCONTRAST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    WINEVENT_OUTOFCONTEXT, WM_CHAR, WM_CLOSE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_SETCURSOR, WM_SETTINGCHANGE,
};

/// Accessibility states, spelled here the way `oleacc.h` declares them: a state
/// is a field of bits, and a case looks at one at a time.
const STATE_SYSTEM_CHECKED: u32 = 0x0000_0010;
const STATE_SYSTEM_FOCUSED: u32 = 0x0000_0004;
const STATE_SYSTEM_HASPOPUP: u32 = 0x4000_0000;

/// The key Windows starts a program from at sign-in.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

/// The colours the pointer-state case paints its button with.
///
/// One flat colour per state is what lets the case ask the window which picture
/// it drew: the three differ from each other and from the page behind them, so a
/// state that never reached the screen reads as the state before it.
const RESTING_BUTTON: (u8, u8, u8) = (0x10, 0x20, 0x30);
const HOVERED_BUTTON: (u8, u8, u8) = (0x20, 0x50, 0x70);
const PRESSED_BUTTON: (u8, u8, u8) = (0x30, 0x80, 0xB0);

/// The three colours the language case's menu draws with, as its layout
/// declares them: the popup behind the rows, the row the language in use stands
/// on, and the row the keyboard is on.
const POPUP_BACKGROUND: (u8, u8, u8) = (0x42, 0x51, 0x5E);
const CHOSEN_ROW: (u8, u8, u8) = (0x49, 0x5A, 0x68);
const HIGHLIGHTED_ROW: (u8, u8, u8) = (0x70, 0x50, 0xB0);

/// The colours the keyboard case reads back: the picture the checkbox draws in
/// each of its two states, and the ring the page declares for the control the
/// keyboard is on.
///
/// They differ from each other and from the page behind them, so a frame says
/// which state the window is in and whether the ring is there.
const UNCHECKED_BOX: (u8, u8, u8) = (0x18, 0x28, 0x38);
const CHECKED_BOX: (u8, u8, u8) = (0x58, 0x98, 0xD8);
const FOCUS_RING: (u8, u8, u8) = (0x00, 0xFF, 0x00);
/// The page the case's button walks to.
const SECOND_PAGE: (u8, u8, u8) = (0x30, 0x30, 0x30);

/// The product exe the second release of an update case ships.
///
/// The two releases differ in their product exe, which is what an update
/// package has to carry and what an install over the wrong release has to
/// leave alone.
const SECOND_RELEASE_EXE: &[u8] = b"MZ end-to-end probe executable, second release\r\n";

/// Resolves a shell folder the way the runtime resolves it.
///
/// A shortcut the install wrote is looked for through the same call, so the
/// case fails when the install put it somewhere Windows does not show rather
/// than when it guessed a directory name.
fn known_folder(folder: &windows::core::GUID) -> anyhow::Result<PathBuf> {
    let raw: windows::core::PWSTR =
        unsafe { SHGetKnownFolderPath(folder, KF_FLAG_DEFAULT, HANDLE::default()) }?;
    let text = unsafe { raw.to_string() }?;
    unsafe { CoTaskMemFree(Some(raw.as_ptr().cast())) };
    Ok(PathBuf::from(text))
}

fn desktop_directory() -> anyhow::Result<PathBuf> {
    known_folder(&FOLDERID_Desktop)
}

fn programs_directory() -> anyhow::Result<PathBuf> {
    known_folder(&FOLDERID_Programs)
}

/// A name unique to one case in one process, so parallel cases and repeated
/// runs do not share a registry key or an installation directory.
fn unique_case_id() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// The workspace root, resolved from this crate's manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate directory has a workspace root")
        .to_path_buf()
}

/// A directory holding the three runtime executables the builder embeds.
///
/// The real stubs are preferred, because only they can extract a payload. A
/// case that stops at the build product does not need that, so it falls back to
/// any executable, which is enough for the builder to embed and for the bundle
/// to be read back.
fn stub_directory(require_real_stubs: bool) -> Option<PathBuf> {
    let debug = workspace_root().join("target/debug");
    let names = [
        "lzma-stub-native.exe",
        "zlib-stub-native.exe",
        "uninst-stub-native.exe",
    ];
    if names.iter().all(|name| debug.join(name).is_file()) {
        return Some(debug);
    }
    if require_real_stubs {
        return None;
    }
    let deps = workspace_root().join("target/debug/deps");
    let fallback = std::fs::read_dir(&deps)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("nano_installer_core-") && name.ends_with(".exe")
                })
        })?;
    let directory = workspace_root().join("target/e2e-stubs");
    std::fs::create_dir_all(&directory).ok()?;
    for name in names {
        std::fs::copy(&fallback, directory.join(name)).ok()?;
    }
    Some(directory)
}

/// Reports that a case cannot run because the runtime executables are absent.
///
/// A local run skips, because building the stubs is a separate step. A job that
/// builds them on purpose sets `NANO_INSTALLER_E2E_REQUIRE_STUBS`, where a skip
/// would quietly turn the whole suite into a no-op.
fn skip_missing_stubs() -> anyhow::Result<()> {
    let required =
        std::env::var_os("NANO_INSTALLER_E2E_REQUIRE_STUBS").is_some_and(|value| value != "0");
    anyhow::ensure!(
        !required,
        "the runtime stubs are missing; build them first with `cargo build -p nano-installer-stub-lzma \
         -p nano-installer-stub-zlib -p nano-installer-uninstaller`"
    );
    eprintln!("skipping: real runtime stubs are not built");
    Ok(())
}

/// One case's project: a fresh directory, a unique registry key, and a
/// generated payload.
struct Fixture {
    /// Held only so the case's directory is removed when the case ends.
    _temp: tempfile::TempDir,
    id: String,
    project: PathBuf,
    setup: PathBuf,
    destination: PathBuf,
    stubs: PathBuf,
    /// A key belonging to this case alone, for a project script to write into or
    /// for a case to watch one value of rather than the whole uninstall entry.
    test_key: String,
    /// Values a case wrote into a key the machine shares, so the drop guard can
    /// take them back even when an assertion failed before uninstall ran.
    extra_registry_values: Vec<(String, String)>,
}

impl Fixture {
    /// Prepares a project that needs no product assets.
    ///
    /// `payload` decides the archive format, which is what routes a setup to
    /// the matching runtime: a ZIP payload embeds the Deflate runtime, a 7z
    /// payload the LZMA one.
    fn new(payload: PayloadFormat, silent: bool, include_exe: bool) -> Option<Self> {
        Self::with_stubs(stub_directory(true)?, payload, silent, include_exe)
    }

    /// Prepares a project that builds against a caller-supplied stub set.
    ///
    /// A case that stops at the build product does not need runtimes that can
    /// extract anything, so it may hand in a stand-in.
    fn with_stubs(
        stubs: PathBuf,
        payload: PayloadFormat,
        silent: bool,
        include_exe: bool,
    ) -> Option<Self> {
        let temp = tempfile::tempdir().expect("a temporary directory");
        let id = unique_case_id();
        let project = temp.path().join("project");
        let setup = temp.path().join("E2eProbe_Setup.exe");
        let destination = temp.path().join("installed");
        let fixture = Self {
            test_key: format!("HKCU\\Software\\nano-installer-e2e-{id}"),
            _temp: temp,
            id,
            project,
            setup,
            destination,
            stubs,
            extra_registry_values: Vec::new(),
        };
        fixture
            .write_project(payload, silent, include_exe)
            .expect("project files");
        Some(fixture)
    }

    fn write_project(
        &self,
        payload: PayloadFormat,
        silent: bool,
        include_exe: bool,
    ) -> anyhow::Result<()> {
        for directory in ["layouts", "assets", "locales", "payload", "scripts"] {
            std::fs::create_dir_all(self.project.join(directory))?;
        }
        let name = "E2eProbe";
        let config = serde_json::json!({
            "project": {
                "name": name,
                "version": "1.0.0",
                "file_version": "1.0.0.0",
                "publisher": "nano-installer e2e",
                "output_name": name
            },
            "output": {
                "installer_name": "E2eProbe_Setup.exe",
                "uninstaller_name": "uninst.exe"
            },
            "install": {
                "exe_name": "E2eProbe.exe",
                "require_admin": false,
                "kill_process_on_install": false,
                "detect_running_process": false,
                "kill_process_on_uninstall": false
            },
            "registry": {
                "uninstall_key": format!(
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\nano-installer-e2e-{}",
                    self.id
                )
            },
            "shortcuts": {"desktop_shortcut": false, "start_menu": false},
            "autostart": {"enabled": false, "default": false},
            "localization": {
                "default_locale": "en-US",
                "supported_locales": ["en-US"]
            },
            "resources": {
                "layouts_dir": "layouts",
                "assets_dir": "assets",
                "locales_dir": "locales",
                "payload_file": "payload/app.archive"
            },
            "ui": {
                "dialog_layout": "layouts/msgBox.xml"
            },
            "wizard": {
                "pages": [{"id": "config", "layout": "layouts/configpage.xml", "title": "Options"}]
            },
            "uninstall": {"data_paths": []},
            // A key of the case's own, so a project script has somewhere to
            // write that is not a key a real product uses.
            "test": {"registry_key": self.test_key},
            "advanced": {
                "silent_mode_support": silent,
                "uninstall_mode_support": silent
            }
        });
        std::fs::write(
            self.project.join("installer_config.json"),
            serde_json::to_vec_pretty(&config)?,
        )?;
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r#"<Page width="720" height="450"><Label text="End to end" /></Page>"#,
        )?;
        std::fs::write(
            self.project.join("layouts/msgBox.xml"),
            r#"<Page width="480" height="180" />"#,
        )?;
        std::fs::write(self.project.join("locales/en-US.json"), b"{}")?;
        self.archive_payload(payload, include_exe)?;
        Ok(())
    }

    /// Declares a wizard of two pages that walk into each other.
    ///
    /// The first page carries a `next` button and the second a `back` one, and
    /// the two declare different client areas, so the window's own size says
    /// which page is up after a click.
    fn walk_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <Button id="next" action="next" text="Next" position="absolute" left="560" top="390" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/secondpage.xml"),
            r##"<Page width="500" height="300" background="#FF202020">
  <Button id="back" action="back" text="Back" position="absolute" left="20" top="240" width="120" height="36" />
</Page>"##,
        )?;
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "First", "layout": "layouts/configpage.xml"},
                {"id": "second", "title": "Second", "layout": "layouts/secondpage.xml"}
            ]);
        })
    }

    /// Declares a wizard of four pages a project's page hook walks, with a
    /// licence page between the welcome page and the options page.
    ///
    /// Each page declares a different client area, so the size of the window
    /// says which page is up -- and whether the hook sent the wizard past the
    /// licence page rather than through it.
    fn hook_project(&self, hook: &str) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <Button id="next" action="next" text="Next" position="absolute" left="560" top="390" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/licencepage.xml"),
            r##"<Page width="640" height="420" background="#FF202020">
  <Button id="next" action="next" text="Next" position="absolute" left="500" top="370" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/optionspage.xml"),
            r##"<Page width="600" height="400" background="#FF303030">
  <Button id="back" action="back" text="Back" position="absolute" left="20" top="20" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/taskspage.xml"),
            r##"<Page width="500" height="300" background="#FF404040" />"##,
        )?;
        self.write_script("pages.rhai", hook)?;
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "welcome", "title": "Welcome", "layout": "layouts/configpage.xml"},
                {"id": "licence", "title": "Licence", "layout": "layouts/licencepage.xml"},
                {"id": "options", "title": "Options", "layout": "layouts/optionspage.xml"},
                {"id": "tasks", "title": "Installing", "layout": "layouts/taskspage.xml",
                 "role": "progress"}
            ]);
        })
    }

    /// Declares a wizard whose install button waits for the page's own field.
    ///
    /// The agreement box and the install directory both have to be in order
    /// before the button answers a click, and the directory is typed into the
    /// field rather than configured: an empty field is what `required` speaks
    /// about, and the value the field ends up holding is the one the install is
    /// given.
    fn validated_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <TextInput id="editDir" required="true" required-message="@dir_needed" min-length="5" min-length-message="@dir_short"
             position="absolute" left="20" top="40" width="400" height="26" />
  <Checkbox id="chkAgree" text="@agree" position="absolute" left="20" top="90" width="200" height="20" />
  <Button id="install" action="install" text="@install_button"
          enabled-when="chkAgree:checked, editDir:valid"
          position="absolute" left="20" top="130" width="140" height="36" />
  <Label id="hint" value-source="field-error:editDir"
         position="absolute" left="20" top="180" width="600" height="20" color="#FFFFFFFF" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/taskspage.xml"),
            r##"<Page width="500" height="300" background="#FF202020" />"##,
        )?;
        std::fs::write(
            self.project.join("locales/en-US.json"),
            br#"{"dir_needed": "Choose a folder", "dir_short": "At least five characters", "agree": "I agree", "install_button": "Install"}"#,
        )?;
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "tasks", "title": "Installing", "layout": "layouts/taskspage.xml", "role": "progress"}
            ]);
        })
    }

    /// Declares a wizard whose install button waits for a radio group.
    ///
    /// The layout marks the first row as the default, so the button starts
    /// inert even though the group always holds a value: what it waits for is
    /// the row a click picks. The page the task reports on declares a
    /// different client area, so the window's own size says whether the
    /// install started.
    fn choice_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <RadioButton id="quick" group="mode" value="quick" text="Quick" checked="true"
               position="absolute" left="20" top="40" width="200" height="20" />
  <RadioButton id="custom" group="mode" value="custom" text="Custom"
               position="absolute" left="20" top="70" width="200" height="20" />
  <Button id="install" action="install" text="Install" enabled-when="mode:custom"
          position="absolute" left="20" top="130" width="140" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/taskspage.xml"),
            r##"<Page width="500" height="300" background="#FF202020" />"##,
        )?;
        self.edit_config(|config| {
            // The page asks for no directory of its own, so the install runs
            // against the one the project configures.
            config["install"]["default_path"] =
                serde_json::json!(self.destination.to_string_lossy());
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "tasks", "title": "Installing", "layout": "layouts/taskspage.xml", "role": "progress"}
            ]);
        })
    }

    /// Declares a wizard that asks the user for a value before it installs.
    ///
    /// The first page carries a field the user types into and a choice group,
    /// and the page the task reports on declares a different client area, so the
    /// window's own size says whether the install started.
    fn values_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <TextInput id="serial" position="absolute" left="20" top="40" width="300" height="26" />
  <RadioButton id="portable" group="edition" value="portable" text="Portable" checked="true"
               position="absolute" left="20" top="90" width="200" height="20" />
  <RadioButton id="installed" group="edition" value="installed" text="Installed"
               position="absolute" left="20" top="120" width="200" height="20" />
  <Button id="install" action="install" text="Install"
          position="absolute" left="20" top="170" width="140" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/taskspage.xml"),
            r##"<Page width="500" height="300" background="#FF202020" />"##,
        )?;
        self.edit_config(|config| {
            // The page asks for no directory of its own, so the install runs
            // against the one the project configures.
            config["install"]["default_path"] =
                serde_json::json!(self.destination.to_string_lossy());
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "tasks", "title": "Installing", "layout": "layouts/taskspage.xml", "role": "progress"}
            ]);
        })
    }

    /// Declares a wizard whose page carries a box for two of four components.
    ///
    /// The boxes are what a component is chosen by: the page places one for
    /// `docs`, which it starts clear, and one for `samples`, which it starts
    /// ticked. `core` is required and `tools` installs by default, and the page
    /// carries a box for neither, so those two are the project's answer alone.
    /// The page the task reports on declares a different client area, so the
    /// window's own size says whether the install started.
    fn components_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <Checkbox id="docs" text="Documentation"
            position="absolute" left="20" top="40" width="200" height="20" />
  <Checkbox id="samples" text="Samples" checked="true"
            position="absolute" left="20" top="70" width="200" height="20" />
  <Button id="install" action="install" text="Install"
          position="absolute" left="20" top="130" width="140" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/taskspage.xml"),
            r##"<Page width="500" height="300" background="#FF202020" />"##,
        )?;
        self.edit_config(|config| {
            // The page asks for no directory of its own, so the install runs
            // against the one the project configures.
            config["install"]["default_path"] =
                serde_json::json!(self.destination.to_string_lossy());
            config["components"] = serde_json::json!({ "items": [
                {"id": "core", "payload": "payload/core.archive", "required": true},
                {"id": "docs", "payload": "payload/docs.archive"},
                {"id": "tools", "payload": "payload/tools.archive", "default": true},
                {"id": "samples", "payload": "payload/samples.archive"}
            ] });
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "tasks", "title": "Installing", "layout": "layouts/taskspage.xml", "role": "progress"}
            ]);
        })
    }

    /// Declares a wizard whose task can be stopped from the page it runs on.
    ///
    /// The first page starts the install, the page the task reports on carries
    /// a cancel button, and the two declare different client areas, so the
    /// window's own size says which page is up. The install script waits for
    /// the request, so the click lands while the task is running rather than
    /// after it.
    fn cancellable_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <Button id="install" action="install" text="Install" position="absolute" left="560" top="390" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/taskspage.xml"),
            r##"<Page width="500" height="300" background="#FF202020">
  <Button id="cancel" action="cancel" text="Cancel" position="absolute" left="20" top="240" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/donepage.xml"),
            r##"<Page width="400" height="200" background="#FF303030" />"##,
        )?;
        self.write_script(
            "install.rhai",
            r#"
            let install_path = get_install_path();
            copy_uninstaller();
            write_file(path_join(install_path, "E2eProbe.exe"), "app");
            // A step that takes as long as the user takes to make up their
            // mind, which is where a real install spends its time.
            let waited = 0;
            while !is_cancelled() && waited < 30000 {
                sleep_ms(20);
                waited += 20;
            }
            "#,
        )?;
        self.edit_config(|config| {
            config["install"]["default_path"] =
                serde_json::Value::from(self.destination.display().to_string());
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "tasks", "layout": "layouts/taskspage.xml", "role": "progress"},
                {"id": "done", "layout": "layouts/donepage.xml", "role": "finish"}
            ]);
        })
    }

    /// Declares a wizard whose progress page says what the task is doing: the
    /// words it publishes and a bar for how far it has come.
    ///
    /// The script sets both by hand, one step at a time with a pause between
    /// them, which is what lets a case read each step back off the page: what a
    /// real install reports arrives whenever the work reaches the next checkpoint
    /// and would be gone before a case could look.
    fn progress_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <Button id="install" action="install" text="Install" position="absolute" left="560" top="390" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/taskspage.xml"),
            r##"<Page width="500" height="300" background="#FF202020">
  <Label id="status" text="Working" value-source="status"
         position="absolute" left="20" top="20" width="460" height="24" />
  <ProgressBar id="bar" position="absolute" left="20" top="60" width="460" height="18"
               progress="0" background="#FF303030" />
  <Button id="cancel" action="cancel" text="Cancel" position="absolute" left="20" top="240" width="120" height="36" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/donepage.xml"),
            r##"<Page width="400" height="200" background="#FF303030" />"##,
        )?;
        self.write_script(
            "install.rhai",
            r#"
            let install_path = get_install_path();
            copy_uninstaller();
            write_file(path_join(install_path, "E2eProbe.exe"), "app");
            // Two steps, each held long enough for a case to read the page back.
            set_status("Copying files");
            set_progress(30.0);
            sleep_ms(1500);
            set_status("Finishing up");
            set_progress(70.0);
            sleep_ms(1500);
            "#,
        )?;
        self.edit_config(|config| {
            config["install"]["default_path"] =
                serde_json::Value::from(self.destination.display().to_string());
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "tasks", "layout": "layouts/taskspage.xml", "role": "progress"},
                {"id": "done", "layout": "layouts/donepage.xml", "role": "finish"}
            ]);
        })
    }

    /// Declares a wizard whose first page holds a list longer than the room it
    /// has, with the page's own button below the part the list cuts off.
    ///
    /// The rows are fifty pixels tall inside a hundred-and-twenty pixel view, so
    /// the last one starts past the bottom edge and would sit over the button
    /// beneath it. The rows above that one walk to the second page and the last
    /// one closes the wizard, which is what makes a click say which row took it.
    fn scrolling_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="720" height="450" background="#FF101010">
  <Button id="below" action="next" text="Next"
          position="absolute" left="20" top="180" width="120" height="30" />
  <VBox id="list" scrollable="true" position="absolute" left="20" top="40"
        width="400" height="120">
    <Button id="row0" action="next" text="Row 0" width="400" height="50" />
    <Button id="row1" action="next" text="Row 1" width="400" height="50" />
    <Button id="row2" action="next" text="Row 2" width="400" height="50" />
    <Button id="row3" action="close" text="Row 3" width="400" height="50" />
  </VBox>
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/secondpage.xml"),
            r##"<Page width="500" height="300" background="#FF202020">
  <Button id="back" action="back" text="Back"
          position="absolute" left="20" top="240" width="120" height="36" />
</Page>"##,
        )?;
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "First", "layout": "layouts/configpage.xml"},
                {"id": "second", "title": "Second", "layout": "layouts/secondpage.xml"}
            ]);
        })
    }

    /// Declares a page whose button draws a different picture in each of the
    /// three states a pointer can put it in.
    ///
    /// The pictures are one flat colour each, written by the case so that the
    /// window can be asked what it drew: a state that never reached the screen
    /// leaves the colour of the state before it in place, and a picture that
    /// never loaded leaves the page's own background.
    fn hover_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="400" height="200" background="#FF000000">
  <Button id="probe" position="absolute" left="40" top="60" width="200" height="60"
          normal-image="assets/normal.png" hover-image="assets/hover.png"
          pressed-image="assets/pressed.png" />
</Page>"##,
        )?;
        for (name, colour) in [
            ("normal.png", RESTING_BUTTON),
            ("hover.png", HOVERED_BUTTON),
            ("pressed.png", PRESSED_BUTTON),
        ] {
            std::fs::write(
                self.project.join("assets").join(name),
                solid_png(4, 4, colour),
            )?;
        }
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"}
            ]);
        })
    }

    /// Declares a page with a button, a field, and a patch of page that is
    /// neither.
    ///
    /// The three are what the runtime promises a cursor for: a hand over
    /// something that answers a click, a beam over something the user can type
    /// into, and the ordinary pointer everywhere else.
    fn cursor_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="400" height="200" background="#FF101010">
  <Button id="next" action="next" text="Next"
          position="absolute" left="20" top="20" width="120" height="30" />
  <TextInput id="edit" position="absolute" left="20" top="80" width="200" height="24" />
</Page>"##,
        )?;
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"}
            ]);
        })
    }

    /// Declares a page whose three controls the keyboard can walk, and the page
    /// the last of them walks to.
    ///
    /// The checkbox draws a picture per state, so a case can read its state back
    /// out of the window, and the ring is a colour the page names, so it cannot
    /// be mistaken for anything a control draws. The second page declares no
    /// control at all, which is what makes Tab there a question of its own.
    fn keyboard_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="400" height="200" background="#FF000000" focus-color="#FF00FF00">
  <Checkbox id="terms" position="absolute" left="40" top="20" width="24" height="24"
            unchecked-image="assets/off.png" checked-image="assets/on.png" />
  <TextInput id="notes" position="absolute" left="40" top="60" width="200" height="24" />
  <Button id="next" action="next" text="Next"
          position="absolute" left="40" top="120" width="120" height="30" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/secondpage.xml"),
            r##"<Page width="300" height="150" background="#FF303030" />"##,
        )?;
        for (name, colour) in [("off.png", UNCHECKED_BOX), ("on.png", CHECKED_BOX)] {
            std::fs::write(
                self.project.join("assets").join(name),
                solid_png(4, 4, colour),
            )?;
        }
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "second", "title": "Second", "layout": "layouts/secondpage.xml"}
            ]);
        })
    }

    /// Declares a page whose controls are every kind a client asks about.
    ///
    /// A field with the words that name it written above, a checkbox and two
    /// radios of one group that carry their own words, a button, and the page
    /// the button walks to. What each control is called is what a screen reader
    /// reads out, so every one of them names words a case can look for, and the
    /// field's name is not written on the field itself: a layout labels a field
    /// beside it, and finding that is part of what is being tested.
    fn accessibility_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="480" height="260" background="#FF101010">
  <Label text="Your name" position="absolute" left="20" top="16" width="220" height="20" />
  <TextInput id="name" position="absolute" left="20" top="40" width="220" height="24" />
  <Checkbox id="terms" text="I accept the terms"
            position="absolute" left="20" top="76" width="260" height="24" />
  <RadioButton id="quick" group="mode" value="quick" text="Quick install"
               position="absolute" left="20" top="108" width="220" height="24" />
  <RadioButton id="custom" group="mode" value="custom" text="Custom install"
               position="absolute" left="20" top="136" width="220" height="24" />
  <Select id="channel" position="absolute" left="20" top="168" width="180" height="26"
          background="#FF303030" color="#FFFFFFFF" popup-row-height="26" popup-padding="2"
          popup-background="#FF42515E" popup-selected-background="#FF495A68"
          popup-highlight-background="#FF7050B0">
    <Option value="stable" text="Stable" />
    <Option value="beta" text="Beta" />
  </Select>
  <Button id="next" action="next" text="Next"
          position="absolute" left="20" top="200" width="120" height="30" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("layouts/secondpage.xml"),
            r##"<Page width="320" height="160" background="#FF202020">
  <Label text="Second page" position="absolute" left="20" top="20" width="200" height="20" />
</Page>"##,
        )?;
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"},
                {"id": "second", "title": "Second", "layout": "layouts/secondpage.xml"}
            ]);
        })
    }

    /// Declares a page whose colours are the machine's own, swapped.
    ///
    /// The page is declared in the colour of the machine's text and the card on
    /// it in the machine's highlight, while the ring is declared in the colour
    /// of the machine's page. A runtime that paints the layout as it was written
    /// shows that swap; one that hands the page over to the scheme the user
    /// picked shows the colours the scheme keeps instead, and a scheme never
    /// names one colour for a page and for the text on it, so the two frames can
    /// never be confused.
    ///
    /// The checkbox carries a picture of its own, which is there to say that
    /// artwork is not a colour the scheme has a name for.
    fn contrast_project(&self) -> anyhow::Result<()> {
        let page = layout_colour(system_colour(COLOR_WINDOW));
        let text = layout_colour(system_colour(COLOR_WINDOWTEXT));
        let highlight = layout_colour(system_colour(COLOR_HIGHLIGHT));
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            format!(
                r##"<Page width="400" height="200" background="{text}" focus-color="{page}">
  <Checkbox id="terms" position="absolute" left="40" top="20" width="24" height="24"
            unchecked-image="assets/off.png" checked-image="assets/on.png" />
  <Box id="card" position="absolute" left="40" top="60" width="200" height="40"
       background="{highlight}" border-color="{highlight}" border-width="1" />
</Page>"##
            ),
        )?;
        for (name, colour) in [("off.png", UNCHECKED_BOX), ("on.png", CHECKED_BOX)] {
            std::fs::write(
                self.project.join("assets").join(name),
                solid_png(4, 4, colour),
            )?;
        }
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"}
            ]);
        })
    }

    /// Declares a page whose language control offers two languages and whose
    /// words change with them.
    ///
    /// The label is the point of the case: a language chosen with the keyboard
    /// has to reach the page, and a sentence that is longer in one language
    /// than in the other is what says that it did.
    fn language_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="400" height="200" background="#FF000000">
  <Select id="lang" action="switch_language" position="absolute" left="100" top="20"
          width="180" height="26" background="#FF303030" color="#FFFFFFFF"
          popup-row-height="26" popup-padding="2" popup-background="#FF42515E"
          popup-selected-background="#FF495A68" popup-highlight-background="#FF7050B0">
    <Option value="en-US" text="English" />
    <Option value="de-DE" text="Deutsch" />
  </Select>
  <Label text="@greeting" color="#FFFFFFFF"
         position="absolute" left="20" top="140" width="360" height="24" />
</Page>"##,
        )?;
        std::fs::write(
            self.project.join("locales/en-US.json"),
            br#"{"greeting": "Welcome to the setup"}"#,
        )?;
        std::fs::write(
            self.project.join("locales/de-DE.json"),
            br#"{"greeting": "Willkommen bei der Installation"}"#,
        )?;
        self.edit_config(|config| {
            config["localization"]["supported_locales"] = serde_json::json!(["en-US", "de-DE"]);
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"}
            ]);
        })
    }

    /// Declares a page with a field and the button that fills it in.
    fn browse_project(&self) -> anyhow::Result<()> {
        std::fs::write(
            self.project.join("layouts/configpage.xml"),
            r##"<Page width="400" height="200" background="#FF000000">
  <TextInput id="editDir" position="absolute" left="20" top="20" width="260" height="24" />
  <Button id="browse" action="pick_directory" text="Browse"
          position="absolute" left="300" top="20" width="80" height="24" />
</Page>"##,
        )?;
        self.edit_config(|config| {
            config["wizard"]["pages"] = serde_json::json!([
                {"id": "config", "title": "Options", "layout": "layouts/configpage.xml"}
            ]);
        })
    }

    /// Rewrites `install.default_path` and returns where that resolves to.
    ///
    /// Windows expands the variable itself, so the expected directory is asked
    /// for rather than guessed at.
    fn set_configured_path(&self, value: &str) -> anyhow::Result<PathBuf> {
        let path = self.project.join("installer_config.json");
        let mut config: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        config["install"]["default_path"] = serde_json::Value::from(value);
        std::fs::write(&path, serde_json::to_vec_pretty(&config)?)?;
        let variable = value
            .trim_start_matches('%')
            .split('%')
            .next()
            .expect("the value names a variable");
        let root = std::env::var(variable)?;
        let tail = value
            .rsplit('\\')
            .next()
            .expect("the value names a directory");
        Ok(PathBuf::from(root).join(tail))
    }

    /// Rewrites the project configuration, which is how a case states the one
    /// setting it is about without a second fixture.
    fn edit_config(&self, edit: impl FnOnce(&mut serde_json::Value)) -> anyhow::Result<()> {
        let path = self.project.join("installer_config.json");
        let mut config: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        edit(&mut config);
        std::fs::write(&path, serde_json::to_vec_pretty(&config)?)?;
        Ok(())
    }

    /// Ships a project script, under the directory the builder packages.
    fn write_script(&self, name: &str, source: &str) -> anyhow::Result<()> {
        let scripts = self.project.join("scripts");
        std::fs::create_dir_all(&scripts)?;
        std::fs::write(scripts.join(name), source)?;
        Ok(())
    }

    /// Remembers a value this case wrote into a key the machine shares.
    fn remember_registry_value(&mut self, key: &str, name: &str) {
        self.extra_registry_values
            .push((key.to_string(), name.to_string()));
    }

    fn build(&self) -> anyhow::Result<()> {
        let mut request = BuildRequest::new(&self.project);
        request.output = Some(self.setup.clone());
        request.stub_directory = Some(self.stubs.clone());
        build_project(request)?;
        Ok(())
    }

    /// Builds this project's full setup to a path of the case's choosing.
    fn build_to(&self, output: &Path) -> anyhow::Result<BuildResult> {
        let mut request = BuildRequest::new(&self.project);
        request.output = Some(output.to_path_buf());
        request.stub_directory = Some(self.stubs.clone());
        build_project(request)
    }

    /// Builds a setup that carries only what changed since a release's payload.
    fn build_update(&self, from: &Path, output: &Path) -> anyhow::Result<BuildResult> {
        let mut request = BuildRequest::new(&self.project);
        request.output = Some(output.to_path_buf());
        request.stub_directory = Some(self.stubs.clone());
        request.delta_from = Some(from.to_path_buf());
        build_project(request)
    }

    /// Builds this project's setup together with the installer package around it.
    ///
    /// The package is what an estate deploys through Windows Installer, and it
    /// carries the setup this build finishes.
    fn build_package(&self, package: &Path) -> anyhow::Result<BuildResult> {
        let mut request = BuildRequest::new(&self.project);
        request.output = Some(self.setup.clone());
        request.stub_directory = Some(self.stubs.clone());
        request.msi = Some(package.to_path_buf());
        build_project(request)
    }

    /// Names this case's product something no other case uses.
    ///
    /// A package identifies its product by codes this build derives from the
    /// name, so two cases sharing a name would install the same product and
    /// fight over it when the suite runs them at once.
    fn uniquely_named(&self) -> anyhow::Result<String> {
        let name = format!("E2eProbeMsi{}", self.id.replace('-', ""));
        self.edit_config(|config| config["project"]["name"] = serde_json::json!(name))?;
        Ok(name)
    }

    /// Keeps the payload archive the project ships now.
    ///
    /// An update package is built against the release it replaces, so the case
    /// has to hold on to the archive before the next version overwrites it.
    fn keep_payload(&self, name: &str) -> anyhow::Result<PathBuf> {
        let kept = self.case_path(name);
        std::fs::copy(self.project.join("payload/app.archive"), &kept)?;
        Ok(kept)
    }

    /// Writes the payload archive with the bundled `7za.exe`.
    ///
    /// The same tool writes both formats, which keeps the two cases comparable
    /// and needs no archive crate in the test build: the runtime under test is
    /// the only thing that has to understand the result.
    fn archive_payload(&self, format: PayloadFormat, include_exe: bool) -> anyhow::Result<()> {
        self.archive_with(format, include_exe, &[])
    }

    fn archive_with(
        &self,
        format: PayloadFormat,
        include_exe: bool,
        extra: &[(&str, &[u8])],
    ) -> anyhow::Result<()> {
        let seven_zip = workspace_root().join("tools/7za.exe");
        anyhow::ensure!(seven_zip.is_file(), "tools/7za.exe is missing");
        let staged = self.project.join("payload-stage");
        if staged.is_dir() {
            std::fs::remove_dir_all(&staged)?;
        }
        std::fs::create_dir_all(staged.join("data"))?;
        // A known sequence, so a byte-level mix-up shows up and not merely a
        // missing file.
        std::fs::write(
            staged.join("data/expected.bin"),
            (0u8..=255).collect::<Vec<u8>>(),
        )?;
        if include_exe {
            std::fs::write(
                staged.join("E2eProbe.exe"),
                b"MZ end-to-end probe executable\r\n",
            )?;
        }
        for (relative, contents) in extra {
            let path = staged.join(relative);
            std::fs::create_dir_all(path.parent().expect("entry has a parent"))?;
            std::fs::write(&path, contents)?;
        }
        let archive = self.project.join("payload/app.archive");
        std::fs::remove_file(&archive).ok();
        let flag = match format {
            PayloadFormat::Zip => "-tzip",
            PayloadFormat::SevenZip => "-t7z",
        };
        let output = Command::new(&seven_zip)
            .current_dir(&staged)
            .arg("a")
            .arg(flag)
            .arg(&archive)
            .arg(".")
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "7za failed for {flag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::remove_dir_all(&staged)?;
        Ok(())
    }

    /// Builds one component's archive under the project's payload directory.
    ///
    /// `entries` are relative paths inside the archive, the way a real product
    /// splits documentation or samples off from the product itself.
    fn archive_component(
        &self,
        name: &str,
        format: PayloadFormat,
        entries: &[(&str, &[u8])],
    ) -> anyhow::Result<()> {
        let seven_zip = workspace_root().join("tools/7za.exe");
        anyhow::ensure!(seven_zip.is_file(), "tools/7za.exe is missing");
        let staged = self.project.join("component-stage");
        if staged.is_dir() {
            std::fs::remove_dir_all(&staged)?;
        }
        std::fs::create_dir_all(&staged)?;
        for (relative, contents) in entries {
            let path = staged.join(relative);
            std::fs::create_dir_all(path.parent().expect("entry has a parent"))?;
            std::fs::write(&path, contents)?;
        }
        let archive = self.project.join("payload").join(name);
        std::fs::remove_file(&archive).ok();
        let flag = match format {
            PayloadFormat::Zip => "-tzip",
            PayloadFormat::SevenZip => "-t7z",
        };
        let output = Command::new(&seven_zip)
            .current_dir(&staged)
            .arg("a")
            .arg(flag)
            .arg(&archive)
            .arg(".")
            .output()?;
        anyhow::ensure!(
            output.status.success(),
            "7za failed for {name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::remove_dir_all(&staged)?;
        Ok(())
    }

    /// A path inside the case's own directory, which goes away with it.
    fn case_path(&self, name: &str) -> PathBuf {
        self._temp.path().join(name)
    }

    /// Ships the repository's archiver as the program a dependency runs.
    ///
    /// A dependency is installed by running a real program, and this is the one
    /// the fixtures already use to write payloads: told to archive a file it
    /// leaves an archive behind, which is what a case watches for.
    fn bundle_dependency_program(&self) -> anyhow::Result<()> {
        let seven_zip = workspace_root().join("tools/7za.exe");
        anyhow::ensure!(seven_zip.is_file(), "tools/7za.exe is missing");
        let target = self.project.join("payload/dependency.exe");
        std::fs::create_dir_all(target.parent().expect("the payload directory"))?;
        std::fs::copy(&seven_zip, &target)?;
        Ok(())
    }

    /// Declares the one dependency a case is about, and where a silent run
    /// installs to.
    fn dependency_project(&self, dependency: serde_json::Value) -> anyhow::Result<()> {
        self.edit_config(|config| {
            config["install"]["default_path"] =
                serde_json::json!(self.destination.to_string_lossy());
            config["dependencies"] = serde_json::json!({ "items": [dependency] });
        })
    }

    fn registry_key(&self) -> String {
        format!(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\nano-installer-e2e-{}",
            self.id
        )
    }

    /// Runs the built setup with no window.
    fn install(&self) -> anyhow::Result<Output> {
        run_silent(&self.setup, Some(&self.destination))
    }

    /// Runs the built setup with no window, expecting it to refuse.
    fn install_expecting_failure(&self) -> anyhow::Result<Output> {
        run_expecting_failure(&self.setup, Some(&self.destination))
    }

    /// Runs the deployed uninstaller with no window.
    fn uninstall(&self) -> anyhow::Result<Output> {
        run_silent(&self.destination.join("uninst.exe"), None)
    }

    fn read_uninstall_entry(&self) -> anyhow::Result<Option<serde_json::Value>> {
        read_uninstall_entry(&self.registry_key())
    }
}

impl Drop for Fixture {
    /// Clears the installation when a case ends, so a failing assertion cannot
    /// leave a product registered on the machine running the tests.
    fn drop(&mut self) {
        let uninstaller = self.destination.join("uninst.exe");
        if uninstaller.is_file() {
            let _ = Command::new(&uninstaller).arg("--silent").output();
            wait_for_removal(&self.destination);
        }
        delete_registry_key(&self.registry_key());
        delete_registry_key(&self.test_key);
        for (key, name) in &self.extra_registry_values {
            delete_registry_value(key, name);
        }
        std::fs::remove_dir_all(&self.destination).ok();
    }
}

/// Runs an executable with `--silent`, optionally naming the install directory.
fn silent_command(exe: &Path, destination: Option<&Path>) -> Command {
    let mut command = Command::new(exe);
    command.arg("--silent");
    if let Some(destination) = destination {
        command.arg("--dir").arg(destination);
    }
    command
}

/// The same, with the file the run leaves its log in.
///
/// A windowless run has no notice to read, so this is where an administrator
/// deploying a machine finds out what happened.
fn silent_command_logged(exe: &Path, destination: Option<&Path>, log: &Path) -> Command {
    let mut command = silent_command(exe, destination);
    command.arg("--log").arg(log);
    command
}

/// Runs an executable with `--silent` and a named log file, expecting it to
/// succeed.
fn run_silent_logged(exe: &Path, destination: Option<&Path>, log: &Path) -> anyhow::Result<Output> {
    let output = silent_command_logged(exe, destination, log).output()?;
    anyhow::ensure!(
        output.status.success(),
        "{} exited with {:?}\nstdout: {}\nstderr: {}",
        exe.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

fn run_silent(exe: &Path, destination: Option<&Path>) -> anyhow::Result<Output> {
    let output = silent_command(exe, destination).output()?;
    anyhow::ensure!(
        output.status.success(),
        "{} exited with {:?}\nstdout: {}\nstderr: {}",
        exe.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

fn run_expecting_failure(exe: &Path, destination: Option<&Path>) -> anyhow::Result<Output> {
    let output = silent_command(exe, destination).output()?;
    anyhow::ensure!(
        !output.status.success(),
        "{} unexpectedly succeeded",
        exe.display()
    );
    Ok(output)
}

/// Runs the installer Windows itself ships, which is what an estate deploys a
/// package with and what takes one away again.
///
/// `/qn` is the unattended mode such an estate uses and `/norestart` keeps a
/// package from rebooting the machine running the suite. A windowless run says
/// nothing on its streams, so the log is what a failing case is read from and
/// what the error carries.
///
/// Windows runs one installation at a time on a machine, so the cases that drive
/// it take turns and a run another installation is already holding is tried
/// again: 1618 is the Installer saying so, and it is not a failure of the package.
fn run_msiexec(msi: &Path, action: &str, extra: &[String], log: &Path) -> anyhow::Result<Output> {
    const ERROR_INSTALL_ALREADY_RUNNING: i32 = 1618;
    let _installer = the_installer();
    for _ in 0..30 {
        let mut command = Command::new("msiexec.exe");
        command
            .arg(action)
            .arg(msi)
            .arg("/qn")
            .arg("/norestart")
            .arg("/l*v")
            .arg(log);
        for argument in extra {
            command.arg(argument);
        }
        let output = command.output()?;
        if output.status.code() == Some(ERROR_INSTALL_ALREADY_RUNNING) {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        anyhow::ensure!(
            output.status.success(),
            "msiexec {action} exited with {:?}\n{}",
            output.status.code(),
            log_tail(log)
        );
        return Ok(output);
    }
    anyhow::bail!("msiexec {action} never got the machine to itself")
}

/// Runs the installer Windows itself ships and expects it to refuse, which is
/// how a package that is no longer installed answers.
fn run_msiexec_expecting_failure(msi: &Path, action: &str, log: &Path) -> anyhow::Result<Output> {
    let _installer = the_installer();
    let output = Command::new("msiexec.exe")
        .arg(action)
        .arg(msi)
        .arg("/qn")
        .arg("/norestart")
        .arg("/l*v")
        .arg(log)
        .output()?;
    anyhow::ensure!(
        !output.status.success(),
        "msiexec {action} unexpectedly succeeded on {}",
        msi.display()
    );
    Ok(output)
}

/// The Installer the machine runs, which only runs one installation at a time.
///
/// Two package cases driving it at once would have the second one refused with
/// "another installation is already in progress" rather than with anything about
/// the package under test, so they take this in turn however the suite happens
/// to schedule them.
static INSTALLER: Mutex<()> = Mutex::new(());

fn the_installer() -> MutexGuard<'static, ()> {
    INSTALLER.lock().unwrap_or_else(|error| error.into_inner())
}

/// The end of an Installer log, which is where a failed action is written down.
fn log_tail(log: &Path) -> String {
    // The Installer writes its log in the machine's own code page, so a product
    // name outside it would fail a strict read.
    let text = String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).to_string();
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| {
            line.contains("Return value")
                || line.contains("error code is")
                || line.contains("Error ")
                || line.contains("Product:")
                || line.contains("NanoInstaller")
        })
        .collect();
    lines
        .iter()
        .rev()
        .take(20)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parses `reg query` output into its values.
///
/// Each value line reads `    Name    REG_SZ    the value`, and the value may
/// itself contain spaces, so only the first two columns are split off.
fn read_uninstall_entry(key: &str) -> anyhow::Result<Option<serde_json::Value>> {
    let output = Command::new("reg").arg("query").arg(key).output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut values = serde_json::Map::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((name, rest)) = line.split_once("    ") else {
            continue;
        };
        let Some((kind, value)) = rest.split_once("    ") else {
            continue;
        };
        if !kind.trim().starts_with("REG_") {
            continue;
        }
        values.insert(
            name.trim().to_string(),
            serde_json::Value::from(value.trim()),
        );
    }
    Ok(Some(serde_json::Value::Object(values)))
}

fn delete_registry_key(key: &str) {
    let _ = Command::new("reg")
        .arg("delete")
        .arg(key)
        .arg("/f")
        .output();
}

/// Removes one value, leaving the key it lives in alone.
fn delete_registry_value(key: &str, name: &str) {
    let _ = Command::new("reg")
        .arg("delete")
        .arg(key)
        .arg("/v")
        .arg(name)
        .arg("/f")
        .output();
}

/// One value of a registry key with the type the machine stored it as, or
/// `None` when the key or the value is absent.
///
/// The type is what a case asks about when it wants the machine's own reading
/// rather than what a primitive handed back. The value may itself hold spaces,
/// so each line is split into its columns rather than on whitespace.
fn read_registry_value(key: &str, name: &str) -> anyhow::Result<Option<(String, String)>> {
    let output = Command::new("reg")
        .arg("query")
        .arg(key)
        .arg("/v")
        .arg(name)
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let line = line.trim();
        let Some((found, rest)) = line.split_once("    ") else {
            continue;
        };
        let Some((kind, value)) = rest.split_once("    ") else {
            continue;
        };
        if kind.trim().starts_with("REG_") && found.trim().eq_ignore_ascii_case(name) {
            return Ok(Some((kind.trim().to_string(), value.trim().to_string())));
        }
    }
    Ok(None)
}

/// The text of one value, whatever type the machine stored it as.
fn read_registry_string(key: &str, name: &str) -> anyhow::Result<Option<String>> {
    Ok(read_registry_value(key, name)?.map(|(_, value)| value))
}

/// Whether a key exists, which is how a case tells "the value is gone" apart
/// from "the whole key is gone".
fn registry_key_exists(key: &str) -> bool {
    Command::new("reg")
        .arg("query")
        .arg(key)
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Whether the machine finds a key when it is told which view to read.
///
/// `reg` takes the copy as a flag, so what it reports is the reading a name like
/// `HKLM32\...` is supposed to reach, arrived at without going through the
/// primitives a case is checking.
fn registry_key_exists_in_view(key: &str, view: u32) -> bool {
    Command::new("reg")
        .arg("query")
        .arg(key)
        .arg(format!("/reg:{view}"))
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Waits for the post-uninstall cleaner to remove the installation directory.
fn wait_for_removal(directory: &Path) {
    for _ in 0..120 {
        if !directory.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// Waits for the cleaner to remove the deployed files, which happens while the
/// uninstaller is still exiting. The directory itself may survive on purpose.
fn wait_for_uninstaller_removal(directory: &Path) {
    for _ in 0..120 {
        if !directory.join("uninst.exe").exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

// ---------------------------------------------------------------------------
// Build product
// ---------------------------------------------------------------------------

/// The build product is stub + bundle + resources, and the bundle has to
/// describe itself at the very end of the file for the runtime to find it.
#[test]
fn a_built_setup_carries_a_readable_bundle_and_real_resources() -> anyhow::Result<()> {
    let Some(stubs) = stub_directory(false) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let Some(fixture) = Fixture::with_stubs(stubs.clone(), PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;

    let bytes = std::fs::read(&fixture.setup)?;
    // The footer, read exactly the way the runtime reads it.
    let footer = &bytes[bytes.len() - 16..];
    assert_eq!(
        &footer[8..],
        b"NATVEND1",
        "the bundle footer marker is wrong"
    );
    let size = u64::from_le_bytes(footer[..8].try_into()?) as usize;
    assert!(
        size > 0 && size < bytes.len(),
        "implausible bundle size {size}"
    );

    // Still a PE, so the file can be launched at all.
    assert_eq!(&bytes[..2], b"MZ", "the setup is no longer an executable");

    // The payload was appended: the image is much larger than the stub alone.
    let stub_size = std::fs::metadata(stubs.join("zlib-stub-native.exe"))?.len();
    assert!(
        (bytes.len() as u64) > stub_size,
        "the setup ({}) is not larger than its stub ({stub_size})",
        bytes.len()
    );

    // The resources the builder promised to inject are present as resources.
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains(
            "V\u{0}S\u{0}_\u{0}V\u{0}E\u{0}R\u{0}S\u{0}I\u{0}O\u{0}N\u{0}_\u{0}I\u{0}N\u{0}F\u{0}O"
        ),
        "the version resource is missing"
    );
    assert!(
        text.contains("asInvoker"),
        "the application manifest is missing"
    );
    Ok(())
}

/// The payload format decides which runtime is embedded. A setup that keeps
/// claiming the wrong one installs nothing, because that stub cannot read the
/// archive.
#[test]
fn the_payload_format_selects_the_runtime_that_gets_embedded() -> anyhow::Result<()> {
    assert_eq!(PayloadFormat::Zip.stub_name(), "zlib-stub-native.exe");
    assert_eq!(PayloadFormat::SevenZip.stub_name(), "lzma-stub-native.exe");

    for (format, expected_backend) in [
        (PayloadFormat::Zip, "zlib-stub-native.exe"),
        (PayloadFormat::SevenZip, "lzma-stub-native.exe"),
    ] {
        let Some(fixture) = Fixture::new(format, true, true) else {
            skip_missing_stubs()?;
            return Ok(());
        };
        // The format is decided from the archive signature, not its extension.
        let summary = nano_installer_core::inspect_project(&fixture.project)?;
        assert_eq!(
            summary.payload_format, format,
            "the archive signature routed to the wrong backend"
        );
        let mut request = BuildRequest::new(&fixture.project);
        request.output = Some(fixture.setup.clone());
        request.stub_directory = Some(fixture.stubs.clone());
        let mut reported = Vec::new();
        let result = build_project_with_progress(request, |event| reported.push(event.message))?;

        // The runtime that got embedded is the one the format names, both as
        // the path the build returned and as the step it reported.
        assert_eq!(
            result.stub_path.file_name().and_then(|name| name.to_str()),
            Some(expected_backend),
            "the build packed the wrong runtime for {}",
            format.label()
        );
        assert!(
            reported
                .iter()
                .any(|message| message.contains(expected_backend)),
            "the build never reported {expected_backend}: {reported:?}"
        );
        assert!(fixture.setup.is_file());
    }
    Ok(())
}

/// A setup an integrator has signed is still a setup.
///
/// Signing is the release pipeline's step, not the builder's, and Authenticode
/// appends its certificate table behind everything the build wrote, footer
/// included. A runtime that only looked at the last bytes of its own file found
/// no bundle at all and refused to install, so this is what a signed setup has
/// to survive.
#[test]
fn a_setup_with_a_signature_appended_still_installs() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;
    append_certificate_table(&fixture.setup)?;

    let output = fixture.install()?;
    assert!(
        output.status.success(),
        "a setup with a signature appended refused to install: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture.destination.join("E2eProbe.exe").is_file(),
        "a setup with a signature appended deployed nothing"
    );
    assert!(
        fixture.read_uninstall_entry()?.is_some(),
        "a setup with a signature appended registered nothing"
    );
    Ok(())
}

/// Appends a certificate table the way Authenticode does: a length, a revision
/// and a type in front of the signature blob, all of it behind the bundle.
///
/// The bytes stand in for a real signature, which this suite has no certificate
/// to produce; what matters to the runtime is only that the footer is no longer
/// the last thing in the file.
fn append_certificate_table(setup: &Path) -> anyhow::Result<()> {
    let signature = [0x5Au8; 1024];
    let mut table = Vec::with_capacity(signature.len() + 8);
    table.extend_from_slice(&u32::try_from(signature.len() + 8)?.to_le_bytes());
    table.extend_from_slice(&0x0200u16.to_le_bytes());
    table.extend_from_slice(&0x0002u16.to_le_bytes());
    table.extend_from_slice(&signature);

    let mut bytes = std::fs::read(setup)?;
    bytes.extend_from_slice(&table);
    std::fs::write(setup, bytes)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// What the project runs on the finished files
// ---------------------------------------------------------------------------

/// A project signs its own build, and the builder hands it the files: the
/// uninstaller while it is still a file of its own, the setup once it is
/// complete on disk. These are NSIS's `!finalize` and `!uninstfinalize`; what
/// the command does with the file is the project's business, so here it records
/// which file it was given and stamps a mark into it.
#[test]
fn a_finalize_command_runs_on_the_finished_setup_and_uninstaller() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let log = fixture.case_path("finalize.log");
    // A program of the project's own, standing in for a signer: it appends the
    // file it was handed to a log and leaves a mark in that file.
    let write_command = |name: &str, label: &str| {
        std::fs::write(
            fixture.case_path(name),
            format!(
                "@echo off\r\n>>\"{log}\" echo {label} %~1\r\n>>\"%~1\" echo {label}-finalize-mark\r\n",
                log = log.display()
            ),
        )
    };
    write_command("uninstaller-finalize.cmd", "uninstaller")?;
    write_command("installer-finalize.cmd", "installer")?;
    fixture.edit_config(|config| {
        // Both are quoted, as a real command naming a program under a path with
        // a space in it would be.
        config["finalize"] = serde_json::json!({
            "uninstaller": format!(
                "\"{}\" \"%1\"",
                fixture.case_path("uninstaller-finalize.cmd").display()
            ),
            "installer": format!(
                "\"{}\" \"%1\"",
                fixture.case_path("installer-finalize.cmd").display()
            ),
        });
    })?;

    fixture.build()?;

    // The uninstaller was stamped before it was embedded, so the mark is inside
    // the entry the setup carries rather than on some file beside it.
    let bytes = std::fs::read(&fixture.setup)?;
    let (offset, size) = bundle_entry(&fixture.setup, "runtime/uninst.exe")?;
    assert!(
        bytes[offset..offset + size].ends_with(b"uninstaller-finalize-mark\r\n"),
        "the embedded uninstaller does not carry what the project's command wrote into it"
    );
    // The setup was stamped after it was finished, footer and all: the mark sits
    // behind the bundle, which is exactly what a signature does to the file.
    assert!(
        bytes.ends_with(b"installer-finalize-mark\r\n"),
        "the finished setup does not carry what the project's command wrote into it"
    );

    let log_text = std::fs::read_to_string(&log)?;
    let lines: Vec<&str> = log_text.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "each command should have run once: {log_text}"
    );
    assert!(lines[0].starts_with("uninstaller "), "{log_text}");
    assert!(lines[1].starts_with("installer "), "{log_text}");
    // The uninstaller is handed a file of its own, and the setup is handed the
    // file this case asked for.
    assert!(
        !lines[0].contains("E2eProbe_Setup.exe"),
        "the uninstaller command was handed the setup instead: {log_text}"
    );
    assert!(
        lines[1].ends_with(&fixture.setup.display().to_string()),
        "the installer command was handed something else: {log_text}"
    );

    // And a setup that grew behind its footer still installs.
    fixture.install()?;
    assert!(
        fixture.destination.join("E2eProbe.exe").is_file(),
        "a stamped setup deployed nothing"
    );
    assert!(
        fixture.read_uninstall_entry()?.is_some(),
        "a stamped setup registered nothing"
    );
    Ok(())
}

/// A command that refuses the file stops the build, and the setup it refused is
/// not left where the next step of a pipeline would pick it up and ship a file
/// nobody signed.
#[test]
fn a_finalize_command_that_fails_stops_the_build() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.edit_config(|config| {
        config["finalize"] = serde_json::json!({
            "installer": "echo the signer refused this file& exit /b 7"
        });
    })?;

    let mut messages = Vec::new();
    let mut request = BuildRequest::new(&fixture.project);
    request.output = Some(fixture.setup.clone());
    request.stub_directory = Some(fixture.stubs.clone());
    let error = build_project_with_progress(request, |event| messages.push(event.message))
        .expect_err("a command that refused the file stops the build");
    let error = format!("{error:#}");
    assert!(error.contains("finalize.installer"), "{error}");
    assert!(error.contains("exit code 7"), "{error}");
    // What the command printed is in the build log: that is where the reason a
    // signer gave for refusing the file has to be readable.
    assert!(
        messages
            .iter()
            .any(|message| message.contains("the signer refused this file")),
        "the command's own output never reached the log: {messages:?}"
    );
    assert!(
        !fixture.setup.exists(),
        "a setup the project's own command refused stayed at {}",
        fixture.setup.display()
    );
    Ok(())
}

/// A project signs the package it deploys, and not only the setup inside it.
///
/// The package is a file this build finished too: an estate deploys the `.msi`,
/// so that is the file whose signature a machine checks. The command here stands
/// in for a signer -- it records which file it was handed and rewrites it, which
/// is what signing does -- and the case holds the build to handing over the
/// package and to reporting the package that is really on disk afterwards.
#[test]
fn a_finalize_command_runs_on_the_package_a_build_writes() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let log = fixture.case_path("package-finalize.log");
    let signer = fixture.case_path("package-finalize.cmd");
    std::fs::write(
        &signer,
        format!(
            "@echo off\r\n>>\"{log}\" echo package %~1\r\n>>\"%~1\" echo package-finalize-mark\r\n",
            log = log.display()
        ),
    )?;
    fixture.edit_config(|config| {
        config["finalize"] = serde_json::json!({
            "package": format!("\"{}\" \"%1\"", signer.display()),
        });
    })?;

    let package = fixture.case_path("E2eProbe-Package.msi");
    let built = fixture
        .build_package(&package)?
        .msi
        .expect("a package was written");

    let text = std::fs::read_to_string(&log)?;
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "the command should have run once: {text}");
    assert!(lines[0].starts_with("package "), "{text}");
    assert!(
        lines[0].ends_with(&package.display().to_string()),
        "the command was handed something other than the package: {text}"
    );
    // What the command wrote into the file is the file that ships, so the size
    // the build reports has to be read after it rather than from the wrapper this
    // build wrote first.
    let on_disk = std::fs::metadata(&package)?.len();
    assert!(on_disk > 0);
    assert_eq!(
        built.output_size, on_disk,
        "the build reports a package size that is not the file the command left"
    );
    Ok(())
}

/// A command that refuses the package stops the build, and the package it refused
/// is not left where the next step of a pipeline would pick it up and deploy a
/// package nobody signed.
#[test]
fn a_finalize_command_that_refuses_the_package_stops_the_build() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.edit_config(|config| {
        config["finalize"] = serde_json::json!({
            "package": "echo the signer refused this package& exit /b 9"
        });
    })?;

    let package = fixture.case_path("E2eProbe-Package.msi");
    let error = fixture
        .build_package(&package)
        .expect_err("a command that refused the package stops the build");
    let error = format!("{error:#}");
    assert!(error.contains("finalize.package"), "{error}");
    assert!(error.contains("exit code 9"), "{error}");
    assert!(
        !package.exists(),
        "a package the project's own command refused stayed at {}",
        package.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

/// The whole point of a setup: the payload lands on disk, a manifest records
/// what was written, an uninstall entry is registered, and the payload's own
/// bytes survive the trip.
#[test]
fn a_built_setup_installs_its_payload_and_registers_an_uninstall_entry() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;
    fixture.install()?;

    let destination = &fixture.destination;
    assert!(
        destination.join("E2eProbe.exe").is_file(),
        "the product exe is missing"
    );
    assert!(
        destination.join("uninst.exe").is_file(),
        "no uninstaller was embedded"
    );
    assert!(
        destination.join("nano-installer-manifest.json").is_file(),
        "no manifest was written"
    );

    // The nested payload file has to come back byte for byte: a truncating or
    // reordering extractor still produces a file of the right name.
    let nested = destination.join("data/expected.bin");
    assert!(
        nested.is_file(),
        "the nested payload directory was not recreated"
    );
    assert_eq!(
        std::fs::read(&nested)?,
        (0u8..=255).collect::<Vec<u8>>(),
        "the extracted payload does not match what was archived"
    );

    // The manifest is what the uninstaller replays, so it has to list the files
    // that were actually deployed.
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        destination.join("nano-installer-manifest.json"),
    )?)?;
    assert_eq!(manifest["version"], 1);
    let listed = manifest["files"].to_string();
    assert!(
        listed.contains("E2eProbe.exe"),
        "the manifest does not list the product exe"
    );
    assert!(
        listed.contains("expected.bin"),
        "the manifest does not list the nested file"
    );

    // The registration points Windows back at the deployed uninstaller.
    let entry = fixture.read_uninstall_entry()?.expect("an uninstall entry");
    let uninstall_string = entry["UninstallString"].as_str().unwrap_or_default();
    assert!(
        uninstall_string.contains("uninst.exe"),
        "the uninstall entry does not name the uninstaller: {uninstall_string}"
    );
    assert_eq!(entry["DisplayName"], "E2eProbe");
    Ok(())
}

/// A package carries the setup and drives it with no window, so an estate that
/// deploys through Windows Installer gets the product the setup installs, and
/// taking the package away again takes the product with it.
///
/// The package decides where the product goes, so the case names a directory on
/// the command line the way an administrator would: the removal has to find the
/// product there rather than where the package would have put it by itself.
#[test]
fn a_package_installs_the_product_the_setup_carries_and_removes_it_again() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let name = fixture.uniquely_named()?;
    let package = fixture.case_path("E2eProbe.msi");
    fixture.build_package(&package)?;
    assert!(package.is_file(), "no package was written");

    let install_log = fixture.case_path("package-install.log");
    run_msiexec(
        &package,
        "/i",
        &[format!("INSTALLDIR={}", fixture.destination.display())],
        &install_log,
    )?;

    let destination = &fixture.destination;
    assert!(
        destination.join("E2eProbe.exe").is_file(),
        "the package did not install the product"
    );
    assert!(
        destination.join("uninst.exe").is_file(),
        "the package did not deploy the uninstaller"
    );
    assert!(
        destination.join("nano-installer-manifest.json").is_file(),
        "the package did not install the product the setup knows about"
    );
    let entry = fixture
        .read_uninstall_entry()?
        .expect("the product registered itself");
    assert_eq!(entry["DisplayName"], serde_json::json!(name));
    // The registration points at the directory the package really used, so a
    // trailing separator is the only difference a comparison may forgive.
    let location = entry["InstallLocation"].as_str().unwrap_or_default();
    assert_eq!(
        Path::new(location.trim_end_matches('\\')),
        destination,
        "the product does not say where the package put it"
    );
    // What the package recorded is what its removal reads, so the directory it
    // installed into has to be there whether or not the caller named one.
    let tracking = read_registry_string(
        &format!(r"HKCU\Software\nano-installer e2e\{name}"),
        "InstallLocation",
    )?;
    assert!(
        tracking
            .as_deref()
            .is_some_and(|value| value.starts_with(&destination.display().to_string())),
        "the package did not record where it installed the product: {tracking:?}"
    );

    let uninstall_log = fixture.case_path("package-uninstall.log");
    run_msiexec(&package, "/x", &[], &uninstall_log)?;
    // The removal is what the product's own uninstaller does, and that happens
    // while `msiexec` is still exiting: it copies itself out, waits for itself,
    // and only then takes the directory. Asking whether the directory is gone
    // the moment `msiexec` returns is asking a question that has not been
    // answered yet -- which is how this case failed once, under the load of the
    // whole suite, and passed alone in 13.6 s.
    wait_for_removal(destination);
    assert!(
        !destination.exists(),
        "the product is still on disk after the package was removed"
    );
    assert!(
        fixture.read_uninstall_entry()?.is_none(),
        "the product is still registered after the package was removed"
    );
    Ok(())
}

/// A package re-released on the same day replaces what the earlier one put
/// there.
///
/// Windows Installer compares three version fields, so `1.0.0-r2` ships the same
/// package version as `1.0.0`. What has to happen is that installing it leaves
/// the release it carries on the machine: the Installer refuses a package whose
/// product code and version are ones it already has (1638), so the re-release is
/// a product of its own and the search in its own tables is what takes the
/// release it replaces away. Nothing here may end with two products, a refusal,
/// or the first release still installed.
#[test]
fn a_package_released_again_on_the_same_day_replaces_what_it_installed() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let name = fixture.uniquely_named()?;
    let first = fixture.case_path("E2eProbe-rebuild-1.msi");
    let built = fixture.build_package(&first)?;
    let built = built.msi.expect("a package");
    let product_code = built.product_code.clone();
    let upgrade_code = built.upgrade_code.clone();
    run_msiexec(
        &first,
        "/i",
        &[format!("INSTALLDIR={}", fixture.destination.display())],
        &fixture.case_path("rebuild-first.log"),
    )?;
    let entry = fixture
        .read_uninstall_entry()?
        .expect("the product registered itself");
    assert_eq!(
        entry["DisplayVersion"],
        serde_json::json!("1.0.0"),
        "the first release did not register itself"
    );

    // The second release of the same day carries one more file and its version
    // suffix, which is the only part of the version Windows Installer cannot
    // compare.
    fixture.archive_with(
        PayloadFormat::Zip,
        true,
        &[("data/second.txt", b"second release\r\n")],
    )?;
    fixture.edit_config(|config| {
        config["project"]["version"] = serde_json::json!("1.0.0-r2");
    })?;
    let second = fixture.case_path("E2eProbe-rebuild-2.msi");
    let rebuilt = fixture.build_package(&second)?;
    let rebuilt = rebuilt.msi.expect("a package");
    assert_ne!(
        rebuilt.product_code, product_code,
        "a release with a version of its own cannot share a product code with the one it re-releases"
    );
    assert_eq!(
        rebuilt.upgrade_code, upgrade_code,
        "a re-release has to stay findable by the code that names the product"
    );
    run_msiexec(
        &second,
        "/i",
        &[format!("INSTALLDIR={}", fixture.destination.display())],
        &fixture.case_path("rebuild-second.log"),
    )?;

    assert!(
        fixture.destination.join("data/second.txt").is_file(),
        "the re-release did not install its own payload"
    );
    let entry = fixture
        .read_uninstall_entry()?
        .expect("the product registered itself again");
    assert_eq!(entry["DisplayName"], serde_json::json!(name));
    assert_eq!(
        entry["DisplayVersion"],
        serde_json::json!("1.0.0-r2"),
        "the release on the machine is not the one that was installed last"
    );
    // The release it replaces is gone rather than sitting beside it: a package
    // whose product is still installed answers a removal, and one whose product
    // was taken away has nothing to remove.
    run_msiexec_expecting_failure(
        &first,
        "/x",
        &fixture.case_path("rebuild-first-uninstall.log"),
    )?;

    // One product, so one removal: what the re-release left behind is what the
    // machine gives up.
    run_msiexec(
        &second,
        "/x",
        &[],
        &fixture.case_path("rebuild-uninstall.log"),
    )?;
    // For the same reason as the removal above: what the package removed is
    // taken away by the product's own uninstaller, which is still running when
    // `msiexec` returns.
    wait_for_removal(&fixture.destination);
    assert!(
        !fixture.destination.exists(),
        "the product is still on disk after the package was removed"
    );
    assert!(
        fixture.read_uninstall_entry()?.is_none(),
        "the product is still registered after the package was removed"
    );
    Ok(())
}

/// A newer package replaces the older product instead of sitting beside it.
///
/// The two versions carry the same upgrade code, which is what lets the newer
/// one find the older one, and the older product is taken away before the newer
/// setup runs: the setup installs over whatever it finds, so a removal that ran
/// afterwards would take the new product away with the old one.
#[test]
fn a_newer_package_upgrades_the_product_the_older_one_installed() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let name = fixture.uniquely_named()?;
    let first = fixture.case_path("E2eProbe-1.msi");
    let built = fixture.build_package(&first)?;
    let upgrade_code = built.msi.expect("a package").upgrade_code.clone();
    let install_log = fixture.case_path("first-install.log");
    run_msiexec(
        &first,
        "/i",
        &[format!("INSTALLDIR={}", fixture.destination.display())],
        &install_log,
    )?;

    fixture.edit_config(|config| config["project"]["version"] = serde_json::json!("1.0.1"))?;
    let second = fixture.case_path("E2eProbe-2.msi");
    let rebuilt = fixture.build_package(&second)?;
    let rebuilt = rebuilt.msi.expect("a package");
    assert_eq!(
        rebuilt.upgrade_code, upgrade_code,
        "a new version has to keep the code that names the product across versions"
    );
    let upgrade_log = fixture.case_path("second-install.log");
    run_msiexec(
        &second,
        "/i",
        &[format!("INSTALLDIR={}", fixture.destination.display())],
        &upgrade_log,
    )?;

    let entry = fixture
        .read_uninstall_entry()?
        .expect("the newer product registered itself");
    assert_eq!(
        entry["DisplayVersion"],
        serde_json::json!("1.0.1"),
        "the machine still holds the older product"
    );
    assert_eq!(entry["DisplayName"], serde_json::json!(name));
    // The older product is gone rather than sitting beside the newer one: a
    // package whose product is still installed answers a removal, and one whose
    // product was taken away has nothing to remove.
    run_msiexec_expecting_failure(&first, "/x", &fixture.case_path("first-uninstall.log"))?;
    run_msiexec(
        &second,
        "/x",
        &[],
        &fixture.case_path("second-uninstall.log"),
    )?;
    // The product's own uninstaller does the removing, and it is still running
    // when `msiexec` returns.
    wait_for_removal(&fixture.destination);
    assert!(
        !fixture.destination.exists(),
        "the upgraded product is still on disk after the package was removed"
    );
    Ok(())
}

/// A package drives the setup with no window, so a project that never declared
/// a windowless flow has nothing for the package to run.
///
/// Refusing the build is what keeps a project from shipping a package that
/// cannot install: the failure names the setting the project has to turn on.
#[test]
fn a_project_that_cannot_run_without_a_window_is_refused_a_package() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, false, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let error = fixture
        .build_package(&fixture.case_path("E2eProbe.msi"))
        .expect_err("a project without a windowless flow cannot be packaged");
    let text = format!("{error:#}");
    assert!(
        text.contains("silent_mode_support"),
        "the failure does not name what the project is missing: {text}"
    );
    assert!(
        !fixture.case_path("E2eProbe.msi").exists(),
        "a refused build left a package behind"
    );
    Ok(())
}

/// The uninstall entry is the whole of what Windows shows a user, so it has to
/// carry the fields an installation list reads: the size as a number, a command
/// that removes the product without a window, and the two flags that keep
/// Windows from offering a repair or a modify step this installer does not
/// have. A string where a number belongs reads as an absent field.
#[test]
fn the_uninstall_entry_reports_the_size_the_quiet_uninstall_and_no_repair() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;
    fixture.install()?;
    let key = fixture.registry_key();

    // What a script or an administrator runs to remove the product unattended:
    // the deployed uninstaller, told to keep its window out of the way.
    let (kind, quiet) =
        read_registry_value(&key, "QuietUninstallString")?.expect("no quiet uninstall command");
    assert_eq!(kind, "REG_SZ", "the quiet command is stored as {kind}");
    assert!(
        quiet.contains("uninst.exe") && quiet.ends_with("--silent"),
        "the quiet command does not run the uninstaller silently: {quiet}"
    );

    // The size is what the directory takes: every file the installation owns,
    // the uninstaller included, in the kilobytes Windows counts in.
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        fixture.destination.join("nano-installer-manifest.json"),
    )?)?;
    let owned = manifest["files"]
        .as_array()
        .expect("the manifest lists the deployed files")
        .iter()
        .filter_map(|value| value.as_str())
        .map(|relative| fixture.destination.join(relative))
        .chain(std::iter::once(fixture.destination.join("uninst.exe")))
        .filter_map(|path| std::fs::metadata(path).ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum::<u64>()
        / 1024;
    let (kind, size) = read_registry_value(&key, "EstimatedSize")?.expect("no estimated size");
    assert_eq!(kind, "REG_DWORD", "the size is stored as {kind}");
    let reported = u32::from_str_radix(size.trim_start_matches("0x"), 16)
        .unwrap_or_else(|_| panic!("the size is not a number: {size}"));
    assert_eq!(
        reported,
        u32::try_from(owned).unwrap_or(u32::MAX).max(1),
        "the entry reports {reported} KiB while the installation holds {owned} KiB"
    );

    // No repair and no modify step exists, and the flags say so: a button that
    // leads nowhere is worse than no button.
    for name in ["NoModify", "NoRepair"] {
        let (kind, value) = read_registry_value(&key, name)?.expect("a missing flag");
        assert_eq!(kind, "REG_DWORD", "{name} is stored as {kind}");
        assert_eq!(value, "0x1", "{name} is {value}");
    }
    Ok(())
}

/// A configured `%LOCALAPPDATA%` path has to be expanded before use, because an
/// unexpanded one is not absolute and an install refuses a relative directory.
/// This runs with no `--dir` at all, which is what a silent run without an
/// override does.
#[test]
fn a_configured_percent_path_is_expanded_and_used() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let name = format!("nano-installer-e2e-{}", fixture.id);
    let expected = fixture.set_configured_path(&format!("%TEMP%\\{name}"))?;
    // The fixture's own cleanup looks at `destination`, so point it at the path
    // this case actually installs into.
    let mut fixture = fixture;
    fixture.destination = expected.clone();
    fixture.build()?;

    run_silent(&fixture.setup, None)?;
    assert!(
        expected.join("E2eProbe.exe").is_file(),
        "nothing was installed into the configured path {}",
        expected.display()
    );
    Ok(())
}

/// The directory named on the command line wins over the configured one, which
/// is what lets a silent run choose where a product lands.
#[test]
fn an_explicit_directory_wins_over_the_configured_one() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let configured =
        fixture.set_configured_path(&format!("%TEMP%\\nano-installer-e2e-{}", fixture.id))?;
    fixture.build()?;

    fixture.install()?;
    assert!(
        fixture.destination.join("E2eProbe.exe").is_file(),
        "the explicit directory was not used"
    );
    assert!(
        !configured.join("E2eProbe.exe").exists(),
        "the configured path was used even though one was given: {}",
        configured.display()
    );
    let _ = std::fs::remove_dir_all(&configured);
    Ok(())
}

/// A project that never declared silent support must refuse a windowless run
/// rather than install unattended.
#[test]
fn a_project_without_silent_support_refuses_a_windowless_install() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, false, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;

    let result = fixture.install_expecting_failure()?;
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("silent_mode_support"),
        "the failure should name the switch to set: {stderr}"
    );
    assert!(
        !fixture.destination.exists(),
        "a refused run still created {}",
        fixture.destination.display()
    );
    Ok(())
}

/// A mistyped option has to stop the run. Installing into the configured default
/// instead would put files somewhere nobody asked for.
#[test]
fn an_unknown_silent_option_is_refused() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;

    let output = Command::new(&fixture.setup)
        .arg("--silent")
        .arg("--silnet")
        .output()?;
    assert!(!output.status.success(), "an unknown option was accepted");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--silnet"),
        "the error should name the offending option"
    );
    assert!(!fixture.destination.exists(), "files were written anyway");
    Ok(())
}

/// A windowless run leaves the record of itself where it was told to, and that
/// record holds what the wizard's own tail cannot.
///
/// The wizard keeps the last lines of a run in memory to explain a failure, and
/// a machine nobody can look at during an unattended deployment has no wizard at
/// all. The file is the whole run: which product went where, what the machine
/// was, and every line the project's own script wrote -- more of them than the
/// wizard would have kept.
#[test]
fn a_setup_writes_the_log_of_its_run_where_a_windowless_run_asks_for_it() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.write_script(
        "install.rhai",
        r#"
            let install_path = get_install_path();
            if !extract_payload_with_progress(0.0, 60.0) {
                return;
            }
            copy_uninstaller();
            for i in 1..=40 {
                log_info("script line " + i);
            }
            set_progress(90.0);
        "#,
    )?;
    fixture.write_script("uninstall.rhai", "run_tracked_uninstall(10.0, 90.0);")?;
    fixture.build()?;

    let installed_log = fixture.destination.with_extension("install.log");
    run_silent_logged(&fixture.setup, Some(&fixture.destination), &installed_log)?;
    let written = std::fs::read_to_string(&installed_log)?;
    for expected in [
        "nano-installer setup log",
        "run: install",
        "elevated: ",
        "windows: ",
        "machine: ",
        // The project's own name and version, which is what a support reader
        // needs to tell one product's log from another's.
        "product: E2eProbe 1.0.0",
        "install into: ",
        "info: preparing the installation",
        "info: running the project's own install script",
        "info: the installation is complete",
    ] {
        assert!(
            written.contains(expected),
            "{expected} is missing:\n{written}"
        );
    }
    assert!(
        written.contains(&format!("install into: {}", fixture.destination.display())),
        "the log does not say where the product went:\n{written}"
    );
    // Forty lines, where the wizard would have kept the last thirty-two: the
    // file is the whole run and not the tail of one.
    let script_lines = written
        .lines()
        .filter(|line| line.contains("info: script line "))
        .count();
    assert_eq!(script_lines, 40, "the script's own lines are not all here");

    // An uninstall logs the same way, and says which of the two it is.
    let removed_log = fixture.destination.with_extension("uninstall.log");
    run_silent_logged(&fixture.destination.join("uninst.exe"), None, &removed_log)?;
    let written = std::fs::read_to_string(&removed_log)?;
    for expected in [
        "run: uninstall",
        "product: E2eProbe 1.0.0",
        "info: running the project's own uninstall script",
        "info: the product is removed",
    ] {
        assert!(
            written.contains(expected),
            "{expected} is missing:\n{written}"
        );
    }
    assert!(
        written.contains(&format!("remove from: {}", fixture.destination.display())),
        "the log does not say what was removed:\n{written}"
    );
    Ok(())
}

/// A run that fails leaves its log behind and names it to whoever started it.
///
/// This is the file a user is asked for: the run is over, the machine is back
/// the way it was, and the only thing left that says what happened is the log
/// the setup wrote -- which is why it may not live inside the installation the
/// failed run removed.
#[test]
fn a_failing_run_leaves_its_log_behind_and_names_it() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.write_script(
        "install.rhai",
        r#"
            log_info("about to fail");
            throw "the probe refuses to install";
        "#,
    )?;
    fixture.build()?;

    let log = fixture.destination.with_extension("failure.log");
    let output =
        silent_command_logged(&fixture.setup, Some(&fixture.destination), &log).output()?;
    assert!(!output.status.success(), "the failing script was accepted");

    // The caller is told where the log is, because that is the one thing it can
    // pass on.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("A log of this run is at:") && stderr.contains(&log.display().to_string()),
        "the failure does not name its log: {stderr}"
    );

    let written = std::fs::read_to_string(&log)?;
    for expected in [
        "product: E2eProbe 1.0.0",
        "info: preparing the installation",
        "info: about to fail",
        "error: project script failed",
        "the probe refuses to install",
    ] {
        assert!(
            written.contains(expected),
            "{expected} is missing:\n{written}"
        );
    }
    assert!(
        !written.contains("info: the run finished"),
        "a failed run reported itself as finished:\n{written}"
    );
    // The log outlives the failure even though the failed run took the
    // installation with it.
    assert!(
        !fixture.destination.exists(),
        "a failed run left {} behind",
        fixture.destination.display()
    );
    assert!(log.is_file(), "the log did not survive the rollback");
    Ok(())
}

// ---------------------------------------------------------------------------
// Upgrade
// ---------------------------------------------------------------------------

/// Installing this version over the previous one replaces the product and drops
/// the files the new payload no longer ships.
#[test]
fn installing_over_an_existing_installation_drops_stale_files() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    // The first payload carries a file the second one will not.
    fixture.archive_with(PayloadFormat::Zip, true, &[("stale/old.txt", b"obsolete")])?;
    fixture.build()?;
    fixture.install()?;
    let stale = fixture.destination.join("stale/old.txt");
    assert!(
        stale.is_file(),
        "the first install did not deploy the extra file"
    );

    // Rebuild without it, and install over the top.
    fixture.archive_payload(PayloadFormat::Zip, true)?;
    fixture.build()?;
    fixture.install()?;

    assert!(
        fixture.destination.join("E2eProbe.exe").is_file(),
        "the product went missing"
    );
    assert!(
        !stale.exists(),
        "the upgrade left the dropped file behind: {}",
        stale.display()
    );
    Ok(())
}

/// A local settings file the product wrote is not part of the payload, so an
/// upgrade must leave it alone.
#[test]
fn an_upgrade_keeps_a_file_the_payload_does_not_own() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;
    fixture.install()?;

    let user_file = fixture.destination.join("user-settings.ini");
    std::fs::write(&user_file, b"[user]\nvalue=1\n")?;

    fixture.build()?;
    fixture.install()?;

    assert!(
        user_file.is_file(),
        "the upgrade deleted a file it does not own: {}",
        user_file.display()
    );
    assert_eq!(std::fs::read(&user_file)?, b"[user]\nvalue=1\n");
    Ok(())
}

/// An update package is a setup built from the release it replaces: it carries
/// only the files whose bytes changed, installs over exactly that release, and
/// leaves the files it did not carry where the first install put them.
#[test]
fn an_update_package_carries_only_what_changed() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    // A file both releases ship unchanged, large enough that carrying it again
    // would be visible in the size of the setup. The bytes come from a hash of
    // the offset, so the archiver cannot squeeze them away.
    let runtime = (0..512 * 1024u32)
        .map(|index| (index.wrapping_mul(2_654_435_761) ^ (index >> 7)) as u8)
        .collect::<Vec<u8>>();
    // The release the update is built from: what the next version keeps, and one
    // file it drops.
    fixture.archive_with(
        PayloadFormat::Zip,
        true,
        &[
            ("data/runtime.bin", runtime.as_slice()),
            ("data/legacy.bin", b"dropped in the next release"),
        ],
    )?;
    let previous = fixture.keep_payload("app-v1.archive")?;
    fixture.build()?;
    fixture.install()?;
    let kept = fixture.destination.join("data/expected.bin");
    let installed_runtime = fixture.destination.join("data/runtime.bin");
    assert_eq!(
        std::fs::read(&kept)?,
        (0u8..=255).collect::<Vec<u8>>(),
        "the first install did not deploy the payload as it was archived"
    );
    assert_eq!(
        std::fs::read(&installed_runtime)?,
        runtime,
        "the first install did not deploy the large file as it was archived"
    );

    // The next version: a new product exe, one new file, and the file the
    // release before it shipped is gone.
    fixture.archive_with(
        PayloadFormat::Zip,
        true,
        &[
            ("data/runtime.bin", runtime.as_slice()),
            ("E2eProbe.exe", SECOND_RELEASE_EXE),
            ("data/added.bin", b"added in this release"),
        ],
    )?;
    fixture.edit_config(|config| config["project"]["version"] = serde_json::json!("1.1.0"))?;

    // The same release as a full setup as well, so the size the update package
    // came to is measured against what it saves rather than against a number.
    let full = fixture.build_to(&fixture.setup)?;
    assert!(
        full.update.is_none(),
        "a full setup reported an update package"
    );
    let update_setup = fixture.case_path("E2eProbe_Update.exe");
    let update = fixture.build_update(&previous, &update_setup)?;
    let summary = update
        .update
        .context("the build reported no update summary")?;
    assert_eq!(
        summary.kept, 2,
        "the two releases share the payload file and the large file, and nothing else"
    );
    assert_eq!(
        summary.changed, 2,
        "the update carries the product exe and the file this release adds"
    );
    let full_size = std::fs::metadata(&fixture.setup)?.len();
    let update_size = std::fs::metadata(&update_setup)?.len();
    assert!(
        update_size < full_size,
        "the update package ({update_size} bytes) is no smaller than the full setup ({full_size} bytes)"
    );

    // What the update does on the machine it was built for.
    let log = fixture.case_path("update.log");
    run_silent_logged(&update_setup, Some(&fixture.destination), &log)?;
    assert_eq!(
        std::fs::read(fixture.destination.join("E2eProbe.exe"))?,
        SECOND_RELEASE_EXE,
        "the update did not deploy the new product exe"
    );
    assert!(
        fixture.destination.join("data/added.bin").is_file(),
        "the update did not deploy the file this release adds"
    );
    assert!(
        !fixture.destination.join("data/legacy.bin").exists(),
        "the update left the file this release drops behind"
    );
    // The files the update does not carry are the ones the first install wrote,
    // byte for byte and untouched.
    assert_eq!(std::fs::read(&kept)?, (0u8..=255).collect::<Vec<u8>>());
    assert_eq!(std::fs::read(&installed_runtime)?, runtime);
    let written = std::fs::read_to_string(&log)?;
    assert!(
        written.contains("update package: 2 file(s) verified in place, 2 deployed"),
        "the log does not report what the update did:\n{written}"
    );

    // What the update kept is part of the installation, so an uninstall takes it
    // back along with the files it wrote.
    fixture.uninstall()?;
    assert!(
        !kept.exists() && !installed_runtime.exists(),
        "the uninstall left a file the update had kept behind"
    );
    Ok(())
}

/// An update package installs over one release, and says so where the files it
/// expects are missing or hold something else.
#[test]
fn an_update_package_refuses_a_machine_it_does_not_fit() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let previous = fixture.keep_payload("app-v1.archive")?;
    fixture.build()?;

    fixture.archive_with(
        PayloadFormat::Zip,
        true,
        &[("E2eProbe.exe", SECOND_RELEASE_EXE)],
    )?;
    let update_setup = fixture.case_path("E2eProbe_Update.exe");
    fixture.build_update(&previous, &update_setup)?;

    // A machine with nothing installed: an update replaces a release, and there
    // is nothing here for it to replace.
    let empty = fixture.case_path("empty-machine");
    let refused = run_expecting_failure(&update_setup, Some(&empty))?;
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("full setup"),
        "the refusal should say which setup to run instead: {stderr}"
    );
    assert!(
        !empty.exists(),
        "a refused update created {}",
        empty.display()
    );

    // A machine holding another release of the same product: the file the plan
    // expects is there, and it is not the file the plan was built from.
    fixture.install()?;
    std::fs::write(
        fixture.destination.join("data/expected.bin"),
        b"someone else's bytes",
    )?;
    let refused = run_expecting_failure(&update_setup, Some(&fixture.destination))?;
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("data/expected.bin") && stderr.contains("full"),
        "the refusal should name the file that does not fit and the way out: {stderr}"
    );
    assert_eq!(
        std::fs::read(fixture.destination.join("E2eProbe.exe"))?,
        b"MZ end-to-end probe executable\r\n",
        "a refused update wrote part of the next release anyway"
    );
    Ok(())
}

/// An update package is built from a project whose content is one payload.
///
/// A project that cuts its content into components has no one archive to compare
/// against, so it is told to ship the full setup rather than given an update
/// package that would install part of a product.
#[test]
fn a_component_project_is_refused_an_update_package() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let previous = fixture.keep_payload("app-v1.archive")?;
    fixture.components_project()?;
    for (name, entry) in [
        ("core.archive", ("core/runtime.txt", b"core".as_slice())),
        ("docs.archive", ("docs/readme.txt", b"docs".as_slice())),
        ("tools.archive", ("tools/tool.txt", b"tools".as_slice())),
        (
            "samples.archive",
            ("samples/sample.txt", b"samples".as_slice()),
        ),
    ] {
        fixture.archive_component(name, PayloadFormat::Zip, &[entry])?;
    }

    let update_setup = fixture.case_path("E2eProbe_Update.exe");
    let error = fixture
        .build_update(&previous, &update_setup)
        .expect_err("a project cut into components was given an update package");
    let message = format!("{error:#}");
    assert!(
        message.contains("components") && message.contains("full setup"),
        "the refusal should name the reason and the way out: {message}"
    );
    assert!(
        !update_setup.exists(),
        "a refused build left a setup behind: {}",
        update_setup.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Uninstall
// ---------------------------------------------------------------------------

/// Uninstall removes what it deployed, removes the registration, and removes the
/// installation directory itself.
#[test]
fn uninstalling_removes_the_product_the_registration_and_the_directory() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;
    fixture.install()?;
    assert!(
        fixture.read_uninstall_entry()?.is_some(),
        "nothing was registered to uninstall"
    );

    fixture.uninstall()?;
    // The directory goes once the cleaner copy exits, a moment after the
    // uninstaller itself returns.
    wait_for_removal(&fixture.destination);

    assert!(
        !fixture.destination.exists(),
        "the installation directory survived uninstall: {}",
        fixture.destination.display()
    );
    assert!(
        fixture.read_uninstall_entry()?.is_none(),
        "the uninstall registration survived uninstall"
    );
    Ok(())
}

/// A file the user put in the installation directory keeps the directory, which
/// is the promise the manifest cleanup makes, while the deployed files still go.
#[test]
fn uninstall_keeps_a_directory_that_still_holds_user_files() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;
    fixture.install()?;

    let keep = fixture.destination.join("user-notes.txt");
    std::fs::write(&keep, b"a file the user added")?;

    fixture.uninstall()?;
    wait_for_uninstaller_removal(&fixture.destination);

    assert!(keep.is_file(), "the user's own file was deleted");
    assert!(
        !fixture.destination.join("E2eProbe.exe").exists(),
        "the deployed product was not removed"
    );
    assert!(
        fixture.destination.exists(),
        "the directory holding a user file was removed anyway"
    );
    Ok(())
}

/// Uninstalling a project that never declared silent support is refused, so a
/// product cannot be removed unattended by a flag its author did not agree to.
#[test]
fn a_project_without_silent_support_refuses_a_windowless_uninstall() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;
    fixture.install()?;

    // Rewrite only the uninstall switch, leaving the install one on.
    let path = fixture.project.join("installer_config.json");
    let mut config: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    config["advanced"]["uninstall_mode_support"] = serde_json::Value::from(false);
    std::fs::write(&path, serde_json::to_vec_pretty(&config)?)?;
    fixture.build()?;
    // The deployed uninstaller still carries the old project, so install again
    // to deploy one built from the edited project.
    fixture.install()?;

    let output = Command::new(fixture.destination.join("uninst.exe"))
        .arg("--silent")
        .output()?;
    assert!(
        !output.status.success(),
        "a project without uninstall support was removed anyway"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("uninstall_mode_support"),
        "the failure should name the switch to set"
    );
    assert!(
        fixture.destination.join("E2eProbe.exe").is_file(),
        "the product was removed despite the refusal"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Project scripts
// ---------------------------------------------------------------------------

/// A project that ships its own steps has them carried into the setup and run
/// by the real runtime.
///
/// The in-process script tests drive the driver directly. What sits between the
/// project folder and that driver is packaging `scripts/` into the bundle and
/// finding it again from the stub, and a setup that loses either still passes
/// every one of them.
#[test]
fn a_setup_runs_the_projects_own_install_and_uninstall_scripts() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.write_script(
        "install.rhai",
        r#"
            let install_path = get_install_path();
            if !extract_payload_with_progress(0.0, 60.0) {
                return;
            }
            copy_uninstaller();
            write_file(path_join(install_path, "script-note.txt"), "written by the project script");
            reg_write_string(get_config_value("test.registry_key"), "InstallPath", install_path);
            set_status_key("status.installing_uninstaller");
            set_progress(80.0);
        "#,
    )?;
    fixture.write_script("uninstall.rhai", "run_tracked_uninstall(10.0, 90.0);")?;
    fixture.build()?;
    fixture.install()?;

    // The script's own file, the payload it extracted, and its registry value.
    assert_eq!(
        std::fs::read_to_string(fixture.destination.join("script-note.txt"))?,
        "written by the project script",
        "the project's install script never ran"
    );
    assert!(
        fixture.destination.join("data/expected.bin").is_file(),
        "extract_payload inside the script deployed nothing"
    );
    assert_eq!(
        read_registry_string(&fixture.test_key, "InstallPath")?,
        Some(fixture.destination.display().to_string()),
        "the value the script wrote is not in the registry"
    );

    fixture.uninstall()?;
    wait_for_removal(&fixture.destination);
    assert!(
        read_registry_string(&fixture.test_key, "InstallPath")?.is_none(),
        "uninstall left the value the script wrote behind"
    );
    Ok(())
}

/// A setup's script stores every type the machine keeps and reads back what it
/// stored.
///
/// The in-process cases drive the same primitives. What sits between them and a
/// real setup is packaging: the script travels through the bundle and runs inside
/// the stub, and the type it wrote is read here through `reg query`, which is the
/// machine's own reading rather than the primitive's.
#[test]
fn a_setup_stores_every_registry_type_its_script_names() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.write_script(
        "install.rhai",
        r#"
            let install_path = get_install_path();
            if !extract_payload_with_progress(0.0, 50.0) {
                return;
            }
            copy_uninstaller();
            let key = get_config_value("test.registry_key");
            let report = "";
            report += "string=" + reg_write_string(key, "Text", "plain").to_string() + "\n";
            report += "expand=" + reg_write_expand_string(key, "Home", "%TEMP%\\nano-installer").to_string() + "\n";
            report += "multi=" + reg_write_multi_string(key, "List", ["alpha", "beta"]).to_string() + "\n";
            report += "dword=" + reg_write_dword(key, "Count", 12).to_string() + "\n";
            report += "qword=" + reg_write_qword(key, "Big", 4294967297).to_string() + "\n";
            report += "binary=" + reg_write_binary(key, "Blob", [222, 173, 190, 239]).to_string() + "\n";
            report += "read_text=" + reg_read(key, "Text") + "\n";
            report += "read_home=" + reg_read_expand_string(key, "Home") + "\n";
            report += "read_list=" + (reg_read_multi_string(key, "List") == ["alpha", "beta"]).to_string() + "\n";
            report += "read_count=" + reg_read_dword(key, "Count").to_string() + "\n";
            report += "read_big=" + reg_read_qword(key, "Big").to_string() + "\n";
            report += "read_blob=" + (reg_read_binary(key, "Blob") == [222, 173, 190, 239]).to_string() + "\n";
            report += "type_list=" + reg_read_type(key, "List") + "\n";
            // The case's own key reached through the name of a view: what a project
            // writes below `HKCU64\` is what that name reads back and takes away.
            let viewed = "HKCU64\\" + key.sub_string(5) + "\\viewed";
            report += "view_written=" + reg_write_string(viewed, "View", "64").to_string() + "\n";
            report += "view_read=" + reg_read(viewed, "View") + "\n";
            report += "view_deleted=" + reg_delete_key(viewed).to_string() + "\n";
            report += "view_gone=" + reg_key_exists(viewed).to_string() + "\n";
            // Where the machine keeps a branch in one view only, the two names part
            // company: the branch a 32-bit product left is in the copy a 32-bit
            // program reads and in no other.
            report += "marker32=" + reg_key_exists("HKLM32\\SOFTWARE\\Microsoft\\EdgeUpdate").to_string() + "\n";
            report += "marker64=" + reg_key_exists("HKLM64\\SOFTWARE\\Microsoft\\EdgeUpdate").to_string() + "\n";
            write_file(path_join(install_path, "registry-report.txt"), report);
        "#,
    )?;
    fixture.build()?;
    fixture.install()?;

    let report = std::fs::read_to_string(fixture.destination.join("registry-report.txt"))?;
    let value = |name: &str| -> String {
        report
            .lines()
            .find_map(|line| {
                line.split_once('=')
                    .filter(|(field, _)| *field == name)
                    .map(|(_, value)| value)
            })
            .unwrap_or_default()
            .to_string()
    };
    for name in ["string", "expand", "multi", "dword", "qword", "binary"] {
        assert_eq!(value(name), "true", "{name} was not written");
    }
    assert_eq!(value("read_text"), "plain");
    let expanded = value("read_home");
    assert!(
        !expanded.contains('%'),
        "the reference was handed back unexpanded: {expanded}"
    );
    assert_eq!(
        std::path::PathBuf::from(&expanded),
        std::env::temp_dir().join("nano-installer"),
        "`%TEMP%` was not expanded on the way out of the registry"
    );
    assert_eq!(value("read_list"), "true");
    assert_eq!(value("read_count"), "12");
    assert_eq!(value("read_big"), "4294967297");
    assert_eq!(value("read_blob"), "true");
    assert_eq!(value("type_list"), "REG_MULTI_SZ");
    assert_eq!(value("view_written"), "true");
    assert_eq!(value("view_read"), "64");
    assert_eq!(value("view_deleted"), "true");
    assert_eq!(value("view_gone"), "false");
    // The machine's own answer about the branch, taken with the view as a flag,
    // decides what the two names had to report. A machine that keeps the branch
    // in both views, or a 32-bit Windows, has one copy and nothing to tell apart.
    let marker = r"HKLM\SOFTWARE\Microsoft\EdgeUpdate";
    let thirty_two = registry_key_exists_in_view(marker, 32);
    let sixty_four = registry_key_exists_in_view(marker, 64);
    if thirty_two != sixty_four {
        assert_eq!(value("marker32"), thirty_two.to_string());
        assert_eq!(value("marker64"), sixty_four.to_string());
    }

    // What the machine holds, rather than only what the script read back.
    assert_eq!(
        read_registry_value(&fixture.test_key, "Text")?,
        Some(("REG_SZ".to_string(), "plain".to_string()))
    );
    assert_eq!(
        read_registry_value(&fixture.test_key, "Home")?,
        Some((
            "REG_EXPAND_SZ".to_string(),
            r"%TEMP%\nano-installer".to_string()
        )),
        "an expandable value is stored as it was written, references and all"
    );
    assert_eq!(
        read_registry_value(&fixture.test_key, "List")?,
        Some(("REG_MULTI_SZ".to_string(), r"alpha\0beta".to_string()))
    );
    assert_eq!(
        read_registry_value(&fixture.test_key, "Count")?,
        Some(("REG_DWORD".to_string(), "0xc".to_string()))
    );
    assert_eq!(
        read_registry_value(&fixture.test_key, "Big")?,
        Some(("REG_QWORD".to_string(), "0x100000001".to_string()))
    );
    assert_eq!(
        read_registry_value(&fixture.test_key, "Blob")?,
        Some(("REG_BINARY".to_string(), "DEADBEEF".to_string()))
    );

    // Every one of them is in the manifest, so removing the product takes them
    // away without the project saying anything about them.
    fixture.uninstall()?;
    wait_for_removal(&fixture.destination);
    for name in ["Text", "Home", "List", "Count", "Big", "Blob"] {
        assert!(
            read_registry_value(&fixture.test_key, name)?.is_none(),
            "the uninstall left {name} behind"
        );
    }
    Ok(())
}

/// A setup's script reads what a program it ran wrote.
///
/// The words a tool prints are the only answer a project can get from one, and
/// the exit code alone does not say which of two failures it was.
#[test]
fn a_setup_reads_what_a_command_its_script_ran_wrote() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.write_script(
        "install.rhai",
        r#"
            let install_path = get_install_path();
            if !extract_payload_with_progress(0.0, 50.0) {
                return;
            }
            copy_uninstaller();
            let ran = run_command_output(get_env("ComSpec"), ["/C", "echo captured& echo failed 1>&2& exit 5"]);
            let report = "";
            report += "code=" + ran.code.to_string() + "\n";
            report += "stdout=" + ran.stdout + "|";
            report += "stderr=" + ran.stderr + "|";
            write_file(path_join(install_path, "command-report.txt"), report);
        "#,
    )?;
    fixture.build()?;
    fixture.install()?;

    assert_eq!(
        std::fs::read_to_string(fixture.destination.join("command-report.txt"))?,
        "code=5\nstdout=captured\r\n|stderr=failed \r\n|",
        "the setup did not read what the command it ran wrote"
    );
    Ok(())
}

/// A project script installs a service, and the uninstall takes it away again.
///
/// A service is not a file of the installation but an entry the machine keeps
/// pointing at one, so this is the seam between a script and the service control
/// manager: the name the setup installed is the one the machine answers for, and
/// the name the manifest recorded is the one the uninstaller deletes. Installing
/// a service needs an elevated process; where the run has none, the Windows
/// refusal has to come back as a failed call rather than as a service nobody
/// asked for. Either way the case holds the same thing -- the machine keeps the
/// service exactly when the script reported installing one, and nothing of it is
/// left after the uninstall -- and both are asked of the machine directly rather
/// than of the script's own answer.
///
/// That the service then runs is not held: the program a service runs is the
/// product's own, and no test can ship one.
#[test]
fn a_setup_installs_a_service_the_uninstall_takes_away() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let service = format!("nano-installer-e2e-service-{}", fixture.id);
    fixture.edit_config(|config| {
        config["test"]["service_name"] = serde_json::json!(&service);
    })?;
    fixture.write_script(
        "install.rhai",
        r#"
            let install_path = get_install_path();
            if !extract_payload_with_progress(0.0, 60.0) {
                return;
            }
            copy_uninstaller();
            let name = get_config_value("test.service_name");
            let installed = service_install(name, "nano-installer e2e service", "E2eProbe.exe", "--serve");
            let report = "";
            report += "installed=" + installed.to_string() + "\n";
            report += "exists=" + service_exists(name).to_string() + "\n";
            report += "running=" + service_running(name).to_string() + "\n";
            write_file(path_join(install_path, "service-report.txt"), report);
        "#,
    )?;
    fixture.build()?;
    fixture.install()?;

    let report = std::fs::read_to_string(fixture.destination.join("service-report.txt"))?;
    let observed = |key: &str| -> Option<String> {
        report
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")).map(str::to_string))
    };
    let installed = observed("installed").as_deref() == Some("true");
    // What the script's own two calls said, and what the machine says when
    // asked from outside the setup, all have to agree.
    assert_eq!(
        observed("exists").as_deref(),
        Some(if installed { "true" } else { "false" }),
        "the script's install and its lookup disagree: {report}"
    );
    assert_eq!(
        service_exists(&service),
        installed,
        "the machine does not hold the service the setup reported: {report}"
    );
    // Installing a service does not start it: the script never asked it to run.
    assert_eq!(
        observed("running").as_deref(),
        Some("false"),
        "the service started on its own: {report}"
    );

    fixture.uninstall()?;
    assert!(
        !service_exists(&service),
        "the service the manifest recorded outlived the uninstall"
    );
    Ok(())
}

/// Whether the machine keeps a service of this name.
///
/// Asked of the service control manager through the tool Windows ships for it,
/// so the answer comes from the machine rather than from the setup that
/// installed the service. A name nothing owns is answered with an error, which
/// is what makes this a yes or no rather than "it depends on the wording".
fn service_exists(name: &str) -> bool {
    Command::new("sc.exe")
        .args(["query", name])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// A project that bundles helper programs has them carried into the setup and
/// unpacked where its script can run them.
///
/// The in-process cases prove the build bundles the directory and that the
/// primitive unpacks it. What sits between them is a real setup: the tools have
/// to survive packaging and reach the script through the bundle, because the
/// project folder is not beside the installation when it runs.
#[test]
fn a_setup_unpacks_the_tools_its_project_bundles() -> anyhow::Result<()> {
    // A batch file is a program Windows runs through the command shell every
    // machine has, so the case needs no second executable of its own.
    const TOOL: &[u8] = b"@echo off\r\necho tool ran\r\nexit /b 7\r\n";
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    std::fs::create_dir_all(fixture.project.join("tools/bin"))?;
    std::fs::write(fixture.project.join("tools/bin/hello.cmd"), TOOL)?;
    fixture.edit_config(|config| {
        config["resources"]["tools_dir"] = serde_json::json!("tools");
    })?;
    fixture.write_script(
        "install.rhai",
        r#"
            let install_path = get_install_path();
            if !extract_payload_with_progress(0.0, 60.0) {
                return;
            }
            copy_uninstaller();
            let tool = path_join(path_join(get_tools_dir(), "bin"), "hello.cmd");
            copy_file(tool, path_join(install_path, "bundled-tool.cmd"));
            let code = run_command(get_env("ComSpec"), ["/C", tool]);
            write_file(path_join(install_path, "tool-exit.txt"), code.to_string());
        "#,
    )?;
    fixture.build()?;
    fixture.install()?;

    // Byte for byte the file the project shipped: a tool the setup never
    // carried, or carried under another name, cannot match this.
    assert_eq!(
        std::fs::read(fixture.destination.join("bundled-tool.cmd"))?,
        TOOL,
        "the tool get_tools_dir returned is not the one the project bundled"
    );
    // And it runs: the exit code is what a script checks before it leans on a
    // helper it shipped.
    assert_eq!(
        std::fs::read_to_string(fixture.destination.join("tool-exit.txt"))?,
        "7",
        "the tool the setup unpacked did not run"
    );
    Ok(())
}

/// A project script fetches a file of its own, checks what arrived against the
/// digest it names, and reads the digest of a file it holds.
///
/// The built-in dependency flow and a script go through the same fetch, and the
/// dependency cases prove the flow. This is the half a script reaches for when
/// the project decides for itself when to fetch what it does not ship, and it
/// runs against a real server answering on the loopback interface.
#[test]
fn a_script_downloads_a_file_and_checks_what_arrived() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    // Real bytes from a real server: the archiver the fixtures already write
    // payloads with, answered over HTTP.
    let program = workspace_root().join("tools/7za.exe");
    anyhow::ensure!(program.is_file(), "tools/7za.exe is missing");
    let digest = sha256_of(&program)?;
    let server = LocalServer::serve(&program)?;
    fixture.write_script(
        "install.rhai",
        &format!(
            r#"
            let install_path = get_install_path();
            if !extract_payload_with_progress(0.0, 60.0) {{
                return;
            }}
            copy_uninstaller();
            let fetched = path_join(install_path, "fetched.exe");
            let arrived = download_file_with_hash("{0}", fetched, "{1}");
            write_file(path_join(install_path, "report.txt"), arrived.to_string() + "\n" + sha256_of_file(fetched));
            "#,
            server.url, digest
        ),
    )?;
    fixture.build()?;
    fixture.install()?;

    // Byte for byte what the server answered with, and the digest the script
    // read back is the one the project recorded.
    assert_eq!(
        std::fs::read(fixture.destination.join("fetched.exe"))?,
        std::fs::read(&program)?,
        "the file the script downloaded is not the one the server answered with"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.destination.join("report.txt"))?,
        format!("true\n{digest}"),
        "the script's download, or its own hash of the result, did not come out right"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Shortcuts and autostart
// ---------------------------------------------------------------------------

/// A windowless install has no checkboxes to read, so it follows the project's
/// defaults for shortcuts and autostart, and the manifest it writes names every
/// file and value it created outside the installation directory.
///
/// These entries are what the user meets before ever launching the product, and
/// they are the only things an install writes outside its own directory. A
/// setup that skips them still installs correctly and leaves a Start menu entry
/// that nothing will ever take back.
#[test]
fn a_silent_install_writes_the_shortcuts_and_the_autostart_entry() -> anyhow::Result<()> {
    let Some(mut fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    // Every name here is unique to this case: the files land on the real
    // desktop and in the real Start menu, and the value lands in the Run key the
    // rest of the machine shares, so a name another run also used would either
    // fail this case or leave something behind.
    let name = format!("E2eProbe-{}", fixture.id);
    let folder = name.clone();
    fixture.edit_config(|config| {
        config["project"]["name"] = serde_json::Value::from(name.clone());
        config["shortcuts"] = serde_json::json!({
            "desktop_shortcut": true,
            "desktop_default": true,
            "start_menu": true,
            "start_menu_folder": folder,
        });
        config["autostart"] = serde_json::json!({
            "enabled": true,
            "default": true,
            "registry_key": RUN_KEY,
            "registry_value_name": name.clone(),
        });
    })?;
    fixture.remember_registry_value(RUN_KEY, &name);
    fixture.build()?;
    fixture.install()?;

    let desktop = desktop_directory()?.join(format!("{name}.lnk"));
    let programs = programs_directory()?.join(&name);
    let start_menu = programs.join(format!("{name}.lnk"));
    let uninstall_link = programs.join(format!("Uninstall {name}.lnk"));
    assert!(
        desktop.is_file(),
        "the desktop shortcut is not at {}",
        desktop.display()
    );
    assert!(
        start_menu.is_file(),
        "the Start menu shortcut is not at {}",
        start_menu.display()
    );
    assert!(
        uninstall_link.is_file(),
        "the Start menu uninstall entry is not at {}",
        uninstall_link.display()
    );

    // The manifest is what uninstall replays, so an artifact the manifest does
    // not name is an artifact nothing will take back.
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        fixture.destination.join("nano-installer-manifest.json"),
    )?)?;
    let recorded: Vec<String> = manifest["shortcuts"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    for expected in [&desktop, &start_menu, &uninstall_link] {
        let expected = expected.to_string_lossy();
        assert!(
            recorded
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(&expected)),
            "the manifest does not list {expected}: {recorded:?}"
        );
    }
    assert_eq!(
        manifest["autostart"]["value_name"].as_str(),
        Some(name.as_str()),
        "the manifest does not record the autostart value it wrote"
    );

    // Windows starts this command at sign-in, so it has to name the installed
    // executable, and quoting it is what keeps a path with a space working.
    assert_eq!(
        read_registry_string(RUN_KEY, &name)?,
        Some(format!(
            "\"{}\"",
            fixture.destination.join("E2eProbe.exe").display()
        )),
        "the autostart entry does not start the installed executable"
    );

    fixture.uninstall()?;
    wait_for_removal(&fixture.destination);
    assert!(!desktop.exists(), "the desktop shortcut survived uninstall");
    assert!(
        !programs.exists(),
        "the Start menu folder survived uninstall: {}",
        programs.display()
    );
    assert!(
        read_registry_string(RUN_KEY, &name)?.is_none(),
        "the autostart entry survived uninstall"
    );
    // The Run key holds other products' entries, so uninstall removes the value
    // it wrote and not the key around it.
    assert!(
        registry_key_exists(RUN_KEY),
        "uninstall removed the Run key the machine shares"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// A running product
// ---------------------------------------------------------------------------

/// A project can ask for running copies of its product to be closed first,
/// which is what lets an in-place upgrade run while the application is open.
#[test]
fn an_install_closes_a_running_copy_of_the_product() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.edit_config(|config| {
        config["install"]["kill_process_on_install"] = serde_json::Value::from(true);
    })?;
    fixture.build()?;

    let stand_in = tempfile::tempdir()?;
    let mut running = start_stand_in_product(&stand_in.path().join("product"))?;
    // Give it the moment it needs to reach the process list the install takes
    // its snapshot from.
    std::thread::sleep(Duration::from_millis(500));
    let alive = running.try_wait()?.is_none();

    let installed = fixture.install();
    let closed = wait_for_exit(&mut running, Duration::from_secs(30));
    let _ = running.kill();

    // A stand-in that died on its own would make "the install closed it" true
    // for the wrong reason, so that is reported before anything else.
    anyhow::ensure!(alive, "the stand-in product exited before the install ran");
    installed?;
    assert!(
        closed,
        "the install left a running copy of the product alone"
    );
    assert!(
        fixture.destination.join("E2eProbe.exe").is_file(),
        "the product was not installed after the running copy was closed"
    );
    Ok(())
}

/// Starts a process whose image name is the one the payload deploys.
///
/// Only the file name matters, because that is what the runtime matches on.
/// `ping` is copied because it is on every Windows and keeps running for as long
/// as it is asked to, so the case can tell "the install closed it" apart from
/// "it was never running".
fn start_stand_in_product(directory: &Path) -> anyhow::Result<Child> {
    let system_root = std::env::var("SystemRoot")?;
    std::fs::create_dir_all(directory)?;
    let image = directory.join("E2eProbe.exe");
    std::fs::copy(Path::new(&system_root).join("System32/ping.exe"), &image)?;
    let child = Command::new(&image)
        .arg("-n")
        .arg("120")
        .arg("127.0.0.1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child)
}

/// Waits for a process to end, which is what the install is supposed to bring
/// about.
fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// A windowless run has no boxes to read, so the project's own answer decides:
/// the required component installs, the default one installs, and the component
/// with neither stays out.
///
/// Each component carries one file, so what the run installed is readable on
/// the disk afterwards rather than only in the manifest.
#[test]
fn a_silent_run_installs_the_components_the_project_defaults_to() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.components_project()?;
    fixture.archive_component(
        "core.archive",
        PayloadFormat::Zip,
        &[("core/runtime.txt", b"core")],
    )?;
    fixture.archive_component(
        "docs.archive",
        PayloadFormat::Zip,
        &[("docs/readme.txt", b"docs")],
    )?;
    fixture.archive_component(
        "tools.archive",
        PayloadFormat::Zip,
        &[("tools/tool.txt", b"tools")],
    )?;
    fixture.archive_component(
        "samples.archive",
        PayloadFormat::Zip,
        &[("samples/sample.txt", b"samples")],
    )?;
    fixture.build()?;
    fixture.install()?;

    assert!(
        fixture.destination.join("core/runtime.txt").is_file(),
        "the required component did not install"
    );
    assert!(
        fixture.destination.join("tools/tool.txt").is_file(),
        "the component the project defaults to did not install"
    );
    assert!(
        !fixture.destination.join("docs/readme.txt").exists()
            && !fixture.destination.join("samples/sample.txt").exists(),
        "a component nobody asked for was installed"
    );
    Ok(())
}

/// Two payloads that carry one path are refused rather than unpacked over each
/// other: which of the two the user ends up with would otherwise depend on the
/// order the project happens to declare them in.
#[test]
fn two_payloads_that_carry_one_file_are_refused() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.components_project()?;
    fixture.archive_component(
        "core.archive",
        PayloadFormat::Zip,
        &[("core/runtime.txt", b"core")],
    )?;
    fixture.archive_component(
        "docs.archive",
        PayloadFormat::Zip,
        &[("docs/readme.txt", b"docs")],
    )?;
    // The same file the product payload carries: a case that would install
    // whichever archive happened to be unpacked second.
    fixture.archive_component(
        "tools.archive",
        PayloadFormat::Zip,
        &[("data/expected.bin", b"clash")],
    )?;
    fixture.archive_component(
        "samples.archive",
        PayloadFormat::Zip,
        &[("samples/sample.txt", b"samples")],
    )?;
    fixture.build()?;

    let result = fixture.install_expecting_failure()?;
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("component tools carries")
            && stderr.contains("already installed by another payload"),
        "the failure should name the component and the file both payloads carry: {stderr}"
    );
    assert!(
        !fixture.destination.exists(),
        "a refused install still created {}",
        fixture.destination.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// A payload that cannot be installed
// ---------------------------------------------------------------------------

/// `install.exe_name` names the application the payload has to contain. A setup
/// that deployed a payload without it would install a product that cannot
/// start, so the run stops before it writes anything at all.
#[test]
fn a_payload_without_the_declared_executable_is_refused() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, false) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;

    let result = fixture.install_expecting_failure()?;
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("E2eProbe.exe"),
        "the failure should name the missing executable: {stderr}"
    );
    assert!(
        !fixture.destination.exists(),
        "a refused install still created {}",
        fixture.destination.display()
    );
    Ok(())
}

/// The offset and length of one bundle entry inside a built setup.
///
/// The bundle is read here the way the runtime reads it: the footer is searched
/// for near the end of the file rather than taken to be the last thing in it,
/// because a project's own finalize command may have appended a signature
/// behind it.
fn bundle_entry(image: &Path, name: &str) -> anyhow::Result<(usize, usize)> {
    let bytes = std::fs::read(image)?;
    let window = bytes.len().min(1 << 20);
    let window_start = bytes.len() - window;
    let footer = (window_start + 8..=bytes.len() - 8)
        .rev()
        .find(|at| &bytes[*at..*at + 8] == b"NATVEND1")
        .context("the setup has no bundle footer")?;
    let size = u64::from_le_bytes(bytes[footer - 8..footer].try_into()?) as usize;
    let start = footer
        .checked_sub(size + 8)
        .context("implausible bundle size")?;
    anyhow::ensure!(
        &bytes[start..start + 8] == b"NATVRS01",
        "the setup's bundle header is wrong"
    );
    // The digest the runtime checks an entry against sits behind its length.
    let version = u16::from_le_bytes(bytes[start + 8..start + 10].try_into()?);
    anyhow::ensure!(version == 2, "unexpected bundle version {version}");
    let count = u32::from_le_bytes(bytes[start + 10..start + 14].try_into()?);
    let mut cursor = start + 14;
    for _ in 0..count {
        let name_length = u16::from_le_bytes(bytes[cursor..cursor + 2].try_into()?) as usize;
        cursor += 2;
        let entry_name = std::str::from_utf8(&bytes[cursor..cursor + name_length])?;
        cursor += name_length;
        let entry_size = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into()?) as usize;
        cursor += 8 + 32;
        if entry_name == name {
            return Ok((cursor, entry_size));
        }
        cursor += entry_size;
    }
    anyhow::bail!("the setup carries no {name} entry")
}

/// Damage inside the setup is damage to the product: a payload that no longer
/// hashes to what the build recorded is refused by name, before the machine is
/// touched at all, rather than unpacked into a half-broken installation.
#[test]
fn a_setup_whose_payload_was_damaged_in_transit_installs_nothing() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;

    // One byte of the payload flipped and nothing else: the footer, the entry
    // table, and the digest stay exactly as the build wrote them, which is what
    // a truncated download or a bad sector leaves behind.
    let at = bundle_entry(&fixture.setup, "payload/app.archive")?.0;
    let mut bytes = std::fs::read(&fixture.setup)?;
    bytes[at] ^= 0xFF;
    std::fs::write(&fixture.setup, &bytes)?;

    let result = fixture.install_expecting_failure()?;
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("payload/app.archive") && stderr.contains("is damaged"),
        "the failure should name the damaged entry: {stderr}"
    );
    assert!(
        !fixture.destination.exists(),
        "a refused install still created {}",
        fixture.destination.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// What the product needs from the machine
// ---------------------------------------------------------------------------

/// The SHA-256 of a file, computed by a program that is not the one under test.
///
/// `certutil` ships with Windows and hashes a file its own way, so the digest a
/// case hands the setup to check against is an independent answer rather than
/// the runtime's own arithmetic read back to itself.
fn sha256_of(path: &Path) -> anyhow::Result<String> {
    let output = Command::new("certutil")
        .arg("-hashfile")
        .arg(path)
        .arg("SHA256")
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "certutil failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let digest = text
        .lines()
        .nth(1)
        .unwrap_or_default()
        .trim()
        .replace(' ', "")
        .to_ascii_lowercase();
    anyhow::ensure!(
        digest.len() == 64
            && digest
                .chars()
                .all(|character| character.is_ascii_hexdigit()),
        "certutil did not report a digest: {text}"
    );
    Ok(digest)
}

/// Serves one file over the loopback for the length of one case.
///
/// A download is only a download when something answers it, so a case that
/// checks one puts a server on the other end: the bytes travel through the same
/// stack a product's own update would use, over a socket that answers the way a
/// release host does.
struct LocalServer {
    url: String,
    _thread: std::thread::JoinHandle<()>,
}

impl LocalServer {
    fn serve(path: &Path) -> anyhow::Result<Self> {
        let body = std::fs::read(path)?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let thread = std::thread::spawn(move || {
            // How many connections a download makes is the client's business,
            // so the server answers whatever arrives and then stops on its own
            // rather than holding the port for the rest of the run.
            let deadline = Instant::now() + Duration::from_secs(30);
            while Instant::now() < deadline {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                };
                // The whole request is read before the answer is written. A
                // connection closed with bytes still unread is reset rather
                // than ended, and a client that has already received the answer
                // reports that reset instead of the answer: WinHTTP calls it
                // error 12030. An accepted socket inherits the listener's
                // non-blocking mode, so a request that has not arrived yet is
                // waited for rather than mistaken for one that has ended.
                let mut request: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    match std::io::Read::read(&mut stream, &mut chunk) {
                        Ok(0) => break,
                        Ok(read) => request.extend_from_slice(&chunk[..read]),
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if Instant::now() >= deadline {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
                let mut response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes();
                response.extend_from_slice(&body);
                let _ = std::io::Write::write_all(&mut stream, &response);
                let _ = std::io::Write::flush(&mut stream);
            }
        });
        Ok(Self {
            url: format!("http://127.0.0.1:{port}/tool.exe"),
            _thread: thread,
        })
    }
}

/// What the archiver is told to do: write `archive` out of `source`.
fn archive_arguments(source: &Path, archive: &Path) -> serde_json::Value {
    serde_json::json!([
        "a",
        "-tzip",
        archive.display().to_string(),
        source.display().to_string()
    ])
}

/// A machine missing what the product needs is given it, and the product lands
/// afterwards.
///
/// The dependency's rule looks for a file its installer leaves behind, so the
/// case reads the same thing the setup read: the file is not there before the
/// run, and is after it. Nothing about the check is simulated -- a real program
/// is run with the arguments the project declares.
#[test]
fn a_silent_run_installs_the_dependency_the_machine_is_missing() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.bundle_dependency_program()?;
    let source = fixture.case_path("marker.txt");
    std::fs::write(&source, b"the dependency ran")?;
    let archive = fixture.case_path("dependency.zip");
    fixture.dependency_project(serde_json::json!({
        "id": "probe",
        "detect": { "file": archive.display().to_string() },
        "payload": "payload/dependency.exe",
        "arguments": archive_arguments(&source, &archive),
        "required": true
    }))?;
    fixture.build()?;
    assert!(
        !archive.exists(),
        "the case has to start with the machine missing the dependency"
    );

    fixture.install()?;

    assert!(
        archive.is_file(),
        "the dependency's installer never ran, so the machine was left without it"
    );
    assert!(
        fixture.destination.join("E2eProbe.exe").is_file(),
        "the product did not install after the dependency it needs did"
    );
    Ok(())
}

/// A machine that already has what the product needs is not given it again.
///
/// The rule finds the file, so the installer must not run at all: it is shipped
/// with an argument no archiver understands, and the project marks the
/// dependency required, so a run that decided to install it anyway would stop
/// with the installer's own failure instead of passing quietly.
#[test]
fn a_dependency_the_machine_already_has_is_not_installed_again() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.bundle_dependency_program()?;
    let archive = fixture.case_path("dependency.zip");
    std::fs::write(&archive, b"already installed")?;
    fixture.dependency_project(serde_json::json!({
        "id": "probe",
        "detect": { "file": archive.display().to_string() },
        "payload": "payload/dependency.exe",
        "arguments": ["__not_a_command__"],
        "required": true
    }))?;
    fixture.build()?;

    fixture.install()?;

    assert_eq!(
        std::fs::read(&archive)?,
        b"already installed",
        "the dependency was installed over what the machine already had"
    );
    Ok(())
}

/// A dependency that cannot be installed stops the run before the product is
/// written.
///
/// This is also what says the check happens before the payload: the destination
/// the payload would have been unpacked into does not exist afterwards, so a
/// machine that cannot be given what the product needs is told so instead of
/// being left with a product that will not start.
#[test]
fn a_dependency_that_cannot_be_installed_stops_the_install() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.bundle_dependency_program()?;
    let source = fixture.case_path("marker.txt");
    std::fs::write(&source, b"the dependency ran")?;
    let archive = fixture.case_path("dependency.zip");
    fixture.dependency_project(serde_json::json!({
        "id": "probe",
        "detect": { "file": archive.display().to_string() },
        "payload": "payload/dependency.exe",
        "arguments": ["__not_a_command__"],
        "required": true
    }))?;
    fixture.build()?;

    let result = fixture.install_expecting_failure()?;
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("dependency probe failed with exit code 7"),
        "the failure should name the dependency and what its installer returned: {stderr}"
    );
    assert!(
        !fixture.destination.exists(),
        "a refused install still created {}",
        fixture.destination.display()
    );
    Ok(())
}

/// A dependency the project cannot ship is fetched, checked, and only then run.
#[test]
fn a_downloaded_dependency_is_checked_before_it_runs() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let program = workspace_root().join("tools/7za.exe");
    anyhow::ensure!(program.is_file(), "tools/7za.exe is missing");
    let digest = sha256_of(&program)?;
    let server = LocalServer::serve(&program)?;
    let source = fixture.case_path("marker.txt");
    std::fs::write(&source, b"the downloaded dependency ran")?;
    let archive = fixture.case_path("downloaded.zip");
    fixture.dependency_project(serde_json::json!({
        "id": "probe",
        "detect": { "file": archive.display().to_string() },
        "download": { "url": server.url.clone(), "sha256": digest },
        "arguments": archive_arguments(&source, &archive),
        "required": true
    }))?;
    fixture.build()?;

    fixture.install()?;

    assert!(
        archive.is_file(),
        "the downloaded program never ran, so the machine was left without it"
    );
    Ok(())
}

/// A download that is not the file the project recorded never runs.
///
/// The URL answers with a real executable and the digest is the right shape but
/// not the file's, which is what a tampered or truncated release looks like
/// from the machine installing it.
#[test]
fn a_download_that_is_not_the_recorded_file_is_refused() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    let program = workspace_root().join("tools/7za.exe");
    anyhow::ensure!(program.is_file(), "tools/7za.exe is missing");
    let server = LocalServer::serve(&program)?;
    let source = fixture.case_path("marker.txt");
    std::fs::write(&source, b"the downloaded dependency ran")?;
    let archive = fixture.case_path("downloaded.zip");
    // A digest of the right shape and not the file's, so what fails is the
    // check rather than the way the check was written down.
    let wrong = "0".repeat(64);
    fixture.dependency_project(serde_json::json!({
        "id": "probe",
        "detect": { "file": archive.display().to_string() },
        "download": { "url": server.url.clone(), "sha256": wrong },
        "arguments": archive_arguments(&source, &archive),
        "required": true
    }))?;
    fixture.build()?;

    let result = fixture.install_expecting_failure()?;
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("arrived as sha256") && stderr.contains("but the project expects"),
        "the failure should say what arrived and what was expected: {stderr}"
    );
    assert!(
        !archive.exists(),
        "a download that failed its check was run anyway"
    );
    assert!(
        !fixture.destination.exists(),
        "a refused install still created {}",
        fixture.destination.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// The window
// ---------------------------------------------------------------------------

/// The class name the runtime registers its window under.
const RUNTIME_WINDOW_CLASS: &str = "NanoInstallerNativeRuntime";

/// Opens the real setup without `--silent` and waits for its wizard window.
///
/// This is the one seam no windowless case can reach. A layout that fails to
/// load, a bundle that lost its assets, or a window class that was never
/// registered still leaves a silent install working and every check that only
/// reads files passing, so the window itself has to be looked at once. Nothing
/// is clicked: the setup is closed the moment it has drawn, and the page it drew
/// is what the client area is measured against.
///
/// Opening a window needs an interactive desktop session. A process that has
/// none, which is how a build agent started as a service runs, cannot, so the
/// case skips there and says why. On a machine that is meant to have a desktop,
/// `NANO_INSTALLER_E2E_REQUIRE_DESKTOP` turns that skip into a failure, because
/// a check that never ran must not pass for one that did.
#[test]
fn the_setup_opens_its_wizard_window() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::SevenZip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.build()?;

    // The client area is read in real pixels, so the run and the reader are
    // pinned to the same scale: on a scaled display a window measured through
    // the scaling answers with the display rather than with the layout.
    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;

    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));

    // The window is measured while its process is still alive, because closing
    // the process closes the window and takes the handle with it.
    let measured = match &waited {
        WindowWait::Found(window) => Some(client_size(*window)),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let _ = setup.kill();
    let _ = setup.wait();

    let Some((width, height)) = measured else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            // Measured above, so a window that was found cannot arrive here.
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        return skip_missing_desktop(&reason);
    };

    assert_eq!(
        (width, height),
        (720, 450),
        "the wizard window measures {width}x{height}, not the 720x450 client area its project declares"
    );
    assert!(
        !fixture.destination.exists(),
        "looking at the wizard installed something"
    );
    Ok(())
}

/// Walks the wizard with the two page actions a project can put on a button.
///
/// A page list belongs to the project, so the only way to know that `next` and
/// `back` really move between the pages it declares -- and that a click lands
/// on the page the layout put the button on -- is to click one in a real
/// window. The two pages declare different client areas, so the size of the
/// window says which page is up.
#[test]
fn a_next_button_walks_to_the_page_the_project_declares() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::SevenZip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.walk_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;

    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let first = client_size(window);
    // The centre of the button the first page places at 560,390.
    click_client_point(window, 620, 408);
    let second = wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(10));
    let walked_back = match second {
        Some(_) => {
            // The centre of the button the second page places at 20,240.
            click_client_point(window, 80, 258);
            wait_for_client_size(window, first, Instant::now() + Duration::from_secs(10))
        }
        None => None,
    };
    let _ = setup.kill();
    let _ = setup.wait();

    assert_eq!(
        first,
        (720, 450),
        "the wizard opened on {first:?} rather than the 720x450 page the project declares"
    );
    assert_eq!(
        second,
        Some((500, 300)),
        "the next button did not move the wizard to the second page"
    );
    assert_eq!(
        walked_back,
        Some(first),
        "the back button did not move the wizard back to the first page"
    );
    Ok(())
}

/// A project's page hook decides the page the wizard goes to, and the way back
/// is the way the user came rather than the order the project declares.
///
/// The licence page sits between the two the hook connects, and every page
/// declares its own client area, so the window's own size says both things: the
/// hook sent the wizard past the licence page, and the back button returned to
/// the page it came from rather than to the page the hook skipped.
#[test]
fn a_page_hook_sends_the_wizard_past_a_page_the_project_skips() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.hook_project(
        r#"
        fn next_page(from) {
            if from == "welcome" { "options" } else { "" }
        }
        "#,
    )?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;

    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let welcome = client_size(window);
    // The centre of the button the welcome page places at 560,390.
    click_client_point(window, 620, 408);
    let sent = wait_for_client_size(window, (600, 400), Instant::now() + Duration::from_secs(10));
    // The centre of the button the options page places at 20,20.
    click_client_point(window, 80, 38);
    let returned = wait_for_client_size(window, welcome, Instant::now() + Duration::from_secs(10));

    let _ = setup.kill();
    let _ = setup.wait();

    assert_eq!(
        welcome,
        (720, 450),
        "the wizard opened on {welcome:?} rather than the 720x450 welcome page"
    );
    assert_eq!(
        sent,
        Some((600, 400)),
        "the page hook did not send the wizard to the options page the hook named; \
         the 640x420 licence page it skipped is what the declared order shows"
    );
    assert_eq!(
        returned,
        Some(welcome),
        "the back button did not return to the page the user came from"
    );
    Ok(())
}

/// A page hook that fails is reported in the wizard, and the order the project
/// declares still moves the person on rather than leaving them trapped.
///
/// The card the runtime reports it in is drawn from the project's own dialog
/// layout, and this one closes the wizard when it is clicked: the process only
/// ends if that card was on screen, which is what separates "the failure was
/// reported" from "the wizard quietly went somewhere else".
#[test]
fn a_page_hook_that_fails_is_reported_and_the_wizard_walks_on() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.hook_project(r#"fn next_page(from) { throw "no idea where to go"; }"#)?;
    std::fs::write(
        fixture.project.join("layouts/msgBox.xml"),
        r##"<Page width="400" height="180" background="#FF2A3844">
  <Button id="btnOK" action="close" text="OK"
          position="absolute" left="40" top="120" width="140" height="36" />
</Page>"##,
    )?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;

    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    // The centre of the button the welcome page places at 560,390.
    click_client_point(window, 620, 408);
    let walked = wait_for_client_size(window, (640, 420), Instant::now() + Duration::from_secs(10));
    // The runtime centres a 400x180 card on the page it is drawn over, which
    // puts that card's button at the centre of 160,240 to 300,276 on the
    // 640x420 page the declared order moved to.
    click_client_point(window, 230, 258);
    let closed = wait_for_exit(&mut setup, Duration::from_secs(10));

    assert_eq!(
        walked,
        Some((640, 420)),
        "the failed page hook left the wizard where it was instead of walking on"
    );
    assert!(
        closed,
        "no card was drawn to report the failed page hook: the click landed on the page"
    );
    Ok(())
}

/// A cancel button stops the task the wizard is running.
///
/// The click is the only way in: the flag the task reads is reachable from the
/// window's own thread and nowhere else, and what a stopped task leaves behind
/// is what this checks -- the wizard returns to the page the task started from
/// rather than showing its completion page, and the directory the script had
/// already created is gone.
#[test]
fn a_cancel_button_stops_the_project_script_and_leaves_nothing_installed() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.cancellable_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;

    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let opened = client_size(window);
    // The centre of the install button the first page places at 560,390.
    click_client_point(window, 620, 408);
    let running =
        wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(20));
    // The centre of the cancel button the progress page places at 20,240.
    click_client_point(window, 80, 258);
    let stopped = wait_for_client_size(window, opened, Instant::now() + Duration::from_secs(20));
    let _ = setup.kill();
    let _ = setup.wait();

    assert_eq!(
        opened,
        (720, 450),
        "the wizard opened on {opened:?} rather than the 720x450 page the project declares"
    );
    assert_eq!(
        running,
        Some((500, 300)),
        "the install button did not start the task on its progress page"
    );
    assert_eq!(
        stopped,
        Some(opened),
        "the cancel button did not stop the task and return to the page it started from"
    );
    assert!(
        !fixture.destination.exists(),
        "the cancelled install left the directory its script created behind"
    );
    Ok(())
}

/// The click is the only way in here too: the install button's own condition
/// names the field, so what this checks is the whole path a project asks a user
/// to walk -- an empty field leaves the button inert, the agreement box alone
/// does not hand it over, and typing an acceptable value does, after which the
/// install runs into the directory that was typed.
#[test]
fn a_field_the_user_fills_in_is_what_lets_the_install_start() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.validated_project()?;
    fixture.build()?;

    let typed = fixture.destination.with_file_name("typed");
    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;

    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let opened = client_size(window);
    // The centre of the install button the page places at 20,130, while the
    // field is empty: the click has nothing to land on.
    click_client_point(window, 90, 148);
    let early = wait_for_client_size(
        window,
        (500, 300),
        Instant::now() + Duration::from_millis(1500),
    );
    // The agreement box alone is not enough either.
    click_client_point(window, 60, 100);
    click_client_point(window, 90, 148);
    let agreed = wait_for_client_size(
        window,
        (500, 300),
        Instant::now() + Duration::from_millis(1500),
    );
    // Typing the directory into the field is what hands the button its click.
    let destination = typed.to_string_lossy().to_string();
    press_client_point(window, 100, 53);
    type_client_text(window, &destination);
    click_client_point(window, 90, 148);
    let started =
        wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(20));
    let installed = wait_for_directory(&typed, Instant::now() + Duration::from_secs(30));
    // What the field held and what the page looked like, read while the window is
    // still up: a case that fails here has to say whether every typed character
    // arrived, because that is what the install used as its destination.
    let held = accessible_field_value(window);
    let described = describe_page(window);
    let _ = setup.kill();
    let _ = setup.wait();

    assert_eq!(
        opened,
        (720, 450),
        "the wizard opened on {opened:?} rather than the 720x450 page the project declares"
    );
    assert_eq!(
        early, None,
        "the install started while the field the project asks for was still empty"
    );
    assert_eq!(
        agreed, None,
        "the agreement box alone let the install start with the field empty"
    );
    assert_eq!(
        started,
        Some((500, 300)),
        "the field the user filled in did not let the install start"
    );
    assert!(
        installed && typed.join("E2eProbe.exe").is_file(),
        "the install did not write the product into {destination:?}; the field held {held:?} and the page read as {described:#?}"
    );
    Ok(())
}

/// The row a click lands on is what the button beside it waits for.
///
/// The layout cases prove a choice turns into a click region and back into a
/// page; this one proves the window itself records the row that was clicked,
/// in a real window, by watching the install start on the value the click
/// handed over and the product land in the configured directory.
#[test]
fn a_click_on_a_radio_is_the_value_the_install_waits_for() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.choice_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let Some(window) = (match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    }) else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let opened = client_size(window);
    // The install button, with the row the layout defaults to still on: the
    // group holds a value, and it is not the one the button names.
    click_client_point(window, 90, 148);
    let defaulted = wait_for_client_size(
        window,
        (500, 300),
        Instant::now() + Duration::from_millis(1500),
    );
    // Clicking that row again changes nothing: a radio keeps the value it has.
    click_client_point(window, 60, 50);
    click_client_point(window, 90, 148);
    let same = wait_for_client_size(
        window,
        (500, 300),
        Instant::now() + Duration::from_millis(1500),
    );
    // The other row is the value the condition names, and picking it is what
    // hands the button its click.
    click_client_point(window, 60, 80);
    click_client_point(window, 90, 148);
    let started =
        wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(20));
    let installed = wait_for_directory(
        &fixture.destination,
        Instant::now() + Duration::from_secs(30),
    );
    // Which row the page recorded, read while the window is still up: a case
    // that fails here has to say whether the click that picks the value arrived.
    let described = describe_page(window);
    let _ = setup.kill();
    let _ = setup.wait();

    assert_eq!(
        opened,
        (720, 450),
        "the wizard opened on {opened:?} rather than the 720x450 page the project declares"
    );
    assert_eq!(
        defaulted, None,
        "the install started on the row the layout defaults to"
    );
    assert_eq!(
        same, None,
        "clicking the row that was already on let the install start"
    );
    assert_eq!(
        started,
        Some((500, 300)),
        "the row the user clicked did not let the install start"
    );
    assert!(
        installed && fixture.destination.join("E2eProbe.exe").is_file(),
        "the install did not write the product into the configured directory {}; the page read as {described:#?}",
        fixture.destination.display()
    );
    Ok(())
}

/// The boxes the page carries are the user's answer, and the boxes it does not
/// carry leave the answer to the project.
///
/// Four components, four rules in one run: `core` is required and has no box,
/// `docs` installs because the user ticked the box the page placed clear,
/// `tools` installs because the project made it a default the page did not
/// override, and `samples` stays out because the user cleared the box the page
/// started ticked — the page wins over the project's own answer. Each component
/// carries one file, so the run's answer is on the disk afterwards.
#[test]
fn the_boxes_the_page_carries_decide_which_components_install() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.components_project()?;
    fixture.archive_component(
        "core.archive",
        PayloadFormat::Zip,
        &[("core/runtime.txt", b"core")],
    )?;
    fixture.archive_component(
        "docs.archive",
        PayloadFormat::Zip,
        &[("docs/readme.txt", b"docs")],
    )?;
    fixture.archive_component(
        "tools.archive",
        PayloadFormat::Zip,
        &[("tools/tool.txt", b"tools")],
    )?;
    fixture.archive_component(
        "samples.archive",
        PayloadFormat::Zip,
        &[("samples/sample.txt", b"samples")],
    )?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let Some(window) = (match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    }) else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let opened = client_size(window);
    // The box the page places at 20,40 and starts clear, and the one at 20,70
    // the layout starts ticked. The user's answer is these two clicks: the first
    // adds a component, the second takes one away.
    click_client_point(window, 30, 50);
    click_client_point(window, 30, 80);
    // The install button, at 20,130.
    click_client_point(window, 90, 148);
    let started =
        wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(20));
    // The last file of the deploy in the order the install writes them, so a run
    // that wrote it has written everything else too.
    let tool = wait_for_text(
        &fixture.destination.join("tools/tool.txt"),
        "tools",
        Instant::now() + Duration::from_secs(20),
    );
    let _ = setup.kill();
    let _ = setup.wait();

    let readme = std::fs::read_to_string(fixture.destination.join("docs/readme.txt")).ok();
    let runtime = std::fs::read_to_string(fixture.destination.join("core/runtime.txt")).ok();
    let sample = fixture.destination.join("samples/sample.txt");
    assert_eq!(
        opened,
        (720, 450),
        "the wizard opened on {opened:?} rather than the 720x450 page the project declares"
    );
    assert_eq!(
        started,
        Some((500, 300)),
        "the install did not start from the button on the page"
    );
    assert_eq!(
        tool.as_deref(),
        Some("tools"),
        "the component the project defaults to did not install"
    );
    assert_eq!(
        readme.as_deref(),
        Some("docs"),
        "the component the user ticked did not install"
    );
    assert_eq!(
        runtime.as_deref(),
        Some("core"),
        "the required component did not install"
    );
    assert!(
        !sample.exists(),
        "the component the user cleared was installed anyway: {} was written",
        sample.display()
    );
    Ok(())
}

/// What the user left on the page reaches the script, control by control.
///
/// A script could install with a constant, so the install finishing proves
/// nothing here. This case types into a field, picks the row of a choice group
/// the layout does not default to, and clicks install; the script writes what it
/// read into the destination, which is where the typing and the click have to
/// have arrived. Two of the four values are asked for by ids the page never
/// declares: a script reads those as empty text rather than failing, because a
/// page that carries no such control is a page the project can still run.
#[test]
fn the_values_the_page_holds_reach_the_script() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.values_project()?;
    fixture.write_script(
        "install.rhai",
        r#"
        let install_path = get_install_path();
        copy_uninstaller();
        write_file(path_join(install_path, "E2eProbe.exe"), "app");
        let report = "";
        report += "serial=" + get_text_value("serial") + "\n";
        report += "edition=" + get_choice_value("edition") + "\n";
        report += "untyped=" + get_text_value("nowhere") + "\n";
        report += "unchosen=" + get_choice_value("nothing") + "\n";
        write_file(path_join(install_path, "values.txt"), report);
        "#,
    )?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let Some(window) = (match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    }) else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let opened = client_size(window);
    // The field the page places at 20,40: a press takes its caret, and the
    // characters that follow land in it.
    press_client_point(window, 170, 53);
    type_client_text(window, "TT-2026-0001");
    // The second row of the group, which the layout does not default to.
    click_client_point(window, 120, 130);
    // The install button, at 20,170.
    click_client_point(window, 90, 188);
    let started =
        wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(20));
    let expected = "serial=TT-2026-0001\nedition=installed\nuntyped=\nunchosen=\n";
    let written = wait_for_text(
        &fixture.destination.join("values.txt"),
        expected,
        Instant::now() + Duration::from_secs(20),
    );
    let _ = setup.kill();
    let _ = setup.wait();

    // What the file really holds, so a failure names the values the script read
    // rather than only saying it read the wrong ones.
    let asked = std::fs::read_to_string(fixture.destination.join("values.txt")).ok();
    assert_eq!(
        opened,
        (720, 450),
        "the wizard opened on {opened:?} rather than the 720x450 page the project declares"
    );
    assert_eq!(
        started,
        Some((500, 300)),
        "the install did not start from the button on the page"
    );
    assert_eq!(
        asked.as_deref(),
        Some(expected),
        "the script did not read the values the page held (the wait saw {written:?})"
    );
    Ok(())
}

/// A wheel over a list is what brings the rows it does not show into reach.
///
/// The layout cases prove a scrollable container cuts what it does not show;
/// this one proves the window drives it, in the direction a user drives it. A
/// click in the slice below the list reaches the page's own button rather than
/// the row the list cut off, and one notch of the wheel moves that row up under
/// the button -- so the click that walked to the next page now lands on the row
/// the wheel brought in.
#[test]
fn a_wheel_over_a_list_brings_the_rows_below_into_reach() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.scrolling_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let opened = client_size(window);
    // The point is below the list, where the fourth row would have been drawn
    // and where the page put a button of its own: the row is cut off, so the
    // button is the one that answers.
    click_client_point(window, 60, 195);
    let below = wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(10));
    // The second page carries the button that walks back to the list.
    let returned = match below {
        Some(_) => {
            click_client_point(window, 80, 258);
            wait_for_client_size(window, opened, Instant::now() + Duration::from_secs(10))
        }
        None => None,
    };
    // One notch of the wheel towards the user, over the list, then a click on
    // the point that is over the third row until the wheel moves and over the
    // fourth one after.
    wheel_client_point(window, 200, 100, -1);
    click_client_point(window, 200, 150);
    let closed = wait_for_exit(&mut setup, Duration::from_secs(10));
    let _ = setup.kill();
    let _ = setup.wait();

    assert_eq!(
        opened,
        (720, 450),
        "the wizard opened on {opened:?} rather than the 720x450 page the project declares"
    );
    assert_eq!(
        below,
        Some((500, 300)),
        "a row the list had cut off took the click meant for the button below it"
    );
    assert_eq!(
        returned,
        Some(opened),
        "the button below the list did not walk to the second page"
    );
    assert!(
        closed,
        "the wheel did not bring the row below the list under the pointer"
    );
    Ok(())
}

/// A script's message and question are drawn in the wizard, in the product's own
/// skin, and a click on the card answers the script that is waiting.
///
/// `ask_yes_no`, `show_message` and `show_error` block until someone presses a
/// button, which is what kept them out of the suite: a case that cannot press
/// one waits for a person who is not there. They are answered here by clicking
/// the card the runtime draws from the project's dialog layout, whose buttons
/// this case places where it can reach them, so what the click proves is the
/// whole path: the card is drawn in the wizard window rather than in a window of
/// its own, its buttons take the click, and the answer reaches the script, which
/// then carries on.
#[test]
fn a_script_dialog_is_drawn_in_the_wizard() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.cancellable_project()?;
    // A card of the case's own, so its buttons sit where the case clicks them:
    // the runtime centres a 400x180 card in the page, which puts it at 50,60 on
    // the 500x300 page the task reports on and its two buttons at the points
    // below. Both buttons are on the same line, so one point answers the
    // question and dismisses the two messages that follow it.
    std::fs::write(
        fixture.project.join("layouts/msgBox.xml"),
        r##"<Page width="400" height="180" background="#FF2A3844">
  <Label id="lblMsg" text="@ok" value-source="dialog:message"
         position="absolute" left="20" top="20" width="360" height="60" wrap="true" />
  <Button id="btnNo" action="dialog_cancel" visible-with="dismiss"
          text="@cancel" value-source="dialog:dismiss"
          position="absolute" left="40" top="120" width="140" height="36" />
  <Button id="btnYes" action="dialog_ok"
          text="@ok" value-source="dialog:accept"
          position="absolute" left="220" top="120" width="140" height="36" />
</Page>"##,
    )?;
    let title = format!("Nano question {}", fixture.id);
    fixture.write_script(
        "install.rhai",
        &format!(
            r#"
            let install_path = get_install_path();
            copy_uninstaller();
            write_file(path_join(install_path, "E2eProbe.exe"), "app");
            write_file(path_join(install_path, "asking.txt"), "asking");
            let answered = ask_yes_no("{title}", "Install this product?");
            write_file(path_join(install_path, "answer.txt"), answered.to_string());
            show_message("{title}", "The product is installed.");
            write_file(path_join(install_path, "notice.txt"), "dismissed");
            show_error("{title}", "This is what a failure looks like.");
            write_file(path_join(install_path, "failure.txt"), "dismissed");
            "#,
        ),
    )?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    // The centre of the install button the first page places at 560,390: the
    // task the script runs in starts here, and the window moves to the 500x300
    // page it reports on.
    click_client_point(window, 620, 408);
    let running =
        wait_for_client_size(window, (500, 300), Instant::now() + Duration::from_secs(20));
    let asking = wait_for_text(
        &fixture.destination.join("asking.txt"),
        "asking",
        Instant::now() + Duration::from_secs(20),
    );
    // Nothing can answer the script while the card is up, which is what the file
    // it writes after the answer says: it is still missing here.
    std::thread::sleep(Duration::from_millis(300));
    let answered_early = std::fs::read_to_string(fixture.destination.join("answer.txt")).ok();
    let behind = client_size(window);

    // The card is centred on the page the task reports on, so its buttons sit at
    // these points of the window's client area.
    let (yes_x, yes_y) = (340, 198);
    let answer = click_until_text(
        window,
        yes_x,
        yes_y,
        &fixture.destination.join("answer.txt"),
        "true",
        Instant::now() + Duration::from_secs(20),
    );
    let dismissed = click_until_text(
        window,
        yes_x,
        yes_y,
        &fixture.destination.join("notice.txt"),
        "dismissed",
        Instant::now() + Duration::from_secs(20),
    );
    let reported = click_until_text(
        window,
        yes_x,
        yes_y,
        &fixture.destination.join("failure.txt"),
        "dismissed",
        Instant::now() + Duration::from_secs(20),
    );
    // The finish page the wizard declares is 400x200, and the wizard only gets
    // there with a task that ran to its end.
    let finished =
        wait_for_client_size(window, (400, 200), Instant::now() + Duration::from_secs(20));
    let _ = setup.kill();
    let _ = setup.wait();

    assert_eq!(
        running,
        Some((500, 300)),
        "the install button did not start the task on its progress page"
    );
    assert_eq!(
        asking.as_deref(),
        Some("asking"),
        "the script never reached the question it asks"
    );
    assert_eq!(
        answered_early, None,
        "the script answered itself instead of waiting for a click"
    );
    assert_eq!(
        behind,
        (500, 300),
        "the card took the window somewhere else instead of being drawn inside it"
    );
    assert_eq!(
        answer.as_deref(),
        Some("true"),
        "the click on the card's yes did not reach ask_yes_no"
    );
    assert_eq!(
        dismissed.as_deref(),
        Some("dismissed"),
        "the script did not carry on after the message was dismissed"
    );
    assert_eq!(
        reported.as_deref(),
        Some("dismissed"),
        "the script did not carry on after the failure report was dismissed"
    );
    assert_eq!(
        finished,
        Some((400, 200)),
        "the install did not reach the page the wizard finishes on"
    );
    Ok(())
}

/// A button draws the picture each of its states names, and the window shows it
/// as the pointer moves over the control.
///
/// The runtime picks the picture from its own state: the pointer arriving on a
/// control is a hover, a press on one is a press, and leaving the window takes
/// both away. The pictures here are one flat colour each and the page behind
/// them is another, so asking the window what it drew at the button's centre
/// says which one is up, and asking where else the frame changed says the state
/// belongs to the button rather than to the page.
#[test]
fn a_hover_and_a_press_show_the_pictures_the_button_declares() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.hover_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    // The pointer and the window are both borrowed for the length of the case:
    // the pointer goes back where it was found, and the window back among the
    // others. Lifting it matters, because the suite runs its window cases at the
    // same time and which window the pointer is over decides which of them sees
    // it at all -- which is why the two cases that move the pointer take
    // `the_pointer` in turn instead of each lifting a window over the other.
    let _pointer_case = the_pointer();
    let _pointer = PointerRestore::capture();
    let _in_front = WindowInFront::lift(window);
    let _ = unsafe { SetForegroundWindow(window) };

    // The button is 200 by 60 at (40, 60): this point is its centre and that one
    // is the page beside it.
    let centre = (140, 90);
    let beside = (360, 180);
    let button = (40, 60, 240, 120);
    let deadline = Instant::now() + Duration::from_secs(20);

    let resting = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(centre.0, centre.1) == RESTING_BUTTON
    });
    assert!(
        resting.is_some(),
        "the button did not show the picture it rests on: {}",
        colour_at_window(window, centre)
    );
    let resting = resting.expect("checked just above");
    assert_eq!(
        resting.colour_at(beside.0, beside.1),
        (0x00, 0x00, 0x00),
        "the page around the button is not the colour its layout declares"
    );

    // The pointer moves onto the button, which is what a hover is.
    move_pointer_onto(window, centre)?;
    let hovered = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(centre.0, centre.1) == HOVERED_BUTTON
    });
    assert!(
        hovered.is_some(),
        "the pointer over the button did not show its hover picture: {}",
        colour_at_window(window, centre)
    );
    assert_eq!(
        hovered
            .expect("checked just above")
            .differences_outside(&resting, button),
        0,
        "hovering the button repainted the page around it"
    );

    // A press is a button-down. The button-up is not sent: that would be the
    // click, and this case is about how the button looks while it is held.
    let lparam = LPARAM(((centre.1 << 16) | (centre.0 & 0xFFFF)) as isize);
    unsafe { PostMessageW(window, WM_LBUTTONDOWN, WPARAM(0), lparam) }
        .expect("the runtime window accepts a press");
    let pressed = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(centre.0, centre.1) == PRESSED_BUTTON
    });
    assert!(
        pressed.is_some(),
        "the press did not show the picture the button declares for one: {}",
        colour_at_window(window, centre)
    );
    assert_eq!(
        pressed
            .expect("checked just above")
            .differences_outside(&resting, button),
        0,
        "the press repainted the page around the button"
    );

    // The pointer leaves the window, which is where both of the other states
    // end: the button goes back to the picture it rests on, exactly as it was.
    move_pointer_off(window)?;
    let left = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(centre.0, centre.1) == RESTING_BUTTON
    });
    assert!(
        left.is_some(),
        "the pointer leaving did not put the resting picture back: {}",
        colour_at_window(window, centre)
    );
    assert_eq!(
        left.expect("checked just above")
            .differences_outside(&resting, button),
        0,
        "the window after the pointer left is not the window it was before it arrived, \
         outside the button the press put the keyboard on"
    );

    Ok(())
}

/// The pointer over the wizard is the shape the layout promises for whatever
/// sits under it.
///
/// A cursor is not drawn into the window: Windows asks the window which shape
/// it wants, and the window answers with one of the standard pointers. The
/// question is put to it here by moving the pointer for real and posting the
/// same message Windows posts, so what is read back is the shape the desktop
/// would be showing a user.
#[test]
fn the_pointer_decides_which_cursor_the_wizard_shows() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.cursor_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    // A session with no cursor to show cannot answer this question. A hosted
    // runner has a window to draw into but no pointer, and `GetCursorInfo`
    // reports a null cursor there; a desktop that is showing one always has a
    // handle, whether or not the pointer is over this window.
    if shown_cursor().is_none_or(|cursor| cursor.is_invalid()) {
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_without_a_cursor();
    }

    // The pointer belongs to the machine rather than to the case, so it is put
    // back where it was found, including when an assertion fails first -- and
    // only one case moves it at a time.
    let _pointer_case = the_pointer();
    let _pointer = PointerRestore::capture();
    // A window that is not in front can set the cursor without the desktop
    // taking any notice of it, so this window is lifted above the rest of the
    // desk for the length of the case.
    let _in_front = WindowInFront::lift(window);
    let _ = unsafe { SetForegroundWindow(window) };

    let hand = unsafe { LoadCursorW(None, IDC_HAND) }?;
    let beam = unsafe { LoadCursorW(None, IDC_IBEAM) }?;
    let arrow = unsafe { LoadCursorW(None, IDC_ARROW) }?;
    // Three shapes that are told apart is what makes the rest of this case say
    // anything: one handle for all of them would pass whatever the window did.
    assert_ne!(hand, beam, "the hand and the beam cursor are one pointer");
    assert_ne!(hand, arrow, "the hand and the arrow cursor are one pointer");
    assert_ne!(beam, arrow, "the beam and the arrow cursor are one pointer");
    let deadline = Instant::now() + Duration::from_secs(20);

    // The button is 120 by 30 at (20, 20), the field 200 by 24 at (20, 80), and
    // (300, 150) is page with nothing on it.
    let over_button = wait_for_cursor(window, (80, 35), hand, deadline);
    let over_field = wait_for_cursor(window, (100, 92), beam, deadline);
    let over_page = wait_for_cursor(window, (300, 150), arrow, deadline);
    // Back to the button: the shape follows what the pointer is over rather
    // than staying on whichever control it was left on.
    let back_on_button = wait_for_cursor(window, (80, 35), hand, deadline);

    let report = || cursor_report(window, hand, beam, arrow);
    assert_eq!(
        over_button,
        Some(hand),
        "the pointer over a button that answers a click did not show the hand: {}",
        report()
    );
    assert_eq!(
        over_field,
        Some(beam),
        "the pointer over a field the user can type into did not show the beam: {}",
        report()
    );
    assert_eq!(
        over_page,
        Some(arrow),
        "the pointer over the page itself did not show the ordinary arrow: {}",
        report()
    );
    assert_eq!(
        back_on_button,
        Some(hand),
        "the cursor did not follow the pointer back onto the button: {}",
        report()
    );
    Ok(())
}

/// The language menu answers to the keyboard: the arrows move its highlight,
/// Enter takes the row it rests on, and Escape closes it without choosing.
///
/// Both the highlight and the row the language in use stands on are colours the
/// layout declares, so the window can be read back, and the sentence a language
/// writes is what says the choice reached the page. The two languages here
/// write sentences of different lengths, so a frame that changed says which one
/// is up.
#[test]
fn the_language_menu_answers_to_the_keyboard() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.language_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let deadline = Instant::now() + Duration::from_secs(20);
    // The control is 180 by 26 at (100, 20) and its popup hangs four pixels
    // under it with rows 26 pixels tall, so these two points are just inside
    // the first and the second row, clear of the words those rows draw.
    let control = (100, 20, 280, 46);
    let first_row = (104, 60);
    let second_row = (104, 90);
    // The label that carries the sentence the language writes.
    let sentence = (20, 140, 380, 164);

    let closed = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(first_row.0, first_row.1) == (0x00, 0x00, 0x00)
    });
    assert!(
        closed.is_some(),
        "the page did not come up with the menu closed: {}",
        colour_at_window(window, first_row)
    );
    let closed = closed.expect("checked just above");

    // A click on the control opens its menu, which is how a user reaches it.
    click_client_point(window, 190, 33);
    let opened = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(first_row.0, first_row.1) == CHOSEN_ROW
            && frame.colour_at(second_row.0, second_row.1) == POPUP_BACKGROUND
    });
    assert!(
        opened.is_some(),
        "a click on the language control did not open its menu: {}",
        colour_at_window(window, first_row)
    );

    // One arrow key walks the highlight off the language in use, because a menu
    // opens on the row the current choice stands on and moves away from it.
    press_key(window, VK_DOWN);
    let walked = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(second_row.0, second_row.1) == HIGHLIGHTED_ROW
    });
    assert!(
        walked.is_some(),
        "the down arrow did not move the highlight onto the row below: {}",
        colour_at_window(window, second_row)
    );
    assert_eq!(
        walked
            .expect("checked just above")
            .colour_at(first_row.0, first_row.1),
        CHOSEN_ROW,
        "moving the highlight took the mark off the row the language in use stands on"
    );

    // Escape closes the menu and leaves the language alone, so the window is
    // the window it was before the menu opened.
    press_key(window, VK_ESCAPE);
    let escaped = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(first_row.0, first_row.1) == (0x00, 0x00, 0x00)
    });
    assert!(
        escaped.is_some(),
        "Escape did not close the menu: {}",
        colour_at_window(window, first_row)
    );
    assert_eq!(
        escaped
            .expect("checked just above")
            .differing_pixels(&closed),
        0,
        "Escape changed the page instead of only closing the menu"
    );

    // The menu is opened again, the highlight walks to the other language, and
    // Enter takes it: the sentence the page shows is the one that language
    // writes.
    click_client_point(window, 190, 33);
    let reopened = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(first_row.0, first_row.1) == CHOSEN_ROW
    });
    assert!(
        reopened.is_some(),
        "the menu did not open a second time: {}",
        colour_at_window(window, first_row)
    );
    press_key(window, VK_DOWN);
    let moved = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(second_row.0, second_row.1) == HIGHLIGHTED_ROW
    });
    assert!(
        moved.is_some(),
        "the down arrow did not move the highlight the second time: {}",
        colour_at_window(window, second_row)
    );
    press_key(window, VK_RETURN);
    let chosen = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(first_row.0, first_row.1) == (0x00, 0x00, 0x00)
    });
    assert!(
        chosen.is_some(),
        "Enter did not close the menu: {}",
        colour_at_window(window, first_row)
    );
    let chosen = chosen.expect("checked just above");
    assert!(
        chosen.differences_inside(&closed, sentence) > 0,
        "choosing the other language left the sentence the page shows unchanged"
    );
    assert_eq!(
        chosen.differences_outside_all(&closed, &[sentence, control]),
        0,
        "choosing the other language repainted more than the sentence and the control"
    );

    Ok(())
}

/// The keyboard walks the controls a page lays out and acts on the one it
/// reaches.
///
/// Tab walks them in the order the page declares them and the ring says where
/// the keyboard is, so every step is read back as the colour of a control's
/// corner. Space and Enter do what the control under the ring does: here that is
/// a checkbox changing its picture and a button walking to the next page. From
/// the last control Tab wraps round to the first, and a page that declares no
/// control leaves the keyboard nothing to do.
///
/// The other direction, Shift+Tab, is covered where it can be held still: the
/// walk itself is a case of its own, and a window case cannot press Shift --
/// a key message carries no modifier, the runtime asks Windows for it, and this
/// suite posts its keys rather than typing them.
#[test]
fn the_keyboard_walks_the_page_and_acts_on_what_it_reaches() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.keyboard_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };

    let deadline = Instant::now() + Duration::from_secs(20);
    // The checkbox is 24 by 24 at (40, 20), the field 200 by 24 at (40, 60) and
    // the button 120 by 30 at (40, 120). The ring is drawn on the corners of the
    // control and on every other pixel of its edges, so a corner is where a case
    // can read it off, and the middle of the checkbox is where the picture shows.
    let picture = (52, 32);
    let terms = (40, 20);
    let notes = (40, 60);
    let next = (40, 120);
    let empty_page = (350, 170);

    // Nothing has been touched yet: no ring, and a checkbox in its first state.
    let start = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(picture.0, picture.1) == UNCHECKED_BOX
            && frame.colour_at(terms.0, terms.1) == UNCHECKED_BOX
            && frame.colour_at(empty_page.0, empty_page.1) == (0x00, 0x00, 0x00)
    });
    assert!(
        start.is_some(),
        "the page did not come up as it was declared: {} and {}",
        colour_at_window(window, picture),
        colour_at_window(window, empty_page)
    );
    let start = start.expect("checked just above");

    // The first Tab lands on the first control the page declares.
    press_key(window, VK_TAB);
    let first = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(terms.0, terms.1) == FOCUS_RING
    });
    assert!(
        first.is_some(),
        "Tab did not put the ring on the first control: {}",
        colour_at_window(window, terms)
    );
    let first = first.expect("checked just above");
    assert_eq!(
        first.colour_at(picture.0, picture.1),
        UNCHECKED_BOX,
        "the ring covered the control it was drawn around"
    );
    assert_eq!(
        first.differences_outside(&start, (40, 20, 64, 44)),
        0,
        "putting the ring on a control repainted the page around it"
    );

    // Space does what the control under the ring does, which for a checkbox is
    // to flip it.
    press_key(window, VK_SPACE);
    let toggled = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(picture.0, picture.1) == CHECKED_BOX
    });
    assert!(
        toggled.is_some(),
        "Space did not flip the checkbox the ring was on: {}",
        colour_at_window(window, picture)
    );
    assert_eq!(
        toggled
            .expect("checked just above")
            .colour_at(terms.0, terms.1),
        FOCUS_RING,
        "flipping the checkbox took the ring off it"
    );

    // Tab walks on to the field the page declares next, and the ring leaves the
    // control behind it.
    press_key(window, VK_TAB);
    let second = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(notes.0, notes.1) == FOCUS_RING
    });
    assert!(
        second.is_some(),
        "Tab did not move the ring onto the field: {}",
        colour_at_window(window, notes)
    );
    assert_eq!(
        second
            .expect("checked just above")
            .colour_at(terms.0, terms.1),
        CHECKED_BOX,
        "the checkbox kept the ring after the keyboard left it"
    );

    // And once more, onto the button, which is the last control on the page.
    press_key(window, VK_TAB);
    let third = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(next.0, next.1) == FOCUS_RING
    });
    assert!(
        third.is_some(),
        "Tab did not move the ring onto the button: {}",
        colour_at_window(window, next)
    );
    assert_eq!(
        third
            .expect("checked just above")
            .colour_at(notes.0, notes.1),
        (0x00, 0x00, 0x00),
        "the field kept the ring after the keyboard left it"
    );

    // Either end wraps, which is what keeps the keyboard on a page it can walk:
    // Tab from the last control comes round to the first, and the ring leaves the
    // control behind it.
    press_key(window, VK_TAB);
    let wrapped = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(terms.0, terms.1) == FOCUS_RING
    });
    assert!(
        wrapped.is_some(),
        "Tab from the last control did not wrap round to the first: {}",
        colour_at_window(window, terms)
    );
    assert_eq!(
        wrapped
            .expect("checked just above")
            .colour_at(next.0, next.1),
        (0x00, 0x00, 0x00),
        "the button kept the ring after the keyboard wrapped round"
    );

    // Back to the last control, where the button carries the action Enter runs.
    press_key(window, VK_TAB);
    press_key(window, VK_TAB);
    let last = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(next.0, next.1) == FOCUS_RING
    });
    assert!(
        last.is_some(),
        "Tab did not walk back onto the button: {}",
        colour_at_window(window, next)
    );

    // Enter runs what the control under the ring carries, which here is the page
    // the button walks to. The window takes the size of the page it shows, so the
    // client area is waited on rather than the frame: a frame read while the
    // window is still resizing would say the next key changed the window when the
    // page change did.
    press_key(window, VK_RETURN);
    let walked = wait_for_client_size(window, (300, 150), deadline);
    assert!(
        walked.is_some(),
        "Enter did not do what the button under the ring carries: the client area is {:?}",
        client_size(window)
    );

    // The second page declares no control, so Tab has nowhere to go: it leaves
    // the window as it was rather than falling over.
    let before = capture_frame(window)?;
    assert_eq!(
        before.colour_at(150, 75),
        SECOND_PAGE,
        "the page the button walks to did not draw itself: {}",
        colour_at_window(window, (150, 75))
    );
    press_key(window, VK_TAB);
    std::thread::sleep(Duration::from_millis(200));
    let after = capture_frame(window)?;
    assert_eq!(
        after.differing_pixels(&before),
        0,
        "Tab changed a page that declares no control"
    );
    assert!(
        unsafe { IsWindow(window) }.as_bool(),
        "the window went away when Tab was pressed on a page with no control"
    );

    Ok(())
}

/// The colour the machine keeps for one part of a page, in the order a frame
/// reads a pixel.
///
/// The values belong to whoever is sitting at the machine, so a case about high
/// contrast asks the machine for them instead of naming them: what it can hold
/// the runtime to is that the frame shows the same colours Windows reports.
fn system_colour(index: SYS_COLOR_INDEX) -> (u8, u8, u8) {
    let colour = unsafe { GetSysColor(index) };
    (
        (colour & 0xFF) as u8,
        ((colour >> 8) & 0xFF) as u8,
        ((colour >> 16) & 0xFF) as u8,
    )
}

/// The same colour written the way a layout declares one.
fn layout_colour(colour: (u8, u8, u8)) -> String {
    format!("#FF{:02X}{:02X}{:02X}", colour.0, colour.1, colour.2)
}

/// Waits for the window of a setup that was just started.
///
/// It is the wait every window case makes, with the leaving in one place: a
/// machine whose desktop cannot hold a window skips the case, and a window that
/// was required is the reason it failed.
fn wait_for_a_window(setup: &mut SetupGuard) -> anyhow::Result<Option<HWND>> {
    let waited = wait_for_runtime_window(setup, Instant::now() + Duration::from_secs(30));
    let reason = match waited {
        WindowWait::Found(window) => return Ok(Some(window)),
        WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
        WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
    };
    let _ = setup.kill();
    let _ = setup.wait();
    skip_missing_desktop(&reason)?;
    Ok(None)
}

/// Asks the wizard what it is showing, the way a screen reader does.
///
/// The client here is an ordinary one in another process, so this is the whole
/// path a reader takes: the window answers `WM_GETOBJECT`, Windows marshals the
/// object across, and every question after that is answered from the page the
/// wizard is painting.
fn screen_reader(window: HWND) -> anyhow::Result<IAccessible> {
    // A client has to have joined an apartment to be handed a marshaled object.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let mut raw: *mut core::ffi::c_void = std::ptr::null_mut();
    unsafe {
        AccessibleObjectFromWindow(
            window,
            OBJID_CLIENT.0 as u32,
            &IAccessible::IID,
            &mut raw as *mut *mut core::ffi::c_void,
        )
    }
    .context("the wizard did not describe itself")?;
    Ok(unsafe { IAccessible::from_raw(raw) })
}

/// One child of the object, named the way a client names it.
fn child(id: i32) -> VARIANT {
    VARIANT::from(id)
}

/// What a child is called, or nothing when the client was refused.
fn accessible_name(object: &IAccessible, id: i32) -> String {
    unsafe { object.get_accName(&child(id)) }
        .map(|name| name.to_string())
        .unwrap_or_default()
}

/// What a child holds, which for a field is its text and for a select the
/// option it shows.
fn accessible_value(object: &IAccessible, id: i32) -> String {
    unsafe { object.get_accValue(&child(id)) }
        .map(|value| value.to_string())
        .unwrap_or_default()
}

/// What a child is, in the terms `oleacc.h` names.
fn accessible_role(object: &IAccessible, id: i32) -> u32 {
    let role = unsafe { object.get_accRole(&child(id)) }.expect("a child has a role");
    u32::try_from(&role).unwrap_or_default()
}

/// What a child's state is, which is a field of bits.
fn accessible_state(object: &IAccessible, id: i32) -> u32 {
    let state = unsafe { object.get_accState(&child(id)) }.expect("a child has a state");
    u32::try_from(&state).unwrap_or_default()
}

/// Where a child is on the screen, which is where a client draws its highlight.
fn accessible_rect(object: &IAccessible, id: i32) -> (i32, i32, i32, i32) {
    let (mut left, mut top, mut width, mut height) = (0, 0, 0, 0);
    unsafe { object.accLocation(&mut left, &mut top, &mut width, &mut height, &child(id)) }
        .expect("a child has a place on the page");
    (left, top, width, height)
}

/// The child with the keyboard, or nothing while the page has not been walked.
fn accessible_focus(object: &IAccessible) -> Option<i32> {
    let focus = unsafe { object.accFocus() }.ok()?;
    if focus.is_empty() {
        return None;
    }
    i32::try_from(&focus).ok()
}

/// The child a point is over, in screen coordinates.
fn accessible_hit(object: &IAccessible, x: i32, y: i32) -> Option<i32> {
    let hit = unsafe { object.accHitTest(x, y) }.ok()?;
    i32::try_from(&hit).ok()
}

/// The number of the child that plays a role, while the page is showing one.
///
/// A case that is about one control asks for it by what it is rather than by the
/// number it happens to sit at, so a page that grows a control does not move the
/// case onto another one.
fn child_with_role(object: &IAccessible, role: u32) -> Option<i32> {
    let count = unsafe { object.accChildCount() }.ok()?;
    (1..=count).find(|id| accessible_role(object, *id) == role)
}

/// What the page is telling the user about a value it will not accept, while it
/// is telling them anything: the words of the hint it draws beside the field.
fn accessible_hint(object: &IAccessible) -> Option<String> {
    let hint = child_with_role(object, ROLE_SYSTEM_STATICTEXT)?;
    let words = accessible_name(object, hint);
    (!words.is_empty()).then_some(words)
}

/// What a client is told about a child beyond its name and its value.
fn accessible_description(object: &IAccessible, id: i32) -> String {
    unsafe { object.get_accDescription(&child(id)) }
        .map(|text| text.to_string())
        .unwrap_or_default()
}

/// Pumps this thread's messages until the client has heard `wanted` reports
/// about the task that is running.
fn pump_until_live(wanted: usize, deadline: Instant) -> Vec<(u32, i32)> {
    let mut message = MSG::default();
    while Instant::now() < deadline {
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            let _ = unsafe { TranslateMessage(&message) };
            unsafe { DispatchMessageW(&message) };
        }
        {
            let heard = HEARD_LIVE.lock().unwrap_or_else(|error| error.into_inner());
            if heard.len() >= wanted {
                return heard.clone();
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    HEARD_LIVE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// Every child of the object, written out as a client reads it.
///
/// A case that fails on one control prints the whole page this way, because
/// what a reader was told is the only thing worth seeing when it is wrong.
fn accessible_children(object: &IAccessible, count: i32) -> Vec<String> {
    (1..=count)
        .map(|id| {
            format!(
                "#{id} role={} name={:?} value={:?} state={:#x} at={:?}",
                accessible_role(object, id),
                accessible_name(object, id),
                accessible_value(object, id),
                accessible_state(object, id),
                accessible_rect(object, id),
            )
        })
        .collect()
}

/// A screen reader that asks the wizard what it is showing is told about the
/// page's controls: what each one is, what it is called, what it holds, where
/// it sits, and which of them the keyboard is on.
///
/// The runtime paints every control itself, so a client that walks the window's
/// children finds nothing: this is the only description of the page a reader
/// can get, and a control the user can reach but a reader cannot name is a
/// control that user cannot use.
#[test]
fn a_screen_reader_reads_the_page_the_wizard_is_showing() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.accessibility_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    let object = screen_reader(window)?;
    let count = unsafe { object.accChildCount() }.context("the page has controls")?;
    assert_eq!(
        count,
        6,
        "the page reads as {:#?}",
        accessible_children(&object, count)
    );

    // The field is called by the words the layout wrote above it rather than by
    // its id, and it holds nothing until the user types.
    assert_eq!(accessible_role(&object, 1), ROLE_SYSTEM_TEXT);
    assert_eq!(accessible_name(&object, 1), "Your name");
    assert_eq!(accessible_value(&object, 1), "");

    // The box carries its own words and is not filled in yet.
    assert_eq!(accessible_role(&object, 2), ROLE_SYSTEM_CHECKBUTTON);
    assert_eq!(accessible_name(&object, 2), "I accept the terms");
    assert_eq!(accessible_state(&object, 2) & STATE_SYSTEM_CHECKED, 0);

    // The radios are the two rows of one group, and neither is chosen yet.
    assert_eq!(accessible_role(&object, 3), ROLE_SYSTEM_RADIOBUTTON);
    assert_eq!(accessible_name(&object, 3), "Quick install");
    assert_eq!(accessible_role(&object, 4), ROLE_SYSTEM_RADIOBUTTON);
    assert_eq!(accessible_name(&object, 4), "Custom install");
    assert_eq!(accessible_state(&object, 4) & STATE_SYSTEM_CHECKED, 0);

    // The select shows one of its options, which is what a client reads as its
    // value, and it says that a press opens a list.
    assert_eq!(accessible_role(&object, 5), ROLE_SYSTEM_COMBOBOX);
    assert_eq!(accessible_value(&object, 5), "Stable");
    assert_ne!(accessible_state(&object, 5) & STATE_SYSTEM_HASPOPUP, 0);

    // The button, and the window itself above it.
    assert_eq!(accessible_role(&object, 6), ROLE_SYSTEM_PUSHBUTTON);
    assert_eq!(accessible_name(&object, 6), "Next");
    assert_eq!(accessible_role(&object, 0), ROLE_SYSTEM_WINDOW);
    assert!(
        !accessible_name(&object, 0).is_empty(),
        "the window has no name of its own"
    );

    // Where a control is, as the screen coordinates a client draws its
    // highlight at: the page is measured from the window's client corner.
    let mut corner = POINT { x: 20, y: 108 };
    unsafe { ClientToScreen(window, &mut corner) }.expect("the window has a screen position");
    assert_eq!(accessible_rect(&object, 3), (corner.x, corner.y, 220, 24));

    // A point over a control is that control, and a point on the page where
    // nothing sits is the wizard.
    assert_eq!(accessible_hit(&object, corner.x + 5, corner.y + 5), Some(3));
    let mut empty = POINT { x: 450, y: 240 };
    unsafe { ClientToScreen(window, &mut empty) }.expect("the window has a screen position");
    assert_eq!(accessible_hit(&object, empty.x, empty.y), Some(0));

    // Nothing has the keyboard yet, so nothing is announced as focused.
    assert_eq!(accessible_focus(&object), None);

    // Asking to take the focus is what a reader does when it wants a user to
    // type into a field, and the field is then the one with the keyboard.
    unsafe { object.accSelect(SELFLAG_TAKEFOCUS as i32, &child(1)) }
        .expect("the field takes the focus");
    assert_eq!(accessible_focus(&object), Some(1));
    assert_ne!(accessible_state(&object, 1) & STATE_SYSTEM_FOCUSED, 0);

    // A client asking for a control's default action works it the way a press
    // does, and the box it pressed comes back filled in.
    unsafe { object.accDoDefaultAction(&child(2)) }.expect("the box accepts its default action");
    assert_ne!(
        accessible_state(&object, 2) & STATE_SYSTEM_CHECKED,
        0,
        "the box a client pressed is still empty"
    );

    // The button walks to the page after this one, which declares nothing the
    // keyboard can reach: the description follows the page rather than staying
    // behind with the controls that are gone.
    unsafe { object.accDoDefaultAction(&child(6)) }.expect("the button accepts its default action");
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut later = -1;
    while Instant::now() < deadline {
        later = unsafe { object.accChildCount() }.unwrap_or(-1);
        if later == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        later, 0,
        "the description kept the controls of the page the wizard left"
    );

    let _ = setup.kill();
    let _ = setup.wait();
    Ok(())
}

/// A question the runtime draws over the page is what a client reads while it is
/// up: the question itself, and the two answers beside it.
///
/// The page behind the card cannot be reached while it is up, so a description
/// that still listed its controls would hand a user controls nothing answers.
/// The words of the answers belong to the case, so what a client hears is
/// checked word for word, and the answer a client presses is the answer the
/// waiting script gets.
#[test]
fn a_question_over_the_page_is_what_a_screen_reader_reads() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.cancellable_project()?;
    std::fs::write(
        fixture.project.join("layouts/msgBox.xml"),
        r##"<Page width="400" height="180" background="#FF2A3844">
  <Label id="lblMsg" text="@ok" value-source="dialog:message"
         position="absolute" left="20" top="20" width="360" height="60" wrap="true" />
  <Button id="btnNo" action="dialog_cancel" visible-with="dismiss"
          text="@cancel" value-source="dialog:dismiss"
          position="absolute" left="40" top="120" width="140" height="36" />
  <Button id="btnYes" action="dialog_ok"
          text="@ok" value-source="dialog:accept"
          position="absolute" left="220" top="120" width="140" height="36" />
</Page>"##,
    )?;
    // The two answers say what the case looks for, and the question is the title
    // the script asks with: a question a script asks answers with yes and no,
    // which are the two keys its card reads.
    std::fs::write(
        fixture.project.join("locales/en-US.json"),
        r#"{"yes": "Install now", "no": "Not now"}"#,
    )?;
    let title = format!("Nano question {}", fixture.id);
    fixture.write_script(
        "install.rhai",
        &format!(
            r#"
            let install_path = get_install_path();
            copy_uninstaller();
            write_file(path_join(install_path, "E2eProbe.exe"), "app");
            write_file(path_join(install_path, "asking.txt"), "asking");
            let answered = ask_yes_no("{title}", "Install this product?");
            write_file(path_join(install_path, "answer.txt"), answered.to_string());
            "#
        ),
    )?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    // The centre of the install button the first page places: the script starts
    // there and stops at the question it asks.
    click_client_point(window, 620, 408);
    let asking = wait_for_text(
        &fixture.destination.join("asking.txt"),
        "asking",
        Instant::now() + Duration::from_secs(20),
    );
    assert_eq!(
        asking.as_deref(),
        Some("asking"),
        "the script never reached the question it asks"
    );

    let object = screen_reader(window)?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut count = -1;
    while Instant::now() < deadline {
        count = unsafe { object.accChildCount() }.unwrap_or(-1);
        if count == 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        count, 2,
        "the page behind the question is still what a client reads"
    );
    assert_eq!(accessible_role(&object, 0), ROLE_SYSTEM_DIALOG);
    // What the window is named by is the question the card draws, which is the
    // title the script asked with and the words under it.
    let question = accessible_name(&object, 0);
    assert!(
        question.starts_with(&title) && question.contains("Install this product?"),
        "the window is not named by the question it is asking: {question:?}"
    );
    assert_eq!(accessible_role(&object, 1), ROLE_SYSTEM_PUSHBUTTON);
    assert_eq!(accessible_name(&object, 1), "Not now");
    assert_eq!(accessible_role(&object, 2), ROLE_SYSTEM_PUSHBUTTON);
    assert_eq!(accessible_name(&object, 2), "Install now");

    // Answering through the reader is answering the script: a client that asks
    // for the button's default action presses the button the user would press.
    unsafe { object.accDoDefaultAction(&child(2)) }.expect("the answer accepts its default action");
    let answered = wait_for_text(
        &fixture.destination.join("answer.txt"),
        "true",
        Instant::now() + Duration::from_secs(20),
    );
    assert_eq!(
        answered.as_deref(),
        Some("true"),
        "the answer a client gave never reached the script"
    );

    // The card goes away with the answer, which is the page coming back.
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if unsafe { object.accChildCount() }.unwrap_or(-1) != 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = setup.kill();
    let _ = setup.wait();
    Ok(())
}

/// The field a page is showing, as the value it holds, or nothing when the page
/// could not be asked about.
///
/// A case that fails because an install went somewhere else has to say whether
/// the input that decided where it went ever reached the page, and the page's own
/// description is the only place that answer lives.
fn accessible_field_value(window: HWND) -> Option<String> {
    let object = screen_reader(window).ok()?;
    let count = unsafe { object.accChildCount() }.unwrap_or(0);
    (1..=count)
        .filter(|id| accessible_role(&object, *id) == ROLE_SYSTEM_TEXT)
        .map(|id| accessible_value(&object, id))
        .find(|value| !value.is_empty())
}

/// The page as a client reads it, for a failure to print.
fn describe_page(window: HWND) -> Vec<String> {
    match screen_reader(window) {
        Ok(object) => {
            let count = unsafe { object.accChildCount() }.unwrap_or(0);
            accessible_children(&object, count)
        }
        Err(error) => vec![format!("the page could not be read: {error}")],
    }
}

/// A user who cannot see the page is told what the task is doing while it runs,
/// without asking and without a control to reach.
///
/// An install has nothing left to click once it starts, so the words the page
/// publishes and the bar beside them are the whole of what such a user has to go
/// on: the case reads both off the page as a client does, and listens for the
/// announcements that carry them while nobody is asking.
#[test]
fn what_a_running_task_publishes_is_what_a_screen_reader_hears() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.progress_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    // One listener at a time: two cases listening at once read each other's
    // events. See `the_listener`.
    let _listening = the_listener();
    HEARD_LIVE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
    LISTENED_WINDOW.store(window.0 as usize, Ordering::SeqCst);
    // The range covers the events a page's own reports are sent as: the words as
    // a name and a live region, the bar as a value.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_NAMECHANGE,
            EVENT_OBJECT_LIVEREGIONCHANGED,
            None,
            Some(heard_live),
            setup.id(),
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_invalid() {
        let _ = setup.kill();
        let _ = setup.wait();
        anyhow::bail!("the case could not listen for what the wizard announces");
    }

    // The install button the first page places at 560,390.
    click_client_point(window, 620, 408);
    let object = screen_reader(window)?;

    // The first step the script publishes, read off the page: the words are the
    // status line's own name and the percentage is the bar's value.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut described = (String::new(), String::new());
    while Instant::now() < deadline {
        let status = child_with_role(&object, ROLE_SYSTEM_STATICTEXT);
        let progress = child_with_role(&object, ROLE_SYSTEM_PROGRESSBAR);
        if let (Some(status), Some(progress)) = (status, progress) {
            described = (
                accessible_name(&object, status),
                accessible_value(&object, progress),
            );
            if described == ("Copying files".to_string(), "30".to_string()) {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        described,
        ("Copying files".to_string(), "30".to_string()),
        "the page does not say what the task published"
    );

    // A client that listens hears it too, which is what a reader needs: nobody
    // is going to ask whether anything changed while an install runs.
    let heard = pump_until_live(3, Instant::now() + Duration::from_secs(20));
    let status = child_with_role(&object, ROLE_SYSTEM_STATICTEXT);
    let progress = child_with_role(&object, ROLE_SYSTEM_PROGRESSBAR);
    assert!(
        heard.contains(&(EVENT_OBJECT_NAMECHANGE, status.unwrap_or_default())),
        "the words the task published were not announced: {heard:?}"
    );
    assert!(
        heard.contains(&(EVENT_OBJECT_LIVEREGIONCHANGED, status.unwrap_or_default())),
        "the words the task published were not announced as a live region: {heard:?}"
    );
    assert!(
        heard.contains(&(EVENT_OBJECT_VALUECHANGE, progress.unwrap_or_default())),
        "the value of the bar was not announced: {heard:?}"
    );

    // And the second step arrives the same way, so what a user hears is the task
    // moving on rather than one reading of it.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut later = (String::new(), String::new());
    while Instant::now() < deadline {
        if let (Some(status), Some(progress)) = (
            child_with_role(&object, ROLE_SYSTEM_STATICTEXT),
            child_with_role(&object, ROLE_SYSTEM_PROGRESSBAR),
        ) {
            later = (
                accessible_name(&object, status),
                accessible_value(&object, progress),
            );
            if later == ("Finishing up".to_string(), "70".to_string()) {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        later,
        ("Finishing up".to_string(), "70".to_string()),
        "the page did not follow the task to its second step"
    );
    let heard = pump_until_live(6, Instant::now() + Duration::from_secs(20));
    assert!(
        heard.len() >= 6,
        "the second step was not announced: {heard:?}"
    );

    let _ = unsafe { UnhookWinEvent(hook) };
    let _ = setup.kill();
    let _ = setup.wait();
    Ok(())
}

/// A client reading the page while a field is edited does not take the wizard
/// down with it.
///
/// This is what a crash looked like. With a client connected, filling a field in
/// and then emptying it ended the process with `0xc000041d` -- the code Windows
/// reports when an exception escapes a callback it called -- and the faulting
/// instruction was inside `user32`'s `DrawTextW`, reached with a run of text that
/// held no characters. That is what an empty field draws, since the field has the
/// caret and holds nothing, and the pointer an empty buffer has is a dangling one
/// that the text layout dereferences on its way through. Skipping the call for a
/// run with nothing in it is the fix; this case is what would notice it coming
/// back.
#[test]
fn a_client_reading_the_page_does_not_take_the_wizard_down() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.validated_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    // A reader connects and asks for the page the wizard opens on.
    let object = screen_reader(window)?;
    let count = unsafe { object.accChildCount() }.unwrap_or(-1);
    assert!(count > 0, "the page a client reads has no controls at all");

    // The user fills the field in and empties it again, which takes the hint
    // away and brings it back.
    press_client_point(window, 220, 53);
    type_client_text(window, "D:\\E2E");
    std::thread::sleep(Duration::from_millis(400));
    for _ in 0..6 {
        press_key(window, VK_BACK);
    }
    std::thread::sleep(Duration::from_millis(800));

    // The reader asks about the page that came back, which is where the wizard
    // dies today.
    let after = unsafe { object.accChildCount() }.unwrap_or(-1);
    let ended = setup
        .try_wait()
        .ok()
        .flatten()
        .map(|status| status.to_string());
    let _ = setup.kill();
    let _ = setup.wait();
    assert_eq!(
        ended, None,
        "the wizard ended while a client was reading a page a user had just edited"
    );
    assert!(
        after > 0,
        "the page a client reads after editing has no controls"
    );
    Ok(())
}

/// A user who cannot see the page is told which rule their value breaks, which
/// is the only thing that explains a button that does nothing.
///
/// The page draws the hint beside the field and nothing at all while the value is
/// acceptable, so the case types a value that is too short, then one that is
/// fine, and reads both off the page: a reader told about the hint only when it
/// asked would leave the user typing into a dead button.
#[test]
fn the_rule_a_fields_value_breaks_is_what_a_screen_reader_hears() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.validated_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    // One listener at a time: two cases listening at once read each other's
    // events. See `the_listener`.
    let _listening = the_listener();
    HEARD_LIVE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
    LISTENED_WINDOW.store(window.0 as usize, Ordering::SeqCst);
    let hook = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_NAMECHANGE,
            EVENT_OBJECT_LIVEREGIONCHANGED,
            None,
            Some(heard_live),
            setup.id(),
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_invalid() {
        let _ = setup.kill();
        let _ = setup.wait();
        anyhow::bail!("the case could not listen for what the wizard announces");
    }

    let object = screen_reader(window)?;
    let field = child_with_role(&object, ROLE_SYSTEM_TEXT).expect("the page shows a field");
    // The field starts empty and the project asks for it, so the hint is on the
    // page before anything is typed -- drawn, and described against the field.
    // Nothing on this page writes words beside the field, so it is named by the
    // id its layout gave it, which is the fallback the description is built on.
    assert_eq!(accessible_name(&object, field), "editDir");
    assert_eq!(
        accessible_hint(&object),
        Some("Choose a folder".to_string())
    );
    assert_eq!(
        accessible_description(&object, field),
        "Choose a folder",
        "the rule a value breaks is not part of what a client is told about the field"
    );

    // A value that breaks the other rule the page declares replaces the words, and
    // the change is announced: nothing else says why the button is inert.
    press_client_point(window, 220, 53);
    type_client_text(window, "abc");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline
        && accessible_hint(&object) != Some("At least five characters".to_string())
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        accessible_hint(&object),
        Some("At least five characters".to_string()),
        "the page does not say which rule the value breaks; the field holds {:?}",
        accessible_value(&object, field)
    );
    let hint = child_with_role(&object, ROLE_SYSTEM_STATICTEXT).expect("the hint is on the page");
    let heard = pump_until_live(2, Instant::now() + Duration::from_secs(20));
    assert!(
        heard.contains(&(EVENT_OBJECT_NAMECHANGE, hint)),
        "the rule the value broke was not announced: {heard:?}"
    );
    assert!(
        heard.contains(&(EVENT_OBJECT_LIVEREGIONCHANGED, hint)),
        "the rule the value broke was not announced as a live region: {heard:?}"
    );

    // And a value the project accepts takes the hint away, from the page and from
    // the field's own description.
    type_client_text(window, "defgh");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && accessible_hint(&object).is_some() {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        accessible_hint(&object),
        None,
        "the hint stayed on the page after the value became acceptable; the field holds {:?}",
        accessible_value(&object, field)
    );
    assert_eq!(accessible_description(&object, field), "");

    let _ = unsafe { UnhookWinEvent(hook) };
    let _ = setup.kill();
    let _ = setup.wait();
    Ok(())
}

/// A user typing into a field is heard while they type, not once when they stop.
///
/// The directory the wizard installs into is a field like any other: what a reader
/// follows as someone types is the value changing, once per keystroke. A key that
/// only moves the caret inside the text changes nothing, and telling a client that
/// it did would make a reader repeat the whole field on every arrow key -- so the
/// case listens for value changes alone, types three characters, and then moves
/// the caret with none of them arriving.
#[test]
fn a_field_the_user_types_in_is_what_a_screen_reader_hears() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.validated_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    // One listener at a time: two cases listening at once read each other's
    // events. See `the_listener`.
    let _listening = the_listener();
    HEARD_LIVE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
    LISTENED_WINDOW.store(window.0 as usize, Ordering::SeqCst);
    // A client that listens for value changes and nothing else: the hint the page
    // puts up while the value is too short is announced too, and a case that
    // counted those would be counting the wrong thing.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_VALUECHANGE,
            EVENT_OBJECT_VALUECHANGE,
            None,
            Some(heard_live),
            setup.id(),
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_invalid() {
        let _ = setup.kill();
        let _ = setup.wait();
        anyhow::bail!("the case could not listen for what the wizard announces");
    }

    let object = screen_reader(window)?;
    let field = child_with_role(&object, ROLE_SYSTEM_TEXT).expect("the page shows a field");
    // The field is the one the page asks for a directory in; clicking it is what
    // puts the caret in it, exactly as a user does.
    press_client_point(window, 220, 53);

    type_client_text(window, "abc");
    let typed = pump_until_live(3, Instant::now() + Duration::from_secs(20));
    assert_eq!(
        accessible_value(&object, field),
        "abc",
        "the field does not hold what was typed into it"
    );
    let announced = typed
        .iter()
        .filter(|(event, child)| *event == EVENT_OBJECT_VALUECHANGE && *child == field)
        .count();
    assert!(
        announced >= 3,
        "three characters were typed and the field was announced {announced} time(s): {typed:?}"
    );

    // Moving inside the text is not a change to it, and a reader told otherwise
    // would repeat the field on every arrow key.
    let after_typing = typed.len();
    press_key(window, VK_LEFT);
    let later = pump_until_live(after_typing + 1, Instant::now() + Duration::from_secs(3));
    assert_eq!(
        later.len(),
        after_typing,
        "moving the caret announced the value again: {later:?}"
    );

    let _ = unsafe { UnhookWinEvent(hook) };
    let _ = setup.kill();
    let _ = setup.wait();
    Ok(())
}

/// The same object reached the other way: a client that asks the window for
/// `IDispatch` and calls members by name rather than through `IAccessible`.
fn late_binding(window: HWND) -> anyhow::Result<IDispatch> {
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let mut raw: *mut core::ffi::c_void = std::ptr::null_mut();
    unsafe {
        AccessibleObjectFromWindow(
            window,
            OBJID_CLIENT.0 as u32,
            &IDispatch::IID,
            &mut raw as *mut *mut core::ffi::c_void,
        )
    }
    .context("the wizard did not describe itself to a late-binding client")?;
    Ok(unsafe { IDispatch::from_raw(raw) })
}

/// Calls a member of the object by name, the way a late-binding client does.
///
/// The name is turned into the number MSAA gives that member and the number is
/// then called: that is the whole of what late binding is. A member that takes no
/// child is called with no argument at all rather than with one that means the
/// window.
fn by_name(dispatch: &IDispatch, member: &str, id: Option<i32>) -> anyhow::Result<VARIANT> {
    let name: Vec<u16> = member.encode_utf16().chain(std::iter::once(0)).collect();
    let mut dispid = 0i32;
    unsafe { dispatch.GetIDsOfNames(&GUID::zeroed(), &PCWSTR(name.as_ptr()), 1, 0, &mut dispid) }
        .with_context(|| format!("{member} is not a member this object has"))?;
    let mut argument = child(id.unwrap_or(0));
    let params = DISPPARAMS {
        rgvarg: &mut argument,
        rgdispidNamedArgs: std::ptr::null_mut(),
        cArgs: u32::from(id.is_some()),
        cNamedArgs: 0,
    };
    let mut result = VARIANT::new();
    unsafe {
        dispatch.Invoke(
            dispid,
            &GUID::zeroed(),
            0,
            DISPATCH_PROPERTYGET,
            &params,
            Some(&mut result as *mut VARIANT),
            None,
            None,
        )
    }
    .with_context(|| format!("{member} could not be called"))?;
    Ok(result)
}

/// A client that asks for members by name gets the same page.
///
/// `IAccessible` is the interface every Windows reader calls, but a client may
/// reach the same object through `IDispatch` and name what it wants instead:
/// late binding is a way of calling the interface rather than a second one, so
/// this case asks by name what the case above asks through the interface and
/// compares the answers. A name MSAA does not have, and a member this object
/// does not serve, are both refused rather than answered with something made up.
#[test]
fn a_client_asking_by_name_is_told_the_same_page() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.accessibility_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    let object = screen_reader(window)?;
    let dispatch = late_binding(window)?;

    // The number of children is what a client asks first, and it takes no child
    // of its own.
    let count = by_name(&dispatch, "accChildCount", None)?;
    assert_eq!(
        i32::try_from(&count)?,
        unsafe { object.accChildCount() }?,
        "the two ways of asking do not agree on how many controls the page has"
    );

    // The field is the first control: what it is called and what it is.
    let name = by_name(&dispatch, "accName", Some(1))?;
    assert_eq!(
        BSTR::try_from(&name)?.to_string(),
        accessible_name(&object, 1),
        "the name of the first control is not the one the interface gives"
    );
    let role = by_name(&dispatch, "accRole", Some(1))?;
    assert_eq!(
        i32::try_from(&role)?,
        accessible_role(&object, 1) as i32,
        "the role of the first control is not the one the interface gives"
    );

    // A name MSAA has no member for is refused, which is how a client learns it
    // spelled it wrong rather than being handed an empty answer.
    assert!(
        by_name(&dispatch, "accNothingOfTheSort", None).is_err(),
        "a member MSAA does not have was answered"
    );
    // And a member the interface itself does not serve is refused the same way
    // through either door: there is no selection for a client to read.
    assert!(
        by_name(&dispatch, "accSelection", None).is_err(),
        "the selection was answered by a page that has none"
    );

    let _ = setup.kill();
    let _ = setup.wait();
    Ok(())
}

/// The client that listens for what a wizard announces, which only one case can
/// be at a time.
///
/// A hook and the list it fills are process-wide, and so is the window they are
/// filtered by: two cases listening at once would each be handed the other's
/// events, and the case that expected its own would read the other page -- which
/// is how one of them failed when another case started listening beside it. They
/// take this in turn however the suite happens to schedule them.
static LISTENER: Mutex<()> = Mutex::new(());

fn the_listener() -> MutexGuard<'static, ()> {
    LISTENER.lock().unwrap_or_else(|error| error.into_inner())
}

/// The focus events a client listening for them has heard.
///
/// The callback runs on the thread that installed the hook, while the case
/// below pumps that thread's message queue.
static HEARD: Mutex<Vec<i32>> = Mutex::new(Vec::new());

/// What a client listening for the task's own reports has heard, as the event
/// and the child it was about.
static HEARD_LIVE: Mutex<Vec<(u32, i32)>> = Mutex::new(Vec::new());

/// What a listening client hears about the task that is running: the words a
/// page publishes and the value of the bar beside them.
unsafe extern "system" fn heard_live(
    _hook: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    if object != OBJID_CLIENT.0 || window.0 as usize != LISTENED_WINDOW.load(Ordering::SeqCst) {
        return;
    }
    if let Ok(mut heard) = HEARD_LIVE.lock() {
        heard.push((event, child));
    }
}

/// The window the case is listening for, as the integer a static can hold.
static LISTENED_WINDOW: AtomicUsize = AtomicUsize::new(0);

/// What a listening client hears: the focus events the wizard announced for the
/// window the case is watching.
unsafe extern "system" fn heard_focus(
    _hook: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    // A hook installed for one process still sees every window that process
    // owns, so the window is what says the event belongs to this run.
    if event != EVENT_OBJECT_FOCUS
        || object != OBJID_CLIENT.0
        || window.0 as usize != LISTENED_WINDOW.load(Ordering::SeqCst)
    {
        return;
    }
    if let Ok(mut heard) = HEARD.lock() {
        heard.push(child);
    }
}

/// The controls a client heard announced.
///
/// Windows announces the window itself taking the keyboard as child 0, and that
/// is not a control moving: what these cases are about is the controls the wizard
/// moved to, in the order it moved to them.
fn announced_controls(heard: &[i32]) -> Vec<i32> {
    heard.iter().copied().filter(|child| *child != 0).collect()
}

/// Pumps until the client has heard `wanted` controls announced.
fn pump_until_controls(wanted: usize, deadline: Instant) -> Vec<i32> {
    let mut heard = Vec::new();
    while Instant::now() < deadline {
        heard = pump_until_heard(wanted, Instant::now() + Duration::from_millis(50));
        if announced_controls(&heard).len() >= wanted {
            break;
        }
        // The list can hold an event that is not a control moving and nothing
        // else, in which case the pump above returns at once; waiting here keeps
        // this from becoming a loop that only reads its own list.
        std::thread::sleep(Duration::from_millis(20));
    }
    announced_controls(&heard)
}

/// Pumps this thread's messages until the client has heard `wanted` events.
///
/// An out-of-context hook is delivered by posting to the thread that installed
/// it, so waiting for one is waiting for that thread's queue to be read.
fn pump_until_heard(wanted: usize, deadline: Instant) -> Vec<i32> {
    let mut message = MSG::default();
    while Instant::now() < deadline {
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            let _ = unsafe { TranslateMessage(&message) };
            unsafe { DispatchMessageW(&message) };
        }
        {
            let heard = HEARD.lock().unwrap_or_else(|error| error.into_inner());
            if heard.len() >= wanted {
                return heard.clone();
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    HEARD
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

/// A user who cannot see the page is told where the keyboard went, without
/// having to ask: the wizard announces the focus the way a control with a
/// window class does, and a reader that only answered questions would leave the
/// user guessing.
///
/// The client here is an ordinary one in another process, listening for the
/// events Windows delivers to it rather than reading the page back.
#[test]
fn the_keyboard_moving_is_what_a_screen_reader_hears() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.accessibility_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let Some(window) = wait_for_a_window(&mut setup)? else {
        return Ok(());
    };

    // One listener at a time: two cases listening at once read each other's
    // events. See `the_listener`.
    let _listening = the_listener();
    HEARD
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
    LISTENED_WINDOW.store(window.0 as usize, Ordering::SeqCst);
    // The hook is told which process to listen to, so the case hears the wizard
    // and nothing else on the machine.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_FOCUS,
            EVENT_OBJECT_FOCUS,
            None,
            Some(heard_focus),
            setup.id(),
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_invalid() {
        let _ = setup.kill();
        let _ = setup.wait();
        anyhow::bail!("the case could not listen for what the wizard announces");
    }

    // The first Tab lands on the field, and the wizard says so: the client
    // hears the number of the control the keyboard reached. Windows announces
    // the window itself taking the keyboard as child 0, which is not a control
    // moving and is filtered out here -- on a busy desk it can arrive first, and
    // a case that read the first event of any kind would fail on it.
    let deadline = Instant::now() + Duration::from_secs(20);
    press_key(window, VK_TAB);
    let heard = pump_until_controls(1, deadline);
    assert_eq!(
        heard,
        vec![1],
        "the wizard did not announce the control the keyboard reached"
    );

    // The page itself agrees about where the keyboard is.
    let object = screen_reader(window)?;
    assert_eq!(accessible_focus(&object), Some(1));

    // And the next Tab is announced in its turn: a reader hears every move
    // rather than the first one only.
    press_key(window, VK_TAB);
    let heard = pump_until_controls(2, deadline);
    assert_eq!(
        heard,
        vec![1, 2],
        "the second move of the keyboard was not announced"
    );
    assert_eq!(accessible_focus(&object), Some(2));

    let _ = unsafe { UnhookWinEvent(hook) };
    let _ = setup.kill();
    let _ = setup.wait();
    Ok(())
}

/// A machine whose user has high contrast on gets the scheme that user picked
/// instead of the colours the page declares.
///
/// A page this runtime paints is a picture, so nothing in it follows the setting
/// on its own: a surface, a line, a word, and the ring around the focused
/// control all have to be painted with the colour the machine names for them,
/// and a theme is free to give two of those roles one colour. The fixture
/// declares the machine's own colours swapped, so the frame says which set was
/// painted with, and the same fixture is run with the answer stated each way --
/// the setting belongs to whoever is at the machine, so a case asks the runtime
/// to paint as such a machine would rather than turning it on for them.
#[test]
fn a_high_contrast_machine_gets_the_colours_the_scheme_keeps() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.contrast_project()?;
    fixture.build()?;

    // What the machine keeps, and what the page declares in its place: a scheme
    // never names one colour for a page and for the text on it, so a frame can
    // only show one of the two sets.
    let page = system_colour(COLOR_WINDOW);
    let text = system_colour(COLOR_WINDOWTEXT);
    let face = system_colour(COLOR_BTNFACE);
    let edge = system_colour(COLOR_WINDOWFRAME);
    let band = system_colour(COLOR_HIGHLIGHT);

    // Where the case reads: the page away from everything on it, the middle of
    // the card, the card's own edge, and the picture the checkbox carries.
    let empty = (350, 170);
    let card = (140, 80);
    let card_edge = (40, 60);
    let picture = (52, 32);
    let terms = (40, 20);

    let _ = unsafe { SetProcessDPIAware() };

    // With the setting off, the page is painted the way it was written.
    let mut off = SetupGuard::spawn_stating_contrast(&fixture.setup, false)?;
    let Some(window) = wait_for_a_window(&mut off)? else {
        return Ok(());
    };
    let declared = wait_for_frame(window, Instant::now() + Duration::from_secs(20), |frame| {
        frame.colour_at(empty.0, empty.1) == text
            && frame.colour_at(card.0, card.1) == band
            && frame.colour_at(card_edge.0, card_edge.1) == band
    });
    assert!(
        declared.is_some(),
        "the page was not painted as it was declared: page {} card {} edge {}",
        colour_at_window(window, empty),
        colour_at_window(window, card),
        colour_at_window(window, card_edge)
    );
    let declared = declared.expect("checked just above");

    // The picture the checkbox carries is the product's artwork rather than a
    // colour the scheme has a name for, so it is the one thing the setting
    // cannot move.
    assert_eq!(
        declared.colour_at(picture.0, picture.1),
        UNCHECKED_BOX,
        "the artwork was not painted the way it was drawn"
    );

    // With the setting on, the same page is painted with the scheme: the page in
    // the colour of a page, the card in the colour of a control face, and its
    // line in the colour of a frame.
    let mut on = SetupGuard::spawn_stating_contrast(&fixture.setup, true)?;
    let Some(window) = wait_for_a_window(&mut on)? else {
        return Ok(());
    };
    let painted = wait_for_frame(window, Instant::now() + Duration::from_secs(20), |frame| {
        frame.colour_at(empty.0, empty.1) == page
            && frame.colour_at(card.0, card.1) == face
            && frame.colour_at(card_edge.0, card_edge.1) == edge
    });
    assert!(
        painted.is_some(),
        "the page was not painted with the scheme: page {} card {} edge {}",
        colour_at_window(window, empty),
        colour_at_window(window, card),
        colour_at_window(window, card_edge)
    );
    assert_eq!(
        painted
            .expect("checked just above")
            .colour_at(picture.0, picture.1),
        UNCHECKED_BOX,
        "the scheme painted over the artwork the page carries"
    );

    // The ring is the one thing the keyboard brings, and under the setting it
    // takes the colour the user picked to be seen rather than the page's own.
    let deadline = Instant::now() + Duration::from_secs(20);
    press_key(window, VK_TAB);
    let ring = wait_for_frame(window, deadline, |frame| {
        frame.colour_at(terms.0, terms.1) == band
    });
    assert!(
        ring.is_some(),
        "the ring was not painted with the scheme's highlight: {}",
        colour_at_window(window, terms)
    );

    // Windows tells every window when the user turns the setting on or off, or
    // picks another scheme, and the page is laid out again for it. The answer
    // cannot change under a case, which states it for the process, but the
    // window has to take the message and keep painting what it was painted with.
    let before = capture_frame(window)?;
    let _ = unsafe {
        PostMessageW(
            window,
            WM_SETTINGCHANGE,
            WPARAM(SPI_SETHIGHCONTRAST.0 as usize),
            LPARAM(0),
        )
    };
    std::thread::sleep(Duration::from_millis(200));
    let after = capture_frame(window)?;
    assert_eq!(
        after.differing_pixels(&before),
        0,
        "the page changed under a message that had nothing to change"
    );
    assert!(
        unsafe { IsWindow(window) }.as_bool(),
        "the window went away when the machine said the setting changed"
    );

    Ok(())
}

/// The button that fills a field in opens Windows' own folder picker, and
/// leaving the picker undecided leaves the wizard as it was.
///
/// The picker is the shell's rather than the product's, so what a case can hold
/// it to is that clicking the button opens a window of the process's own for
/// choosing a folder, that the wizard is still underneath it, and that closing
/// the picker keeps the wizard, its size and its process where they were.
#[test]
fn a_browse_button_opens_the_folder_picker_and_leaving_it_changes_nothing() -> anyhow::Result<()> {
    let Some(fixture) = Fixture::new(PayloadFormat::Zip, true, true) else {
        skip_missing_stubs()?;
        return Ok(());
    };
    fixture.browse_project()?;
    fixture.build()?;

    let _ = unsafe { SetProcessDPIAware() };
    let mut setup = SetupGuard::spawn(&fixture.setup)?;
    let waited = wait_for_runtime_window(&mut setup, Instant::now() + Duration::from_secs(30));
    let found = match &waited {
        WindowWait::Found(window) => Some(*window),
        WindowWait::Exited(_) | WindowWait::Timeout => None,
    };
    let Some(window) = found else {
        let reason = match waited {
            WindowWait::Exited(status) => format!("the setup {status} instead of opening a window"),
            WindowWait::Timeout => "no window appeared within 30 seconds".to_string(),
            WindowWait::Found(_) => "the window could not be measured".to_string(),
        };
        let _ = setup.kill();
        let _ = setup.wait();
        return skip_missing_desktop(&reason);
    };
    let opened_on = client_size(window);

    // The browse button is 80 by 24 at (300, 20).
    click_client_point(window, 340, 32);
    let picker =
        wait_for_other_window(setup.id(), window, Instant::now() + Duration::from_secs(30));
    // The picker is modal: the wizard waits inside it, so a case that cannot
    // close it would leave the setup running for good.
    let closed = match &picker {
        Some((dialog, _)) => dismiss_window(*dialog, Instant::now() + Duration::from_secs(20)),
        None => false,
    };
    let alive = unsafe { IsWindow(window) }.as_bool();
    let after = if alive { client_size(window) } else { (0, 0) };
    let running = setup.try_wait()?.is_none();

    let Some((_, class)) = picker else {
        anyhow::bail!("the browse button opened no window for choosing a folder");
    };
    assert_eq!(
        class, "#32770",
        "the window the browse button opened is not the shell's own folder dialog"
    );
    assert!(
        closed,
        "the folder picker did not close when it was asked to"
    );
    assert!(alive, "the wizard was gone once the folder picker closed");
    assert_eq!(
        after, opened_on,
        "the wizard came back from the folder picker with a different client area"
    );
    assert!(running, "the setup process ended with the folder picker");
    assert!(
        !fixture.destination.exists(),
        "opening the folder picker installed something"
    );
    Ok(())
}

/// A frame of a window: what the window draws, read out of the window itself.
struct Frame {
    width: i32,
    pixels: Vec<u8>,
}

impl Frame {
    /// The red, green and blue at one point.
    ///
    /// A device-independent bitmap is blue first and keeps no alpha channel of
    /// its own, so the fourth byte of a pixel is not read.
    fn colour_at(&self, x: i32, y: i32) -> (u8, u8, u8) {
        let offset = ((y as usize * self.width as usize) + x as usize) * 4;
        (
            self.pixels[offset + 2],
            self.pixels[offset + 1],
            self.pixels[offset],
        )
    }

    /// How many pixels two frames disagree on.
    fn differing_pixels(&self, other: &Frame) -> usize {
        self.differing_points(other).count()
    }

    /// How many pixels differ outside `area`, as `(left, top, right, bottom)`.
    fn differences_outside(&self, other: &Frame, area: (i32, i32, i32, i32)) -> usize {
        self.differences_outside_all(other, &[area])
    }

    /// How many pixels differ outside every one of `areas`.
    fn differences_outside_all(&self, other: &Frame, areas: &[(i32, i32, i32, i32)]) -> usize {
        self.differing_points(other)
            .filter(|(x, y)| {
                !areas.iter().any(|(left, top, right, bottom)| {
                    x >= left && x < right && y >= top && y < bottom
                })
            })
            .count()
    }

    /// How many pixels differ inside `area`.
    fn differences_inside(&self, other: &Frame, area: (i32, i32, i32, i32)) -> usize {
        let (left, top, right, bottom) = area;
        self.differing_points(other)
            .filter(|(x, y)| *x >= left && *x < right && *y >= top && *y < bottom)
            .count()
    }

    /// Where two frames of the same size disagree.
    fn differing_points<'a>(&'a self, other: &'a Frame) -> impl Iterator<Item = (i32, i32)> + 'a {
        let width = self.width;
        self.pixels
            .chunks_exact(4)
            .zip(other.pixels.chunks_exact(4))
            .enumerate()
            .filter(|(_, (left, right))| left != right)
            .map(move |(index, _)| {
                let index = index as i32;
                (index % width, index / width)
            })
    }
}

/// Reads what a window is showing.
///
/// `PrintWindow` asks Windows to draw the window into a bitmap rather than
/// copying the pixels that happen to be on the screen under it, so a case reads
/// the frame the window would show even when the desktop has something else on
/// top of it. The window paints into that bitmap with the same code it paints
/// the screen with.
fn capture_frame(window: HWND) -> anyhow::Result<Frame> {
    let (width, height) = client_size(window);
    anyhow::ensure!(width > 0 && height > 0, "the window has no client area");
    unsafe {
        let screen = GetDC(HWND::default());
        anyhow::ensure!(!screen.is_invalid(), "the desktop has no device context");
        let memory = CreateCompatibleDC(HDC::default());
        if memory.is_invalid() {
            let _ = ReleaseDC(HWND::default(), screen);
            anyhow::bail!("a memory device context could not be created");
        }
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let frame: anyhow::Result<Frame> = (|| {
            if bitmap.is_invalid() {
                anyhow::bail!("a {width} by {height} bitmap could not be created");
            }
            let previous: HGDIOBJ = SelectObject(memory, bitmap);
            let drawn = PrintWindow(window, memory, PRINT_WINDOW_FLAGS(0));
            // The bitmap has to be out of the device context before its pixels
            // are read back.
            let _ = SelectObject(memory, previous);
            anyhow::ensure!(drawn.as_bool(), "the window drew no frame");
            let mut header = BITMAPINFO::default();
            header.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            header.bmiHeader.biWidth = width;
            // A negative height asks for the top row of the bitmap first, so the
            // pixels come back the way up the window is.
            header.bmiHeader.biHeight = -height;
            header.bmiHeader.biPlanes = 1;
            header.bmiHeader.biBitCount = 32;
            header.bmiHeader.biCompression = BI_RGB.0;
            let mut pixels = vec![0u8; width as usize * height as usize * 4];
            let rows = GetDIBits(
                memory,
                bitmap,
                0,
                height as u32,
                Some(pixels.as_mut_ptr().cast()),
                &mut header,
                DIB_RGB_COLORS,
            );
            anyhow::ensure!(
                rows == height,
                "the window drew {rows} of its {height} rows"
            );
            Ok(Frame { width, pixels })
        })();
        if !bitmap.is_invalid() {
            let _ = DeleteObject(bitmap);
        }
        let _ = DeleteDC(memory);
        let _ = ReleaseDC(HWND::default(), screen);
        frame
    }
}

/// Waits for the window to show a frame `ready` accepts, and hands that frame
/// back.
///
/// A window that has just opened has not painted yet and a state a posted
/// message asked for arrives a moment later, so this waits rather than looking
/// once. The frame a caller gets is the one it asked for: a deadline that runs
/// out hands back nothing.
fn wait_for_frame(
    window: HWND,
    deadline: Instant,
    ready: impl Fn(&Frame) -> bool,
) -> Option<Frame> {
    while Instant::now() < deadline {
        if let Ok(frame) = capture_frame(window) {
            if ready(&frame) {
                return Some(frame);
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}

/// What a window holds at one point right now, for a failure that has to say
/// what it saw instead.
fn colour_at_window(window: HWND, at: (i32, i32)) -> String {
    match capture_frame(window) {
        Ok(frame) => format!("{:?} at ({}, {})", frame.colour_at(at.0, at.1), at.0, at.1),
        Err(error) => format!("nothing readable at ({}, {}): {error:#}", at.0, at.1),
    }
}

/// Posts a pointer move to a point in the runtime window.
///
/// A mouse message carries client coordinates, which is what the runtime reads
/// a hover from.
fn hover_client_point(window: HWND, x: i32, y: i32) {
    let lparam = LPARAM(((y << 16) | (x & 0xFFFF)) as isize);
    unsafe { PostMessageW(window, WM_MOUSEMOVE, WPARAM(0), lparam) }
        .expect("the runtime window accepts a pointer move");
}

/// Puts the pointer on a point in a window, and reports whether it could.
///
/// The runtime answers `WM_SETCURSOR` by asking where the pointer is, so the
/// pointer has to be moved for real: the point is converted to screen
/// coordinates and put under the pointer.
fn put_pointer_on(window: HWND, at: (i32, i32)) -> bool {
    let mut point = POINT { x: at.0, y: at.1 };
    if !unsafe { ClientToScreen(window, &mut point) }.as_bool() {
        return false;
    }
    unsafe { SetCursorPos(point.x, point.y) }.is_ok()
}

/// Puts the pointer on a point in the window and tells the window it is there.
///
/// The pointer moves for real, which is what the window's own tracking of the
/// pointer follows, and the move is posted as well, because a pointer that is
/// already on the point moves nowhere and Windows would report no move at all.
fn move_pointer_onto(window: HWND, at: (i32, i32)) -> anyhow::Result<()> {
    anyhow::ensure!(
        put_pointer_on(window, at),
        "the pointer can be moved onto the window"
    );
    hover_client_point(window, at.0, at.1);
    Ok(())
}

/// Moves the pointer off a window, which is what makes Windows report that it
/// left.
///
/// The corner of the desktop is off a wizard centred on the display, which is
/// where every fixture window is. A window that reaches that corner is left by
/// a point under its bottom edge instead.
fn move_pointer_off(window: HWND) -> anyhow::Result<()> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(window, &mut rect) }.context("the window has a rectangle")?;
    let corner = (2, 2);
    let corner_is_inside = corner.0 >= rect.left
        && corner.0 < rect.right
        && corner.1 >= rect.top
        && corner.1 < rect.bottom;
    let (x, y) = if corner_is_inside {
        let below = rect.bottom + 8;
        anyhow::ensure!(
            below < unsafe { GetSystemMetrics(SM_CYSCREEN) },
            "there is nowhere on this desktop to move the pointer off the window"
        );
        ((rect.left + rect.right) / 2, below)
    } else {
        corner
    };
    unsafe { SetCursorPos(x, y) }.context("the pointer can be moved off the window")
}

/// Presses a key in the runtime window, the way the keyboard reports one.
///
/// The runtime reads the key code out of the message, so nothing has to hold
/// the keyboard focus for this to arrive. No character follows, because a key
/// that types has to be asked for by name.
fn press_key(window: HWND, key: VIRTUAL_KEY) {
    unsafe { PostMessageW(window, WM_KEYDOWN, WPARAM(key.0 as usize), LPARAM(0)) }
        .expect("the runtime window accepts a key");
}

/// The pointer the desktop is showing, or `None` when it cannot be read.
fn shown_cursor() -> Option<HCURSOR> {
    let mut info = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetCursorInfo(&mut info) }.ok()?;
    Some(info.hCursor)
}

/// Puts the pointer on a point in the window and waits for the cursor it should
/// show there.
///
/// The runtime answers `WM_SETCURSOR` by asking where the pointer is, so the
/// pointer has to move for real: the point is converted to screen coordinates
/// and put under the pointer, and then the message is posted, which is the pair
/// of things the system does when a user moves the mouse over a window. The
/// message is posted more than once, because the pointer arriving on the window
/// makes Windows set the cursor from the window class first, and the answer
/// this case is after comes after that.
fn wait_for_cursor(
    window: HWND,
    at: (i32, i32),
    expected: HCURSOR,
    deadline: Instant,
) -> Option<HCURSOR> {
    if !put_pointer_on(window, at) {
        return None;
    }
    let mut shown = None;
    while Instant::now() < deadline {
        let _ = unsafe {
            PostMessageW(
                window,
                WM_SETCURSOR,
                WPARAM(window.0 as usize),
                LPARAM(HTCLIENT as isize),
            )
        };
        shown = shown_cursor();
        if shown == Some(expected) {
            return shown;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    shown
}

/// Everything a cursor failure has to explain: which shape the desktop is
/// showing, which of the three names it is, and whether the window the case is
/// about is the one in front, which is what decides whether the shape its
/// thread sets is the shape the desktop shows.
fn cursor_report(window: HWND, hand: HCURSOR, beam: HCURSOR, arrow: HCURSOR) -> String {
    let shown = shown_cursor();
    let shape = match shown {
        Some(shape) if shape == hand => "the hand",
        Some(shape) if shape == beam => "the beam",
        Some(shape) if shape == arrow => "the arrow",
        Some(_) => "a cursor none of the three names",
        None => "nothing readable",
    };
    let front = unsafe { GetForegroundWindow() } == window;
    format!(
        "the desktop is showing {shape} ({shown:?}); the hand is {hand:?}, the beam {beam:?} \
         and the arrow {arrow:?}; the window in front is {}",
        if front { "this one" } else { "another one" }
    )
}

/// Holds a window above the others for as long as a case needs it there, and
/// puts it back afterwards.
///
/// The shape the desktop shows is the one the window under the pointer asks
/// for, and the suite runs its window cases at the same time, so a case about
/// the cursor has to be the window the pointer is really over: the wizard's
/// own client point is where the pointer is put, and something else may be
/// covering that part of the desk.
struct WindowInFront(HWND);

impl WindowInFront {
    /// Lifts a window above every other one, without taking the keyboard.
    fn lift(window: HWND) -> Self {
        let _ = unsafe {
            SetWindowPos(
                window,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
        Self(window)
    }
}

impl Drop for WindowInFront {
    fn drop(&mut self) {
        let _ = unsafe {
            SetWindowPos(
                self.0,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
    }
}

/// Puts the pointer back where a case found it.
///
/// A case that moves the pointer for real is only borrowing it: the point it
/// was on is remembered before the first move and put back when the case ends,
/// an assertion that failed included.
struct PointerRestore(POINT);

impl PointerRestore {
    /// Remembers where the pointer is now.
    fn capture() -> Self {
        let mut point = POINT::default();
        let _ = unsafe { GetCursorPos(&mut point) };
        Self(point)
    }
}

impl Drop for PointerRestore {
    fn drop(&mut self) {
        let _ = unsafe { SetCursorPos(self.0.x, self.0.y) };
    }
}

/// The top-level windows one process is showing, as `(handle, class)`.
///
/// The windows a process is showing, rather than every window it owns: a
/// process with an input method loaded also owns helper windows that are never
/// put on screen, and one of those standing in for a dialog would say nothing
/// about what a click opened.
fn shown_windows(process_id: u32) -> Vec<(HWND, String)> {
    let mut lookup = ProcessWindowList {
        process_id,
        found: Vec::new(),
    };
    let parameter = LPARAM(&mut lookup as *mut ProcessWindowList as isize);
    // A machine with no window station fails the walk, which reads the same as a
    // walk that found nothing.
    let _ = unsafe { EnumWindows(Some(visit_every_window), parameter) };
    lookup.found
}

/// What one walk over the top-level windows of a process collects.
struct ProcessWindowList {
    process_id: u32,
    found: Vec<(HWND, String)>,
}

/// The callback the walk runs per top-level window, for every window of the
/// process rather than for the runtime's alone.
unsafe extern "system" fn visit_every_window(window: HWND, parameter: LPARAM) -> BOOL {
    let lookup = &mut *(parameter.0 as *mut ProcessWindowList);
    let mut owner = 0u32;
    GetWindowThreadProcessId(window, Some(&mut owner));
    if owner == lookup.process_id && IsWindowVisible(window).as_bool() {
        lookup.found.push((window, window_class(window)));
    }
    BOOL(1)
}

/// Waits for a process to show a second top-level window beside the wizard.
///
/// A folder picker is a window of its own rather than a card inside the wizard,
/// so this is what says that the button that asks for one really asked Windows
/// for it.
fn wait_for_other_window(
    process_id: u32,
    wizard: HWND,
    deadline: Instant,
) -> Option<(HWND, String)> {
    while Instant::now() < deadline {
        if let Some(found) = shown_windows(process_id)
            .into_iter()
            .find(|(window, _)| *window != wizard)
        {
            return Some(found);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

/// Closes a window the case did not open, and reports whether it went away.
///
/// The folder picker is modal: the wizard waits inside it until the dialog
/// answers, so a case that cannot close one leaves the setup running for good.
/// `WM_CLOSE` is what a dialog answers to, and one that is still there a moment
/// later is offered the Escape key the shell also reads as a cancel.
fn dismiss_window(window: HWND, deadline: Instant) -> bool {
    let _ = unsafe { PostMessageW(window, WM_CLOSE, WPARAM(0), LPARAM(0)) };
    let offer_escape_from = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if !unsafe { IsWindow(window) }.as_bool() {
            return true;
        }
        if Instant::now() >= offer_escape_from {
            let _ = unsafe {
                PostMessageW(window, WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0))
            };
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// A PNG holding one flat colour, which is what a case uses for the artwork a
/// button draws in each of its states.
///
/// The fixture writes its own pictures rather than carrying any, so a case that
/// needs artwork writes it here. The file is a real PNG, because that is what
/// the builder packs and what the runtime decodes, and it is written by hand
/// because the test build carries no image crate: the picture is four pixels of
/// one colour, so a stored-block deflate stream is smaller to write than a
/// dependency is to add.
fn solid_png(width: u32, height: u32, colour: (u8, u8, u8)) -> Vec<u8> {
    let mut raw = Vec::with_capacity((height * (width * 3 + 1)) as usize);
    // Every row starts with the filter it is stored under, and this one is
    // stored as it is, so every row is the same row.
    let mut row = Vec::with_capacity((width * 3 + 1) as usize);
    row.push(0);
    for _ in 0..width {
        row.extend_from_slice(&[colour.0, colour.1, colour.2]);
    }
    for _ in 0..height {
        raw.extend_from_slice(&row);
    }
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // Eight bits per channel, truecolour, no interlacing.
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&png_chunk(b"IHDR", &header));
    png.extend_from_slice(&png_chunk(b"IDAT", &zlib_stored(&raw)));
    png.extend_from_slice(&png_chunk(b"IEND", &[]));
    png
}

/// The zlib stream a PNG's image data sits in, written as stored blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    // The two bytes every zlib stream starts with: deflate, with the window
    // size and the check bits a stored stream uses.
    let mut stream = vec![0x78, 0x01];
    let mut remaining = data;
    while !remaining.is_empty() {
        let take = remaining.len().min(0xFFFF);
        let (block, rest) = remaining.split_at(take);
        stream.push(u8::from(rest.is_empty()));
        stream.extend_from_slice(&(take as u16).to_le_bytes());
        stream.extend_from_slice(&(!(take as u16)).to_le_bytes());
        stream.extend_from_slice(block);
        remaining = rest;
    }
    stream.extend_from_slice(&adler32(data).to_be_bytes());
    stream
}

/// The Adler-32 checksum zlib ends a stream with.
fn adler32(data: &[u8]) -> u32 {
    let (mut low, mut high) = (1u32, 0u32);
    for byte in data {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

/// One chunk of a PNG: its length, its name, its body, and the checksum over
/// the last two.
fn png_chunk(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut chunk = Vec::with_capacity(body.len() + 12);
    chunk.extend_from_slice(&(body.len() as u32).to_be_bytes());
    chunk.extend_from_slice(kind);
    chunk.extend_from_slice(body);
    let mut checked = Vec::with_capacity(body.len() + 4);
    checked.extend_from_slice(kind);
    checked.extend_from_slice(body);
    chunk.extend_from_slice(&crc32(&checked).to_be_bytes());
    chunk
}

/// The CRC-32 a PNG names each of its chunks with.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A setup a case started, ended when the case is over.
///
/// A case that fails before it reaches its own teardown would otherwise leave a
/// wizard on the desk. That is not only untidy: the run's output is a pipe the
/// process inherited, and a step is over only once everything holding that pipe
/// has let go, so a wizard left behind can keep a whole job waiting.
/// The one pointer this desktop has, lent to one case at a time.
///
/// Two cases move the real pointer: the button state case and the cursor case.
/// Each lifts its own window above the rest of the desk so that the shape and
/// the pixels it reads back belong to that window, and two of them doing it at
/// once would put one window above the other's -- the pointer would land on the
/// wrong window and the case would read that window's answer. They take this in
/// turn instead, however the suite happens to schedule them. A case that fails
/// while holding it poisons the lock, which is why the guard keeps going: the
/// pointer is put back by that case's own guard either way.
static POINTER: Mutex<()> = Mutex::new(());

/// Takes the desktop pointer for the length of the returned guard.
fn the_pointer() -> MutexGuard<'static, ()> {
    POINTER.lock().unwrap_or_else(|error| error.into_inner())
}

struct SetupGuard(Child);

impl SetupGuard {
    /// Starts the built setup at the scale a case measures at.
    fn spawn(setup: &Path) -> anyhow::Result<Self> {
        Ok(Self(
            Command::new(setup)
                .env("NANO_INSTALLER_TEST_DPI", "96")
                .spawn()?,
        ))
    }

    /// Starts the built setup with the answer the machine gives about high
    /// contrast stated instead.
    ///
    /// Whether the user has the setting on belongs to the user, so a case may
    /// not turn it on for them: it tells the runtime which answer to paint with
    /// and reads both frames of the same page that way.
    fn spawn_stating_contrast(setup: &Path, on: bool) -> anyhow::Result<Self> {
        Ok(Self(
            Command::new(setup)
                .env("NANO_INSTALLER_TEST_DPI", "96")
                .env(
                    "NANO_INSTALLER_TEST_HIGH_CONTRAST",
                    if on { "1" } else { "0" },
                )
                .spawn()?,
        ))
    }
}

impl std::ops::Deref for SetupGuard {
    type Target = Child;

    fn deref(&self) -> &Child {
        &self.0
    }
}

impl std::ops::DerefMut for SetupGuard {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}

impl Drop for SetupGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Clicks a point inside the runtime window, the way a user's click arrives.
///
/// The runtime answers a button-up with whatever action the layout placed under
/// that point, so this is the whole input path a page action is reached by.
fn click_client_point(window: HWND, x: i32, y: i32) {
    let lparam = LPARAM(((y << 16) | (x & 0xFFFF)) as isize);
    unsafe { PostMessageW(window, WM_LBUTTONUP, WPARAM(0), lparam) }
        .expect("the runtime window accepts a click");
}

/// Rolls the wheel over a point inside the runtime window.
///
/// A wheel message carries screen coordinates while every mouse message beside
/// it carries client ones, so the point is converted on the way out: the runtime
/// converts it back, exactly as it does for a wheel the system delivered. A
/// notch rolled away from the user is the positive one and scrolls a list up,
/// so moving a list down takes a negative count.
fn wheel_client_point(window: HWND, x: i32, y: i32, notches: i32) {
    let mut point = POINT { x, y };
    unsafe { ClientToScreen(window, &mut point) }.expect("the window has a screen position");
    let delta = notches * 120;
    let wparam = WPARAM(((delta as u32) << 16) as usize);
    let lparam = LPARAM(((point.y << 16) | (point.x & 0xFFFF)) as isize);
    unsafe { PostMessageW(window, WM_MOUSEWHEEL, wparam, lparam) }
        .expect("the runtime window accepts a wheel message");
}

/// Presses and releases a point inside the runtime window.
///
/// A click on a text field takes the caret on the way down, which is what makes
/// the characters posted after it land in that field.
fn press_client_point(window: HWND, x: i32, y: i32) {
    let lparam = LPARAM(((y << 16) | (x & 0xFFFF)) as isize);
    unsafe {
        PostMessageW(window, WM_LBUTTONDOWN, WPARAM(0), lparam)
            .expect("the runtime window accepts a press");
        PostMessageW(window, WM_LBUTTONUP, WPARAM(0), lparam)
            .expect("the runtime window accepts a release");
    }
}

/// Types text into the focused field, one character at a time.
fn type_client_text(window: HWND, text: &str) {
    for character in text.encode_utf16() {
        unsafe {
            PostMessageW(window, WM_CHAR, WPARAM(character as usize), LPARAM(0))
                .expect("the runtime window accepts a character");
        }
    }
}

/// Waits for a directory to exist.
fn wait_for_directory(path: &Path, deadline: Instant) -> bool {
    while Instant::now() < deadline {
        if path.is_dir() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Waits for the window's client area to become `expected`.
///
/// A wizard that answered by closing itself has no client area left to measure
/// and reports the same way a window that never changed does.
fn wait_for_client_size(
    window: HWND,
    expected: (i32, i32),
    deadline: Instant,
) -> Option<(i32, i32)> {
    while Instant::now() < deadline {
        if !unsafe { IsWindow(window).as_bool() } {
            return None;
        }
        let size = client_size(window);
        if size == expected {
            return Some(size);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

/// Waits for a file to hold exactly the text the script was told to write.
fn wait_for_text(path: &Path, expected: &str, deadline: Instant) -> Option<String> {
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(path) {
            if text == expected {
                return Some(text);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    std::fs::read_to_string(path).ok()
}

/// Clicks a point in the wizard until the file the script writes next holds the
/// text the case is waiting for.
///
/// The card opens a moment after the script reaches its question, so the click
/// is repeated: an early one lands on the page under the card and does nothing,
/// which is what keeps the case out of the timing between the two. A click that
/// does land on the card's button is the only thing that can make the script
/// write the file at all.
fn click_until_text(
    window: HWND,
    x: i32,
    y: i32,
    path: &Path,
    expected: &str,
    deadline: Instant,
) -> Option<String> {
    while Instant::now() < deadline {
        click_client_point(window, x, y);
        if let Ok(text) = std::fs::read_to_string(path) {
            if text == expected {
                return Some(text);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    std::fs::read_to_string(path).ok()
}

/// Reports that this machine cannot open a window.
/// Leaves the case when the desktop shows no cursor at all, which is what a
/// hosted runner reports: Windows says there is no pointer on this session, so
/// there is no shape to ask a window about.
///
/// `NANO_INSTALLER_E2E_REQUIRE_DESKTOP=1` turns this skip into a failure, the
/// way it does for a window that never appeared.
fn skip_without_a_cursor() -> anyhow::Result<()> {
    let required =
        std::env::var_os("NANO_INSTALLER_E2E_REQUIRE_DESKTOP").is_some_and(|value| value != "0");
    anyhow::ensure!(
        !required,
        "the desktop is showing no cursor, so the window cannot be asked which shape it wants"
    );
    eprintln!("skipping: the desktop is showing no cursor");
    Ok(())
}

fn skip_missing_desktop(reason: &str) -> anyhow::Result<()> {
    let required =
        std::env::var_os("NANO_INSTALLER_E2E_REQUIRE_DESKTOP").is_some_and(|value| value != "0");
    anyhow::ensure!(!required, "the wizard window never appeared: {reason}");
    eprintln!("skipping: no wizard window ({reason})");
    Ok(())
}

/// How the wait for the runtime's window ended.
enum WindowWait {
    /// The window the runtime owns, found by class and by owning process.
    Found(HWND),
    /// The process ended before it opened a window, with its status.
    Exited(ExitStatus),
    /// No window appeared before the deadline.
    Timeout,
}

/// Waits for the runtime's window, polling because the window is created a
/// moment after the process starts.
fn wait_for_runtime_window(setup: &mut Child, deadline: Instant) -> WindowWait {
    while Instant::now() < deadline {
        if let Some(status) = setup.try_wait().expect("the setup process can be polled") {
            return WindowWait::Exited(status);
        }
        if let Some(window) = runtime_window(setup.id()) {
            return WindowWait::Found(window);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    WindowWait::Timeout
}

/// What one walk over the top-level windows collects.
struct WindowLookup {
    process_id: u32,
    found: Option<HWND>,
}

/// The window of `process_id` whose class is the runtime's, if the process has
/// one. Class and owner together say the window belongs to this run: the silent
/// cases open no window at all, and a window another process left behind belongs
/// to another process.
fn runtime_window(process_id: u32) -> Option<HWND> {
    let mut lookup = WindowLookup {
        process_id,
        found: None,
    };
    let parameter = LPARAM(&mut lookup as *mut WindowLookup as isize);
    // A machine with no window station fails the walk, which is the same answer
    // to the caller as a walk that found nothing.
    let _ = unsafe { EnumWindows(Some(visit_window), parameter) };
    lookup.found
}

/// The callback the walk runs per top-level window. Windows written in C pass a
/// context pointer along, and returning zero stops the walk.
unsafe extern "system" fn visit_window(window: HWND, parameter: LPARAM) -> BOOL {
    let lookup = &mut *(parameter.0 as *mut WindowLookup);
    let mut owner = 0u32;
    GetWindowThreadProcessId(window, Some(&mut owner));
    if owner == lookup.process_id && window_class(window) == RUNTIME_WINDOW_CLASS {
        lookup.found = Some(window);
        return BOOL(0);
    }
    BOOL(1)
}

/// The class name of a window.
fn window_class(window: HWND) -> String {
    let mut name = [0u16; 256];
    let length = unsafe { GetClassNameW(window, &mut name) };
    let length = usize::try_from(length).unwrap_or(0).min(name.len());
    String::from_utf16_lossy(&name[..length])
}

/// The client area of a window, which is the page the runtime drew.
fn client_size(window: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    unsafe { GetClientRect(window, &mut rect) }.expect("the window has a client area");
    (rect.right - rect.left, rect.bottom - rect.top)
}
