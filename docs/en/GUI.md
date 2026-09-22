# Visual builder

The visual builder gives you a Windows 10+ window on the same build engine the CLI uses. It never
spawns the CLI as a child process.

```powershell
.\target\release\nano-installer-gui-x64.exe
```

It opens with an empty workspace and does not load any example for you.

## A build in five steps

1. **Open project** and pick the folder containing `installer_config.json`.
2. **Refresh** re-reads the JSON, XML, assets, and payload. It never overwrites an output path you
   customised for the same project.
3. **Parameters** sets the project folder, the output exe, and an optional runtime directory.
   **Reset output** restores `dist/<output.installer_name>`; **Use automatic stub search** restores
   automatic lookup.
4. **Build setup** builds asynchronously. The build re-validates the project in a worker thread, so
   the summary on screen can never affect the result.
5. **Save log...** exports the full log and **Open output** reveals the setup in Explorer. The
   **After build** option controls whether the output is revealed automatically.

If a build fails, **Retry** repeats it with exactly the same parameters, so you can fix a file or a
runtime and continue.

## What the window shows

The window is laid out like a traditional build workbench: a standard menu bar at the top, the
project summary on the left, the build log or the parameter page in the main area, and a single
build action with an always-visible status bar at the bottom.

The left sidebar groups configuration checks, version and setup language, packaging inputs
(payload, matching runtime, installer and uninstaller icons, uninstaller name), and installation
settings (whether the setup asks for administrator rights, and the default install directory). Long
paths show only their drive and last segment, and the full path appears when you hover. "Last check
passed" carries the time of that check and reflects the project state only at that moment, so
refresh after you change files on disk.

**Build warnings** tells you what the last inspection found: a 1x/2x PNG that has no counterpart, a
locale that is missing page text the default locale defines, and a language listed in
`supported_locales` with no file behind it. It is a quick sanity check between you and your assets,
not a substitute for the release checklist.

## Reading the log

The log lists the steps your build actually ran: the resolved paths and sizes of the installer and
uninstaller runtimes, the uninstaller's name inside the bundle, the file count and size of each
resource directory, the payload format, and the bundle index, which records a SHA-256 beside
every entry. Resources go straight into the custom bundle with no intermediate `skins.zip`, and your
payload keeps the ZIP or 7z format you provided.

Because the uninstaller has its own payload-free UI bundle, the log collects `layouts`, `assets`,
`locales`, and `scripts` once under an `Uninstaller bundle:` heading and then again for the main
setup, while it packages the payload once, into the main setup.

Log lines use local timestamps in `[YYYY-MM-DD HH:mm:ss.SSS]` form. Warnings are amber and errors
are red. The log window is selectable: mouse selection, the scroll wheel, horizontal scrolling,
`Ctrl+A`, and `Ctrl+C` all work in it. **Copy all** copies the complete log without a selection.
Saved and copied text keeps the original wording and timestamps.

Menus keep a fixed minimum width and single-line labels, so switching between English and Chinese
never moves them. From the menu you can also **Open config** to edit `installer_config.json` in your
default editor, clear the log, or exit. Buttons are disabled while a build runs, so you cannot start
two packaging tasks at once.

## Interface language

**View > Interface language** switches between English and Simplified Chinese. This translates the
builder's own controls only; the language of the installers you produce comes from the project's own
`locales` files and XML. Build log wording stays in English whatever the interface language is, and
errors from Windows or the file system keep their original text, so you can still read the paths and
error codes.

## What it does not do

The builder edits build parameters for the current build only. It never rewrites your JSON or XML,
so your project configuration stays the single source of truth. A payload's compression follows its
file signature, which is why there is deliberately no compression dropdown that would have no
effect.
