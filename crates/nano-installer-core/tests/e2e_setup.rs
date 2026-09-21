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

use nano_installer_core::{
    build_project, build_project_with_progress, BuildRequest, PayloadFormat,
};
use windows::Win32::Foundation::{BOOL, HANDLE, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Programs, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetClientRect, GetWindowThreadProcessId, IsWindow, PostMessageW,
    SetProcessDPIAware, WM_CHAR, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEWHEEL,
};

/// The key Windows starts a program from at sign-in.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

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
    let mut setup = Command::new(&fixture.setup)
        .env("NANO_INSTALLER_TEST_DPI", "96")
        .spawn()?;

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
    let mut setup = Command::new(&fixture.setup)
        .env("NANO_INSTALLER_TEST_DPI", "96")
        .spawn()?;

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
    let mut setup = Command::new(&fixture.setup)
        .env("NANO_INSTALLER_TEST_DPI", "96")
        .spawn()?;

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
    let mut setup = Command::new(&fixture.setup)
        .env("NANO_INSTALLER_TEST_DPI", "96")
        .spawn()?;

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
    let mut setup = Command::new(&fixture.setup)
        .env("NANO_INSTALLER_TEST_DPI", "96")
        .spawn()?;
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
    let mut setup = Command::new(&fixture.setup)
        .env("NANO_INSTALLER_TEST_DPI", "96")
        .spawn()?;
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

/// Reports that this machine cannot open a window.
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
