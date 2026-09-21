//! Rhai script execution for projects that ship `scripts/install.rhai`.
//!
//! A project either ships a script or it does not. Without one the runtime
//! runs the built-in flow in `install.rs`; with one the script chooses the
//! step order and calls the primitives below, which reuse the same validated
//! deployment, rollback, and manifest code.
//!
//! Scripts are a project resource, not a plugin system: the primitives are
//! fixed, and the engine carries an operation ceiling so a project cannot hang
//! an installation in a loop.

mod api_association;
mod api_dependency;
mod api_download;
mod api_file;
mod api_process;
mod api_registry;
mod api_shortcut;
mod api_system;
mod api_ui;
mod context;

use anyhow::{bail, Context, Result};
use rhai::{Engine, Scope};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::install::{self, InstallPrep, InstallSelection};
use crate::BundleIndex;
/// The log the run keeps, which the built-in flow writes into as well: a dependency it
/// had to install belongs in the report of a failure that follows.
pub(crate) use context::log;
use context::{log_tail, read_script, reset_log, undo, ScriptContext, ScriptEnvironment, Snapshot};

/// Script entry points inside a project's `scripts/` directory.
pub(super) const INSTALL_SCRIPT: &str = "scripts/install.rhai";
pub(super) const UNINSTALL_SCRIPT: &str = "scripts/uninstall.rhai";

/// A ceiling on script work: a runaway loop must not hang an installation.
const MAX_OPERATIONS: u64 = 100_000_000;

/// Everything the built-in flow already resolved for an install.
pub(super) struct InstallRequest {
    pub(super) setup: PathBuf,
    pub(super) bundle: BundleIndex,
    pub(super) config: serde_json::Value,
    pub(super) destination: PathBuf,
    pub(super) selection: InstallSelection,
    /// The components this run installs, resolved from the page and the project.
    pub(super) components: Vec<String>,
    /// Scratch directory owned by the caller for the duration of the script.
    pub(super) stage: PathBuf,
    pub(super) prep: InstallPrep,
    /// The task this script is part of, so it can be asked to stop.
    pub(super) cancel: install::Cancellation,
}

/// Runs `scripts/install.rhai` in place of the built-in deployment steps.
pub(super) fn run_install(request: InstallRequest) -> Result<()> {
    let InstallRequest {
        setup,
        bundle,
        config,
        destination,
        selection,
        components,
        stage,
        mut prep,
        cancel,
    } = request;
    let source = read_script(&bundle, INSTALL_SCRIPT)?;
    let exe_name = config["install"]["exe_name"]
        .as_str()
        .context("install.exe_name is required")?
        .to_string();
    let context = begin(ScriptEnvironment {
        setup,
        config,
        install_path: destination.clone(),
        checkboxes: selection.checkboxes().clone(),
        texts: selection.texts().clone(),
        choices: selection.choices().clone(),
        components,
        keep_data: false,
        manifest: serde_json::Value::Null,
        previous: prep.previous.take(),
        bundle,
        stage,
        journal: None,
        mode: Mode::Install,
        cancel: cancel.clone(),
    })?;

    if let Err(error) = run(&context, &source) {
        undo(&context);
        return Err(error);
    }
    // A cancel asked while the script ran is honoured before the library
    // registers anything: the script has had its chance to notice it through
    // is_cancelled, and what it wrote is undone below.
    if let Err(error) = install::check_cancelled(&cancel) {
        undo(&context);
        return Err(error);
    }
    publish(crate::report_progress(95, "status.finishing"));
    if let Err(error) = finish_install(&context, &exe_name, &prep) {
        undo(&context);
        return Err(error);
    }
    publish(crate::report_progress(100, "status.install_complete"));
    publish(crate::record_installed_app(
        context.install_path().join(exe_name),
    ));
    Ok(())
}

/// Everything the built-in flow already resolved for an uninstall.
pub(super) struct UninstallRequest {
    pub(super) uninstaller: PathBuf,
    pub(super) bundle: BundleIndex,
    pub(super) config: serde_json::Value,
    pub(super) destination: PathBuf,
    pub(super) manifest: serde_json::Value,
    pub(super) keep_data: bool,
    pub(super) stage: PathBuf,
    pub(super) root: windows::Win32::System::Registry::HKEY,
    pub(super) registry_path: String,
    /// The task this script is part of, so it can be asked to stop.
    pub(super) cancel: install::Cancellation,
}

/// Runs `scripts/uninstall.rhai` in place of the built-in removal steps.
pub(super) fn run_uninstall(request: UninstallRequest) -> Result<()> {
    let UninstallRequest {
        uninstaller,
        bundle,
        config,
        destination,
        manifest,
        keep_data,
        stage,
        root,
        registry_path,
        cancel,
    } = request;
    let source = read_script(&bundle, UNINSTALL_SCRIPT)?;
    let context = begin(ScriptEnvironment {
        setup: uninstaller.clone(),
        config: config.clone(),
        install_path: destination.clone(),
        checkboxes: HashMap::from([("keep_data".to_string(), keep_data)]),
        // An uninstall page hands over no field a script can read: the only
        // value it carries is the keep-data box.
        texts: HashMap::new(),
        choices: HashMap::new(),
        // Nothing is being installed, so no component is: an uninstall script
        // reads what is on disk, not what a page chose.
        components: Vec::new(),
        keep_data,
        manifest: manifest.clone(),
        previous: None,
        bundle,
        stage,
        journal: None,
        mode: Mode::Uninstall,
        cancel,
    })?;

    run(&context, &source)?;
    publish(crate::report_progress(97, "uninstall.status.finishing"));
    // A script that never replayed the manifest would leave the product
    // installed, so the library steps run as the fallback.
    if !context.state().tracked_uninstall {
        fallback_uninstall(&context)?;
    }
    install::finish_uninstall(&destination, &uninstaller, root, &registry_path)?;
    crate::shell::notify_shell();
    publish(crate::report_progress(100, "uninstall.status.complete"));
    Ok(())
}

/// Publishes a wizard update without failing the task.
///
/// The wizard is the only consumer of progress and status, but a deployment
/// that already succeeded must not roll back because a repaint could not be
/// queued, and a script's own step text must never be able to abort it.
fn publish(result: Result<()>) {
    if let Err(error) = result {
        log("warn", &format!("cannot update the wizard: {error:#}"));
    }
}

/// Removes what the manifest records when the script did not do it itself.
///
/// The steps are the library's own uninstall, so a script that forgets
/// `run_tracked_uninstall` still leaves the machine clean.
fn fallback_uninstall(context: &ScriptContext) -> Result<()> {
    let manifest = context.manifest().clone();
    install::remove_recorded_artifacts(&manifest);
    if !context.keep_data() {
        install::remove_user_data(context.config())?;
    }
    install::remove_installed_files(&context.install_path(), &manifest)
}

/// Which entry point is running; the two share every primitive.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Install,
    Uninstall,
}

/// Builds the script context, creating the destination for a fresh install.
fn begin(mut environment: ScriptEnvironment) -> Result<ScriptContext> {
    let destination = environment.install_path.clone();
    let backup_root = environment.stage.join("rollback");
    let journal = match environment.mode {
        // An install may have to create the destination, and only a fresh
        // directory may be removed whole when the script fails.
        Mode::Install => {
            install::begin_deployment(&destination, &backup_root, environment.previous.as_ref())?
        }
        // The destination already exists while uninstalling; its contents are
        // never restored, because the whole point is to remove them.
        Mode::Uninstall => install::RollbackJournal::new(backup_root, None),
    };
    let before = Snapshot::take(&destination);
    environment.journal = Some(journal);
    let context = ScriptContext::new(environment);
    context.set_before(before);
    Ok(context)
}

/// Records what the script deployed and registers the uninstallation.
fn finish_install(context: &ScriptContext, exe_name: &str, prep: &InstallPrep) -> Result<()> {
    let destination = context.install_path();
    let uninstaller_name = prep.uninstaller_name.as_str();
    let mut state = context.state();
    let after = Snapshot::take(&destination);
    // The uninstaller and the manifest are recorded by their own fields, so
    // they never belong in the file list the uninstaller deletes.
    let files = after
        .files_added_since(&state.before)
        .into_iter()
        .filter(|path| path != Path::new(uninstaller_name))
        .filter(|path| path != Path::new(install::MANIFEST_NAME))
        .collect::<Vec<_>>();
    if !files.iter().any(|path| path == Path::new(exe_name)) {
        bail!("the install script did not deploy {exe_name}")
    }
    // An upgrade drops what the previous version shipped and this one does
    // not, plus the links the script did not recreate; a script that forgets
    // either would otherwise leak files and dead shortcuts forever.
    if let Some(previous) = context.previous() {
        for relative in &previous.files {
            if files.iter().any(|path| path == relative) {
                continue;
            }
            let target = destination.join(relative);
            if !target.is_file() {
                continue;
            }
            state.journal.track(&target)?;
            std::fs::remove_file(&target)?;
        }
        for stale in install::recorded_shortcuts(&previous.manifest) {
            if state.shortcuts.contains(&stale) {
                continue;
            }
            if stale.is_file() {
                state.journal.track(&stale)?;
                std::fs::remove_file(&stale)?;
            }
        }
    }
    let artifacts = install::ManifestArtifacts {
        shortcuts: text_paths(&state.shortcuts),
        shortcut_dirs: text_paths(&state.shortcut_dirs),
        autostart: serde_json::Value::Null,
        registry_values: state.registry_values.clone(),
        registry_keys: state.registry_keys.clone(),
    };
    let context_config = context.config().clone();
    drop(state);
    install::write_manifest(
        &destination,
        &mut context.state().journal,
        install::registry_root_name(prep.root),
        &prep.registry_path,
        &files,
        &artifacts,
    )?;
    // The script may have written the uninstall key itself, and `prepare_install`
    // already refused a key that belongs to another installation, so replacing
    // it here is safe.
    install::rewrite_uninstall_registration(
        prep.root,
        &prep.registry_path,
        &destination,
        uninstaller_name,
        &context_config,
    )
}

fn text_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

/// Runs a project script with the installer primitives registered.
fn run(context: &ScriptContext, source: &str) -> Result<()> {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    api_ui::register(&mut engine, context.clone());
    api_association::register(&mut engine, context.clone());
    api_dependency::register(&mut engine, context.clone());
    api_download::register(&mut engine, context.clone());
    api_file::register(&mut engine, context.clone());
    api_registry::register(&mut engine, context.clone());
    api_process::register(&mut engine);
    api_shortcut::register(&mut engine, context.clone());
    api_system::register(&mut engine, context.clone());
    let mut scope = Scope::new();
    reset_log();
    engine.run_with_scope(&mut scope, source).map_err(|error| {
        // The script's own messages usually name the step that failed, so they
        // travel with the error the wizard shows.
        let tail = log_tail();
        if tail.is_empty() {
            anyhow::anyhow!("project script failed: {error}")
        } else {
            anyhow::anyhow!("project script failed: {error}\n{tail}")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::{self, InstallPrep};
    use crate::BundleIndex;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};

    /// One registry key per test, so tests running in parallel never share one.
    fn unique_registry_key() -> String {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!(
            "HKCU\\Software\\nano-installer-script-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    }

    /// Removes the key a test installed into.
    struct TestRegistryKey(String);

    impl Drop for TestRegistryKey {
        fn drop(&mut self) {
            let Ok((root, path)) = install::registry_path(&self.0) else {
                return;
            };
            unsafe {
                let _ = RegDeleteTreeW(root, PCWSTR(install::wide(&path).as_ptr()));
            }
        }
    }

    fn config(registry_key: &str) -> serde_json::Value {
        serde_json::json!({
            "project": {"name": "Script Test", "version": "1.0.0", "publisher": "Test"},
            "install": {"exe_name": "App.exe"},
            "output": {"uninstaller_name": "uninst.exe"},
            "registry": {"uninstall_key": registry_key},
            // The scripts below read their registry target from here, so a test
            // never touches a real product key.
            "test": {"registry_key": registry_key}
        })
    }

    /// The subkey below the hive, as the registry helpers take it.
    fn subkey(registry_key: &str) -> String {
        install::registry_path(registry_key)
            .expect("the test registry key is valid")
            .1
    }

    /// Writes an executable image carrying a native bundle of `entries`.
    ///
    /// The image only has to look like a stub to `BundleIndex::read`; nothing
    /// in these tests launches it.
    fn bundle_image(path: &Path, entries: &[(&str, &str)]) -> Result<()> {
        let mut bundle = Vec::new();
        bundle.extend_from_slice(b"NATVRS01");
        bundle.extend_from_slice(&1u16.to_le_bytes());
        bundle.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (name, contents) in entries {
            let name = name.as_bytes();
            bundle.extend_from_slice(&(name.len() as u16).to_le_bytes());
            bundle.extend_from_slice(name);
            bundle.extend_from_slice(&(contents.len() as u64).to_le_bytes());
            bundle.extend_from_slice(contents.as_bytes());
        }
        let mut file = std::fs::File::create(path)?;
        file.write_all(b"stub-image")?;
        file.write_all(&bundle)?;
        file.write_all(&(bundle.len() as u64).to_le_bytes())?;
        file.write_all(b"NATVEND1")?;
        file.flush()?;
        Ok(())
    }

    struct Fixture {
        _temp: tempfile::TempDir,
        _registry: TestRegistryKey,
        setup: PathBuf,
        destination: PathBuf,
        registry_key: String,
        config: serde_json::Value,
    }

    /// A bundle image that ships both scripts, the uninstaller `copy_uninstaller`
    /// embeds, and the project configuration, all under one throwaway key.
    fn fixture(install_script: &str, uninstall_script: &str) -> Result<Fixture> {
        build_fixture(install_script, uninstall_script, None)
    }

    /// The same image with `entries` stored under `directory`, which its
    /// configuration declares as `resources.tools_dir`, the way a project ships
    /// the programs its script runs.
    fn fixture_with_tools(
        install_script: &str,
        directory: &str,
        entries: &[(&str, &str)],
    ) -> Result<Fixture> {
        build_fixture(install_script, "", Some((directory, entries)))
    }

    fn build_fixture(
        install_script: &str,
        uninstall_script: &str,
        tools: Option<(&str, &[(&str, &str)])>,
    ) -> Result<Fixture> {
        let registry_key = unique_registry_key();
        let mut config = config(&registry_key);
        if let Some((directory, _)) = tools {
            // The fixture's configuration has no resources section of its own.
            config
                .as_object_mut()
                .expect("the test configuration is an object")
                .insert(
                    "resources".to_string(),
                    serde_json::json!({ "tools_dir": directory }),
                );
        }
        let config_text = config.to_string();
        let temp = tempfile::tempdir()?;
        let setup = temp.path().join("setup.exe");
        let mut entries: Vec<(&str, &str)> = vec![
            ("installer_config.json", &config_text),
            ("scripts/install.rhai", install_script),
            ("scripts/uninstall.rhai", uninstall_script),
            ("runtime/uninst.exe", "uninstaller"),
        ];
        let tool_paths: Vec<(String, &str)> = tools
            .map(|(directory, entries)| {
                entries
                    .iter()
                    .map(|(name, contents)| (format!("{directory}/{name}"), *contents))
                    .collect()
            })
            .unwrap_or_default();
        entries.extend(
            tool_paths
                .iter()
                .map(|(name, contents)| (name.as_str(), *contents)),
        );
        bundle_image(&setup, &entries)?;
        Ok(Fixture {
            destination: temp.path().join("installed"),
            _temp: temp,
            _registry: TestRegistryKey(registry_key.clone()),
            setup,
            registry_key,
            config,
        })
    }

    impl Fixture {
        fn bundle(&self) -> Result<BundleIndex> {
            BundleIndex::read(&self.setup)?.context("the fixture image carries a bundle")
        }

        fn stage(&self) -> Result<PathBuf> {
            let stage = self._temp.path().join("stage");
            std::fs::create_dir_all(&stage)?;
            Ok(stage)
        }

        fn install(&self) -> Result<()> {
            self.install_with(install::Cancellation::default())
        }

        /// Runs the install with the handle the caller keeps, so a test can ask
        /// it to stop the way the wizard's cancel button does.
        fn install_with(&self, task: install::Cancellation) -> Result<()> {
            self.install_selection(InstallSelection::default(), task)
        }

        /// Runs the install with what a page would have handed over, the way the
        /// wizard does when the user starts the task: the text its fields held, the
        /// value each choice control stood on, and the boxes the user left ticked.
        fn install_with_values(
            &self,
            texts: &[(&str, &str)],
            choices: &[(&str, &str)],
            checkboxes: &[(&str, bool)],
        ) -> Result<()> {
            let texts = texts
                .iter()
                .map(|(id, value)| (id.to_string(), value.to_string()))
                .collect();
            let choices = choices
                .iter()
                .map(|(id, value)| (id.to_string(), value.to_string()))
                .collect();
            let checkboxes = checkboxes
                .iter()
                .map(|(id, checked)| (id.to_string(), *checked))
                .collect();
            self.install_selection(
                InstallSelection::from_values(texts, choices, checkboxes),
                install::Cancellation::default(),
            )
        }

        fn install_selection(
            &self,
            selection: InstallSelection,
            task: install::Cancellation,
        ) -> Result<()> {
            let bundle = self.bundle()?;
            let (root, registry_path) = install::uninstall_registry_key(&self.config)?;
            // The same rule the wizard applies before it runs any step.
            let components = install::selected_components(&self.config, |id, default| {
                selection.checked(id, default)
            });
            run_install(InstallRequest {
                setup: self.setup.clone(),
                bundle,
                config: self.config.clone(),
                destination: self.destination.clone(),
                selection,
                components,
                stage: self.stage()?,
                prep: InstallPrep {
                    uninstaller_name: "uninst.exe".to_string(),
                    uninstaller: b"uninstaller".to_vec(),
                    root,
                    registry_path,
                    previous: None,
                    upgrade: false,
                },
                cancel: task,
            })
        }

        fn uninstall(&self, keep_data: bool) -> Result<()> {
            let bundle = self.bundle()?;
            let (root, registry_path) = install::uninstall_registry_key(&self.config)?;
            let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
                self.destination.join(install::MANIFEST_NAME),
            )?)?;
            let result = run_uninstall(UninstallRequest {
                uninstaller: self.destination.join("uninst.exe"),
                bundle,
                config: self.config.clone(),
                destination: self.destination.clone(),
                manifest,
                keep_data,
                stage: self.stage()?,
                root,
                registry_path,
                cancel: install::Cancellation::default(),
            });
            remove_cleanup_helpers();
            result
        }
    }

    /// Removes the cleanup-helper copies this test process left behind.
    ///
    /// A finished uninstall copies its own image into the temporary directory
    /// and starts it with `--cleanup`, because Windows will not let a process
    /// delete the image it runs from. In the product that copy is the
    /// uninstaller, and it deletes itself when it is done; here it is the test
    /// binary, which knows no such flag, exits, and leaves a copy of itself
    /// behind on every uninstall. Nothing here fails a case: a copy that is
    /// still starting is simply tried again.
    fn remove_cleanup_helpers() {
        let prefix = format!("nano-installer-cleanup-{}-", std::process::id());
        for _ in 0..40 {
            let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
                return;
            };
            let mut left = 0;
            for entry in entries.flatten() {
                if !entry.file_name().to_string_lossy().starts_with(&prefix) {
                    continue;
                }
                if std::fs::remove_file(entry.path()).is_err() {
                    left += 1;
                }
            }
            if left == 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    const INSTALL_SCRIPT: &str = r#"
        let install_path = get_install_path();
        let registry_key = get_config_value("test.registry_key");
        copy_uninstaller();
        write_file(path_join(install_path, "App.exe"), "app");
        write_file(path_join(install_path, "assets/config.ini"), "config");
        reg_write_string(registry_key, "InstallPath", install_path);
        set_status_key("status.installing_uninstaller");
        set_progress(42.0);
        log_info("installed");
    "#;

    #[test]
    fn an_install_script_deploys_files_and_writes_the_manifest() -> Result<()> {
        let fixture = fixture(INSTALL_SCRIPT, "")?;
        fixture.install()?;

        assert_eq!(std::fs::read(fixture.destination.join("App.exe"))?, b"app");
        assert_eq!(
            std::fs::read(fixture.destination.join("assets/config.ini"))?,
            b"config"
        );
        assert!(fixture.destination.join("uninst.exe").is_file());
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
            fixture.destination.join(install::MANIFEST_NAME),
        )?)?;
        // The uninstaller and the manifest manage themselves, so neither is in
        // the list the uninstaller deletes.
        assert_eq!(
            manifest["files"],
            serde_json::json!(["App.exe", "assets\\config.ini"])
        );
        assert_eq!(
            manifest["registry_values"],
            serde_json::json!([{"path": fixture.registry_key, "name": "InstallPath"}])
        );
        assert_eq!(manifest["registry_root"].as_str(), Some("HKCU"));
        assert_eq!(
            install::read_registry_string(
                HKEY_CURRENT_USER,
                &subkey(&fixture.registry_key),
                "DisplayName"
            )?,
            Some("Script Test".to_string())
        );
        Ok(())
    }

    #[test]
    fn an_install_script_that_never_deploys_the_executable_is_refused() -> Result<()> {
        let fixture = fixture(
            r#"write_file(path_join(get_install_path(), "Other.exe"), "x");"#,
            "",
        )?;
        let error = fixture
            .install()
            .expect_err("the executable is missing from the destination");
        assert!(error.to_string().contains("did not deploy App.exe"));
        // The failed attempt must not leave a half-written directory behind.
        assert!(!fixture.destination.exists());
        Ok(())
    }

    #[test]
    fn a_failing_install_script_removes_what_it_wrote() -> Result<()> {
        let fixture = fixture(
            r#"
                write_file(path_join(get_install_path(), "App.exe"), "app");
                throw "the project script gave up";
            "#,
            "",
        )?;
        let error = fixture.install().expect_err("the script threw");
        assert!(error.to_string().contains("the project script gave up"));
        assert!(!fixture.destination.exists());
        Ok(())
    }

    #[test]
    fn a_script_failure_reports_the_messages_it_logged() -> Result<()> {
        let fixture = fixture(
            r#"
                log_warn("about to fail");
                throw "done";
            "#,
            "",
        )?;
        let error = fixture.install().expect_err("the script threw");
        assert!(error.to_string().contains("warn: about to fail"));
        Ok(())
    }

    #[test]
    fn an_uninstall_script_replays_the_manifest_it_asks_for() -> Result<()> {
        let fixture = fixture(INSTALL_SCRIPT, r#"run_tracked_uninstall(10.0, 90.0);"#)?;
        fixture.install()?;
        fixture.uninstall(true)?;

        assert!(!fixture.destination.join("App.exe").exists());
        assert!(!fixture.destination.join("assets/config.ini").exists());
        assert!(!fixture.destination.join(install::MANIFEST_NAME).exists());
        assert!(install::read_registry_string(
            HKEY_CURRENT_USER,
            &subkey(&fixture.registry_key),
            "InstallPath"
        )?
        .is_none());
        Ok(())
    }

    #[test]
    fn an_uninstall_script_that_skips_the_manifest_still_removes_the_product() -> Result<()> {
        let fixture = fixture(INSTALL_SCRIPT, r#"log_info("nothing to do");"#)?;
        fixture.install()?;
        fixture.uninstall(true)?;

        assert!(!fixture.destination.join("App.exe").exists());
        assert!(!fixture.destination.join("assets/config.ini").exists());
        assert!(!fixture.destination.join(install::MANIFEST_NAME).exists());
        Ok(())
    }

    #[test]
    fn replaying_the_manifest_is_refused_while_installing() -> Result<()> {
        let fixture = fixture(
            r#"
                let replayed = run_tracked_uninstall(0.0, 1.0);
                write_file(path_join(get_install_path(), "App.exe"), "app");
                write_file(path_join(get_install_path(), "replayed.txt"), replayed.to_string());
            "#,
            "",
        )?;
        fixture.install()?;
        assert_eq!(
            std::fs::read_to_string(fixture.destination.join("replayed.txt"))?,
            "false"
        );
        Ok(())
    }

    // The tests below cover the primitives the scripts above never call. Each
    // one needs the same two things: a way to see what a primitive returned, and
    // a way to leave the machine as it was found.

    /// A name no other test in this process, and no real product on the
    /// machine, is using.
    fn unique_name(prefix: &str) -> String {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!(
            "{prefix}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    }

    /// A Rust string as a Rhai string literal, so a Windows path or registry key
    /// survives being embedded in a script.
    fn literal(value: &str) -> String {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }

    /// An install script that deploys the executable every fixture needs, with
    /// `body` appended to it.
    ///
    /// The driver refuses an install that never deploys `App.exe`, so a script
    /// written to exercise one primitive would otherwise fail for a missing
    /// file instead.
    fn deploying_script(body: &str) -> String {
        format!(
            r#"
            let install_path = get_install_path();
            copy_uninstaller();
            write_file(path_join(install_path, "App.exe"), "app");
            {body}
            "#
        )
    }

    /// The manifest an install wrote, as the uninstaller reads it.
    fn manifest(fixture: &Fixture) -> Result<serde_json::Value> {
        Ok(serde_json::from_slice(&std::fs::read(
            fixture.destination.join(install::MANIFEST_NAME),
        )?)?)
    }

    /// The `name=value` lines a script reported, so a test can compare several
    /// values it cannot know in advance.
    fn observations(text: &str) -> std::collections::BTreeMap<String, String> {
        text.lines()
            .filter_map(|line| line.split_once('='))
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    /// The file a script writes its observations into.
    ///
    /// It sits in the temporary directory rather than in the fixture, so the
    /// same script can report from an uninstall as well, and the guard removes
    /// it whether the test passed or failed.
    struct Observation(PathBuf);

    impl Observation {
        fn new(label: &str) -> Self {
            Self(std::env::temp_dir().join(format!("{}.txt", unique_name(label))))
        }

        /// This file as a Rhai string literal, for embedding in a script.
        fn script_path(&self) -> String {
            literal(&self.0.to_string_lossy())
        }

        fn text(&self) -> Result<String> {
            Ok(std::fs::read_to_string(&self.0)?)
        }
    }

    impl Drop for Observation {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    /// Paths a test created in the user profile.
    ///
    /// Shortcuts and Start Menu folders are real files the machine keeps, so the
    /// guard removes them however the test ends: a failing assertion must not
    /// leave the desktop or the Start Menu littered.
    #[derive(Default)]
    struct ProfileCleanup(Vec<PathBuf>);

    impl ProfileCleanup {
        fn keep(&mut self, path: PathBuf) {
            self.0.push(path);
        }
    }

    impl Drop for ProfileCleanup {
        fn drop(&mut self) {
            for path in self.0.iter().rev() {
                let _ = std::fs::remove_file(path);
                let _ = std::fs::remove_dir_all(path);
            }
        }
    }

    /// A value a test wrote into a key it shares with the machine.
    ///
    /// The uninstall removes it again, but an assertion can fail before that
    /// happens, so the guard deletes it either way. The key itself is never
    /// touched: it belongs to Windows.
    struct TestRegistryValue {
        key: String,
        name: String,
    }

    impl Drop for TestRegistryValue {
        fn drop(&mut self) {
            let Ok((root, path)) = install::registry_path(&self.key) else {
                return;
            };
            let _ = install::delete_registry_value(root, &path, &self.name);
        }
    }

    /// The `REG_DWORD` under `key`, read back the way any other program would.
    ///
    /// `reg_read` only reads `REG_SZ`, so a dword a script wrote needs the
    /// registry API itself to be checked.
    fn read_dword(key: &str, name: &str) -> Result<u32> {
        use windows::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, KEY_QUERY_VALUE, REG_DWORD,
            REG_VALUE_TYPE,
        };

        let (root, path) = install::registry_path(key)?;
        let mut handle = Default::default();
        let opened = unsafe {
            RegOpenKeyExW(
                root,
                PCWSTR(install::wide(&path).as_ptr()),
                0,
                KEY_QUERY_VALUE,
                &mut handle,
            )
        };
        opened.ok().context("the script created its registry key")?;
        let mut kind = REG_VALUE_TYPE::default();
        let mut value = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegQueryValueExW(
                handle,
                PCWSTR(install::wide(name).as_ptr()),
                None,
                Some(&mut kind),
                Some(&mut value as *mut u32 as *mut u8),
                Some(&mut size),
            )
        };
        unsafe {
            let _ = RegCloseKey(handle);
        }
        status.ok()?;
        assert_eq!(kind, REG_DWORD);
        Ok(value)
    }

    /// The drive root the temporary directory sits on, such as `C:\`.
    fn temp_drive_root() -> String {
        let temp = std::env::temp_dir();
        let prefix = temp
            .components()
            .next()
            .expect("the temporary directory names a drive");
        format!("{}\\", prefix.as_os_str().to_string_lossy())
    }

    /// A key Windows and other products also write into, so a script that writes
    /// here owns only the value it puts in it, never the key.
    const SHARED_RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

    #[test]
    fn every_log_level_reaches_the_failure_the_wizard_shows() -> Result<()> {
        let fixture = fixture(
            &deploying_script(
                r#"
                log_info("step one");
                log_warn("step two");
                log_error("step three");
                throw "the project script gave up";
                "#,
            ),
            "",
        )?;
        let error = fixture.install().expect_err("the script threw");

        // A project author sees the wizard's error box and nothing else, so all
        // three levels have to travel with it; a level that wrote nowhere would
        // leave them guessing which step failed and why.
        let text = error.to_string();
        assert!(text.contains("info: step one"), "{text}");
        assert!(text.contains("warn: step two"), "{text}");
        assert!(text.contains("error: step three"), "{text}");
        Ok(())
    }

    #[test]
    fn file_primitives_create_copy_list_and_remove_files() -> Result<()> {
        let report = Observation::new("script-file-primitives");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let docs = path_join(install_path, "docs");
                let notes = path_join(docs, "notes.txt");
                let missing = path_join(install_path, "missing.txt");
                let copy = path_join(docs, "copy.txt");
                let report = "";
                report += "create_dir=" + create_dir(docs).to_string() + "\n";
                report += "write_notes=" + write_file(notes, "hello").to_string() + "\n";
                report += "write_a=" + write_file(path_join(docs, "a.txt"), "a").to_string() + "\n";
                report += "write_b=" + write_file(path_join(docs, "b.txt"), "b").to_string() + "\n";
                report += "exists=" + file_exists(notes).to_string() + "\n";
                report += "missing_exists=" + file_exists(missing).to_string() + "\n";
                report += "is_dir=" + is_dir(docs).to_string() + "\n";
                report += "file_is_dir=" + is_dir(notes).to_string() + "\n";
                report += "size=" + get_file_size(notes).to_string() + "\n";
                report += "missing_size=" + get_file_size(missing).to_string() + "\n";
                report += "text=" + read_text_file(notes) + "\n";
                report += "missing_text=" + read_text_file(missing) + "\n";
                let entries = list_dir(docs);
                report += "list_count=" + entries.len().to_string() + "\n";
                report += "lists_b=" + entries.contains("b.txt").to_string() + "\n";
                report += "missing_list_count=" + list_dir(missing).len().to_string() + "\n";
                report += "copied=" + copy_file(notes, copy).to_string() + "\n";
                report += "copied_text=" + read_text_file(copy) + "\n";
                report += "deleted=" + delete_file(path_join(docs, "a.txt")).to_string() + "\n";
                report += "deleted_exists=" + file_exists(path_join(docs, "a.txt")).to_string() + "\n";
                report += "delete_again=" + delete_file(path_join(docs, "a.txt")).to_string() + "\n";
                report += "removed_dir=" + delete_dir(docs).to_string() + "\n";
                report += "dir_gone=" + is_dir(docs).to_string() + "\n";
                report += "relative_refused=" + delete_dir("docs").to_string() + "\n";
                write_file({}, report);
                "#,
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        // Every value a primitive produced is compared, not only the one the
        // test is named after: the empty read, the missing-file size, or a
        // deletion that stops being idempotent is what breaks a project script.
        assert_eq!(
            report.text()?,
            concat!(
                "create_dir=true\n",
                "write_notes=true\n",
                "write_a=true\n",
                "write_b=true\n",
                "exists=true\n",
                "missing_exists=false\n",
                "is_dir=true\n",
                "file_is_dir=false\n",
                "size=5\n",
                "missing_size=-1\n",
                "text=hello\n",
                "missing_text=\n",
                "list_count=3\n",
                "lists_b=true\n",
                "missing_list_count=0\n",
                "copied=true\n",
                "copied_text=hello\n",
                "deleted=true\n",
                "deleted_exists=false\n",
                "delete_again=true\n",
                "removed_dir=true\n",
                "dir_gone=false\n",
                // A relative path names nothing the script can be held to, so
                // the recursive delete refuses it instead of guessing.
                "relative_refused=false\n",
            )
        );
        Ok(())
    }

    #[test]
    fn path_primitives_join_split_and_name_paths() -> Result<()> {
        let report = Observation::new("script-path-primitives");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let nested = path_join(install_path, "sub/file.txt");
                let report = "";
                report += "join=" + nested + "\n";
                report += "parent=" + path_parent(nested) + "\n";
                report += "filename=" + path_filename(nested) + "\n";
                report += "bare_parent=" + path_parent("App.exe") + "\n";
                report += "directory=" + path_filename(install_path) + "\n";
                report += "temp=" + get_temp_path() + "\n";
                write_file({}, report);
                "#,
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        assert_eq!(
            report.text()?,
            format!(
                "join={destination}\\sub/file.txt\n\
                 parent={destination}\\sub\n\
                 filename=file.txt\n\
                 bare_parent=\n\
                 directory=installed\n\
                 temp={temp}\n",
                destination = fixture.destination.display(),
                temp = std::env::temp_dir().display(),
            )
        );
        Ok(())
    }

    #[test]
    fn registry_primitives_round_trip_and_forget_a_key_they_created() -> Result<()> {
        let report = Observation::new("script-registry");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let key = get_config_value("test.registry_key");
                let branch = key + "\\branch";
                let report = "";
                report += "absent=" + reg_key_exists(key).to_string() + "\n";
                report += "absent_read=" + reg_read(key, "Name") + "\n";
                report += "name_write=" + reg_write_string(key, "Name", "value").to_string() + "\n";
                report += "name_read=" + reg_read(key, "Name") + "\n";
                report += "missing_read=" + reg_read(key, "Missing") + "\n";
                report += "dword_write=" + reg_write_dword(key, "Keep", 9).to_string() + "\n";
                report += "dword_read=" + reg_read(key, "Keep") + "\n";
                report += "exists=" + reg_key_exists(key).to_string() + "\n";
                report += "branch_write=" + reg_write_string(branch, "Branch", "x").to_string() + "\n";
                report += "branch_exists=" + reg_key_exists(branch).to_string() + "\n";
                report += "branch_read=" + reg_read(branch, "Branch") + "\n";
                report += "temp_write=" + reg_write_string(branch, "Temp", "x").to_string() + "\n";
                report += "value_delete=" + reg_delete_value(branch, "Temp").to_string() + "\n";
                report += "temp_read=" + reg_read(branch, "Temp") + "\n";
                report += "branch_survives=" + reg_read(branch, "Branch") + "\n";
                report += "key_delete=" + reg_delete_key(branch).to_string() + "\n";
                report += "branch_gone=" + reg_key_exists(branch).to_string() + "\n";
                report += "key_survives=" + reg_key_exists(key).to_string() + "\n";
                write_file({}, report);
                "#,
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        assert_eq!(
            report.text()?,
            concat!(
                "absent=false\n",
                "absent_read=\n",
                "name_write=true\n",
                "name_read=value\n",
                "missing_read=\n",
                "dword_write=true\n",
                // `reg_read` reads `REG_SZ`, so a dword is not a value it can
                // hand back; the test reads that one through the registry API.
                "dword_read=\n",
                "exists=true\n",
                "branch_write=true\n",
                "branch_exists=true\n",
                "branch_read=x\n",
                "temp_write=true\n",
                // Deleting one value leaves the key and its other values alone,
                // which is what makes the primitive safe on a shared key.
                "value_delete=true\n",
                "temp_read=\n",
                "branch_survives=x\n",
                "key_delete=true\n",
                "branch_gone=false\n",
                "key_survives=true\n",
            )
        );

        let key = fixture.registry_key.clone();
        let branch = format!("{key}\\branch");
        assert_eq!(
            install::read_registry_string(HKEY_CURRENT_USER, &subkey(&key), "Name")?,
            Some("value".to_string())
        );
        assert_eq!(read_dword(&key, "Keep")?, 9);
        assert!(!install::registry_key_exists(
            HKEY_CURRENT_USER,
            &subkey(&branch)
        )?);
        // Deleting the branch dropped it from the tracked list, so what the
        // uninstall replays is the key the script still owns and its two values.
        let recorded = manifest(&fixture)?;
        assert_eq!(
            recorded["registry_values"],
            serde_json::json!([
                {"path": key, "name": "Name"},
                {"path": key, "name": "Keep"}
            ])
        );
        assert_eq!(recorded["registry_keys"], serde_json::json!([key]));
        Ok(())
    }

    #[test]
    fn uninstalling_a_shared_key_removes_only_the_value_the_script_wrote() -> Result<()> {
        let value_name = unique_name("nano-installer-script-test-run");
        // The uninstall takes the value back, but the guard covers a run that
        // fails before it gets that far.
        let _guard = TestRegistryValue {
            key: SHARED_RUN_KEY.to_string(),
            name: value_name.clone(),
        };
        let fixture = fixture(
            &deploying_script(&format!(
                "reg_write_string({}, {}, path_join(install_path, \"App.exe\"));",
                literal(SHARED_RUN_KEY),
                literal(&value_name),
            )),
            r#"run_tracked_uninstall(0.0, 100.0);"#,
        )?;
        fixture.install()?;

        let command = format!("{}\\App.exe", fixture.destination.display());
        let recorded = manifest(&fixture)?;
        assert_eq!(
            recorded["registry_values"],
            serde_json::json!([{"path": SHARED_RUN_KEY, "name": value_name}])
        );
        // The key holds other products' values, so the manifest may claim the
        // value alone: a recorded key would make the uninstall delete the whole
        // `Run` key and stop everything else on the machine from starting.
        assert_eq!(recorded["registry_keys"], serde_json::json!([]));
        assert_eq!(
            install::read_registry_string(HKEY_CURRENT_USER, &subkey(SHARED_RUN_KEY), &value_name)?,
            Some(command)
        );

        fixture.uninstall(true)?;

        assert_eq!(
            install::read_registry_string(HKEY_CURRENT_USER, &subkey(SHARED_RUN_KEY), &value_name)?,
            None
        );
        assert!(install::registry_key_exists(
            HKEY_CURRENT_USER,
            &subkey(SHARED_RUN_KEY)
        )?);
        Ok(())
    }

    /// The key Windows keeps a user's environment variables in.
    ///
    /// Every product on the machine writes into it, so nothing here may remove
    /// it: a setup that did would take `PATH` with it.
    const USER_ENVIRONMENT_KEY: &str = r"HKCU\Environment";

    #[test]
    fn an_install_script_sets_a_variable_the_uninstall_takes_back() -> Result<()> {
        let name = unique_name("NANO_INSTALLER_SCRIPT_TEST");
        // The uninstall takes the value back; the guard covers a run that fails
        // before it gets that far.
        let _guard = TestRegistryValue {
            key: USER_ENVIRONMENT_KEY.to_string(),
            name: name.clone(),
        };
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let report = "";
                report += "first=" + set_env({name}, "configured").to_string() + "\n";
                report += "second=" + set_env({name}, "again").to_string() + "\n";
                report += "read=" + get_env({name}) + "\n";
                write_file(path_join(install_path, "env.txt"), report);
                "#,
                name = literal(&name)
            )),
            r#"run_tracked_uninstall(0.0, 100.0);"#,
        )?;
        fixture.install()?;

        // The script's own process keeps the environment it started with, which
        // is what Windows does for the program that writes the value: what the
        // write is for is the processes that start afterwards.
        assert_eq!(
            std::fs::read_to_string(fixture.destination.join("env.txt"))?,
            "first=true\nsecond=true\nread=\n"
        );
        let key = subkey(USER_ENVIRONMENT_KEY);
        assert_eq!(
            install::read_registry_string(HKEY_CURRENT_USER, &key, &name)?,
            Some("again".to_string()),
            "the variable did not land where Windows reads one from"
        );
        // The key belongs to Windows, so the manifest claims the value alone.
        let recorded = manifest(&fixture)?;
        assert_eq!(
            recorded["registry_values"],
            serde_json::json!([{"path": USER_ENVIRONMENT_KEY, "name": name}])
        );
        assert_eq!(recorded["registry_keys"], serde_json::json!([]));

        fixture.uninstall(true)?;

        assert_eq!(
            install::read_registry_string(HKEY_CURRENT_USER, &key, &name)?,
            None
        );
        assert!(
            install::registry_key_exists(HKEY_CURRENT_USER, &key)?,
            "the uninstall removed the key every product on the machine writes into"
        );
        Ok(())
    }

    #[test]
    fn an_install_script_removes_a_variable_it_no_longer_wants() -> Result<()> {
        let name = unique_name("NANO_INSTALLER_SCRIPT_TEST");
        let _guard = TestRegistryValue {
            key: USER_ENVIRONMENT_KEY.to_string(),
            name: name.clone(),
        };
        let key = subkey(USER_ENVIRONMENT_KEY);
        // A value the machine already had, which is what makes this a removal
        // rather than "there was nothing there".
        install::write_registry_string(HKEY_CURRENT_USER, &key, &name, "before")?;
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                write_file(path_join(install_path, "removed.txt"),
                           remove_env({name}).to_string());
                "#,
                name = literal(&name)
            )),
            "",
        )?;
        fixture.install()?;

        assert_eq!(
            std::fs::read_to_string(fixture.destination.join("removed.txt"))?,
            "true"
        );
        assert_eq!(
            install::read_registry_string(HKEY_CURRENT_USER, &key, &name)?,
            None
        );
        // Nothing is left to replay: a value that is gone, and a key that was
        // never this product's to begin with.
        let recorded = manifest(&fixture)?;
        assert_eq!(recorded["registry_values"], serde_json::json!([]));
        assert_eq!(recorded["registry_keys"], serde_json::json!([]));
        Ok(())
    }

    /// One file type of the test's own, so no real product's association is
    /// read, written or removed.
    fn unique_file_type() -> (String, String) {
        let tag = unique_name("nanoinstallerscripttest").replace('-', "");
        (tag.clone(), format!("NanoInstallerScriptTest.{tag}"))
    }

    #[test]
    fn a_script_registers_a_file_type_where_windows_reads_it() -> Result<()> {
        let (extension, prog_id) = unique_file_type();
        let extension_key = format!(r"HKCU\Software\Classes\.{extension}");
        let prog_id_key = format!(r"HKCU\Software\Classes\{prog_id}");
        let _extension_guard = TestRegistryKey(extension_key.clone());
        let _prog_id_guard = TestRegistryKey(prog_id_key.clone());
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let target = path_join(install_path, "App.exe");
                let registered = register_file_association({extension}, {prog_id},
                    "A script test file", target + " \"%1\"", target);
                write_file(path_join(install_path, "registered.txt"), registered.to_string());
                "#,
                extension = literal(&extension),
                prog_id = literal(&prog_id),
            )),
            r#"run_tracked_uninstall(0.0, 100.0);"#,
        )?;
        fixture.install()?;

        assert_eq!(
            std::fs::read_to_string(fixture.destination.join("registered.txt"))?,
            "true"
        );
        let target = fixture.destination.join("App.exe").display().to_string();
        let read = |key: &str| -> Result<Option<String>> {
            install::read_registry_string(HKEY_CURRENT_USER, &subkey(key), "")
        };
        // The four places Windows reads a file type from: the extension that
        // names the program id, and the words, the icon and the command that the
        // program id carries.
        assert_eq!(read(&extension_key)?, Some(prog_id.clone()));
        assert_eq!(read(&prog_id_key)?, Some("A script test file".to_string()));
        assert_eq!(
            read(&format!("{prog_id_key}\\DefaultIcon"))?,
            Some(target.clone())
        );
        assert_eq!(
            read(&format!("{prog_id_key}\\shell\\open\\command"))?,
            Some(format!("{target} \"%1\""))
        );

        fixture.uninstall(true)?;

        assert!(
            !install::registry_key_exists(HKEY_CURRENT_USER, &subkey(&extension_key))?,
            "the uninstall left the extension it claimed behind"
        );
        assert!(
            !install::registry_key_exists(HKEY_CURRENT_USER, &subkey(&prog_id_key))?,
            "the uninstall left the program id it registered behind"
        );
        Ok(())
    }

    #[test]
    fn a_file_type_that_would_write_outside_the_classes_tree_is_refused() -> Result<()> {
        // A name carrying a separator would write into a key of its own making
        // somewhere else in the registry, and a file type with no command opens
        // nothing: all three are refused before anything is written.
        let (extension, prog_id) = unique_file_type();
        let nested = format!("{extension}\\nested");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let target = path_join(install_path, "App.exe");
                let report = "";
                report += "separator=" + register_file_association({nested}, {prog_id}, "", target, "").to_string() + "\n";
                report += "empty_extension=" + register_file_association("", {prog_id}, "", target, "").to_string() + "\n";
                report += "empty_prog_id=" + register_file_association({extension}, "", "", target, "").to_string() + "\n";
                report += "no_command=" + register_file_association({extension}, {prog_id}, "", "", "").to_string() + "\n";
                report += "unregister_separator=" + unregister_file_association({nested}, {prog_id}).to_string() + "\n";
                write_file(path_join(install_path, "refused.txt"), report);
                "#,
                nested = literal(&nested),
                extension = literal(&extension),
                prog_id = literal(&prog_id),
            )),
            "",
        )?;
        fixture.install()?;

        assert_eq!(
            std::fs::read_to_string(fixture.destination.join("refused.txt"))?,
            "separator=false\nempty_extension=false\nempty_prog_id=false\nno_command=false\nunregister_separator=false\n"
        );
        assert!(
            !install::registry_key_exists(
                HKEY_CURRENT_USER,
                &subkey(&format!(r"HKCU\Software\Classes\.{extension}"))
            )?,
            "a refused call wrote into the classes tree anyway"
        );
        let recorded = manifest(&fixture)?;
        assert_eq!(recorded["registry_values"], serde_json::json!([]));
        assert_eq!(recorded["registry_keys"], serde_json::json!([]));
        Ok(())
    }

    #[test]
    fn out_of_range_progress_and_both_status_forms_do_not_disturb_the_install() -> Result<()> {
        let report = Observation::new("script-progress");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                set_progress(-1.0);
                set_progress(150.0);
                set_progress(100.0 / 0.0);
                set_status("literal status");
                set_status_key("status.installing_uninstaller");
                let report = "";
                report += "mode=" + get_mode() + "\n";
                report += "cancelled=" + is_cancelled().to_string() + "\n";
                report += "checkbox=" + get_checkbox_value("chkShotcut").to_string() + "\n";
                write_file({}, report);
                "#,
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        // The wizard is the only observer of progress and status, so a
        // windowless run can pin this much: a percentage below zero, above a
        // hundred, or not a number at all, and both status forms, all go through
        // the same path, and the deployment still finishes and reports itself.
        // A clamp that turned into a failure would abort the install here.
        assert!(fixture.destination.join("App.exe").is_file());
        assert!(fixture.destination.join(install::MANIFEST_NAME).is_file());
        assert_eq!(
            report.text()?,
            "mode=install\ncancelled=false\ncheckbox=false\n"
        );
        Ok(())
    }

    /// Starts the install of a fixture whose script waits for the wizard's
    /// cancel request, and returns what the install answered together with
    /// whether it left a directory behind.
    fn install_stopped_by_the_user(script: &str) -> Result<(Result<()>, bool)> {
        let fixture = fixture(&deploying_script(script), "")?;
        // In the wizard this click arrives on the window's own thread while the
        // worker runs the script.
        let task = install::Cancellation::default();
        let request = task.clone();
        let watcher = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            request.request();
        });
        let result = fixture.install_with(task);
        watcher.join().expect("the watcher thread");
        Ok((result, fixture.destination.exists()))
    }

    #[test]
    fn a_running_script_sees_the_cancel_request() -> Result<()> {
        // A step of the script's own can end early instead of running to its
        // end, which is the whole reason `is_cancelled` exists: the runtime only
        // stops at the checkpoints between steps.
        let (error, _) = install_stopped_by_the_user(
            r#"
            let waited = 0;
            while !is_cancelled() && waited < 8000 {
                sleep_ms(20);
                waited += 20;
            }
            throw "stopped after " + waited + "ms";
            "#,
        )?;
        let error = error.expect_err("the script gave up");
        let message = format!("{error:#}");
        let waited: u64 = message
            .rsplit("stopped after ")
            .next()
            .and_then(|tail| tail.split("ms").next())
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or_else(|| panic!("the script did not say how long it waited: {message}"));
        assert!(
            waited < 4000,
            "the script waited {waited}ms for a request made 50ms in, so is_cancelled told it nothing"
        );
        Ok(())
    }

    #[test]
    fn a_cancelled_install_gives_up_after_the_script_and_undoes_what_it_wrote() -> Result<()> {
        // A script that ends its own long step leaves the install to the
        // runtime, which stops at the checkpoint after it rather than deploy a
        // product the user asked not to have.
        let (error, destination_exists) = install_stopped_by_the_user(
            r#"
            let waited = 0;
            while !is_cancelled() && waited < 8000 {
                sleep_ms(20);
                waited += 20;
            }
            "#,
        )?;
        let error = error.expect_err("a task the user stopped does not install");
        assert_eq!(error.to_string(), "cancelled by the user");
        assert!(
            !destination_exists,
            "the cancelled install left the directory it created behind"
        );
        Ok(())
    }

    #[test]
    fn an_uninstall_script_sees_the_uninstall_mode_and_the_keep_data_checkbox() -> Result<()> {
        // Both states of the box: a script that keeps user data reads the value
        // the other way round from one that deletes it.
        for keep_data in [true, false] {
            let report = Observation::new("script-uninstall-mode");
            let uninstall_script = format!(
                r#"
                let report = "";
                report += "mode=" + get_mode() + "\n";
                report += "keep_data=" + get_checkbox_value("keep_data").to_string() + "\n";
                report += "unknown=" + get_checkbox_value("chkShotcut").to_string() + "\n";
                report += "cancelled=" + is_cancelled().to_string() + "\n";
                write_file({}, report);
                "#,
                report.script_path()
            );
            // The install only has to succeed; the uninstall is the half this
            // test reads.
            let fixture = fixture(&deploying_script(""), &uninstall_script)?;
            fixture.install()?;
            fixture.uninstall(keep_data)?;

            assert_eq!(
                report.text()?,
                format!("mode=uninstall\nkeep_data={keep_data}\nunknown=false\ncancelled=false\n")
            );
        }
        Ok(())
    }

    #[test]
    fn a_script_reads_the_values_the_page_holds() -> Result<()> {
        let report = Observation::new("script-page-values");
        let install_script = format!(
            r#"
            let install_path = get_install_path();
            copy_uninstaller();
            write_file(path_join(install_path, "App.exe"), "app");
            let report = "";
            report += "serial=" + get_text_value("serial") + "\n";
            report += "edition=" + get_choice_value("edition") + "\n";
            report += "untyped=" + get_text_value("nowhere") + "\n";
            report += "unchosen=" + get_choice_value("nothing") + "\n";
            write_file({}, report);
            "#,
            report.script_path()
        );
        let fixture = fixture(&install_script, "")?;
        // What the wizard hands over when the user starts the task: the text the
        // page's field holds, and the value its choice group ended on.
        fixture.install_with_values(
            &[("serial", "TT-2026-0001")],
            &[("edition", "installed")],
            &[],
        )?;

        assert_eq!(
            report.text()?,
            "serial=TT-2026-0001\nedition=installed\nuntyped=\nunchosen=\n"
        );
        Ok(())
    }

    /// The components a run installs are the project's and the page's answer
    /// together, and a script reads them the way the wizard applied them.
    #[test]
    fn a_script_sees_the_components_the_run_installs() -> Result<()> {
        let report = Observation::new("script-components");
        let install_script = format!(
            r#"
            let install_path = get_install_path();
            copy_uninstaller();
            write_file(path_join(install_path, "App.exe"), "app");
            let chosen = selected_components();
            let report = "";
            report += "docs=" + is_component_selected("docs").to_string() + "\n";
            report += "tools=" + is_component_selected("tools").to_string() + "\n";
            report += "samples=" + is_component_selected("samples").to_string() + "\n";
            report += "nowhere=" + is_component_selected("nowhere").to_string() + "\n";
            report += "chosen=" + chosen.len().to_string() + "\n";
            report += "first=" + chosen[0] + "\n";
            write_file({}, report);
            "#,
            report.script_path()
        );
        let mut fixture = fixture(&install_script, "")?;
        fixture.config["components"] = serde_json::json!({ "items": [
            { "id": "docs", "payload": "payload/docs.7z" },
            { "id": "tools", "payload": "payload/tools.7z", "default": true },
            { "id": "samples", "payload": "payload/samples.7z", "required": true },
        ] });
        // The page ticked docs, left tools' default alone, and cannot clear samples.
        fixture.install_with_values(&[], &[], &[("docs", true)])?;

        assert_eq!(
            report.text()?,
            "docs=true\ntools=true\nsamples=true\nnowhere=false\nchosen=3\nfirst=docs\n"
        );
        Ok(())
    }

    /// A script asks the machine about a dependency, and asks for one to be
    /// installed, through the project's own declaration of it.
    ///
    /// The two answers a script needs are "is it there" and "put it there".
    /// Both read `dependencies.items`, so a script that decides *when* to check
    /// a dependency does not also have to restate *what* it is: the project
    /// keeps one rule, and the built-in flow and the script ask it the same
    /// question.
    #[test]
    fn a_script_asks_the_machine_about_the_dependencies_the_project_declares() -> Result<()> {
        let report = Observation::new("script-dependencies");
        let install_script = deploying_script(&format!(
            r#"
            let report = "";
            report += "present=" + dependency_installed("present").to_string() + "\n";
            report += "absent=" + dependency_installed("absent").to_string() + "\n";
            report += "undeclared=" + dependency_installed("nowhere").to_string() + "\n";
            report += "installs_present=" + install_dependency("present").to_string() + "\n";
            report += "installs_absent=" + install_dependency("absent").to_string() + "\n";
            write_file({}, report);
            "#,
            report.script_path()
        ));
        let mut fixture = fixture(&install_script, "")?;
        // The machine has the first dependency and not the second: the rule the
        // project writes is what says so, and the value the test wrote into the
        // fixture's own key is what the first rule reads.
        let (root, path) = install::registry_path(&fixture.registry_key)?;
        install::write_registry_dword(root, &path, "Installed", 1)?;
        fixture.config["dependencies"] = serde_json::json!({ "items": [
            {
                "id": "present",
                "detect": { "registry": {
                    "key": fixture.registry_key,
                    "name": "Installed",
                    "equals": "1"
                } },
                "payload": "payload/present.exe"
            },
            {
                "id": "absent",
                "detect": { "registry": {
                    "key": format!(r"{}\Nowhere", fixture.registry_key),
                    "name": "Installed"
                } },
                "payload": "payload/absent.exe"
            }
        ] });
        fixture.install()?;

        // The one that is there needs nothing done to it, and the one that is
        // not cannot be installed from a bundle that does not carry its
        // program, which is what the last line reports rather than an error that
        // stops the install.
        assert_eq!(
            report.text()?,
            "present=true\nabsent=false\nundeclared=false\ninstalls_present=true\ninstalls_absent=false\n"
        );
        Ok(())
    }

    #[test]
    fn uninstalling_below_appdata_removes_the_data_only_when_the_box_is_cleared() -> Result<()> {
        // The driver's `keep_data` flag has no other way in: a silent uninstall
        // always keeps user data, and the interactive uninstaller's checkbox is
        // out of reach. The directory has to sit under the real profile, because
        // the driver refuses any data path that is not below `%APPDATA%` or
        // `%LOCALAPPDATA%`.
        for keep_data in [false, true] {
            let data_name = unique_name("nano-installer-script-test-data");
            let appdata = std::env::var("APPDATA").context("the profile names APPDATA")?;
            let data_path = PathBuf::from(&appdata).join(&data_name);
            // A real directory in the profile, so the guard removes it however
            // this test ends.
            let mut cleanup = ProfileCleanup::default();
            cleanup.keep(data_path.clone());

            let mut fixture = fixture(&deploying_script(""), r#"log_info("nothing to do");"#)?;
            // `fixture` builds the configuration the common case needs, so the
            // data path this case needs is added to it here.
            fixture
                .config
                .as_object_mut()
                .expect("the fixture configuration is an object")
                .insert(
                    "uninstall".to_string(),
                    serde_json::json!({"data_paths": [format!("%APPDATA%\\{data_name}")]}),
                );
            std::fs::create_dir_all(&data_path)?;
            std::fs::write(data_path.join("settings.ini"), "user data")?;
            fixture.install()?;

            fixture.uninstall(keep_data)?;

            if keep_data {
                // The box was left checked, so the user's settings stay exactly
                // where they were.
                assert_eq!(
                    std::fs::read_to_string(data_path.join("settings.ini"))?,
                    "user data"
                );
            } else {
                // Clearing the box is the only thing that removes user data,
                // and this is the only place that branch can be run.
                assert!(!data_path.exists());
            }
        }
        Ok(())
    }

    #[test]
    fn the_script_reads_the_environment_and_the_project_configuration() -> Result<()> {
        let absent = unique_name("NANO_INSTALLER_ABSENT");
        let report = Observation::new("script-environment");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let report = "";
                report += "system_root=" + get_env("SystemRoot") + "\n";
                report += "absent=" + get_env("{}") + "\n";
                report += "project_name=" + get_config_value("project.name") + "\n";
                report += "exe_name=" + get_config_value("install.exe_name") + "\n";
                report += "missing_type=" + type_of(get_config_value("project.missing")) + "\n";
                report += "project_type=" + type_of(get_config_value("project")) + "\n";
                write_file({}, report);
                "#,
                absent,
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        // A variable the machine has is read exactly, and one it does not have
        // is empty rather than an error. A configuration path that does not
        // exist is unit and an object is a map, so a script can tell a typo from
        // an empty value instead of comparing against a string.
        assert_eq!(
            report.text()?,
            format!(
                "system_root={}\n\
                 absent=\n\
                 project_name=Script Test\n\
                 exe_name=App.exe\n\
                 missing_type=()\n\
                 project_type=map\n",
                std::env::var("SystemRoot").unwrap_or_default()
            )
        );
        Ok(())
    }

    #[test]
    fn the_script_reports_the_image_it_runs_from() -> Result<()> {
        let report = Observation::new("script-image-path");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let report = "";
                report += "exe=" + get_current_exe() + "\n";
                report += "dir=" + get_exe_dir() + "\n";
                write_file({}, report);
                "#,
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        // The script runs inside this test binary, so the image it names has to
        // be this test's own executable rather than, say, the bundle it reads.
        let exe = std::env::current_exe()?;
        let directory = exe
            .parent()
            .expect("the test executable lives in a directory");
        assert_eq!(
            report.text()?,
            format!("exe={}\ndir={}\n", exe.display(), directory.display())
        );
        Ok(())
    }

    #[test]
    fn the_script_queries_fixed_disks_and_notifies_the_shell() -> Result<()> {
        let report = Observation::new("script-drives");
        let drive = temp_drive_root();
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let report = "";
                let listed = "";
                let drives = get_drives();
                for entry in drives {{ listed += entry + "|"; }}
                report += "drives=" + listed + "\n";
                let space = get_drive_space({});
                report += "space_count=" + space.len().to_string() + "\n";
                let free = space[0];
                let total = space[1];
                let positive = free > 0;
                let fits = free <= total;
                report += "free_positive=" + positive.to_string() + "\n";
                report += "free_within_total=" + fits.to_string() + "\n";
                // A shell notification returns nothing and cannot fail, so the
                // only thing a script can rely on is that it carries on after
                // it, which the line below proves.
                shell_notify();
                report += "notified=true\n";
                write_file({}, report);
                "#,
                literal(&drive),
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        let text = report.text()?;
        let observed = observations(&text);
        let listed = observed
            .get("drives")
            .expect("the script listed the fixed disks");
        let drives = listed
            .split('|')
            .filter(|entry| !entry.is_empty())
            .collect::<Vec<_>>();
        for entry in &drives {
            assert!(
                entry.len() == 3 && entry.ends_with(":\\"),
                "{entry} is a drive root such as C:\\"
            );
        }
        // The tests run from the temporary directory, so its drive is one of the
        // fixed disks that has to be reported. Removable media and network
        // shares are deliberately absent.
        assert!(drives.contains(&drive.as_str()));
        assert_eq!(observed.get("space_count").map(String::as_str), Some("2"));
        assert_eq!(
            observed.get("free_positive").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            observed.get("free_within_total").map(String::as_str),
            Some("true")
        );
        assert_eq!(observed.get("notified").map(String::as_str), Some("true"));
        Ok(())
    }

    #[test]
    fn a_script_runs_a_command_and_sees_its_exit_code() -> Result<()> {
        let report = Observation::new("script-run-command");
        let missing = unique_name("nano-installer-no-such-command");
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                // The command shell is the one command every Windows machine
                // has, so its exit code is the one the script has to see.
                let shell = get_env("ComSpec");
                let report = "";
                report += "three=" + run_command(shell, ["/C", "exit 3"]).to_string() + "\n";
                report += "zero=" + run_command(shell, ["/C", "exit 0"]).to_string() + "\n";
                let absent = path_join(get_temp_path(), "{missing}.exe");
                report += "missing=" + run_command(absent, ["arg"]).to_string() + "\n";
                write_file({report}, report);
                "#,
                report = report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        // A command that cannot be started reports -1 rather than an exit code,
        // so a script can tell "it ran and failed" from "it never ran".
        assert_eq!(report.text()?, "three=3\nzero=0\nmissing=-1\n");
        Ok(())
    }

    #[test]
    fn a_script_recognises_a_running_process_by_its_image_name() -> Result<()> {
        let report = Observation::new("script-process-check");
        let running = std::env::current_exe()?
            .file_name()
            .expect("the test executable has a file name")
            .to_string_lossy()
            .to_string();
        let absent = format!("{}.exe", unique_name("nano-installer-no-such-process"));
        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                let report = "";
                report += "running=" + is_process_running({}).to_string() + "\n";
                report += "absent=" + is_process_running({}).to_string() + "\n";
                write_file({}, report);
                "#,
                literal(&running),
                literal(&absent),
                report.script_path()
            )),
            "",
        )?;
        fixture.install()?;

        // The script runs in this test's own process, so its image name is the
        // one process the check has to find. This is what stops an install from
        // replacing a product while it is running.
        assert_eq!(report.text()?, "running=true\nabsent=false\n");
        Ok(())
    }

    #[test]
    fn an_install_script_creates_shortcuts_the_uninstall_takes_back() -> Result<()> {
        let desktop_name = unique_name("nano-installer-script-test-desktop");
        let menu_name = unique_name("nano-installer-script-test-menu");
        let folder_name = unique_name("nano-installer-script-test-folder");
        let uninstall_name = unique_name("nano-installer-script-test-uninstall");

        // These are real links in the profile, so the guard removes them however
        // the test ends.
        let desktop_link = crate::shell::desktop_directory()?.join(format!("{desktop_name}.lnk"));
        let folder = crate::shell::programs_directory()?.join(&folder_name);
        let menu_link = folder.join(format!("{menu_name}.lnk"));
        let uninstall_link = folder.join(format!("{uninstall_name}.lnk"));
        let mut cleanup = ProfileCleanup::default();
        cleanup.keep(desktop_link.clone());
        cleanup.keep(menu_link.clone());
        cleanup.keep(uninstall_link.clone());
        cleanup.keep(folder.clone());

        let fixture = fixture(
            &deploying_script(&format!(
                r#"
                create_desktop_shortcut({desktop}, path_join(install_path, "App.exe"));
                create_start_menu_shortcut({menu}, path_join(install_path, "App.exe"), {folder});
                create_uninstall_shortcut({uninstall}, path_join(install_path, "uninst.exe"), {folder});
                "#,
                desktop = literal(&desktop_name),
                menu = literal(&menu_name),
                folder = literal(&folder_name),
                uninstall = literal(&uninstall_name),
            )),
            r#"run_tracked_uninstall(0.0, 100.0);"#,
        )?;
        fixture.install()?;

        // Each link has to land where Windows looks for it, and the manifest has
        // to name it: a link the manifest does not know about stays on the
        // desktop as a dead icon after the product is gone.
        assert!(desktop_link.is_file());
        assert!(menu_link.is_file());
        assert!(uninstall_link.is_file());
        let recorded = manifest(&fixture)?;
        assert_eq!(
            recorded["shortcuts"],
            serde_json::json!([
                desktop_link.to_string_lossy(),
                menu_link.to_string_lossy(),
                uninstall_link.to_string_lossy()
            ])
        );
        assert_eq!(
            recorded["shortcut_dirs"],
            serde_json::json!([folder.to_string_lossy()])
        );

        fixture.uninstall(true)?;

        assert!(!desktop_link.exists());
        assert!(!menu_link.exists());
        assert!(!uninstall_link.exists());
        // The Start Menu folder the installer created goes with them, so the
        // menu keeps no empty product folder.
        assert!(!folder.exists());
        Ok(())
    }

    #[test]
    fn a_script_deletes_the_desktop_shortcut_and_the_start_menu_folder_it_created() -> Result<()> {
        let desktop_name = unique_name("nano-installer-script-test-remove-desktop");
        let menu_name = unique_name("nano-installer-script-test-remove-menu");
        let folder_name = unique_name("nano-installer-script-test-remove-folder");
        let report = Observation::new("script-delete-shortcuts");

        let desktop_link = crate::shell::desktop_directory()?.join(format!("{desktop_name}.lnk"));
        let folder = crate::shell::programs_directory()?.join(&folder_name);
        let menu_link = folder.join(format!("{menu_name}.lnk"));
        let mut cleanup = ProfileCleanup::default();
        cleanup.keep(desktop_link.clone());
        cleanup.keep(menu_link.clone());
        cleanup.keep(folder.clone());

        let fixture = fixture(
            // The install only has to leave the links behind; the uninstall is
            // the half this test reads.
            &deploying_script(&format!(
                r#"
                create_desktop_shortcut({desktop}, path_join(install_path, "App.exe"));
                create_start_menu_shortcut({menu}, path_join(install_path, "App.exe"), {folder});
                "#,
                desktop = literal(&desktop_name),
                menu = literal(&menu_name),
                folder = literal(&folder_name),
            )),
            &format!(
                r#"
                // Both primitives report whether they removed anything, which is
                // what a script checks before it finishes; the links are gone by
                // then, so the report is the only witness left.
                let report = "";
                report += "desktop=" + delete_desktop_shortcut({desktop}).to_string() + "\n";
                report += "folder=" + delete_start_menu_folder({folder}).to_string() + "\n";
                write_file({report}, report);
                "#,
                desktop = literal(&desktop_name),
                folder = literal(&folder_name),
                report = report.script_path(),
            ),
        )?;
        fixture.install()?;
        assert!(desktop_link.is_file());
        assert!(menu_link.is_file());

        fixture.uninstall(true)?;

        assert_eq!(report.text()?, "desktop=true\nfolder=true\n");
        assert!(!desktop_link.exists());
        assert!(!menu_link.exists());
        assert!(!folder.exists());
        Ok(())
    }

    /// `get_tools_dir()` unpacks the bundled tools and returns the directory
    /// holding them, nested paths intact, without unpacking twice.
    #[test]
    fn a_script_reads_the_tools_the_project_bundled() -> Result<()> {
        let report = Observation::new("script-tools-dir");
        let fixture = fixture_with_tools(
            &deploying_script(&format!(
                r#"
                let tools = get_tools_dir();
                let report = "";
                report += "empty=" + (tools == "").to_string() + "\n";
                report += "exe=" + read_text_file(path_join(tools, "7za.exe")) + "\n";
                report += "nested=" + read_text_file(path_join(path_join(tools, "bin"), "helper.dll")) + "\n";
                report += "same=" + (get_tools_dir() == tools).to_string() + "\n";
                write_file({report}, report);
                "#,
                report = report.script_path()
            )),
            "tools",
            &[("7za.exe", "seven zip"), ("bin/helper.dll", "helper")],
        )?;
        fixture.install()?;

        // A real directory the script can hand to run_command, holding the tools
        // as the project stored them: nested paths included, and asking twice
        // unpacks nothing a second time.
        assert_eq!(
            report.text()?,
            "empty=false\nexe=seven zip\nnested=helper\nsame=true\n"
        );
        Ok(())
    }

    /// A script that asks for tools its setup does not carry gets an empty
    /// string and a warning in the log, and the install finishes.
    #[test]
    fn a_script_that_asks_for_tools_a_project_did_not_bundle_gets_nothing() -> Result<()> {
        let script = |report: &Observation| {
            deploying_script(&format!(
                r#"
                let report = "";
                report += "empty=" + (get_tools_dir() == "").to_string() + "\n";
                write_file({report}, report);
                "#,
                report = report.script_path()
            ))
        };

        // A project that names no tools directory at all.
        let unnamed = Observation::new("script-tools-unnamed");
        fixture(&script(&unnamed), "")?.install()?;
        assert_eq!(unnamed.text()?, "empty=true\n");

        // And one that names a directory its bundle carries nothing under: an
        // older setup, or a build made without the tools. Both come back empty
        // and leave a warning in the log rather than failing the install.
        let missing = Observation::new("script-tools-missing");
        fixture_with_tools(&script(&missing), "tools", &[])?.install()?;
        assert_eq!(missing.text()?, "empty=true\n");
        Ok(())
    }
}
