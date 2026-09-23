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

When the setup is written, the builder hands both finished files to a command of the project's own --
the uninstaller while it is still a file of its own, before it is embedded, and the setup once it is
complete and closed. Every line the command prints goes into the build log, a non-zero exit code
stops the build, and the setup it refused is not left on disk. Signing is what those two hooks
(NSIS's `!finalize` and `!uninstfinalize`) are usually for, and the builder signs nothing itself; the
[configuration reference](CONFIG_REFERENCE.md#commands-that-run-on-the-finished-build) says how to
name them.

## Inside the generated setup

**Startup.** The runtime reads the bundle footer to index the bundle, so it knows where every entry
lives without loading the payload into memory. Each entry carries the SHA-256 the build recorded for
it, and every read is checked against that digest -- a streamed payload is hashed as it goes past --
so a setup damaged after the build is refused by entry name instead of unpacking bytes nobody
vouched for. It reads payload bytes by offset only when extraction starts.

**Interface.** Your pages come from `wizard.pages` (install) and `wizard.uninstall_pages`
(uninstall). The runtime draws each page with Win32, WIC for PNG decoding, and GDI alpha blending.
Supported today: absolute bitmap and text layers, nested `VBox`/`HBox`/`Content` flow layout with
padding, margins, percentage sizing and `flex-wrap`, progress bars, checkbox and expandable-panel
interaction, clickable links, in-place text editing with selection, clipboard, and an undo stack,
the folder chooser, runtime locale switching, a borderless rounded window, a taskbar icon,
double-buffered painting, dragging, minimize, and close. Pages move with `action="next"` and
`action="back"`; with a `scripts/pages.rhai` in the project, every Next asks its `next_page(from)`
where to go -- a hook is given read-only primitives only -- and Back retraces the pages the user
really visited.

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
creates the shortcuts and autostart value you configured. That registration is the set a Windows
installation list reads: the name, version, publisher, install location, uninstall command and
icon, the quiet uninstall command, the size in kilobytes counted from the bytes on disk, and two
flags saying there is no separate modify or repair step. It treats a destination that already
holds this project as an upgrade: it backs replaced files up in a rollback journal, removes files
the new payload no longer ships, and restores the previous version if something fails.

**Updates.** A release can be built as an update package instead of a full setup: `--delta-from`
names the payload archive of the release it replaces, and the builder expands both archives with the
runtime stubs and compares them by size and SHA-256, so it carries only the files whose bytes
changed. What it leaves behind goes into an `update/plan.json` entry of the bundle, naming each
expected file with its byte count and digest. The runtime reads that entry first: an update package
proceeds only where the manifest of that release is present and every expected file still matches,
and refuses before writing anything when one does not. The files it keeps stay part of the
installation -- the manifest names them, so an uninstall takes them back -- and a file the update
does carry is not copied over a target that already holds exactly those bytes.
The framework ships no updater of its own, the way NSIS does not: it offers the primitives
that build an update package and run a new setup, and leaves asking for a new version and
installing it to the product, so there is no "check for updates" switch here.

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
built-in flow, and an operation ceiling keeps a runaway script from hanging an installation. A
project that ships `scripts/pages.rhai` hands the page order to its `next_page(from)`, see the
[script API](SCRIPT_API.md).

## The installer package

A build can also wrap the finished setup in the package an estate deploys:
`--msi <file>` writes a `.msi` around the setup this build wrote, and it does so
after the project's own command has had that setup, so the image inside the
package is the one a machine will install. The package carries the setup as a
stream of its own and runs it from an install action queued into the installation
script, with `--silent --dir "<location>"`; the removal action runs the
uninstaller the setup deployed. What else the package holds is the bookkeeping
Windows Installer needs to treat the product as installed.

The package decides where the product goes, because it also has to know where to
remove it from: `INSTALLDIR` is `%ProgramFiles%\<name>` for a setup that asks for
administrator rights and `%LOCALAPPDATA%\Programs\<name>` otherwise, and an
administrator can name another directory on the `msiexec` command line. The
package writes the directory it really used into a registry value of its own and
searches for that value when the product is removed, so a package installed into
a directory of the caller's choosing is still removed correctly. The product's
own registration is the entry Windows shows; the package keeps its own out of
that list. A newer package carries the same upgrade code as the older release and
takes the older product away before it installs the new one, which is what makes
it an upgrade rather than a second product beside the first.

Two conditions come with that. The wrapped setup has to accept a windowless run
(`advanced.silent_mode_support` and `advanced.uninstall_mode_support`), because
the package drives it with no window at all: a project that never declared one is
refused a package rather than given one that cannot install. And an
administrative install (`msiexec /a`) lays the package out without running those
actions, so it distributes nothing by itself.

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
