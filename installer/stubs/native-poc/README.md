# Native Win32 runtime POC

This experimental x64 runtime validates the smallest useful UI slice before the installer engine
is migrated away from eframe. It uses only:

- a Unicode Win32 window and message loop;
- Windows Imaging Component (WIC) for PNG decoding;
- GDI `StretchDIBits` for bitmap rendering.

It intentionally does not depend on `nano-installer-lib`, eframe, egui, winit, glutin, OpenGL,
the XML parser, Taffy, Rhai, or the Rust image crate.

## Run

```powershell
cargo build --locked --release -p nano-installer-native-poc
.\target\release\native-ui-x64.exe .\examples\TapTap\assets\bg_main.png
```

Press Escape or close the native window to exit. When no path is supplied, the command uses the
TapTap background above relative to the current working directory.

For the Win7 x64 target, use the same `build-std` flow as the production compatibility build:

```powershell
$env:RUSTC_BOOTSTRAP = "1" # local POC only; CI must use the pinned nightly toolchain
cargo build --locked --release `
  -Z build-std=std,panic_abort `
  --target x86_64-win7-windows-msvc `
  -p nano-installer-native-poc
```

## Measured result

| Target | Release size | Win7 import audit |
| --- | ---: | --- |
| `x86_64-pc-windows-msvc` | 151.5 KiB | Not applicable (standard Rust target) |
| `x86_64-win7-windows-msvc` | 227.5 KiB | Passed |

These measurements exclude product assets. Assets remain in the appended installer bundle.

## Next slice

The POC is not an installer yet. The next implementation should read the existing `NANORSRC`
bundle, render a build-time-compiled page description, add mouse hit testing, and dispatch a small
fixed action set. Payload extraction and installation tasks should be added only after that UI path
works without linking `nano-installer-lib`.
