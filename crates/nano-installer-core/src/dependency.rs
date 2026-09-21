//! The runtimes a product needs from the machine, and what a setup does about
//! their absence.
//!
//! The VC++ runtimes, the WebView2 runtime, a .NET Framework version: a product
//! that needs one cannot carry it as part of its own payload, because they
//! belong to the machine, are installed once, and are shared by every product
//! on it. What the setup owns is the check -- is it there? -- and the program
//! that puts it there when the answer is no.
//!
//! Both are declared rather than scripted. `dependencies.items` names each one,
//! the rule that answers the check, and the program to run when it fails; the
//! built-in flow walks them before it unpacks anything, and a project with a
//! script reaches the same code through `dependency_installed` and
//! `install_dependency`.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use crate::install::{parse_registry_key, Cancellation, Cancelled};
use crate::{net, shell, BundleIndex};

/// Exit codes an installer uses to say it worked.
///
/// `1638` is what a redistributable returns when a newer version is already on
/// the machine, and `3010` and `1641` say the install succeeded and Windows
/// wants a restart; refusing those would turn a successful dependency into a
/// failed installation.
const EXIT_ALREADY_INSTALLED: i64 = 1638;
const EXIT_RESTART_REQUIRED: [i64; 2] = [3010, 1641];

/// The dependencies a project declares, in the order it declares them.
pub(super) fn items(config: &Value) -> &[Value] {
    config["dependencies"]["items"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// The entry `id` names.
fn entry<'a>(config: &'a Value, id: &str) -> Result<&'a Value> {
    items(config)
        .iter()
        .find(|item| item["id"].as_str() == Some(id))
        .with_context(|| format!("{id} is not a dependency this project declares"))
}

/// Whether the dependency `id` is already on the machine.
pub(super) fn installed(config: &Value, id: &str) -> Result<bool> {
    detect(&entry(config, id)?["detect"])
}

/// Whether the project's rule for this dependency says the machine has it.
fn detect(rule: &Value) -> Result<bool> {
    if let Some(path) = rule["file"].as_str() {
        let expanded = shell::expand_environment(path)?;
        return Ok(Path::new(expanded.trim()).is_file());
    }
    let registry = &rule["registry"];
    let key = registry["key"]
        .as_str()
        .context("a dependency's detect rule needs a registry key or a file")?;
    let key = parse_registry_key(key)?;
    // A rule that names no value asks about the key itself, which is how a
    // product that registers an installation is detected.
    let Some(name) = registry["name"].as_str() else {
        return key.exists();
    };
    let Some(actual) = key.read_text(name)? else {
        return Ok(false);
    };
    if let Some(expected) = registry["equals"].as_str() {
        return Ok(actual == expected);
    }
    if let Some(wanted) = registry["at_least"].as_str() {
        return Ok(at_least(&actual, wanted));
    }
    Ok(true)
}

/// Compares two dotted versions, the way a detection rule means to.
///
/// A vendor writes a version where one is meaningful and a plain number where
/// it is not -- .NET Framework records a `Release` of `528040`, WebView2 a
/// `pv` of `120.0.2210.91` -- and one comparison serves both, because a missing
/// part counts as zero.
fn version_order(left: &str, right: &str) -> Ordering {
    let mut left = left.split('.');
    let mut right = right.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (left, right) => match part(left).cmp(&part(right)) {
                Ordering::Equal => continue,
                order => return order,
            },
        }
    }
}

/// A version part as a number; a part that is absent, or is text rather than a
/// number, counts as zero.
fn part(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn at_least(actual: &str, wanted: &str) -> bool {
    version_order(actual, wanted) != Ordering::Less
}

/// Installs every dependency the machine is missing, in the order the project
/// declares them.
///
/// The built-in flow calls this before it unpacks the payload, so a machine
/// that cannot be given what the product needs reports that instead of
/// installing a product that will not start. A dependency the project marks
/// `required` stops the run when it cannot be put there; one it does not is
/// noted and the run carries on.
pub(super) fn install_missing(
    config: &Value,
    bundle: &BundleIndex,
    scratch: &Path,
    task: &Cancellation,
) -> Result<()> {
    let mut missing = Vec::new();
    for item in items(config) {
        if item["id"].as_str().is_none() {
            continue;
        }
        if !detect(&item["detect"])? {
            missing.push(item);
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let _ = crate::report_progress(DEPENDENCY_START as u8, "status.dependencies");
    let span = (EXTRACT_START - DEPENDENCY_START) / missing.len() as f64;
    for (index, item) in missing.iter().enumerate() {
        let id = item["id"].as_str().unwrap_or_default();
        let start = DEPENDENCY_START + span * index as f64;
        let mut published = -1i32;
        let outcome = install(
            config,
            id,
            bundle,
            scratch,
            &mut |percent| {
                let percent = (start + span * percent / 100.0) as i32;
                if percent != published {
                    published = percent;
                    let _ = crate::publish_progress(percent as u8);
                }
            },
            task,
        );
        let required = item["required"].as_bool() == Some(true);
        match outcome {
            Ok(true) => {}
            Ok(false) if required => bail!(
                "dependency {id} is required, and the rule the project declares still says the machine is missing it after its installer ran; check that the rule reads the value the installer writes"
            ),
            Ok(false) => note(
                "warn",
                &format!("dependency {id} could not be installed; the run continues without it"),
            ),
            Err(error) if required => return Err(error),
            Err(error) => note(
                "warn",
                &format!("dependency {id} could not be installed ({error:#}); the run continues without it"),
            ),
        }
    }
    Ok(())
}

/// Where the built-in flow's dependency step sits between the steps around it.
const DEPENDENCY_START: f64 = 6.0;
const EXTRACT_START: f64 = 14.0;

/// Installs `id` when the machine is missing it, and reports whether it is
/// there afterwards.
///
/// `progress` is told how far this one dependency has got, from 0 to 100.
pub(super) fn install(
    config: &Value,
    id: &str,
    bundle: &BundleIndex,
    scratch: &Path,
    progress: &mut dyn FnMut(f64),
    task: &Cancellation,
) -> Result<bool> {
    let item = entry(config, id)?;
    if detect(&item["detect"])? {
        return Ok(true);
    }
    let program = program(item, id, bundle, scratch, progress, task)?;
    let arguments = arguments(item);
    note(
        "info",
        &format!("installing dependency {id} with {}", program.display()),
    );
    match run(&program, &arguments, task)? {
        0 => {}
        EXIT_ALREADY_INSTALLED => note(
            "info",
            &format!("dependency {id}: a newer version is already installed"),
        ),
        code if EXIT_RESTART_REQUIRED.contains(&code) => note(
            "info",
            &format!("dependency {id} installed and asks for a restart"),
        ),
        code => bail!("dependency {id} failed with exit code {code}"),
    }
    detect(&item["detect"])
}

/// The program that installs this dependency, brought to the scratch directory
/// so it can be run whatever it was shipped in.
fn program(
    item: &Value,
    id: &str,
    bundle: &BundleIndex,
    scratch: &Path,
    progress: &mut dyn FnMut(f64),
    task: &Cancellation,
) -> Result<PathBuf> {
    let directory = scratch.join("dependencies");
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("cannot create {}", directory.display()))?;
    if let Some(payload) = item["payload"].as_str() {
        if !bundle.contains(payload) {
            bail!("dependency {id} is missing the program it declares: {payload}");
        }
        let target = directory.join(file_name(payload));
        bundle.copy_file_to(payload, &target)?;
        return Ok(target);
    }
    let download = &item["download"];
    let url = download["url"]
        .as_str()
        .context("a dependency that is not bundled needs a download URL")?;
    let target = directory.join(download_name(id, url));
    let mut report = |read: u64, total: Option<u64>| {
        if let Some(total) = total.filter(|total| *total > 0) {
            progress(read as f64 / total as f64 * 100.0);
        }
    };
    net::download(url, &target, download["sha256"].as_str(), &mut report, task)?;
    Ok(target)
}

/// The arguments the project passes to that program.
fn arguments(item: &Value) -> Vec<String> {
    item["arguments"]
        .as_array()
        .map(|arguments| {
            arguments
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Runs the installer and waits, the way the payload extraction waits: a user
/// who asks to stop must not be left watching a step that cannot be stopped.
fn run(program: &Path, arguments: &[String], task: &Cancellation) -> Result<i64> {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;

    note(
        "info",
        &format!("running {} {}", program.display(), arguments.join(" ")),
    );
    let mut child = std::process::Command::new(program)
        .args(arguments)
        // A dependency installer is a console program as often as not, and a
        // setup that is running silently must not flash a console window for
        // one.
        .creation_flags(CREATE_NO_WINDOW.0)
        .spawn()
        .with_context(|| format!("cannot run {}", program.display()))?;
    loop {
        if task.requested() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Cancelled.into());
        }
        match child.try_wait()? {
            Some(status) => return Ok(status.code().map(i64::from).unwrap_or(-1)),
            None => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    }
}

/// The name a bundled program is copied out under.
fn file_name(payload: &str) -> String {
    payload
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("dependency.exe")
        .to_string()
}

/// The name a downloaded program is given on disk.
///
/// A URL that names an executable keeps its name, because a vendor's installer
/// occasionally cares what it is called; everything else -- a URL that ends in
/// a query string, or in a directory-like segment -- is named after the
/// dependency and given an `.exe` suffix, which is what Windows needs to run it
/// later.
fn download_name(id: &str, url: &str) -> String {
    let segment = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    if segment.to_ascii_lowercase().ends_with(".exe") {
        return segment.replace(['\\', ':', '*', '"', '<', '>', '|'], "_");
    }
    format!("{id}.exe")
}

/// Records something the run should be able to report.
///
/// A window has no console behind it and a silent run has no window to put a
/// card in, so a note travels both ways: the log a failure reports, and the
/// stream the caller of a windowless run reads.
fn note(level: &str, message: &str) {
    crate::script::log(level, message);
    if crate::silent_mode() {
        eprintln!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    /// A name unique to one case in one process, for anything a case creates.
    fn unique_name(label: &str) -> String {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!(
            "{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, AtomicOrdering::Relaxed)
        )
    }

    /// One registry key per test, so tests running in parallel never share one.
    fn unique_key() -> String {
        format!(
            "HKCU\\Software\\{}",
            unique_name("nano-installer-dependency-test")
        )
    }

    struct TestKey(String);

    impl Drop for TestKey {
        fn drop(&mut self) {
            let Ok(key) = parse_registry_key(&self.0) else {
                return;
            };
            let _ = key.delete_key();
        }
    }

    /// A project with one dependency under the id the cases ask about.
    fn config_for(detect: Value) -> Value {
        json!({ "dependencies": { "items": [
            { "id": "probe", "detect": detect, "payload": "payload/probe.exe", "required": true }
        ] } })
    }

    #[test]
    fn orders_versions_the_way_a_rule_means_them() {
        assert!(at_least("120.0.2210.91", "90.0.0.0"));
        assert!(at_least("120.0.2210.91", "120.0.2210.91"));
        assert!(!at_least("110.0.0.0", "120.0.2210.91"));
        // A plain number, which is how .NET Framework records its release.
        assert!(at_least("528040", "528040"));
        assert!(!at_least("461808", "528040"));
        // A missing part counts as zero, so a three-part version can still be
        // compared against a four-part one.
        assert!(at_least("14.0.1", "14.0"));
        assert!(!at_least("14.0", "14.0.1"));
    }

    #[test]
    fn reads_the_value_the_machine_actually_stores() -> Result<()> {
        let key = unique_key();
        let _guard = TestKey(key.clone());
        let target = parse_registry_key(&key)?;
        target.write_dword("Installed", 1)?;
        target.write_string("pv", "120.0.2210.91")?;

        // A `REG_DWORD` of 1 and a version written as text: the two types a
        // detection rule has to read, compared as the rule asks.
        let config =
            config_for(json!({ "registry": { "key": key, "name": "Installed", "equals": "1" } }));
        assert!(installed(&config, "probe")?);

        let config =
            config_for(json!({ "registry": { "key": key, "name": "pv", "at_least": "90.0.0.0" } }));
        assert!(installed(&config, "probe")?);

        let config = config_for(
            json!({ "registry": { "key": key, "name": "pv", "at_least": "130.0.0.0" } }),
        );
        assert!(!installed(&config, "probe")?);

        // A rule that names no value asks about the key, and a key that is not
        // there at all answers the same way as a value that is not.
        let config = config_for(json!({ "registry": { "key": key } }));
        assert!(installed(&config, "probe")?);
        let absent = format!("{key}\\Absent");
        let config = config_for(json!({ "registry": { "key": absent, "name": "Installed" } }));
        assert!(!installed(&config, "probe")?);
        Ok(())
    }

    #[test]
    fn reads_a_file_rule_with_the_variables_the_machine_sets() -> Result<()> {
        let name = format!("{}.dll", unique_name("nano-installer-detect"));
        let marker = std::env::temp_dir().join(&name);
        std::fs::write(&marker, b"present")?;
        // The rule names the file the way a project writes one -- through the
        // variable the machine sets, not through the path this process happens
        // to have.
        let rule = json!({ "file": format!("%TEMP%\\{name}") });
        assert!(installed(&config_for(rule.clone()), "probe")?);

        std::fs::remove_file(&marker).ok();
        assert!(!installed(&config_for(rule), "probe")?);
        Ok(())
    }

    #[test]
    fn refuses_a_dependency_the_project_never_declared() {
        let config = config_for(json!({ "file": "probe.exe" }));
        let error = installed(&config, "vcredist").expect_err("an undeclared id");
        assert!(
            format!("{error:#}").contains("vcredist is not a dependency this project declares"),
            "{error:#}"
        );
    }

    #[test]
    fn names_the_file_a_download_is_written_to() {
        assert_eq!(
            download_name("vcredist", "https://aka.ms/vs/17/release/vc_redist.x64.exe"),
            "vc_redist.x64.exe"
        );
        assert_eq!(
            download_name("vcredist", "https://example.test/tool.exe?token=abc#top"),
            "tool.exe"
        );
        // Nothing runnable in the URL, so the dependency's own id names the
        // program and Windows still has something it will start.
        assert_eq!(
            download_name("webview2", "https://example.test/download/"),
            "webview2.exe"
        );
        assert_eq!(
            download_name(
                "webview2",
                "https://go.microsoft.com/fwlink/?linkid=2124703"
            ),
            "webview2.exe"
        );
    }
}
