# Plugin ABI

A plugin is a DLL that a project ships with its setup and a script calls by
name. It is how this framework is extended without the framework changing: the
same shape NSIS uses (`DllName::Function`), written down as an exported-function
contract in [`include/nano_plugin.h`](../../include/nano_plugin.h), which is the
specification. This page is that contract in prose, with the parts an author
needs on the first read.

Everything a plugin needs is here, and nothing else is assumed: a plugin is
native code inside the installer process, so it may call the Win32 API for
anything it wants to *read or check*. What it must not do is put a file or a
registry value on the machine behind the installer's back — see
[Writing](#writing-through-the-host).

## The three rules

1. **Text in, text out.** A script passes an array of strings; the plugin calls
   `host->push` for each value it wants back. Strings are UTF-16, and the host
   copies what it is given, so a plugin never has to care who frees what.
2. **Write through the host.** `host->write_file` and `host->write_registry`
   record what they write the way the script primitives do, so the uninstaller
   takes it back. A plugin that writes with the Win32 API instead leaves the
   machine with files and keys nothing will remove.
3. **Nothing is a guess.** The host fills in `abi_version` and `struct_size`
   before it calls anything. A plugin that does not know the version must return
   non-zero rather than read fields that may not be there.

## Naming and shipping

A project declares a directory:

```toml
[resources]
plugins_dir = "plugins"
```

Every DLL in that directory is embedded in the setup, checked against the digest
the build recorded when it is read back, and unpacked into a temporary directory
on first use. A plugin is called by the DLL's own name:

```rhai
let disk = plugin_call("tapcore::GetDiskId", ["C:\\"]);
let all = plugin_values("tapcore::ListDevices", []);
```

`tapcore::GetDiskId` loads `plugins/tapcore.dll` and calls the export
`GetDiskId`. Files that are not called by name ride along, which is how a plugin
ships a data file or a second DLL it needs.

The build refuses a DLL this runtime could never load, and says which file and
why: a 32-bit image (this runtime is 64-bit only), an ARM64 image, an executable
rather than a DLL, or a file that is not a Windows image at all. Finding that out
at build time is the whole point of the check — an installer that fails on a
machine with "not a valid Win32 application" tells an author nothing.

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

A plugin exports as many functions as it likes, each with that signature. `argv`
holds the script's arguments in the order it wrote them, each UTF-16 and
NUL-terminated, owned by the host and valid only for the call. `argc` is how many
there are.

`NANO_PLUGIN_EXPORT` is the part a C or C++ author must not forget: without it the
DLL builds, loads, and then has no export by that name, and nothing says why.
Rust says the same thing with `#[no_mangle]`, which is why the Rust sample has no
sign of the macro.

## The services

| Service | What it does |
| --- | --- |
| `push(host, value)` | Adds one value to the answer the script reads, in order |
| `log(host, level, message)` | Writes one line into the run's log — the file a failed install leaves behind |
| `window(host)` | The wizard window, or `0` in a windowless run |
| `cancelled(host)` | Whether the user asked the run to stop |
| `install_dir(host)` | The directory the product is installed into |
| `write_file(host, path, text)` | Writes a text file into the installation, recorded |
| `write_registry(host, key, name, kind, value)` | Writes one registry value, recorded |
| `error_text(host)` | Why the last service failed, as a sentence |

`log` levels are `NANO_PLUGIN_LOG_ERROR`, `NANO_PLUGIN_LOG_WARN` and
`NANO_PLUGIN_LOG_INFO`. `error_text` is never null and stays valid until the next
service call.

## Writing through the host

`write_file` takes an absolute path or one relative to the installation
directory. Anything outside the installation is refused: the manifest that the
uninstaller replays is built from what the run recorded, and a file outside it
would be a file this product leaves on the machine with nothing naming it. On an
install, a file written through the host ends up in the manifest because the
install's own snapshot sees it; a registry value is recorded the way
`reg_write_string` records one, so the uninstall takes back exactly that value
(and the whole key when the key belongs to this product).

`write_registry` writes `kind` of `string`, `expand`, `dword` or `qword`.
Anything else, and any key this product does not own, is refused.

During an uninstall there is no manifest left to record into — the manifest is
what is being replayed — which is the same position a project's uninstall script
is in. A plugin is callable from that script as well: the uninstaller carries its
own bundle, and the project's plugins travel in it.

## Failure

| What happened | What the script sees |
| --- | --- |
| The setup ships no plugin by that name | `the setup ships no plugin called `x`` |
| The DLL has no such export | `the plugin x.dll does not export `Function`` |
| The plugin returned non-zero | `the plugin x.dll answered 3 from `Function``, with the plugin's own last log line beside it |
| A host service refused | The script sees the plugin's failure code; the plugin itself reads `error_text` |

A call that cannot be made fails the script, and a script may `try` it if it
wants to carry on. That is deliberately stricter than the other primitives, which
answer with a value and leave the decision to the script: those answer about this
machine, while a plugin call is third-party code the script asked for by name, so
a call that did not happen is not an answer.

**A plugin must not let an exception or a panic escape its export.** Rust aborts
the process where a panic would cross the boundary of a function like this one,
so an installer whose plugin panicked ends as a crash with nothing in its log to
say why. Measured, not assumed.

## What a plugin cannot do

- **Add a script primitive.** The primitives are fixed; a plugin is called by a
  script, it does not become part of the language.
- **Draw the wizard's own controls.** `window` lets a plugin parent its own
  dialog to the wizard, and that window is the plugin's own: the layout engine,
  the high-contrast palette and the screen-reader description do not reach into
  it.
- **Write outside the installation** through the host, as above.
- **Run on Windows 7 without its own imports working there.** A plugin is code
  the product ships, so it is as bound by the platform as the setup is.

## Where to start

`crates/nano-installer-plugin-sample` is a working plugin in Rust: it answers a
value, adds up arguments, writes a file and a registry value through the host,
refuses a path outside the installation, and fails on purpose.
`examples/plugin-c/sample.c` is the same plugin in C, built by MSVC alone from
the header — the proof that this ABI is the header's and not one language's. The
end-to-end cases install a setup that ships each of them and drive every function,
so they are also the thing that holds the ABI to what this page says.
