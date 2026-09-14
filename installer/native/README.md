# Native Win32 builder and runtime stubs

This x64/Unicode runtime targets Windows 7 SP1 and later. It uses only:

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
.\scripts\build.ps1 -Project examples\TapTap
.\target\release\TapTap_Setup.exe
```

The builder collects `installer_config.json`, configured layouts/assets/locales, `scripts/`, and
the configured payload. The generated executable reads its appended bundle, selects
`wizard.pages[0].layout`, parses the XML, and draws the visible background, `Image`, `Icon`, and
`Button` bitmap layers. Absolute `Button`, `Label`, and `Select` text is resolved from the
configured JSON locale and rendered through Unicode `DrawTextW`. Press Escape or close the native
window to exit.

## Measured result

| Minimum OS target | Builder | Each current stub | Import audit |
| --- | ---: | ---: | --- |
| Windows 7 SP1+ x64 | 274.5 KiB | 336.5 KiB | Passed |

The runtime measurements exclude product data. Generated setup size includes the appended TapTap
layouts, assets, locales, scripts, 143.69 MiB payload, and the native uninstaller stub. The current
setup is 147.24 MiB versus the previous eframe baseline of 159.14 MiB, a reduction of about
11.90 MiB. The three stubs are currently the same size because payload extraction and uninstall
backends have not yet been linked into their now-separate binary boundaries.

## Next slice

This runtime does not execute installation tasks yet. The next implementation should add complete
HBox/Content layout, checkbox/select visuals, page transitions, and a small fixed action set.
Payload extraction and installation tasks should follow after the UI path is stable.
