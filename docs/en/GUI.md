# Visual builder

The visual builder is the Windows 10+ front end for the same build engine the CLI uses. It never
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
   the result never depends on the summary currently on screen.
5. **Save log...** exports the full log and **Open output** reveals the setup in Explorer. The
   **After build** option controls whether the output is revealed automatically.

If a build fails, **Retry** repeats it with exactly the same parameters, so you can fix a file or a
runtime and continue.

## What the window shows

The layout follows traditional build workbenches: a standard menu bar at the top, the project
summary on the left, the build log or the parameter page in the main area, and a single build
action with an always-visible status bar at the bottom.

The left sidebar groups configuration checks, version and setup language, packaging inputs
(payload, matching runtime, installer and uninstaller icons, uninstaller name), and the default
install directory. Long paths show their drive and last segment, with the full path on hover.
"Last check passed" carries the time of that check and only reflects the project state at that
moment; refresh after changing files on disk. DPI asset warnings only compare 1x/2x PNG pairs and
are not a full production check.

## Reading the log

The log records the steps that actually ran: the resolved paths and sizes of the installer and
uninstaller runtimes, the uninstaller's name inside the bundle, the file count and size of each
resource directory, the payload format, and the bundle index. Resources go straight into the custom
bundle with no intermediate `skins.zip`, and the payload keeps the ZIP or 7z format you provided.

Because the uninstaller has its own payload-free UI bundle, `layouts`, `assets`, `locales`, and
`scripts` are collected once under an `Uninstaller bundle:` heading and again for the main setup;
the payload is packaged once, into the main setup.

Log lines use local timestamps in `[YYYY-MM-DD HH:mm:ss.SSS]` form. Warnings are amber and errors
are red in the selectable log window, which supports mouse selection, the scroll wheel, horizontal
scrolling, `Ctrl+A`, and `Ctrl+C`. **Copy all** copies the complete log without a selection. Saved
and copied text keeps the original wording and timestamps.

Menus keep a fixed minimum width and single-line labels, so switching between English and Chinese
never changes the menu geometry. The menu also offers **Open config** to edit
`installer_config.json` in your default editor, clearing the log, and exiting. Buttons are disabled
while a build runs, so two packaging tasks cannot start at once.

## Interface language

**View > Interface language** switches between English and Simplified Chinese. This only translates
the builder's own controls; the language of the installers you produce comes from the project's own
`locales` files and XML. Build log wording stays in English regardless of the interface language,
and errors returned by Windows or the file system keep their original text so paths and error codes
stay diagnosable.

## What it does not do

The builder edits build parameters for the current build; it does not rewrite your JSON or XML, and
your project configuration stays the single source of truth. The compression of a payload follows
its file signature, so there is deliberately no compression dropdown that would have no effect.
