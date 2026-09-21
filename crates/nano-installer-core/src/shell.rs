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
/// Calls `visit` with the image name and process id of every running process.
fn for_each_process(mut visit: impl FnMut(&str, u32) -> Result<()>) -> Result<()> {
    let snapshot = OwnedHandle(
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .context("failed to enumerate running processes")?,
    );
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
    while more {
        let name = String::from_utf16_lossy(
            entry
                .szExeFile
                .split(|unit| *unit == 0)
                .next()
                .unwrap_or(&[]),
        );
        visit(&name, entry.th32ProcessID)?;
        more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
    }
    Ok(())
}

/// Reports whether a process with an image name matching `exe_name` is running.
pub(super) fn process_running(exe_name: &str) -> Result<bool> {
    let mut running = false;
    for_each_process(|name, _| {
        if name.eq_ignore_ascii_case(exe_name) {
            running = true;
        }
        Ok(())
    })?;
    Ok(running)
}

pub(super) fn kill_processes(exe_name: &str) -> Result<u32> {
    let mut killed = 0;
    for_each_process(|name, process_id| {
        if !name.eq_ignore_ascii_case(exe_name) {
            return Ok(());
        }
        match unsafe { OpenProcess(PROCESS_TERMINATE, FALSE, process_id) } {
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
        Ok(())
    })?;
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

/// A command that never shows a console window of its own.
///
/// A setup runs in the user's own session, so a program it starts must not flash
/// a console in front of the wizard on its way to doing the work.
pub(super) fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;

    let mut command = std::process::Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW.0);
    command
}

/// What a program wrote, and the code it left with.
pub(super) struct CommandOutput {
    pub(super) code: i64,
    pub(super) stdout: String,
    pub(super) stderr: String,
}

/// Runs a program to completion and collects what it wrote.
///
/// `cancelled` is asked while it runs, because a step that cannot be stopped is
/// a setup the user cannot stop either: a program that never finishes is ended,
/// and the run reports `-1` with whatever the program wrote before that.
pub(super) fn run_captured(
    program: &str,
    args: &[String],
    cancelled: &dyn Fn() -> bool,
) -> Result<CommandOutput> {
    let mut child = hidden_command(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("cannot run {program}"))?;
    // A full pipe stops the program writing into it, so both are drained on
    // their own thread while this one waits for the program to finish.
    let mut stdout = child.stdout.take().expect("the pipe was asked for");
    let mut stderr = child.stderr.take().expect("the pipe was asked for");
    let out = std::thread::spawn(move || read_to_end(&mut stdout));
    let err = std::thread::spawn(move || read_to_end(&mut stderr));
    let code = loop {
        if cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            break -1;
        }
        match child.try_wait()? {
            Some(status) => break status.code().map(i64::from).unwrap_or(-1),
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };
    Ok(CommandOutput {
        code,
        stdout: decode(out.join().unwrap_or_default()),
        stderr: decode(err.join().unwrap_or_default()),
    })
}

/// Reads a pipe to its end; what could not be read is what there is.
fn read_to_end(pipe: &mut impl std::io::Read) -> Vec<u8> {
    let mut buffer = Vec::new();
    let _ = pipe.read_to_end(&mut buffer);
    buffer
}

/// Decodes what a program wrote.
///
/// A console program writes in the code page the machine runs in rather than in
/// UTF-8 -- `ipconfig` on a Chinese Windows writes GBK -- so UTF-8 is tried
/// first, because a program that wrote it says so in its own bytes, and the
/// machine's ANSI code page is what the text becomes otherwise.
pub(super) fn decode(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            let bytes = error.into_bytes();
            ansi_text(&bytes).unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned())
        }
    }
}

/// Decodes bytes with the code page this process runs in, `None` when Windows
/// will not decode them at all.
fn ansi_text(bytes: &[u8]) -> Option<String> {
    use windows::Win32::Globalization::{
        MultiByteToWideChar, CP_ACP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS,
    };

    if bytes.is_empty() {
        return Some(String::new());
    }
    let flags = MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0);
    let needed = unsafe { MultiByteToWideChar(CP_ACP, flags, bytes, None) };
    if needed <= 0 {
        return None;
    }
    let mut wide = vec![0u16; needed as usize];
    let written = unsafe { MultiByteToWideChar(CP_ACP, flags, bytes, Some(&mut wide)) };
    if written <= 0 {
        return None;
    }
    wide.truncate(written as usize);
    String::from_utf16(&wide).ok()
}

#[cfg(test)]
mod tests {
    use super::{decode, run_captured};

    /// What a program wrote comes back beside the code it left with.
    #[test]
    fn runs_a_program_and_collects_what_it_wrote() -> anyhow::Result<()> {
        let output = run_captured(
            "cmd.exe",
            &[
                "/C".to_string(),
                "echo out& echo err 1>&2& exit 4".to_string(),
            ],
            &|| false,
        )?;
        assert_eq!(output.code, 4);
        assert_eq!(output.stdout.trim(), "out");
        assert_eq!(output.stderr.trim(), "err");
        Ok(())
    }

    /// A program that writes UTF-8 says so in its own bytes, and that reading
    /// needs no help.
    #[test]
    fn reads_utf8_from_a_program_that_wrote_it() {
        assert_eq!(decode("你好".as_bytes().to_vec()), "你好");
    }

    /// A console program writes in the code page the machine runs in instead, so
    /// what those bytes say depends on the machine reading them: the case holds
    /// the reading where the machine runs in that code page, and holds that
    /// nothing is thrown away where it does not.
    #[test]
    fn reads_a_programs_output_in_its_own_code_page() {
        // What a Chinese console writes for the same two words.
        let decoded = decode(vec![0xD6, 0xD0, 0xCE, 0xC4]);
        if unsafe { windows::Win32::Globalization::GetACP() } == 936 {
            assert_eq!(decoded, "你好");
        } else {
            assert!(!decoded.is_empty(), "the bytes were thrown away");
        }
    }
}
