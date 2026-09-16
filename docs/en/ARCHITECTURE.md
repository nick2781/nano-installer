# Architecture

## From a project folder to one executable

```text
MyApp/
  installer_config.json + layouts/ + assets/ + locales/ + scripts/ + payload/
                                    |
                                    v
                     nano-installer-native-x64.exe
                                    |
                        picks a runtime by payload signature
                            /                       \
                lzma-stub-native.exe        zlib-stub-native.exe
                            \                       /
                            + bundle + uninstaller
                                    |
                                    v
                             MyApp_Setup.exe
```

The builder and the runtime are separate programs. A generated setup contains a runtime stub and
your project data, never the builder.

The builder reads the first bytes of your payload: a `7z` signature selects the LZMA runtime, a
`PK` signature selects the ZIP runtime. It then copies that stub, writes your icon, version, and
application manifest resources into it, appends a bundle holding your layouts, assets, locales,
scripts, and payload, and appends a self-contained uninstaller. The payload keeps whatever compression you gave it; the
bundler adds no second compression layer.

## Inside the generated setup

**Startup.** The runtime indexes the bundle by reading its footer, so it knows where every entry
lives without loading the payload into memory. Payload bytes are read by offset when extraction
starts.

**Interface.** Pages come from `wizard.pages` (install) and `wizard.uninstall_pages` (uninstall).
Each page is drawn with Win32, WIC for PNG decoding, and GDI alpha blending. Supported today:
absolute bitmap and text layers, nested `VBox`/`HBox`/`Content` flow layout with padding, margins,
percentage sizing and `flex-wrap`, progress bars, checkbox and expandable-panel interaction,
clickable links, in-place text editing with selection, clipboard, and an undo stack, the folder
chooser, runtime locale switching, a borderless rounded window, a taskbar icon, double-buffered
painting, dragging, minimize, and close.

**Tasks.** Install and uninstall run on a worker thread that publishes progress and page changes
through shared state. The UI thread repaints when it receives a refresh message, so painting never
leaves the thread that owns the window and the window stays responsive during extraction.

**Install.** Files are staged, the manifest and uninstall registration are written, and the
configured shortcuts and autostart value are created. A destination that already holds this
project is treated as an upgrade: replaced files are backed up in a rollback journal, files the
new payload no longer ships are removed, and a failure restores the previous version.

**Uninstall.** The product process is terminated, recorded shortcuts and files are deleted, and
the declared user data is removed only when the user clears the keep-data option. Windows refuses
to let a process delete the image it is running from, so the uninstall finishes by copying itself
into a cleaner in the temporary directory. The cleaner waits for the uninstaller to exit, deletes
it, and removes the emptied installation directory; it then hands its own removal to a short-lived
`cmd` script. A directory that still holds files the user added is kept.

**Custom steps.** A project that ships `scripts/install.rhai` or `scripts/uninstall.rhai` runs
those scripts instead of the built-in steps. The engine is embedded in `nano-installer-core`, so
every stub carries it; primitives reuse the same deployment, rollback, and manifest code as the
built-in flow, and an operation ceiling keeps a runaway script from hanging an installation.

## Tooling

`nano-installer-cli` and `nano-installer-gui` are thin frontends over the same core inspection and
build APIs. Argument parsing and the eframe UI stay outside core, so the GUI's egui, eframe, and
winit dependencies never reach a stub or a setup.

The manifest is generated from `install.require_admin` and `ui.dpi_aware`, so a setup asks Windows
for the rights it needs and tells the shell it scales its own pixels. Windows reads the manifest
before the process starts, which is why the settings are resources rather than runtime options.

## Next stages

Production validation on a real Windows 7 SP1 machine, plus code signing.
