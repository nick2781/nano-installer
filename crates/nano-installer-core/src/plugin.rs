//! Third-party plugins: the ABI, loading one, and calling it by name.
//!
//! A plugin is a DLL a project ships in the directory its configuration names as
//! `resources.plugins_dir`, and a script calls it as `dll::function`, which is
//! the shape NSIS uses. The contract is in `include/nano_plugin.h`, which is the
//! specification third parties read; this module is the host side of it: the
//! same structure laid out the same way, the services behind the function
//! pointers, and the call itself.
//!
//! Two decisions are worth knowing before reading on.
//!
//! A plugin gets no way to write a file or a registry value except through the
//! host. That is not a limitation of effort, it is what keeps an installation
//! removable: the manifest the uninstaller replays is built from what the run
//! recorded, and a plugin that wrote with the Win32 API would put files on the
//! machine that no manifest names. What a plugin wants to *read* it reads
//! itself, with the Win32 API, because it is native code in this process.
//!
//! The host structure only ever grows. `struct_size` and `abi_version` are its
//! first two fields and always will be, a plugin refuses a version it does not
//! know, and a host refuses to call a plugin that answers with a version it does
//! not know either. That is the whole compatibility story, and it is why the
//! plugin function's own three parameters can never change.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::path::{Path, PathBuf};
use windows::core::PCSTR;
use windows::Win32::Foundation::{FreeLibrary, HANDLE, HMODULE};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LoadLibraryExW, LOAD_WITH_ALTERED_SEARCH_PATH,
};

/// The ABI this host speaks, matching `NANO_PLUGIN_ABI_VERSION`.
pub const ABI_VERSION: u32 = 1;

/// Level of a plugin's log line, matching `NANO_PLUGIN_LOG_*`.
pub const LOG_ERROR: i32 = 0;
/// Level of a plugin's log line, matching `NANO_PLUGIN_LOG_*`.
pub const LOG_WARN: i32 = 1;
/// Level of a plugin's log line, matching `NANO_PLUGIN_LOG_*`.
pub const LOG_INFO: i32 = 2;

/// What one service returns.
const OK: i32 = 0;
/// What one service returns.
const FAILED: i32 = 1;

/// Adds one value to the answer the script reads.
type PushService = unsafe extern "system" fn(*mut c_void, *const u16) -> i32;
/// Writes one line into the run's log.
type LogService = unsafe extern "system" fn(*mut c_void, i32, *const u16);
/// Hands over the wizard window, or nothing.
type WindowService = unsafe extern "system" fn(*mut c_void) -> *mut c_void;
/// Answers whether the user asked the run to stop.
type CancelledService = unsafe extern "system" fn(*mut c_void) -> i32;
/// Hands over the installation directory.
type InstallDirService = unsafe extern "system" fn(*mut c_void) -> *const u16;
/// Writes a file into the installation, recorded.
type WriteFileService = unsafe extern "system" fn(*mut c_void, *const u16, *const u16) -> i32;
/// Writes one registry value, recorded.
type WriteRegistryService =
    unsafe extern "system" fn(*mut c_void, *const u16, *const u16, *const u16, *const u16) -> i32;
/// Explains the last service that failed.
type ErrorTextService = unsafe extern "system" fn(*mut c_void) -> *const u16;

/// The structure a plugin is handed, field for field `nano_plugin_host`.
///
/// The order and the widths are the ABI: a field is only ever appended, and the
/// unit tests below pin the offsets the C header promises so a change here
/// cannot quietly become a change third parties see.
#[repr(C)]
pub struct HostTable {
    /// How much of this structure the host built.
    pub struct_size: u32,
    /// Which ABI the host speaks.
    pub abi_version: u32,
    /// The plugin's own state, handed back to every service.
    pub host: *mut c_void,
    /// Adds one value to the answer.
    pub push: Option<PushService>,
    /// Writes one log line.
    pub log: Option<LogService>,
    /// The wizard window, or null.
    pub window: Option<WindowService>,
    /// Whether the user asked the run to stop.
    pub cancelled: Option<CancelledService>,
    /// The installation directory.
    pub install_dir: Option<InstallDirService>,
    /// Writes a file into the installation.
    pub write_file: Option<WriteFileService>,
    /// Writes one registry value.
    pub write_registry: Option<WriteRegistryService>,
    /// Explains the last failure.
    pub error_text: Option<ErrorTextService>,
}

/// What the host can do for a plugin, as Rust rather than as function pointers.
///
/// The script side implements this over `ScriptContext`, so a plugin's write
/// goes down exactly the path a script's own primitive takes: the same
/// containment, the same rollback journal, the same manifest record.
pub trait Services {
    /// Writes one line into the run's log.
    fn log(&mut self, level: i32, message: &str);
    /// The wizard window, or null when this run has none.
    fn window(&mut self) -> *mut c_void;
    /// Whether the user asked this run to stop.
    fn cancelled(&mut self) -> bool;
    /// The directory the product is installed into.
    fn install_dir(&mut self) -> String;
    /// Writes a text file into the installation, recorded.
    fn write_file(&mut self, path: &str, text: &str) -> Result<()>;
    /// Writes one registry value, recorded.
    fn write_registry(&mut self, key: &str, name: &str, kind: &str, value: &str) -> Result<()>;
}

/// What one call hands the plugin, and what it collects.
struct HostState<'a> {
    services: &'a mut dyn Services,
    /// The values the plugin pushed, in the order it pushed them.
    values: Vec<String>,
    /// The installation directory as a NUL-terminated buffer for `install_dir`.
    directory: Vec<u16>,
    /// The last failure as a NUL-terminated buffer for `error_text`.
    failure: Vec<u16>,
}

impl HostState<'_> {
    /// Says why the last service failed, and answers with the code that goes
    /// back to the plugin.
    fn refuse(&mut self, error: anyhow::Error) -> i32 {
        let message = format!("{error:#}");
        self.services.log(LOG_ERROR, &message);
        self.failure = wide(&message);
        FAILED
    }
}

/// Turns Rust text into the NUL-terminated buffer a C string is.
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Reads a string a plugin handed over.
///
/// A null pointer is an empty string rather than a crash: a plugin that passes
/// nothing has said nothing, and the service it called answers with the failure
/// that follows from an empty argument.
///
/// # Safety
///
/// `pointer` must be null or point at a NUL-terminated UTF-16 string that stays
/// valid for the read.
unsafe fn read_wide(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0usize;
    while unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length) })
}

/// Runs a service body, turning a panic into the failure the plugin sees.
///
/// A plugin is third-party code inside the installer process, and the host's own
/// services are the one place it can make this process panic from outside. In a
/// build where a panic unwinds, catching it here keeps the run failing with a
/// sentence instead of tearing through the plugin's frames.
fn guarded(body: impl FnOnce() -> i32) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(code) => code,
        Err(_) => FAILED,
    }
}

unsafe extern "system" fn host_push(host: *mut c_void, value: *const u16) -> i32 {
    guarded(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        let text = unsafe { read_wide(value) };
        state.values.push(text);
        OK
    })
}

unsafe extern "system" fn host_log(host: *mut c_void, level: i32, message: *const u16) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        let text = unsafe { read_wide(message) };
        state.services.log(level, &text);
    }));
}

unsafe extern "system" fn host_window(host: *mut c_void) -> *mut c_void {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        state.services.window()
    })) {
        Ok(window) => window,
        Err(_) => std::ptr::null_mut(),
    }
}

unsafe extern "system" fn host_cancelled(host: *mut c_void) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        state.services.cancelled()
    })) {
        Ok(cancelled) => i32::from(cancelled),
        Err(_) => 0,
    }
}

unsafe extern "system" fn host_install_dir(host: *mut c_void) -> *const u16 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        state.directory = wide(&state.services.install_dir());
        state.directory.as_ptr()
    })) {
        Ok(pointer) => pointer,
        Err(_) => std::ptr::null(),
    }
}

unsafe extern "system" fn host_write_file(
    host: *mut c_void,
    path: *const u16,
    text: *const u16,
) -> i32 {
    guarded(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        let path = unsafe { read_wide(path) };
        let text = unsafe { read_wide(text) };
        match state.services.write_file(&path, &text) {
            Ok(()) => OK,
            Err(error) => state.refuse(error),
        }
    })
}

unsafe extern "system" fn host_write_registry(
    host: *mut c_void,
    key: *const u16,
    name: *const u16,
    kind: *const u16,
    value: *const u16,
) -> i32 {
    guarded(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        let key = unsafe { read_wide(key) };
        let name = unsafe { read_wide(name) };
        let kind = unsafe { read_wide(kind) };
        let value = unsafe { read_wide(value) };
        match state.services.write_registry(&key, &name, &kind, &value) {
            Ok(()) => OK,
            Err(error) => state.refuse(error),
        }
    })
}

unsafe extern "system" fn host_error_text(host: *mut c_void) -> *const u16 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = unsafe { &mut *(host as *mut HostState) };
        state.failure.as_ptr()
    })) {
        Ok(pointer) => pointer,
        Err(_) => std::ptr::null(),
    }
}

/// Splits `dll::function` into the two names a call needs.
///
/// The shape is NSIS's, and it is checked here rather than at load time so a
/// script that spelled a call wrong is told so by name.
pub fn split_call(spec: &str) -> Result<(String, String)> {
    let (library, function) = spec
        .split_once("::")
        .with_context(|| format!("`{spec}` is not a plugin call: write it as dll::function"))?;
    let library = library.trim();
    let function = function.trim();
    if library.is_empty() {
        bail!("`{spec}` names no plugin: write it as dll::function");
    }
    if function.is_empty() {
        bail!("`{spec}` names no function: write it as dll::function");
    }
    if library.contains(['/', '\\', ':']) || library.contains("..") {
        bail!("`{spec}` names a plugin by path: a plugin is named by the DLL the project ships, as dll::function");
    }
    Ok((library.to_string(), function.to_string()))
}

/// A DLL this run has loaded, unloaded when the last handle to it goes.
///
/// The handle is process-wide and safe to use from any thread, which is what
/// the two impls below say; a script's context keeps one of these per plugin so
/// a second call does not load the file again.
pub struct Library {
    module: HMODULE,
    path: PathBuf,
}

// SAFETY: a module handle is an address in this process, owned by the loader,
// and usable from any thread. Nothing here is thread-local.
unsafe impl Send for Library {}
// SAFETY: as above. The host calls a loaded plugin one call at a time, which is
// a rule of the ABI rather than a property of the handle.
unsafe impl Sync for Library {}

impl Library {
    /// Loads one plugin DLL.
    pub fn load(path: &Path) -> Result<Self> {
        let name = wide(&path.to_string_lossy());
        // The directory the DLL sits in is searched first, because a plugin
        // ships with whatever it needs beside it. This flag rather than the
        // search-path flags Windows 7 only has with an update installed:
        // `LOAD_WITH_ALTERED_SEARCH_PATH` has been there since XP.
        let module = unsafe {
            LoadLibraryExW(
                windows::core::PCWSTR(name.as_ptr()),
                HANDLE::default(),
                LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        }
        .with_context(|| {
            format!(
                "the plugin {} could not be loaded",
                path.file_name().unwrap_or_default().to_string_lossy()
            )
        })?;
        Ok(Self {
            module,
            path: path.to_path_buf(),
        })
    }

    /// Calls one exported function, handing it the services.
    ///
    /// The answers are the values the plugin pushed, in order. A plugin that
    /// answered a non-zero code fails the call, and the reason carries both the
    /// code and the file the call was in.
    pub fn call(
        &self,
        function: &str,
        arguments: &[String],
        services: &mut dyn Services,
    ) -> Result<Vec<String>> {
        let name = CString::new(function)
            .with_context(|| format!("`{function}` is not a name a DLL can export"))?;
        let entry = unsafe { GetProcAddress(self.module, PCSTR::from_raw(name.as_ptr().cast())) };
        let Some(entry) = entry else {
            bail!(
                "the plugin {} does not export `{function}`",
                self.path.file_name().unwrap_or_default().to_string_lossy()
            );
        };
        // SAFETY: the signature is the ABI's, declared the same way on both
        // sides; the header is what a third party writes against.
        let entry: unsafe extern "system" fn(*mut HostTable, i32, *const *const u16) -> i32 =
            unsafe { std::mem::transmute(entry) };

        let owned: Vec<Vec<u16>> = arguments.iter().map(|argument| wide(argument)).collect();
        let pointers: Vec<*const u16> = owned.iter().map(|argument| argument.as_ptr()).collect();

        let mut state = HostState {
            services,
            values: Vec::new(),
            directory: wide(""),
            failure: wide(""),
        };
        let mut table = HostTable {
            struct_size: std::mem::size_of::<HostTable>() as u32,
            abi_version: ABI_VERSION,
            host: std::ptr::from_mut(&mut state).cast::<c_void>(),
            push: Some(host_push),
            log: Some(host_log),
            window: Some(host_window),
            cancelled: Some(host_cancelled),
            install_dir: Some(host_install_dir),
            write_file: Some(host_write_file),
            write_registry: Some(host_write_registry),
            error_text: Some(host_error_text),
        };

        let code = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            entry(
                std::ptr::from_mut(&mut table),
                pointers.len() as i32,
                pointers.as_ptr(),
            )
        })) {
            Ok(code) => code,
            Err(_) => bail!(
                "the plugin {} panicked inside `{function}`; a plugin has to answer with a code rather than let an error escape",
                self.path.file_name().unwrap_or_default().to_string_lossy()
            ),
        };
        if code != 0 {
            let reason = String::from_utf16_lossy(
                state.failure.strip_suffix(&[0]).unwrap_or(&state.failure),
            );
            let name = self.path.file_name().unwrap_or_default().to_string_lossy();
            if reason.is_empty() {
                bail!("the plugin {name} answered {code} from `{function}`");
            }
            bail!("the plugin {name} answered {code} from `{function}`: {reason}");
        }
        Ok(state.values)
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        let _ = unsafe { FreeLibrary(self.module) };
    }
}

/// Refuses a file that is not a 64-bit DLL before Windows is asked to load it.
///
/// `LoadLibrary` answers a file of the wrong shape with a message about the
/// file not being a valid Win32 application, which tells an author nothing about
/// which half was wrong. The PE header says it plainly, and this runs at build
/// time as well, so a project learns before it ships.
pub fn check_x64_dll(name: &str, bytes: &[u8]) -> Result<()> {
    let short = |what: &str| -> anyhow::Error {
        anyhow::anyhow!("{name} is not a DLL this runtime can load: {what}")
    };
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return Err(short("it is not a Windows executable at all"));
    }
    let header = u32::from_le_bytes(bytes[0x3c..0x40].try_into().expect("four bytes")) as usize;
    if bytes.len() < header + 24 || &bytes[header..header + 4] != b"PE\0\0" {
        return Err(short("it has no PE header"));
    }
    let machine = u16::from_le_bytes(bytes[header + 4..header + 6].try_into().expect("two bytes"));
    let characteristics = u16::from_le_bytes(
        bytes[header + 22..header + 24]
            .try_into()
            .expect("two bytes"),
    );
    match machine {
        0x8664 => {}
        0x014c => {
            return Err(short(
                "it is a 32-bit image, and this runtime is 64-bit only",
            ))
        }
        0xaa64 => return Err(short("it is an ARM64 image, and this runtime is x64")),
        other => return Err(short(&format!("it is built for machine type {other:#06x}"))),
    }
    if characteristics & 0x2000 == 0 {
        return Err(short("it is an executable rather than a DLL"));
    }
    Ok(())
}

/// The plugins a run has loaded, by the name a script calls them by.
#[derive(Default)]
pub struct Loaded {
    libraries: HashMap<String, std::sync::Arc<Library>>,
}

impl Loaded {
    /// The library behind a plugin name, loading it the first time.
    pub fn library(&mut self, name: &str, path: &Path) -> Result<std::sync::Arc<Library>> {
        if let Some(library) = self.libraries.get(name) {
            return Ok(library.clone());
        }
        let library = std::sync::Arc::new(Library::load(path)?);
        self.libraries.insert(name.to_string(), library.clone());
        Ok(library)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PE image with the parts the check reads, so each refusal can be built
    /// by changing one field of an image that would otherwise pass.
    fn image(machine: u16, characteristics: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
        bytes[0x96..0x98].copy_from_slice(&characteristics.to_le_bytes());
        bytes
    }

    #[test]
    fn an_x64_dll_passes_the_check() {
        check_x64_dll("sample.dll", &image(0x8664, 0x2000)).expect("an x64 DLL");
    }

    #[test]
    fn a_thirty_two_bit_image_is_refused_by_name() {
        let error = check_x64_dll("legacy.dll", &image(0x014c, 0x2000)).unwrap_err();
        let said = format!("{error:#}");
        assert!(said.contains("legacy.dll"), "{said}");
        assert!(said.contains("32-bit"), "{said}");
    }

    #[test]
    fn an_executable_is_not_a_plugin() {
        let error = check_x64_dll("tool.exe", &image(0x8664, 0x0022)).unwrap_err();
        assert!(format!("{error:#}").contains("rather than a DLL"));
    }

    #[test]
    fn a_file_that_is_not_an_image_is_refused() {
        let error = check_x64_dll("notes.dll", b"just text").unwrap_err();
        assert!(format!("{error:#}").contains("not a Windows executable"));
    }

    #[test]
    fn a_call_is_two_names_and_a_path_is_not_one() {
        assert_eq!(
            split_call("sample::Add").expect("a well formed call"),
            ("sample".to_string(), "Add".to_string())
        );
        assert_eq!(
            split_call("tapcore::GetDiskId")
                .expect("a well formed call")
                .1,
            "GetDiskId"
        );
        for wrong in ["sample", "sample:", "::Add", "sample::", "..\\sample::Add"] {
            assert!(split_call(wrong).is_err(), "`{wrong}` was accepted");
        }
    }

    /// The offsets the C header promises, so a field cannot be inserted or
    /// reordered without this test saying so.
    #[test]
    fn the_host_table_is_the_abi_the_header_declares() {
        assert_eq!(std::mem::size_of::<HostTable>(), 16 + 8 * 8);
        assert_eq!(std::mem::offset_of!(HostTable, struct_size), 0);
        assert_eq!(std::mem::offset_of!(HostTable, abi_version), 4);
        assert_eq!(std::mem::offset_of!(HostTable, host), 8);
        assert_eq!(std::mem::offset_of!(HostTable, push), 16);
        assert_eq!(std::mem::offset_of!(HostTable, log), 24);
        assert_eq!(std::mem::offset_of!(HostTable, window), 32);
        assert_eq!(std::mem::offset_of!(HostTable, cancelled), 40);
        assert_eq!(std::mem::offset_of!(HostTable, install_dir), 48);
        assert_eq!(std::mem::offset_of!(HostTable, write_file), 56);
        assert_eq!(std::mem::offset_of!(HostTable, write_registry), 64);
        assert_eq!(std::mem::offset_of!(HostTable, error_text), 72);
    }
}
