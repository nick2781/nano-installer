//! The sample plugin: what a third party copies to write their own.
//!
//! It is a DLL exporting functions a project script calls as `sample::Name`, and
//! it exists so the ABI has something real on the other side of it: the
//! end-to-end cases install a setup that ships this DLL and drive it, which is
//! how the structure, the services and the failure modes are held to what
//! `include/nano_plugin.h` promises.
//!
//! Two things are worth copying from it. The ABI types are declared here rather
//! than imported from the runtime: a plugin is written against the header, and a
//! plugin that linked the installer's own crate would be a plugin that cannot be
//! built without it. And every function checks `abi_version` before it uses
//! anything, because a field a newer host added is not a field this plugin may
//! read.
//!
//! Nothing here is unit tested on purpose: the structure above is the ABI, so the
//! thing that holds it is a real call. The end-to-end cases install a setup that
//! ships this DLL and drive every function in it through the real host, and a
//! field that moved would fail them rather than a copy of itself.

// Every exported function here is an ABI entry point, and they all have the same
// safety contract, which `include/nano_plugin.h` states: the host passes an
// argument array that is valid for the call, and the plugin returns before it
// ends. Restating that above each of nine exports would be noise rather than
// documentation, so the lint is allowed once, here.
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_void;

/// The ABI this plugin was written against, from `NANO_PLUGIN_ABI_VERSION`.
const ABI_VERSION: u32 = 1;

/// Level 0 of `log`.
const LOG_ERROR: i32 = 0;
/// Level 1 of `log`.
const LOG_WARN: i32 = 1;
/// Level 2 of `log`.
const LOG_INFO: i32 = 2;

/// What one service returns.
const OK: i32 = 0;

/// `nano_plugin_host`, declared the way the header declares it.
///
/// The order and widths are the ABI. A field is only ever appended, which is why
/// every function here looks at `abi_version` and `struct_size` before it uses
/// anything past them.
///
/// Public because the exported functions name it: a plugin's exports are the
/// DLL's public surface, and the type they take has to be part of it.
#[repr(C)]
pub struct Host {
    struct_size: u32,
    abi_version: u32,
    host: *mut c_void,
    push: Option<unsafe extern "system" fn(*mut c_void, *const u16) -> i32>,
    log: Option<unsafe extern "system" fn(*mut c_void, i32, *const u16)>,
    window: Option<unsafe extern "system" fn(*mut c_void) -> *mut c_void>,
    cancelled: Option<unsafe extern "system" fn(*mut c_void) -> i32>,
    install_dir: Option<unsafe extern "system" fn(*mut c_void) -> *const u16>,
    write_file: Option<unsafe extern "system" fn(*mut c_void, *const u16, *const u16) -> i32>,
    write_registry: Option<
        unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            *const u16,
            *const u16,
            *const u16,
        ) -> i32,
    >,
    error_text: Option<unsafe extern "system" fn(*mut c_void) -> *const u16>,
}

impl Host {
    /// Whether this host speaks an ABI this plugin knows.
    fn understood(&self) -> bool {
        self.abi_version == ABI_VERSION && self.struct_size as usize >= std::mem::size_of::<Host>()
    }

    /// Adds one value to the answer the script reads.
    fn push(&self, value: &str) {
        let Some(push) = self.push else {
            return;
        };
        let text = wide(value);
        unsafe { push(self.host, text.as_ptr()) };
    }

    /// Writes one line into the run's log.
    fn log(&self, level: i32, message: &str) {
        let Some(log) = self.log else {
            return;
        };
        let text = wide(message);
        unsafe { log(self.host, level, text.as_ptr()) };
    }

    /// The installation directory, or an empty string when the host has none.
    fn install_dir(&self) -> String {
        self.install_dir
            .map(|service| unsafe { read(service(self.host)) })
            .unwrap_or_default()
    }

    /// Writes a file into the installation.
    fn write_file(&self, path: &str, text: &str) -> i32 {
        let Some(service) = self.write_file else {
            return 1;
        };
        let path = wide(path);
        let text = wide(text);
        unsafe { service(self.host, path.as_ptr(), text.as_ptr()) }
    }

    /// Writes one registry value.
    fn write_registry(&self, key: &str, name: &str, kind: &str, value: &str) -> i32 {
        let Some(service) = self.write_registry else {
            return 1;
        };
        let key = wide(key);
        let name = wide(name);
        let kind = wide(kind);
        let value = wide(value);
        unsafe {
            service(
                self.host,
                key.as_ptr(),
                name.as_ptr(),
                kind.as_ptr(),
                value.as_ptr(),
            )
        }
    }

    /// Why the last service failed.
    fn error_text(&self) -> String {
        self.error_text
            .map(|service| unsafe { read(service(self.host)) })
            .unwrap_or_default()
    }
}

/// Rust text as the UTF-16 the ABI carries.
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A string the host handed over.
///
/// # Safety
///
/// `pointer` must be null or a NUL-terminated UTF-16 string.
unsafe fn read(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0usize;
    while unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length) })
}

/// The arguments of one call, as Rust strings.
///
/// # Safety
///
/// The pointers are the host's, valid for the duration of the call.
unsafe fn arguments(argc: i32, argv: *const *const u16) -> Vec<String> {
    if argc <= 0 || argv.is_null() {
        return Vec::new();
    }
    (0..argc as usize)
        .map(|index| unsafe { read(*argv.add(index)) })
        .collect()
}

/// Refuses a host this plugin does not speak to, rather than reading fields that
/// may not be there.
fn refused(host: *mut Host) -> bool {
    if host.is_null() {
        return true;
    }
    !unsafe { &*host }.understood()
}

/// `sample::Add` -- sums what it was given, which is the simplest proof that
/// arguments arrive in order and that an answer gets back to the script.
#[no_mangle]
pub unsafe extern "system" fn Add(host: *mut Host, argc: i32, argv: *const *const u16) -> i32 {
    if refused(host) {
        return 4;
    }
    let host = unsafe { &*host };
    let values = unsafe { arguments(argc, argv) };
    let sum: i64 = values
        .iter()
        .map(|value| value.trim().parse::<i64>().unwrap_or_default())
        .sum();
    host.push(&sum.to_string());
    OK
}

/// `sample::Echo` -- pushes back everything it was given, in order.
#[no_mangle]
pub unsafe extern "system" fn Echo(host: *mut Host, argc: i32, argv: *const *const u16) -> i32 {
    if refused(host) {
        return 4;
    }
    let host = unsafe { &*host };
    for value in unsafe { arguments(argc, argv) } {
        host.push(&value);
    }
    OK
}

/// `sample::About` -- one value, which is what most calls answer with.
#[no_mangle]
pub unsafe extern "system" fn About(host: *mut Host, _argc: i32, _argv: *const *const u16) -> i32 {
    if refused(host) {
        return 4;
    }
    unsafe { &*host }.push("nano-plugin-sample 1.0");
    OK
}

/// `sample::Where` -- the installation directory the host offers.
#[no_mangle]
pub unsafe extern "system" fn Where(host: *mut Host, _argc: i32, _argv: *const *const u16) -> i32 {
    if refused(host) {
        return 4;
    }
    let host = unsafe { &*host };
    let directory = host.install_dir();
    host.push(&directory);
    OK
}

/// `sample::Note` -- writes to the run's log and answers normally.
#[no_mangle]
pub unsafe extern "system" fn Note(host: *mut Host, argc: i32, argv: *const *const u16) -> i32 {
    if refused(host) {
        return 4;
    }
    let host = unsafe { &*host };
    let values = unsafe { arguments(argc, argv) };
    host.log(
        LOG_INFO,
        &format!("note: called with {} argument(s)", values.len()),
    );
    host.log(
        LOG_WARN,
        "note: this is a warning, and the call still succeeds",
    );
    host.push("logged");
    OK
}

/// `sample::Install` -- writes a file and a registry value through the host, so
/// both are recorded and the uninstall takes them back.
///
/// Arguments: the key to write into, the name of the value, and the text to
/// store. The file goes to `plugin-sample.txt` in the installation.
#[no_mangle]
pub unsafe extern "system" fn Install(host: *mut Host, argc: i32, argv: *const *const u16) -> i32 {
    if refused(host) {
        return 4;
    }
    let host = unsafe { &*host };
    let values = unsafe { arguments(argc, argv) };
    let key = values.first().cloned().unwrap_or_default();
    let name = values
        .get(1)
        .cloned()
        .unwrap_or_else(|| "Sample".to_string());
    let text = values
        .get(2)
        .cloned()
        .unwrap_or_else(|| "sample".to_string());
    if key.is_empty() {
        host.log(
            LOG_ERROR,
            "install: the first argument is the registry key to write",
        );
        return 2;
    }

    if host.write_file("plugin-sample.txt", "written by the sample plugin\n") != OK {
        host.log(
            LOG_ERROR,
            &format!("install: write_file failed: {}", host.error_text()),
        );
        return 3;
    }
    host.log(
        LOG_INFO,
        &format!("install: writing {key}\\{name} = {text}"),
    );
    if host.write_registry(&key, &name, "string", &text) != OK {
        host.log(
            LOG_ERROR,
            &format!("install: write_registry failed: {}", host.error_text()),
        );
        return 3;
    }
    host.push("installed");
    OK
}

/// `sample::WriteOutside` -- asks the host for something it must refuse, and
/// answers with the reason so a case can read it.
#[no_mangle]
pub unsafe extern "system" fn WriteOutside(
    host: *mut Host,
    argc: i32,
    argv: *const *const u16,
) -> i32 {
    if refused(host) {
        return 4;
    }
    let host = unsafe { &*host };
    let values = unsafe { arguments(argc, argv) };
    let path = values.first().cloned().unwrap_or_default();
    if host.write_file(&path, "this must not land\n") == OK {
        host.push("written");
        return OK;
    }
    let reason = host.error_text();
    host.push(&reason);
    OK
}

/// `sample::RequiresAbi` -- refuses a host older than the ABI it asks for, which
/// is what the header tells every plugin to do with a version it does not know.
#[no_mangle]
pub unsafe extern "system" fn RequiresAbi(
    host: *mut Host,
    argc: i32,
    argv: *const *const u16,
) -> i32 {
    if host.is_null() {
        return 4;
    }
    let host = unsafe { &*host };
    let values = unsafe { arguments(argc, argv) };
    let needed: u32 = values
        .first()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(ABI_VERSION);
    if host.abi_version < needed {
        host.log(
            LOG_ERROR,
            &format!(
                "this plugin needs ABI {needed} and the host speaks {}",
                host.abi_version
            ),
        );
        return 5;
    }
    host.push(&format!("abi {}", host.abi_version));
    OK
}

/// `sample::Fail` -- explains itself and fails, which is what a plugin does when
/// the machine cannot give it what it needs.
#[no_mangle]
pub unsafe extern "system" fn Fail(host: *mut Host, _argc: i32, _argv: *const *const u16) -> i32 {
    if refused(host) {
        return 4;
    }
    unsafe { &*host }.log(LOG_ERROR, "the sample plugin was asked to fail");
    3
}
