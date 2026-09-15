//! Win32 shell helpers used by the install and uninstall flows: process
//! termination, `.lnk` creation, and well-known folder lookup.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use windows::core::{Interface, PCWSTR, PWSTR};

use windows::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, FALSE, HANDLE};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IPersistFile,
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Programs, IShellLinkW, SHChangeNotify, ShellLink,
    SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
};
use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn com() -> ComGuard {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    ComGuard
}

/// Owns a kernel handle so an early return still closes it.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Terminates every process whose image name matches `exe_name`.
///
/// Returns the number of processes that were terminated. A process that exits
/// between the snapshot and the open is skipped: that is the outcome the caller
/// wanted. A process that cannot be opened at all is reported, because leaving
/// it running would keep the destination locked.
pub(super) fn kill_processes(exe_name: &str) -> Result<u32> {
    let snapshot = OwnedHandle(
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .context("failed to enumerate running processes")?,
    );
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut killed = 0;
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
    while more {
        let name = String::from_utf16_lossy(
            entry
                .szExeFile
                .split(|unit| *unit == 0)
                .next()
                .unwrap_or(&[]),
        );
        if name.eq_ignore_ascii_case(exe_name) {
            match unsafe { OpenProcess(PROCESS_TERMINATE, FALSE, entry.th32ProcessID) } {
                Ok(process) => {
                    let process = OwnedHandle(process);
                    if unsafe { TerminateProcess(process.0, 1) }.is_ok() {
                        killed += 1;
                    }
                }
                // The process exited after the snapshot was taken.
                Err(error) if error.code() != ERROR_ACCESS_DENIED.into() => {}
                Err(_) => bail!("cannot terminate {name}: access denied"),
            }
        }
        more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
    }
    Ok(killed)
}

/// Resolves a `KNOWNFOLDERID` to a filesystem path.
pub(super) fn known_folder(folder: &windows::core::GUID) -> Result<PathBuf> {
    let raw: PWSTR = unsafe { SHGetKnownFolderPath(folder, KF_FLAG_DEFAULT, HANDLE::default()) }
        .context("failed to resolve a shell folder")?;
    let text = unsafe { raw.to_string() }.context("shell folder path is not Unicode")?;
    unsafe { CoTaskMemFree(Some(raw.as_ptr().cast())) };
    Ok(PathBuf::from(text))
}

pub(super) fn desktop_directory() -> Result<PathBuf> {
    known_folder(&FOLDERID_Desktop)
}

pub(super) fn programs_directory() -> Result<PathBuf> {
    known_folder(&FOLDERID_Programs)
}

/// Writes a `.lnk` that launches `target`.
pub(super) fn create_shortcut(link_path: &Path, target: &Path, working_dir: &Path) -> Result<()> {
    let _com = com();
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
        .context("failed to create a shell link")?;
    unsafe {
        link.SetPath(PCWSTR(wide(&target.display().to_string()).as_ptr()))?;
        link.SetWorkingDirectory(PCWSTR(wide(&working_dir.display().to_string()).as_ptr()))?;
        link.SetIconLocation(PCWSTR(wide(&target.display().to_string()).as_ptr()), 0)?;
        let file: IPersistFile = link.cast()?;
        file.Save(
            PCWSTR(wide(&link_path.display().to_string()).as_ptr()),
            true,
        )?;
    }
    Ok(())
}

/// Expands `%VAR%` references in a configured path.
pub(super) fn expand_environment(value: &str) -> Result<String> {
    let source = wide(value);
    let needed = unsafe { ExpandEnvironmentStringsW(PCWSTR(source.as_ptr()), None) };
    if needed == 0 {
        bail!("cannot expand environment variables in {value}");
    }
    let mut buffer = vec![0u16; needed as usize];
    let written = unsafe { ExpandEnvironmentStringsW(PCWSTR(source.as_ptr()), Some(&mut buffer)) };
    if written == 0 || written > needed {
        bail!("cannot expand environment variables in {value}");
    }
    buffer.truncate(written.saturating_sub(1) as usize);
    String::from_utf16(&buffer).context("expanded path is not Unicode")
}

/// The per-user data roots an uninstall is allowed to delete from:
/// `%APPDATA%` and `%LOCALAPPDATA%`.
pub(super) fn user_data_roots() -> Vec<PathBuf> {
    ["APPDATA", "LOCALAPPDATA"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .collect()
}

/// Reports whether `path` is a proper descendant of a per-user data root.
///
/// This keeps a misconfigured `%APPDATA%`-relative pattern from expanding to
/// something broad like `C:\` and deleting it.
pub(super) fn is_user_data_path(path: &Path) -> bool {
    user_data_roots().iter().any(|root| {
        path != root
            && path
                .strip_prefix(root)
                .is_ok_and(|rest| rest.components().next().is_some())
    })
}

/// Asks the shell to refresh cached icons and associations.
pub(super) fn notify_shell() {
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}
