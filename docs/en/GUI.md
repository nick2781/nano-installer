# Visual builder

The visual builder is a front end to the CLI, on Windows 10 or later. It shares the CLI's build engine,
so it never spawns the CLI as a child process. Take `nano-installer-gui-x64.exe` and the three runtimes
from the [latest release](https://github.com/nick2781/nano-installer/releases/latest), put them in one
folder, then start it:

```powershell
.\nano-installer-gui-x64.exe
```

It opens with an empty workspace and loads no example, while a source build puts the same executable
under `target/release/` — the path a contributor takes.

## A build in five steps

1. **Open project** and pick the folder holding `installer_config.json`.
2. **Refresh** re-reads the JSON, XML, assets, and payload, but an output path you changed in the same
   project is never overwritten.
3. **Parameters** sets the project folder, output exe, and an optional runtime directory; it is also
   where you decide whether this build hands an estate a package. If so, fill in the `.msi` wrapping
   the setup. **Reset output** restores `dist/<output.installer_name>`; **Use automatic stub search**
   restores automatic lookup. The package path follows the setup's name until you type one, and the
   preview line below shows the `--msi` the build will get.
4. **Build setup** builds asynchronously, re-validating the project in a worker thread, so the summary
   on screen cannot affect the result.
5. **Save log...** exports the full log, **Open output** reveals the setup in Explorer, and the
   **After build** option sets whether the output is revealed automatically.

If a build fails, **Retry** repeats it with exactly the same parameters, so fix a file or a runtime,
then carry on.

## What the window shows

The window follows a traditional build workbench layout. A standard menu bar sits at the top and the
project summary on the left; the main area shows the build log or the parameter page, and the bottom
holds one build action and an always-visible status bar.

The left sidebar groups by purpose: configuration checks, version and setup language, packaging inputs,
installation settings. Packaging inputs cover the payload, matching runtime, installer and uninstaller
icons, and uninstaller name; installation settings cover whether the setup asks for administrator
rights and the default install directory.

Long paths show only their drive and last segment, with the full path on hover. "Last check passed"
carries the check time, and it reflects the project state only at that moment, so refresh after you
change files on disk.

**Build warnings** lists what the last inspection found: a 1x/2x PNG with no counterpart, a locale
missing page text the default locale defines, and a language listed in `supported_locales` with no file
behind it. It is a quick sanity check between you and your assets, not a substitute for the release
checklist.

## Reading the log

The log lists the steps your build actually ran: the resolved paths and sizes of the installer and
uninstaller runtimes, the uninstaller's name in the bundle, the file count and size of each resource
directory, the payload format. Then comes the bundle index, with a SHA-256 beside every entry. Resources go
straight into the custom bundle, so no intermediate `skins.zip` appears, and your payload keeps the
ZIP or 7z format you provided.

The uninstaller has its own UI bundle, which holds no payload, so the log collects `layouts`, `assets`,
`locales`, and `scripts` once under an `Uninstaller bundle:` heading, then again for the main setup.
The payload goes in once, into the main setup.

A project with a `finalize.uninstaller` or `finalize.installer` command sees that step in the log too:
a `Running finalize.uninstaller: ...` line saying which file goes to which command, followed by every
line the command printed. When it fails, the log gives its exit code, and no setup is left behind.

Log lines use local timestamps in `[YYYY-MM-DD HH:mm:ss.SSS]` form, and the log window is selectable:
mouse selection, the scroll wheel, horizontal scrolling, `Ctrl+A`, and `Ctrl+C` all work, with warnings
in amber and errors in red. **Copy all** copies the whole log with no selection, and saved or copied
text keeps the original wording and timestamps.

Menus keep a fixed minimum width and single-line labels, so switching between English and Chinese never
moves them. **Open config** opens `installer_config.json` in your default editor, and the menu also has
clear log and exit; buttons stay disabled while a build runs, so you cannot start two packaging tasks
at once.

## Interface language

**View > Interface language** switches between English and Simplified Chinese, but it translates the
builder's own controls only; the language of the installers you produce comes from the project's own
`locales` files and XML.

Build log wording stays in English whatever the interface language is, and errors from Windows or the
file system keep their original text, so you can still read the paths and error codes.

## What it does not do

The builder edits build parameters for the current build only and never rewrites your JSON or XML, so
your project configuration stays the single source of truth. A payload's compression follows its file
signature, which is why there is deliberately no compression dropdown that would have no effect.
