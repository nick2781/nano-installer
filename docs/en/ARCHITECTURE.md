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

The builder and the runtime are separate programs. A setup you generate holds a runtime stub and
your project data; it does not hold the builder.

You point the builder at a payload and it reads the first bytes to pick a runtime: a `7z` signature
selects the LZMA runtime, a `PK` signature selects the ZIP runtime. It then copies that stub, writes
your icon, version, and application manifest resources into it, appends a bundle holding your
layouts, assets, locales, scripts, and payload, and appends a self-contained uninstaller. The
payload keeps whatever compression you gave it; the bundler adds no second compression layer.

## Inside the generated setup

**Startup.** The runtime reads the bundle footer to index the bundle, so it knows where every entry
lives without loading the payload into memory. It reads payload bytes by offset only when
extraction starts.

**Interface.** Your pages come from `wizard.pages` (install) and `wizard.uninstall_pages`
(uninstall). The runtime draws each page with Win32, WIC for PNG decoding, and GDI alpha blending.
Supported today: absolute bitmap and text layers, nested `VBox`/`HBox`/`Content` flow layout with
padding, margins, percentage sizing and `flex-wrap`, progress bars, checkbox and expandable-panel
interaction, clickable links, in-place text editing with selection, clipboard, and an undo stack,
the folder chooser, runtime locale switching, a borderless rounded window, a taskbar icon,
double-buffered painting, dragging, minimize, and close.

**Questions and notices.** The setup never hands a question or a notice to a system message box.
Anything the user has to answer, such as the close confirmation, and anything they have to
acknowledge, is drawn inside the window from the layout named by `ui.dialog_layout`. A dialog is an
ordinary layout whose `value-source="dialog:*"` reads the question and button labels of the moment;
only its own controls respond while it is open, and `Enter`/`Escape` confirm and dismiss it. The
card takes a project script's `show_message`, `show_error` and `ask_yes_no` too, and the button it
answers with is what the script carries on from. The runtime centres the window on the work area of
the monitor it is on, and clamps it into that work
area when a layout is larger than the desktop.

**Tasks.** Install and uninstall run on a worker thread that publishes progress and page changes
through shared state. The UI thread repaints when it receives a refresh message, so painting never
leaves the thread that owns the window and your window stays responsive while it extracts.

**Install.** The runtime stages files, writes the manifest and the uninstall registration, and
creates the shortcuts and autostart value you configured. It treats a destination that already
holds this project as an upgrade: it backs replaced files up in a rollback journal, removes files
the new payload no longer ships, and restores the previous version if something fails.

**Uninstall.** The runtime ends the product process, deletes the recorded shortcuts and files, and
removes the declared user data only when the user clears the keep-data option. Windows refuses to
let a process delete the image it is running from, so the uninstall finishes by copying itself into
a cleaner in the temporary directory. The cleaner waits for the uninstaller to exit, deletes it,
and removes the emptied installation directory; it then hands its own removal to a short-lived
`cmd` script. It keeps a directory that still holds files the user added.

**The run log.** Every install and uninstall leaves a log on disk: in the `nano-installer` directory
of the temporary directory by default, named after the setup image, the moment and which task this
is, or in the file a windowless run names with `--log`. It holds the machine, the product and the
directory, every step the run took and every line a project script wrote. It never sits inside the
installation: a failed fresh install removes the directory it was writing into, which is exactly
when the log is wanted. The wizard puts the path under the error it reports, and a windowless run
writes it to standard error.

**Custom steps.** If your project ships `scripts/install.rhai` or `scripts/uninstall.rhai`, those
scripts run instead of the built-in steps. The engine is embedded in `nano-installer-core`, so
every stub carries it; primitives reuse the same deployment, rollback, and manifest code as the
built-in flow, and an operation ceiling keeps a runaway script from hanging an installation.

## Tooling

`nano-installer-cli` and `nano-installer-gui` are thin frontends over the same core inspection and
build APIs. Argument parsing and the eframe UI stay outside core, so the GUI's egui, eframe, and
winit dependencies never reach a stub or the setup you ship.

The builder generates the manifest from `install.require_admin` and `ui.dpi_aware`, so your setup
asks Windows for the rights it needs and tells the shell it scales its own pixels. Windows reads
the manifest before the process starts, which is why those settings are resources rather than
runtime options.

Scaling is declared in two elements: `dpiAware` for Windows 7/8/8.1 and `dpiAwareness` for Windows
10 1607 and later, set to `PerMonitorV2, PerMonitor` so each supported release gets the sharpest
behaviour it offers. On `WM_DPICHANGED` the runtime lays the page out again for the new display
rather than letting the shell stretch a bitmap.

## Next stages

Production validation on a real Windows 7 SP1 machine, plus code signing.
