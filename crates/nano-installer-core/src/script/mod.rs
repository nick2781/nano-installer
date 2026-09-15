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
use context::{
    log, log_tail, read_script, reset_log, undo, ScriptContext, ScriptEnvironment, Snapshot,
};

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
    /// Scratch directory owned by the caller for the duration of the script.
    pub(super) stage: PathBuf,
    pub(super) prep: InstallPrep,
}

/// Runs `scripts/install.rhai` in place of the built-in deployment steps.
pub(super) fn run_install(request: InstallRequest) -> Result<()> {
    let InstallRequest {
        setup,
        bundle,
        config,
        destination,
        selection,
        stage,
        mut prep,
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
        keep_data: false,
        manifest: serde_json::Value::Null,
        previous: prep.previous.take(),
        bundle,
        stage,
        journal: None,
        mode: Mode::Install,
    })?;

    if let Err(error) = run(&context, &source) {
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
    } = request;
    let source = read_script(&bundle, UNINSTALL_SCRIPT)?;
    let context = begin(ScriptEnvironment {
        setup: uninstaller.clone(),
        config: config.clone(),
        install_path: destination.clone(),
        checkboxes: HashMap::from([("keep_data".to_string(), keep_data)]),
        keep_data,
        manifest: manifest.clone(),
        previous: None,
        bundle,
        stage,
        journal: None,
        mode: Mode::Uninstall,
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
        let registry_key = unique_registry_key();
        let config = config(&registry_key);
        let config_text = config.to_string();
        let temp = tempfile::tempdir()?;
        let setup = temp.path().join("setup.exe");
        bundle_image(
            &setup,
            &[
                ("installer_config.json", &config_text),
                ("scripts/install.rhai", install_script),
                ("scripts/uninstall.rhai", uninstall_script),
                ("runtime/uninst.exe", "uninstaller"),
            ],
        )?;
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
            let bundle = self.bundle()?;
            let (root, registry_path) = install::uninstall_registry_key(&self.config)?;
            run_install(InstallRequest {
                setup: self.setup.clone(),
                bundle,
                config: self.config.clone(),
                destination: self.destination.clone(),
                selection: InstallSelection::default(),
                stage: self.stage()?,
                prep: InstallPrep {
                    uninstaller_name: "uninst.exe".to_string(),
                    uninstaller: b"uninstaller".to_vec(),
                    root,
                    registry_path,
                    previous: None,
                    upgrade: false,
                },
            })
        }

        fn uninstall(&self, keep_data: bool) -> Result<()> {
            let bundle = self.bundle()?;
            let (root, registry_path) = install::uninstall_registry_key(&self.config)?;
            let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
                self.destination.join(install::MANIFEST_NAME),
            )?)?;
            run_uninstall(UninstallRequest {
                uninstaller: self.destination.join("uninst.exe"),
                bundle,
                config: self.config.clone(),
                destination: self.destination.clone(),
                manifest,
                keep_data,
                stage: self.stage()?,
                root,
                registry_path,
            })
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
}
