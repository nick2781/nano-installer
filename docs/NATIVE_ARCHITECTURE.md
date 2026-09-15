# Native architecture

```text
examples/TapTap
  installer_config.json + layouts + assets + locales + scripts + payload
                                |
                                v
                 nano-installer-native-x64.exe
                                |
                     selects by payload signature
                    /                           \
       lzma-stub-native.exe            zlib-stub-native.exe
                    \                           /
                     + bundle + native uninstaller
                                |
                                v
                         TapTap_Setup.exe
```

The builder and runtime are separate binaries and Cargo packages under `crates/`. Setup packages
contain a native runtime stub, never the builder. The LZMA package links only `sevenz-rust`, the
zlib package links only ZIP/Deflate, and the uninstaller links neither archive backend.

`nano-installer-cli` and `nano-installer-gui` are thin frontends over the same core inspection and
build APIs. Argument parsing and eframe UI code remain outside core.

The runtime renders through Win32, WIC, GDI alpha blending, and Unicode `DrawTextW`. It currently
supports absolute bitmap/text layers, nested VBox/HBox/Content flow layout with padding, margins
and percentage sizing, progress bars, checkbox and expandable-panel interaction, static link
rendering, runtime locale switching, locale key resolution, a borderless rounded window, taskbar
icon, double-buffered GDI painting, dragging, minimize, and close actions.

The wizard walks `wizard.pages`, `wizard.update_pages`, or `wizard.uninstall_pages`. A worker
thread performs the task and publishes progress and page changes through a shared state; the UI
thread repaints when it receives the posted refresh message, so painting never leaves the thread
that owns the window.

Installation deploys the payload, writes the manifest and uninstall registration, and creates the
configured shortcuts and autostart value. A destination that already holds this project is treated
as an upgrade: replaced files are backed up through a rollback journal, files the new payload no
longer ships are dropped, and a failure restores the previous version. Removal terminates the
product process, deletes the recorded shortcuts and files, and deletes the declared user data
unless the keep-data option is selected.

The next implementation stages are Rhai script execution, link input, and production validation on
a real Windows 7 SP1 VM.

## Bundle caveat

The validation bundle stores resources and payload without another compression layer; the payload
remains in its original ZIP or 7z form. The runtime indexes the bundle footer and reads individual
entries by offset, so startup no longer loads the full payload into memory.
