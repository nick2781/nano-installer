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
crates/
├── nano-installer-core/       # shared bundle and Win32 layout runtime
├── nano-installer-cli/        # nano-installer-native-x64.exe
├── nano-installer-gui/        # Windows 10+ eframe/egui frontend
├── nano-installer-stub-lzma/  # lzma-stub-native.exe
├── nano-installer-stub-zlib/  # zlib-stub-native.exe
└── nano-installer-uninstaller/# uninst-stub-native.exe
```

The builder detects ZIP versus 7z from the payload signature and copies the corresponding sibling
stub before appending project data. It injects the configured setup resources and Windows version
properties before the bundle, and never copies the builder into the generated setup. Raw stubs carry
no product resources; the builder injects them into generated executables only.

## Run

```powershell
.\scripts\build.ps1 -Project examples\TapTap
.\examples\TapTap\dist\TapTap_Setup.exe
```

The builder collects `installer_config.json`, configured layouts/assets/locales, `scripts/`, and
the configured payload. The generated executable reads its appended bundle, walks the configured
page list, parses each XML page, and draws the visible background, `Image`, `Icon`, `Button`, and
`ProgressBar` layers. Text plus the nested VBox/HBox/Content/Checkbox layout are resolved from the
configured JSON locale and rendered through Unicode `DrawTextW`. The setup resource icon is also
assigned to the Win32 window class for the taskbar and Alt+Tab.

Install and uninstall run on a worker thread that publishes progress and page changes, so the
progress pages advance their bar and status text while the task runs. The finish page can launch
the deployed application. Press Escape or close the native window to exit.

## Verification

The minimum OS target is Windows 7 SP1+ x64 for the builder, the LZMA and zlib stubs, and the
uninstaller; all four pass the PE import audit. The separate Windows 10+ GUI is not linked into any
of these artifacts. Per-binary sizes are measured from `target/release/` after each build and are
tracked as release metrics rather than documented here.

## Next slice

The backend executables securely extract real 7z and ZIP archives for the install action. Core
records deployed files, shortcuts, and autostart entries in a manifest, backs up replaced files for
rollback, and writes an uninstall registry key; the uninstaller removes only tracked files. Project
Rhai script execution, cancellation, automatic UAC, and code signing remain open.
