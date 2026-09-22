//! The log a run leaves on disk.
//!
//! An installer has no console, so what it did exists only as long as the
//! process does: the wizard keeps the last lines of it in memory and shows them
//! with a failure, and everything they pushed out is gone. A run therefore
//! writes every line to a file of its own as it goes. A failure leaves that file
//! behind as the report a user can hand over, and a success leaves it as the
//! record of what the machine was given.
//!
//! The file never goes inside the installation. A fresh install that fails
//! removes the directory it was writing into -- which is exactly the moment the
//! log is wanted -- so the default place is the user's temporary directory,
//! where Windows lets an elevated and a plain run alike write, and where the
//! other installers of this platform keep theirs.
//!
//! Nothing here is allowed to fail a run. A machine that will not let the setup
//! write a log is a machine the product still has to install on, so the caller
//! ignores the error from [`begin`] and every later call turns into no work.

use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
use windows::Win32::System::SystemInformation::{GetLocalTime, GetNativeSystemInfo, SYSTEM_INFO};

/// The task a run is doing, which is what the file name and the header say.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunKind {
    Install,
    Uninstall,
}

impl RunKind {
    fn word(self) -> &'static str {
        match self {
            RunKind::Install => "install",
            RunKind::Uninstall => "uninstall",
        }
    }
}

/// How much a run writes before it stops adding to its log.
///
/// The script engine bounds a runaway loop, so this is not about a script that
/// never ends; it is about an installer never being able to fill a disk with
/// its own log. What is already written stays, with a line saying where it
/// ended, which keeps the tail of a long run readable.
const LIMIT: u64 = 1024 * 1024;

/// The directory a run writes into when nothing else was asked for.
const DIRECTORY: &str = "nano-installer";

struct RunLog {
    path: PathBuf,
    file: File,
    written: u64,
    full: bool,
    ended: bool,
}

static LOG: OnceLock<Mutex<Option<RunLog>>> = OnceLock::new();

/// The run's log, if it has one. A poisoned lock still hands the log back: a
/// script that panicked while logging must not take the installation with it.
fn log() -> MutexGuard<'static, Option<RunLog>> {
    LOG.get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

/// Opens the log this run writes to and returns where it is.
///
/// `at` is the file a `--log` flag named, or `None` for the temporary directory
/// the run falls back to. A run that never called this writes nothing, which is
/// what every in-process caller that is not an installation does.
pub(crate) fn begin(kind: RunKind, at: Option<PathBuf>) -> Result<PathBuf> {
    let path = match at {
        Some(path) => path,
        None => default_path(kind),
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to prepare {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let header = vec![
        "nano-installer setup log".to_string(),
        format!("run: {}", kind.word()),
        format!("image: {}", image().display()),
        format!("elevated: {}", yes_no(crate::shell::is_elevated())),
        format!("windows: {}", windows()),
        format!("machine: {}", machine()),
        format!("locale: {}", user_locale()),
        format!("started: {}", now_stamp()),
        String::new(),
    ];
    let mut written = 0;
    for line in header {
        written += write_line(&mut file, &line)?;
    }
    // A run replaces the log of an earlier run of the same name rather than
    // failing on it: the file is the record of what happened this time.
    *log() = Some(RunLog {
        path: path.clone(),
        file,
        written,
        full: false,
        ended: false,
    });
    Ok(path)
}

/// Records what this run is: the product, and where it puts what it installs.
///
/// Called once the project's own configuration has been read, which is the
/// first moment the product has a name. A run that never opened a log writes
/// nothing.
pub(crate) fn describe(facts: &[(&str, String)]) {
    let mut log = log();
    let Some(log) = log.as_mut() else {
        return;
    };
    for (name, value) in facts {
        write_entry(log, &format!("{name}: {value}"), false);
    }
    write_entry(log, "", false);
}

/// Appends one line of the run, with the moment it happened.
pub(crate) fn note(level: &str, message: &str) {
    let mut log = log();
    let Some(log) = log.as_mut() else {
        return;
    };
    write_entry(log, &format!("{level}: {message}"), true);
}

/// How a run ended.
pub(crate) enum Outcome<'a> {
    /// The task did everything it was asked to.
    Finished,
    /// The user stopped the task, which is not a failure the log has to
    /// explain but is still worth telling apart from a run that completed.
    Stopped,
    Failed(&'a anyhow::Error),
}

/// Records how the run ended.
///
/// Returns the line that names the log for whoever is told about a failure --
/// the wizard's notice, or the standard error of a windowless run -- and `None`
/// when there is nothing to report: a run that succeeded, a run that already
/// ended, and a run that never opened a log.
pub(crate) fn end(outcome: Outcome<'_>) -> Option<String> {
    let mut log = log();
    let entry = log.as_mut()?;
    if entry.ended {
        return None;
    }
    entry.ended = true;
    match outcome {
        Outcome::Finished => {
            write_entry(entry, "info: the run finished", true);
            None
        }
        Outcome::Stopped => {
            write_entry(entry, "info: the run stopped before it finished", true);
            None
        }
        Outcome::Failed(error) => {
            let path = entry.path.clone();
            // The whole chain, one line per cause: this file is what a support
            // ticket gets, and the wording of a system error is part of it.
            write_entry(entry, &format!("error: {error:#}"), true);
            write_entry(
                entry,
                "info: the lines above are the whole record of this run",
                true,
            );
            Some(format!("A log of this run is at:\n{}", path.display()))
        }
    }
}

/// Appends `text` as one timestamped line, if the run has room for it.
fn write_entry(entry: &mut RunLog, text: &str, stamped: bool) {
    if entry.full {
        return;
    }
    let line = if stamped {
        format!("[{}] {text}", now_stamp())
    } else {
        text.to_string()
    };
    match write_line(&mut entry.file, &line) {
        Ok(bytes) => entry.written += bytes,
        // A log that cannot be written to from here on is worth no more than a
        // truncated one, and the run it belongs to keeps going.
        Err(_) => {
            entry.full = true;
            return;
        }
    }
    if entry.written >= LIMIT {
        entry.full = true;
        let _ = write_line(&mut entry.file, "[log truncated: the file is full]");
    }
}

fn write_line(file: &mut File, line: &str) -> Result<u64> {
    let mut text = String::with_capacity(line.len() + 2);
    text.push_str(line);
    text.push_str("\r\n");
    file.write_all(text.as_bytes())?;
    file.flush()?;
    Ok(text.len() as u64)
}

/// Where a run writes when nothing named a file: the temporary directory, with
/// the setup image and the moment in the name so runs do not overwrite each
/// other and a support engineer can tell them apart.
fn default_path(kind: RunKind) -> PathBuf {
    let directory = std::env::temp_dir().join(DIRECTORY);
    let stem = image()
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "setup".to_string());
    let name = format!("{stem}-{}-{}.log", now_file_stamp(), kind.word());
    unique(directory, name)
}

/// A name nothing holds yet, so two runs in one second both keep their log.
fn unique(directory: PathBuf, name: String) -> PathBuf {
    let candidate = directory.join(&name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem.to_string(), format!(".{extension}")),
        None => (name.clone(), String::new()),
    };
    for index in 2..100 {
        let candidate = directory.join(format!("{stem}-{index}{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(name)
}

/// The image this run came out of, which is the setup or the uninstaller.
fn image() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

fn yes_no(flag: bool) -> &'static str {
    if flag {
        "yes"
    } else {
        "no"
    }
}

/// The version of Windows as the machine itself reports it.
///
/// Read from the registry on purpose: `GetVersionExW` answers with the version
/// a manifest claims unless the application declares one, and this runtime
/// declares none, so it would report Windows 8 on a machine running Windows 10.
/// The product name alone is not enough either, because Windows 11 reports
/// itself as "Windows 10"; the build number is what tells a support reader
/// which machine this was.
fn windows() -> String {
    let key = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let value = |name: &str| {
        crate::install::read_registry_string(HKEY_LOCAL_MACHINE, key, name)
            .ok()
            .flatten()
            .filter(|value| !value.trim().is_empty())
    };
    let name = value("ProductName").unwrap_or_else(|| "Windows".to_string());
    let mut details = Vec::new();
    if let Some(build) = value("CurrentBuildNumber") {
        details.push(format!("build {build}"));
    }
    if let Some(release) = value("DisplayVersion").or_else(|| value("ReleaseId")) {
        details.push(release);
    }
    if details.is_empty() {
        name
    } else {
        format!("{name} ({})", details.join(", "))
    }
}

/// The machine's processor architecture and the width of this process.
fn machine() -> String {
    let mut info = SYSTEM_INFO::default();
    let architecture = unsafe {
        GetNativeSystemInfo(&mut info);
        match info.Anonymous.Anonymous.wProcessorArchitecture.0 {
            0 => "x86",
            5 => "arm",
            9 => "x64",
            12 => "arm64",
            _ => "unknown",
        }
    };
    format!(
        "{architecture}, {}-bit process",
        std::mem::size_of::<usize>() * 8
    )
}

/// The language of the user who started the run, which is what the product's
/// own localization is chosen against when the machine offers several.
fn user_locale() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if length <= 0 {
        return "unknown".to_string();
    }
    String::from_utf16_lossy(&buffer[..length as usize - 1])
}

/// The local time as `YYYY-MM-DD HH:MM:SS.mmm`, the same shape the builder's
/// own log uses, so a support reader meets one format in both.
fn now_stamp() -> String {
    let time = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        time.wYear,
        time.wMonth,
        time.wDay,
        time.wHour,
        time.wMinute,
        time.wSecond,
        time.wMilliseconds
    )
}

/// The same moment in a shape a file name can carry, and without characters a
/// path cannot hold.
fn now_file_stamp() -> String {
    let time = unsafe { GetLocalTime() };
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default name carries the image, the moment and the task, and lives
    /// in the one directory a run falls back to. A name with the local time in
    /// it is what tells two runs of the same setup apart in a support ticket.
    #[test]
    fn a_default_log_is_named_after_the_image_the_moment_and_the_task() {
        let directory = std::env::temp_dir().join(DIRECTORY);
        let path = default_path(RunKind::Install);
        assert_eq!(path.parent(), Some(directory.as_path()));
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let stem = image()
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "setup".to_string());
        assert!(name.starts_with(&stem), "{name} does not start with {stem}");
        let moment = name
            .strip_prefix(&format!("{stem}-"))
            .and_then(|rest| rest.strip_suffix("-install.log"))
            .unwrap_or_default();
        assert_eq!(
            moment.len(),
            15,
            "{name} carries no moment of its own: {moment}"
        );
        assert!(moment.chars().all(|c| c.is_ascii_digit() || c == '-'));
    }

    /// A run writes the machine it ran on, and a failure leaves the file as the
    /// report: where it is, and what went wrong with the whole chain of causes.
    #[test]
    fn a_run_that_fails_leaves_a_log_that_names_it_and_what_happened() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("nested").join("run.log");
        begin(RunKind::Uninstall, Some(path.clone()))?;
        describe(&[("product", "Probe 1.0.0".to_string())]);
        note("info", "removing files");
        let line = end(Outcome::Failed(&anyhow::anyhow!(
            "cannot remove the product"
        )))
        .context("a failure has to name its log")?;
        assert!(line.ends_with(&path.display().to_string()), "{line}");

        let written = std::fs::read_to_string(&path)?;
        for expected in [
            "nano-installer setup log",
            "run: uninstall",
            "elevated:",
            "windows:",
            "machine:",
            "locale:",
            "product: Probe 1.0.0",
            "info: removing files",
            "error: cannot remove the product",
        ] {
            assert!(
                written.contains(expected),
                "{expected} is missing:\n{written}"
            );
        }
        // A run that failed is not also a run that finished.
        assert!(!written.contains("info: the run finished"));

        // A run reports its own failure once: a second call has nothing to add.
        assert!(end(Outcome::Failed(&anyhow::anyhow!("again"))).is_none());
        *log() = None;
        Ok(())
    }
}
