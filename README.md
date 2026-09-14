# nano-installer native

This branch validates a native Win32 replacement for the previous eframe runtime. It builds one
x64/Unicode toolchain whose minimum operating system is Windows 7 SP1 and which also runs on later
Windows versions. There is no separate Windows 10 build.

## Components

| Source | Release artifact | Purpose |
| --- | --- | --- |
| `installer/native/cli` | `nano-installer-native-x64.exe` | Parse a project and package a setup |
| `installer/native/stubs/lzma` | `native-lzma-x64.exe` | Native runtime selected for 7z payloads |
| `installer/native/stubs/zlib` | `native-zlib-x64.exe` | Native runtime selected for ZIP payloads |
| `installer/native/stubs/uninst` | `native-uninst-x64.exe` | Native uninstaller runtime boundary |

The builder detects payload format from its signature, copies the matching sibling stub, and
appends configuration, XML layouts, assets, locales, scripts, payload, and the uninstaller stub.
The generated setup renders directly through Win32, WIC, GDI, and `DrawTextW`; it does not link
eframe, egui, winit, glutin, OpenGL, Taffy, Rhai, or the Rust image crate.

## Build

The only release build uses Rust's `x86_64-win7-windows-msvc` target, a statically linked CRT, and
`build-std`:

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

Public artifacts are copied to `target/release/`:

```text
nano-installer-native-x64.exe
native-lzma-x64.exe
native-zlib-x64.exe
native-uninst-x64.exe
TapTap_Setup.exe
```

`target/x86_64-win7-windows-msvc/` is Cargo's internal cross-target cache, not a second release.
The build script audits the PE imports of the builder, all stubs, and generated setup.

Current clean-build measurements with the 143.69 MiB TapTap payload:

| Artifact | Size |
| --- | ---: |
| `nano-installer-native-x64.exe` | 274.5 KiB |
| each current native stub | 336.5 KiB |
| native `TapTap_Setup.exe` | 147.24 MiB |
| previous eframe `TapTap_Setup.exe` baseline | 159.14 MiB |

The native setup is currently about 11.90 MiB (7.5%) smaller. The comparison includes the payload
and an embedded native uninstaller stub on both sides.

## Current scope

The native implementation currently packages the complete TapTap payload and renders the first
page's absolute bitmap/text layers, borderless rounded window, drag region, minimize action, and
close action. HBox/Content flow layout, complete checkbox/select visuals, page transitions,
payload extraction, installation tasks, and uninstallation remain to be implemented.

The previous eframe implementation remains available on `main`; it is intentionally absent from
this native-only branch.

## TapTap resource notice

The TapTap name, trademarks, images, copy, and related materials under `examples/TapTap` belong to
易玩（上海）网络科技有限公司 and the applicable rights holders. They are included only for
development, testing, and compatibility validation of this tool and are not licensed under this
project's MIT license.

## License

Code in this repository is licensed under MIT except for the TapTap resources described above.
