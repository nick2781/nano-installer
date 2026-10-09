# Plugin ABI

A plugin is a DLL you ship with your setup, and a script calls it by name, which is how the framework is extended: the framework itself does not change. It has the same shape NSIS uses (`DllName::Function`); the specification is in [`include/nano_plugin.h`](../../include/nano_plugin.h), and that header is the contract, which is what this page sets out in prose, for the parts you need on a first read.

Everything a plugin needs is on this page, and nothing else is a guess. A plugin is native code running inside the installer process, so anything you want to *read or check*, call the Win32 API for it; one thing it must not do, though, is put a file or a registry value on the machine behind the installer's back. See [Writing](#writing-through-the-host).

## The three rules

1. **Text in, text out.** A script passes in an array of strings, and you call `host->push` once for each value you want back; strings are UTF-16, and the host copies what it is given, so you never have to care who frees what.
2. **Write through the host.** `host->write_file` and `host->write_registry` record what they write the way the script primitives do, so the uninstaller can take it back; if you go around them and write with the Win32 API, the machine keeps files and keys nothing will remove.
3. **Nothing is a guess.** The host fills in `abi_version` and `struct_size` before it calls anything, and a plugin that does not know the version returns non-zero rather than reading fields that may not be there.

## Naming and shipping

A project declares a directory:

```toml
[resources]
plugins_dir = "plugins"
```

Every DLL in that directory is embedded in the setup, checked against the digest the build recorded when it is read back, and unpacked into a temporary directory on first use. A plugin is called by the DLL's own name:

```rhai
let disk = plugin_call("tapcore::GetDiskId", ["C:\\"]);
let all = plugin_values("tapcore::ListDevices", []);
```

`tapcore::GetDiskId` loads `plugins/tapcore.dll` and calls the export `GetDiskId`. A file that is not called by name still rides along, which is how a plugin ships a data file, or a second DLL it needs.

The build refuses a DLL this runtime could never load, and says which file and why: a 32-bit image (this runtime is 64-bit only), an ARM64 image, an executable rather than a DLL, or a file that is not a Windows image at all.

Finding that out at build time is the whole point of the check, because getting "not a valid Win32 application" on somebody else's machine tells you nothing.

## The export

```c
#include "nano_plugin.h"

NANO_PLUGIN_EXPORT int32_t NANO_PLUGIN_CALL Hello(nano_plugin_host *host, int32_t argc,
                                                  const wchar_t *const *argv) {
    if (host->abi_version != NANO_PLUGIN_ABI_VERSION) {
        return 1;
    }
    host->push(host, L"hello");
    return 0;   /* 0 is success; anything else fails the script's call */
}
```

A plugin exports as many functions as it likes, each with that signature. `argv` holds the script's arguments in the order it wrote them: each one is UTF-16 and NUL-terminated, the host owns them, and they are valid only for the call, while `argc` is how many there are.

In C or C++, `NANO_PLUGIN_EXPORT` is the part you must not forget: without it the DLL still builds and still loads, but it has no export by that name, and nothing says why. Rust says the same thing with `#[no_mangle]`, which is why the Rust sample has no sign of the macro.

## The services

| Service | What it does |
| --- | --- |
| `push(host, value)` | Adds one value to the answer the script reads, in the order of the calls |
| `log(host, level, message)` | Writes one line into the run's log. A failed install leaves that file behind |
| `window(host)` | The wizard window. A run with no window gets `0` |
| `cancelled(host)` | Whether the user has already asked this run to stop |
| `install_dir(host)` | The directory the product is installed into |
| `write_file(host, path, text)` | Writes a text file into the installation, and records it |
| `write_registry(host, key, name, kind, value)` | Writes one registry value, and records it |
| `error_text(host)` | Why the last service failed, in one sentence |

`log` levels are `NANO_PLUGIN_LOG_ERROR`, `NANO_PLUGIN_LOG_WARN` and `NANO_PLUGIN_LOG_INFO`. `error_text` is never null, and it stays valid until the next service call.

## Writing through the host

`write_file` takes an absolute path, or one relative to the installation directory, and anything outside the installation is refused. The manifest the uninstaller replays is built from what this run recorded, so a file outside it would be a file this product leaves on the machine with nothing naming it.

On an install, a file written through the host ends up in the manifest, because the install's own snapshot sees it. A registry value is recorded the way `reg_write_string` records one, so the uninstall takes back exactly that value, and when the key belongs to this product, it takes back the whole key.

`write_registry` writes a `kind` of `string`, `expand`, `dword` or `qword`; anything else, and any key this product does not own, is refused.

During an uninstall there is no manifest left to record into, because the manifest is what is being replayed, which matches a project's uninstall script. A plugin is callable from that script as well: the uninstaller carries its own bundle, and the project's plugins travel in it.

## Failure

| What happened | What the script sees |
| --- | --- |
| The setup ships no plugin by that name | `the setup ships no plugin called `x`` |
| The DLL has no such export | `the plugin x.dll does not export `Function`` |
| The plugin returned non-zero | `the plugin x.dll answered 3 from `Function``, with the plugin's own last log line after it |
| A host service refused | The script sees the plugin's failure code. The plugin itself reads `error_text` for the reason |

A call that cannot be made fails the script, and you can catch it with `try` if you want to carry on. Being stricter here than the other primitives is deliberate: those primitives answer about this machine, and you decide what to do with the answer, while a plugin call is third-party code the script asked for by name, so **a call that did not happen is not an answer**.

**A plugin must not let an exception or a panic escape its export.** Where a panic would cross the boundary of a function like this one, Rust aborts the process, so an installer whose plugin panicked ends as a crash, with nothing in its log to say why. Measured, not assumed.

## What a plugin cannot do

- **Add a script primitive.** The primitives are fixed, and a plugin, called by a script, does not become part of the language.
- **Draw the wizard's own controls.** `window` lets a plugin parent its own dialog to the wizard, and that window is the plugin's own, so the layout engine, the high-contrast palette and the screen-reader description do not reach into it.
- **Write outside the installation** through the host, as above.
- **Run on the minimum supported version without its own imports working there.** A plugin is code the product ships, so it is as bound by the platform as the setup is.

## Where to start

`crates/nano-installer-plugin-sample` is a working plugin in Rust: it answers a value, adds up arguments, writes a file and a registry value through the host, refuses a path outside the installation, and fails on purpose. `examples/plugin-c/sample.c` is the same plugin in C, built by MSVC alone from the header, which is the proof that this ABI is the header's and not one language's. The end-to-end cases install a setup that ships each of them and drive every function, so they are also what holds the ABI to what this page says.
