# Native Win32 builder and runtime stubs

This x64 runtime validates the smallest useful UI slice before the installer engine
is migrated away from eframe. It uses only:

- a Unicode Win32 window and message loop;
- Windows Imaging Component (WIC) for PNG decoding;
- GDI alpha blending for bitmap rendering;
- a self-contained project bundle appended to the generated executable.

It intentionally does not depend on `nano-installer-lib`, eframe, egui, winit, glutin, OpenGL,
Taffy, Rhai, or the Rust image crate. The native builder currently uses `roxmltree` and
`serde_json` to validate the first implementation slice.

```text
installer/native/
├── cli/                 # nano-installer-native-x64.exe
├── src/                 # shared bundle and Win32 layout runtime
└── stubs/
    ├── lzma/            # native-lzma-x64.exe
    ├── zlib/            # native-zlib-x64.exe
    └── uninst/          # native-uninst-x64.exe
```

The builder detects ZIP versus 7z from the payload signature and copies the corresponding sibling
stub before appending project data. It never copies the builder into the generated setup.

## Run

```powershell
cargo build --locked --release -p nano-installer-native
.\target\release\nano-installer-native-x64.exe build `
  --project .\examples\TapTap `
  --output .\target\TapTap_Native_Setup.exe

.\target\TapTap_Native_Setup.exe
```

The builder collects `installer_config.json`, configured layouts/assets/locales, `scripts/`, and
the configured payload. The generated executable reads its appended bundle, selects
`wizard.pages[0].layout`, parses the XML, and draws the visible background, `Image`, `Icon`, and
`Button` bitmap layers. Absolute `Button`, `Label`, and `Select` text is resolved from the
configured JSON locale and rendered through Unicode `DrawTextW`. Press Escape or close the native
window to exit.

For the Win7 x64 target, use the same `build-std` flow as the production compatibility build:

```powershell
$env:RUSTC_BOOTSTRAP = "1" # local validation only; CI must use the pinned nightly toolchain
cargo build --locked --release `
  -Z build-std=std,panic_abort `
  --target x86_64-win7-windows-msvc `
  -p nano-installer-native
```

## Measured result

| Target | Builder | Each current stub | Win7 import audit |
| --- | ---: | ---: | --- |
| `x86_64-pc-windows-msvc` | 210.5 KiB | 265.5 KiB | Not applicable |
| `x86_64-win7-windows-msvc` | 274 KiB | 336.5 KiB | Passed |

The runtime measurements exclude product data. Generated setup size includes the appended TapTap
layouts, assets, locales, scripts, and 143.69 MiB payload. The current standard setup is
146.84 MiB and the Win7 SP1 x64 setup is 146.91 MiB. The three stubs are currently the same size
because payload extraction and uninstall backends have not yet been linked into their now-separate
binary boundaries.

## Next slice

This runtime does not execute installation tasks yet. The next implementation should add complete
HBox/Content layout, checkbox/select visuals, page transitions, and a small fixed action set.
Payload extraction and installation tasks should follow after the UI path is stable.
