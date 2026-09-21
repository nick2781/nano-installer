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
use std::time::{Duration, Instant};

use anyhow::Context;
use nano_installer_core::{
    build_project, build_project_with_progress, BuildRequest, PayloadFormat,
};
use windows::Win32::Foundation::{BOOL, HANDLE, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC,
    HGDIOBJ,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_DOWN, VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Programs, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetClientRect, GetCursorInfo, GetCursorPos, GetForegroundWindow,
    GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    LoadCursorW, PostMessageW, SetCursorPos, SetForegroundWindow, SetProcessDPIAware, SetWindowPos,
    CURSORINFO, HCURSOR, HTCLIENT, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW, IDC_HAND, IDC_IBEAM,
    SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WM_CHAR, WM_CLOSE, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_SETCURSOR,
};

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
  <TextInput id="editDir" required="true" required-message="@dir_needed"
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
            br#"{"dir_needed": "Choose a folder", "agree": "I agree", "install_button": "Install"}"#,
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

/// One value of a registry key, or `None` when the key or the value is absent.
///
/// The value may itself hold spaces, so each line is split into its columns
/// rather than on whitespace.
fn read_registry_string(key: &str, name: &str) -> anyhow::Result<Option<String>> {
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
            return Ok(Some(value.trim().to_string()));
        }
    }
    Ok(None)
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
    let installed = wait_for_directory(&typed, Instant::now() + Duration::from_secs(20));
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
        "the install did not write the product into the directory the field held"
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
        Instant::now() + Duration::from_secs(20),
    );
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
        "the install did not write the product into the configured directory"
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
    // it at all.
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
        left.expect("checked just above").differing_pixels(&resting),
        0,
        "the window after the pointer left is not the window it was before it arrived"
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
    // back where it was found, including when an assertion fails first.
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
