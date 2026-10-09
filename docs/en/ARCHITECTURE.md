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

The builder and the runtime are separate programs. The runtime is the program inside a setup that does
the work. A setup you generate holds a runtime and your project data; it never holds the builder.

You point the builder at a payload and it reads the first bytes to pick a runtime. A `7z` signature
selects the LZMA runtime, a `PK` signature selects the ZIP runtime. It copies that stub. It writes
your icon, version, and application manifest resources into it. Then it appends a bundle holding your
layouts, assets, locales, scripts, and payload. Last comes a self-contained uninstaller. The payload
keeps whatever compression you gave it; the bundler adds no second compression layer.

When the setup is written, the builder hands both finished files to a command of the project's own.
The uninstaller runs while it is still a file of its own, before it is embedded. The setup runs once
it is complete and closed. Every line the command prints goes into the build log. A non-zero exit code
stops the build, and the setup it refused is not left on disk. Signing is what those two hooks are
usually for: NSIS's `!finalize` and `!uninstfinalize`. The builder signs nothing itself; the
[configuration reference](CONFIG_REFERENCE.md#commands-that-run-on-the-finished-build) says how to
name them.

## Inside the generated setup

**Startup.** The runtime reads the bundle footer to index the bundle, so it knows where every entry
lives without loading the payload into memory. Each entry carries the SHA-256 the build recorded for
it. Every read is checked against that digest, and a streamed payload is hashed as it goes past. A
setup damaged after the build is refused by entry name, instead of unpacking bytes nobody vouched
for. It reads payload bytes by offset only when extraction starts.

**Interface.** Your pages come from `wizard.pages` (install) and `wizard.uninstall_pages`
(uninstall). The runtime draws each page with Win32, WIC for PNG decoding, and GDI alpha blending.
Supported today: absolute bitmap and text layers; nested `VBox`/`HBox`/`Content` flow layout, with
padding, margins, percentage sizing and `flex-wrap`; progress bars; checkbox and expandable-panel
interaction; clickable links; in-place text editing with selection, clipboard, and an undo stack; the
folder chooser; runtime locale switching; a borderless rounded window; a taskbar icon; double-buffered
painting; dragging, minimize, and close. Pages move with `action="next"` and `action="back"`. With a
`scripts/pages.rhai` in the project, every Next asks its `next_page(from)` where to go, and a hook is
given read-only primitives only. Back retraces the pages the user really visited.

**Screen readers.** The runtime paints every control itself, so the window owns no child window a
client could walk. The window answers the one question Windows asks about it. A `WM_GETOBJECT` for
the client area is answered with an `IAccessible` handed out through `LresultFromObject`, which COM
marshals into the client's own process. What a reader gets back is the page as it is now, in the
order the layout recorded the controls: role, name, value, state, and screen coordinates. The name is
the words the layout wrote; a text field is named by the label above it, a select by the option it
shows. Asking for a control's default action runs the same action a press runs. The window announces
focus moves, state and value changes, and page changes on its own. While a dialog is up, the tree
becomes the card's two answers and the window is named by the question. **While a task runs** there is
no control left to reach, so what the task publishes and the bar beside it are described with the
rest, the bar's value being the percentage it draws. Each change of either is announced as it happens:
the status words as a live region, the bar as a value change. The hint a page shows when a field's
value breaks a rule is part of the same description: it is the field's own description, and it is
announced when it appears or changes to another rule. The frame the wizard opens on is recorded
rather than announced. It is what a reader finds when it asks, and a user can type before that frame
has been drawn, so the baseline is taken when the window is built. The bridge lives in
`crates/nano-installer-core/src/accessibility.rs` and builds its description from the runtime's
shared state on every call, so there is no second table to keep in step with the page.

**Questions and notices.** The setup never hands a question or a notice to a system message box.
Anything the user has to answer, such as the close confirmation, and anything they have to
acknowledge, is drawn inside the window from the layout named by `ui.dialog_layout`. A dialog is an
ordinary layout whose `value-source="dialog:*"` reads the question and button labels of the moment.
Only its own controls respond while it is open, and `Enter`/`Escape` confirm and dismiss it. The card
takes a project script's `show_message`, `show_error` and `ask_yes_no` too. The button it answers with
is what the script carries on from. The runtime centres the window on the work area of the monitor it
is on, and clamps it into that work area when a layout is larger than the desktop.

**Tasks.** Install and uninstall run on a worker thread that publishes progress and page changes
through shared state. The UI thread repaints when it receives a refresh message. Painting never
leaves the thread that owns the window, so your window stays responsive while it extracts.

**Install.** The runtime stages files, writes the manifest and the uninstall registry entry, and
creates the shortcuts and autostart value you configured. That entry is the one Windows' Programs and
Features shows. It holds the set a Windows installation list reads: the name, version, publisher,
install location, uninstall command and icon, the quiet uninstall command, the size in kilobytes
counted from the bytes on disk, and two flags saying there is no separate modify or repair step. It
treats a destination that already holds this project as an upgrade. It backs replaced files up in a
rollback journal and removes files the new payload no longer ships. If something fails it restores
the previous version.

**Updates.** A release can be built as an update package instead of a full setup. `--delta-from`
names the payload archive of the release it replaces. The builder expands both archives with the
runtime stubs and compares them by size and SHA-256, so it carries only the files whose bytes
changed. What it leaves behind goes into an `update/plan.json` entry of the bundle, naming each
expected file with its byte count and digest. The runtime reads that entry first. An update package
proceeds only where the manifest of that release is present and every expected file still matches. It
refuses before writing anything when one does not. The files it keeps stay part of the installation:
the manifest names them, so an uninstall takes them back. A file the update does carry is not copied
over a target that already holds exactly those bytes. The framework ships no updater of its own, the
way NSIS does not. It offers the primitives that build an update package and run a new setup. Asking
for a new version and installing it is left to the product, so there is no "check for updates" switch
here.

**Uninstall.** The runtime ends the product process, deletes the recorded shortcuts and files, and
removes the declared user data only when the user clears the keep-data option. Windows refuses to let
a process delete the image it is running from, so the uninstall finishes by copying itself into a
cleaner in the temporary directory. The cleaner waits for the uninstaller to exit and deletes it.
Then it removes the emptied installation directory and hands its own removal to a short-lived `cmd`
script. It keeps a directory that still holds files the user added.

**The run log.** Every install and uninstall leaves a log on disk. By default it sits in the
`nano-installer` directory of the temporary directory, named after the setup image, the moment and
which task this is. A silent run names the file with `--log`. It holds the machine, the product
and the directory, every step the run took and every line a project script wrote. It never sits
inside the installation: a failed fresh install removes the directory it was writing into, which is
exactly when the log is wanted. The wizard puts the path under the error it reports, and a windowless
run writes it to standard error.

**Custom steps.** If your project ships `scripts/install.rhai` or `scripts/uninstall.rhai`, those
scripts run instead of the built-in steps. The engine is embedded in `nano-installer-core`, so every
stub carries it. Primitives reuse the same deployment, rollback, and manifest code as the built-in
flow. An operation ceiling keeps a runaway script from hanging an installation. A project that ships
`scripts/pages.rhai` hands the page order to its `next_page(from)`, see the
[script API](SCRIPT_API.md).

## The installer package

A build can also wrap the finished setup in the package an estate deploys. `--msi <file>` writes a
`.msi` around the setup this build wrote, after the project's own command has had that setup. So the
image inside the package is the one a machine will install. The package carries the setup as a stream
of its own. It runs that stream from an install action queued into the installation script, with
`--silent --dir "<location>"`. The removal action runs the uninstaller the setup deployed. What else
the package holds is the bookkeeping Windows Installer needs to treat the product as installed.

The package decides where the product goes, because it also has to know where to remove it from.
`INSTALLDIR` is `%ProgramFiles%\<name>` for a setup that asks for administrator rights and
`%LOCALAPPDATA%\Programs\<name>` otherwise. An administrator can name another directory on the
`msiexec` command line. The package writes the directory it really used into a registry value of its
own, and searches for that value when the product is removed. So a package installed into a directory
of the caller's choosing is still removed correctly. The product's own registration is the entry
Windows shows; the package keeps its own out of that list. A newer package carries the same upgrade
code as the older release and takes the older product away before it installs the new one. That is
what makes it an upgrade rather than a second product beside the first. A release cut again on the
same day keeps those three fields, and Windows Installer does not allow a package with a product code
and a version it already has to be installed again. That release is a product code of its own, and
its upgrade search counts the version it names among those to take away. So it removes the release
before it and installs itself rather than sitting beside it.

Two conditions come with that. The wrapped setup has to accept a silent run
(`advanced.silent_mode_support` and `advanced.uninstall_mode_support`), because the package drives it
with no window at all. A project that never declared one is refused a package rather than given one
that cannot install. And an administrative install (`msiexec /a`) lays the package out without
running those actions, so it distributes nothing by itself.

## Tooling

`nano-installer-cli` and `nano-installer-gui` are thin frontends over the same core inspection and
build APIs. Argument parsing and the eframe UI stay outside core. So the GUI's egui, eframe, and
winit dependencies never reach a stub or the setup you ship.

The builder generates the manifest from `install.require_admin` and `ui.dpi_aware`, so your setup
asks Windows for the rights it needs and tells the shell it scales its own pixels. Windows reads the
manifest before the process starts. That is why those settings are resources rather than runtime
options.

Scaling is declared in two elements: `dpiAware` for the minimum supported version, Windows 8 and 8.1,
and `dpiAwareness` for Windows 10 1607 and later, set to `PerMonitorV2, PerMonitor`. That way each
supported release gets the sharpest behaviour it offers. On `WM_DPICHANGED` the runtime lays the page
out again for the new display rather than letting the shell stretch a bitmap.

## Next stages

Production validation on a real machine running the minimum supported version, plus code signing.
