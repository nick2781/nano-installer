//! Windows services: installing one for a product, controlling it, and taking it
//! away again.
//!
//! A service is not part of the installation: it is an entry the machine keeps
//! in its own database, pointing at a program inside the installation. Every
//! call therefore goes through the service control manager, and installing,
//! changing, starting and deleting one need an elevated process -- a plain call
//! is refused by Windows, and that refusal is reported as it comes.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::time::{Duration, Instant};

use windows::core::{Error as WindowsError, PCWSTR};
use windows::Win32::Foundation::{
    BOOL, ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_CANNOT_ACCEPT_CTRL,
    ERROR_SERVICE_DOES_NOT_EXIST, ERROR_SERVICE_EXISTS, ERROR_SERVICE_NOT_ACTIVE, WIN32_ERROR,
};
use windows::Win32::System::Services::{
    ChangeServiceConfig2W, ChangeServiceConfigW, CloseServiceHandle, ControlService,
    CreateServiceW, DeleteService, OpenSCManagerW, OpenServiceW, QueryServiceConfigW,
    QueryServiceStatus, StartServiceW, ENUM_SERVICE_TYPE, QUERY_SERVICE_CONFIGW, SC_HANDLE,
    SC_MANAGER_CONNECT, SC_MANAGER_CREATE_SERVICE, SERVICE_ALL_ACCESS, SERVICE_AUTO_START,
    SERVICE_CHANGE_CONFIG, SERVICE_CONFIG_DELAYED_AUTO_START_INFO, SERVICE_CONTROL_STOP,
    SERVICE_DELAYED_AUTO_START_INFO, SERVICE_DEMAND_START, SERVICE_DISABLED, SERVICE_ERROR,
    SERVICE_ERROR_NORMAL, SERVICE_NO_CHANGE, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS,
    SERVICE_RUNNING, SERVICE_START, SERVICE_START_TYPE, SERVICE_STATUS,
    SERVICE_STATUS_CURRENT_STATE, SERVICE_STOP, SERVICE_STOPPED, SERVICE_WIN32_OWN_PROCESS,
};

use crate::install::wide;

/// How long a service is given to reach the state a call asked it to reach.
///
/// A service that ignores a request must not hold the setup: the wizard is
/// waiting, and a service that stays in a pending state is something the
/// service manager resolves on its own.
const STATE_DEADLINE: Duration = Duration::from_secs(10);

/// How often that state is read while it is waited for.
const POLL: Duration = Duration::from_millis(100);

/// How a service starts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum StartType {
    /// Starts with the machine.
    Automatic,
    /// Starts with the machine, once the machine has finished starting up.
    Delayed,
    /// Starts when something asks for it.
    Manual,
    /// Does not start until its configuration says otherwise.
    Disabled,
}

impl StartType {
    /// The kind a script named, `None` for a word this installer does not know.
    pub(super) fn parse(word: &str) -> Option<Self> {
        match word.trim().to_ascii_lowercase().as_str() {
            "auto" | "automatic" => Some(Self::Automatic),
            "delayed" | "delayed-auto" => Some(Self::Delayed),
            "manual" | "demand" => Some(Self::Manual),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    /// What the service manager is told to store for this kind.
    fn stored(self) -> SERVICE_START_TYPE {
        match self {
            Self::Automatic | Self::Delayed => SERVICE_AUTO_START,
            Self::Manual => SERVICE_DEMAND_START,
            Self::Disabled => SERVICE_DISABLED,
        }
    }

    /// How the kind reads in a message a person has to act on.
    fn name(self) -> &'static str {
        match self {
            Self::Automatic => "auto",
            Self::Delayed => "delayed",
            Self::Manual => "manual",
            Self::Disabled => "disabled",
        }
    }
}

/// Owns a service or manager handle, so an early return still closes it.
struct ScHandle(SC_HANDLE);

impl Drop for ScHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseServiceHandle(self.0) };
    }
}

/// Whether a service of this name exists on the machine.
///
/// This is the question a script asks before it decides whether it still has a
/// service to install, so a name the machine does not know is an answer rather
/// than a failure.
pub(super) fn exists(name: &str) -> Result<bool> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    match open(&manager, name, SERVICE_QUERY_STATUS) {
        Ok(service) => {
            drop(service);
            Ok(true)
        }
        Err(error) if code(&error) == Some(ERROR_SERVICE_DOES_NOT_EXIST) => Ok(false),
        Err(error) => Err(error).with_context(|| format!("cannot look up the service {name}")),
    }
}

/// Whether the service is running, `false` when there is no such service.
pub(super) fn running(name: &str) -> Result<bool> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    match open(&manager, name, SERVICE_QUERY_STATUS) {
        Ok(service) => Ok(state(&service)? == SERVICE_RUNNING),
        Err(error) if code(&error) == Some(ERROR_SERVICE_DOES_NOT_EXIST) => Ok(false),
        Err(error) => Err(error).with_context(|| format!("cannot look up the service {name}")),
    }
}

/// Installs the service, pointing it at the program in the installation.
///
/// A service that is already there and runs the same command line is left as it
/// is, which is what an upgrade replays: the script installs its service again,
/// and the manifest records it again. One that runs something else belongs to
/// another program, and taking it over would delete that program's service at
/// uninstall, so it is refused with both command lines shown.
pub(super) fn install(
    name: &str,
    display_name: &str,
    binary: &Path,
    arguments: &str,
    start: StartType,
) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("a service needs a name");
    }
    if display_name.trim().is_empty() {
        bail!("the service {name} needs a display name");
    }
    let command_line = command_line(binary, arguments);
    let manager = manager(SC_MANAGER_CREATE_SERVICE)?;
    let service_name = wide(name);
    let shown_as = wide(display_name);
    let program = wide(&command_line);
    let created = unsafe {
        CreateServiceW(
            manager.0,
            PCWSTR(service_name.as_ptr()),
            PCWSTR(shown_as.as_ptr()),
            SERVICE_ALL_ACCESS,
            SERVICE_WIN32_OWN_PROCESS,
            start.stored(),
            SERVICE_ERROR_NORMAL,
            PCWSTR(program.as_ptr()),
            PCWSTR::null(),
            None,
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
        )
    };
    match created {
        Ok(service) => {
            let service = ScHandle(service);
            if start == StartType::Delayed {
                set_delayed(&service, name, true)?;
            }
            Ok(())
        }
        Err(error) if code(&error) == Some(ERROR_SERVICE_EXISTS) => {
            let running_as = command_line_of(&manager, name)?;
            if running_as.eq_ignore_ascii_case(&command_line) {
                return Ok(());
            }
            bail!(
                "{name} is already a service of another program: it runs {running_as}, this installation would run {command_line}"
            )
        }
        Err(error) => Err(error).with_context(|| format!("cannot install the service {name}")),
    }
}

/// Deletes the service, stopping it first.
///
/// A service that is not there is not an error: an install that failed before
/// creating it, and a second uninstall, both reach this.
pub(super) fn delete(name: &str) -> Result<()> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    let Ok(service) = open(&manager, name, SERVICE_ALL_ACCESS) else {
        return Ok(());
    };
    // Windows keeps a running service until it stops, and its program lives in
    // the directory an uninstall is about to empty.
    let _ = stop_handle(&service, name);
    unsafe { DeleteService(service.0) }.with_context(|| format!("cannot delete the service {name}"))
}

/// Starts the service, waiting for it to report that it runs.
pub(super) fn start(name: &str) -> Result<()> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    let service = open(&manager, name, SERVICE_START | SERVICE_QUERY_STATUS)
        .with_context(|| format!("cannot find the service {name}"))?;
    if let Err(error) = unsafe { StartServiceW(service.0, None) } {
        // A service that already runs is in the state the caller asked for.
        if code(&error) == Some(ERROR_SERVICE_ALREADY_RUNNING) {
            return Ok(());
        }
        return Err(error).with_context(|| format!("cannot start the service {name}"));
    }
    wait_for(&service, name, SERVICE_RUNNING)
}

/// Stops the service, waiting for it to report that it stopped.
pub(super) fn stop(name: &str) -> Result<()> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    let service = open(&manager, name, SERVICE_STOP | SERVICE_QUERY_STATUS)
        .with_context(|| format!("cannot find the service {name}"))?;
    stop_handle(&service, name)
}

/// Sets how the service starts, delayed auto start included.
pub(super) fn set_start_type(name: &str, start: StartType) -> Result<()> {
    let manager = manager(SC_MANAGER_CONNECT)?;
    let service = open(&manager, name, SERVICE_CHANGE_CONFIG | SERVICE_QUERY_CONFIG)
        .with_context(|| format!("cannot find the service {name}"))?;
    unsafe {
        ChangeServiceConfigW(
            service.0,
            ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
            start.stored(),
            SERVICE_ERROR(SERVICE_NO_CHANGE),
            PCWSTR::null(),
            PCWSTR::null(),
            None,
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
        )
    }
    .with_context(|| format!("cannot set {name} to start {}", start.name()))?;
    set_delayed(&service, name, start == StartType::Delayed)
}

/// The command line a service runs: the program quoted, then its arguments.
///
/// A service command line is not a command a shell parses, so the program is
/// always quoted: a path with a space in it is the ordinary case under
/// `Program Files`, and Windows has to be told where the program ends.
fn command_line(binary: &Path, arguments: &str) -> String {
    let program = binary.display().to_string();
    let arguments = arguments.trim();
    if arguments.is_empty() {
        format!("\"{program}\"")
    } else {
        format!("\"{program}\" {arguments}")
    }
}

fn manager(access: u32) -> Result<ScHandle> {
    let handle = unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), access) }.context(
        "cannot open the service control manager, which installing or removing a service needs an \
         elevated process for",
    )?;
    Ok(ScHandle(handle))
}

fn open(manager: &ScHandle, name: &str, access: u32) -> windows::core::Result<ScHandle> {
    let name = wide(name.trim());
    unsafe { OpenServiceW(manager.0, PCWSTR(name.as_ptr()), access) }.map(ScHandle)
}

/// Stops the service through a handle that is already open.
fn stop_handle(service: &ScHandle, name: &str) -> Result<()> {
    let mut status = SERVICE_STATUS::default();
    if let Err(error) = unsafe { ControlService(service.0, SERVICE_CONTROL_STOP, &mut status) } {
        match code(&error) {
            // A service that is not running is already where the caller wants it.
            Some(ERROR_SERVICE_NOT_ACTIVE) => return Ok(()),
            // A service can refuse the request while it is still starting or
            // stopping; the state is what decides whether that is a problem.
            Some(ERROR_SERVICE_CANNOT_ACCEPT_CTRL) => {
                if state(service)? == SERVICE_STOPPED {
                    return Ok(());
                }
            }
            _ => {}
        }
        return Err(error).with_context(|| format!("cannot stop the service {name}"));
    }
    wait_for(service, name, SERVICE_STOPPED)
}

/// Waits for the service to reach the state that was asked of it.
fn wait_for(service: &ScHandle, name: &str, wanted: SERVICE_STATUS_CURRENT_STATE) -> Result<()> {
    let deadline = Instant::now() + STATE_DEADLINE;
    loop {
        if state(service)? == wanted {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!(
                "the service {name} did not reach the state the request asked for within {} seconds",
                STATE_DEADLINE.as_secs()
            );
        }
        std::thread::sleep(POLL);
    }
}

fn state(service: &ScHandle) -> Result<SERVICE_STATUS_CURRENT_STATE> {
    let mut status = SERVICE_STATUS::default();
    unsafe { QueryServiceStatus(service.0, &mut status) }
        .context("cannot read a service's state")?;
    Ok(status.dwCurrentState)
}

/// Turns the delayed part of an automatic start on or off.
///
/// A service that starts automatically holds the machine's start-up until it
/// reports that it is running, which is what this flag takes it out of.
fn set_delayed(service: &ScHandle, name: &str, delayed: bool) -> Result<()> {
    let info = SERVICE_DELAYED_AUTO_START_INFO {
        fDelayedAutostart: BOOL::from(delayed),
    };
    unsafe {
        ChangeServiceConfig2W(
            service.0,
            SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
            Some(std::ptr::from_ref(&info).cast()),
        )
    }
    .with_context(|| format!("cannot set the delayed start of the service {name}"))
}

/// The command line a service already runs.
fn command_line_of(manager: &ScHandle, name: &str) -> Result<String> {
    let service = open(manager, name, SERVICE_QUERY_CONFIG)
        .with_context(|| format!("cannot look up the service {name}"))?;
    let mut needed = 0u32;
    // The first call asks how much room the answer needs; a configuration that
    // Windows will not report at all leaves nothing to compare.
    let _ = unsafe { QueryServiceConfigW(service.0, None, 0, &mut needed) };
    if needed == 0 {
        bail!("cannot read the command line of the service {name}");
    }
    // The buffer is held as words rather than bytes, so the configuration's own
    // pointers sit where Windows expects to write them.
    let mut buffer = vec![0u64; (needed as usize).div_ceil(std::mem::size_of::<u64>())];
    let config = buffer.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>();
    unsafe { QueryServiceConfigW(service.0, Some(config), needed, &mut needed) }
        .with_context(|| format!("cannot read the command line of the service {name}"))?;
    let text = unsafe { (*config).lpBinaryPathName.to_string() }.unwrap_or_default();
    if text.is_empty() {
        bail!("the service {name} does not say what it runs");
    }
    Ok(text)
}

/// The Windows error a call reported, when it reported one.
fn code(error: &WindowsError) -> Option<WIN32_ERROR> {
    WIN32_ERROR::from_error(error)
}

#[cfg(test)]
mod tests {
    use super::{command_line, delete, exists, install, running, start, stop, StartType};
    use anyhow::Result;
    use std::path::{Path, PathBuf};

    /// A service name belonging to this case alone, so two of them never share
    /// one entry and a failure cannot touch another run's.
    fn unique_name() -> String {
        format!(
            "nano-installer-service-test-{}-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.subsec_nanos())
                .unwrap_or_default()
        )
    }

    /// A program every Windows has, for a service to point at.
    fn a_program() -> PathBuf {
        Path::new(&std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string()))
            .join("System32\\cmd.exe")
    }

    /// The command line a service runs is what the installer wrote, with the
    /// program quoted so a path under `Program Files` is read whole.
    #[test]
    fn a_service_command_line_quotes_the_program() {
        assert_eq!(
            command_line(Path::new(r"C:\Program Files\App\app.exe"), ""),
            r#""C:\Program Files\App\app.exe""#
        );
        assert_eq!(
            command_line(Path::new(r"C:\App\app.exe"), " --serve "),
            r#""C:\App\app.exe" --serve"#
        );
    }

    /// A word that names no start kind is refused rather than guessed at.
    #[test]
    fn a_service_start_kind_is_read_from_its_word() {
        assert_eq!(StartType::parse(" auto "), Some(StartType::Automatic));
        assert_eq!(StartType::parse("DELAYED"), Some(StartType::Delayed));
        assert_eq!(StartType::parse("manual"), Some(StartType::Manual));
        assert_eq!(StartType::parse("disabled"), Some(StartType::Disabled));
        assert_eq!(StartType::parse("sometimes"), None);
        assert_eq!(StartType::parse(""), None);
    }

    /// A service every Windows has is found, and a name nothing owns is told
    /// apart from it.
    ///
    /// The Windows Event Log is started before anyone can log on and is not
    /// optional, so it answers both questions: it is there and it runs. The
    /// other name belongs to this run, so nothing else can hold it.
    #[test]
    fn a_service_is_found_and_told_apart_from_one_that_is_not_there() -> Result<()> {
        assert!(
            exists("EventLog")?,
            "the Windows Event Log has gone missing"
        );
        assert!(running("EventLog")?, "the Windows Event Log is not running");
        let absent = unique_name();
        assert!(!exists(&absent)?, "{absent} exists on this machine");
        assert!(!running(&absent)?, "{absent} is running on this machine");
        // Controlling a service the machine does not have is refused rather
        // than acted on.
        assert!(start(&absent).is_err(), "{absent} was started");
        assert!(stop(&absent).is_err(), "{absent} was stopped");
        Ok(())
    }

    /// Installing a service and taking it away again, where this process may do
    /// that at all.
    ///
    /// Changing the machine's services needs an elevated process, so a run
    /// without one is refused by Windows itself, and what a case can still hold
    /// there is that the refusal comes back as an error rather than as a service
    /// nobody asked for. Where the run may, the round trip is held instead,
    /// including the replay an upgrade performs and the deletion an uninstall
    /// performs. What is not held either way is that the service then runs: a
    /// service program is the product's own, and no test can ship one.
    #[test]
    fn a_service_is_installed_and_removed_where_the_run_may() -> Result<()> {
        let name = unique_name();
        let program = a_program();
        let installed = install(
            &name,
            "nano-installer service test",
            &program,
            "/C exit 0",
            StartType::Automatic,
        );
        if !crate::shell::is_elevated() {
            assert!(
                installed.is_err(),
                "installing a service without an elevated process reported success"
            );
            assert!(!exists(&name)?, "a refused install left a service behind");
            return Ok(());
        }
        let _guard = ServiceGuard(name.clone());
        installed?;
        assert!(exists(&name)?, "the service was not installed");
        // What an upgrade does: the same service is installed again, and the
        // one already there runs the command line this installation asked for.
        install(
            &name,
            "nano-installer service test",
            &program,
            "/C exit 0",
            StartType::Automatic,
        )?;
        delete(&name)?;
        assert!(
            !exists(&name)?,
            "the service was still there after deletion"
        );
        Ok(())
    }

    /// A name that belongs to another program is refused rather than taken over.
    #[test]
    fn a_service_that_runs_another_program_is_not_taken_over() -> Result<()> {
        if !crate::shell::is_elevated() {
            return Ok(());
        }
        let name = unique_name();
        let program = a_program();
        let _guard = ServiceGuard(name.clone());
        install(
            &name,
            "nano-installer service test",
            &program,
            "/C exit 0",
            StartType::Automatic,
        )?;
        let taken_over = install(
            &name,
            "somebody else's service",
            &program,
            "/C exit 1",
            StartType::Automatic,
        );
        let error = taken_over.expect_err("another program's service was taken over");
        let message = format!("{error:#}");
        assert!(
            message.contains("/C exit 1") && message.contains("/C exit 0"),
            "the refusal does not show what the service runs: {message}"
        );
        Ok(())
    }

    /// Removes a service a case installed, even when an assertion failed first.
    struct ServiceGuard(String);

    impl Drop for ServiceGuard {
        fn drop(&mut self) {
            let _ = delete(&self.0);
        }
    }
}
