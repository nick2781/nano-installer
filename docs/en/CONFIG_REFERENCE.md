# Configuration reference

Every project has one `installer_config.json`. This page lists the settings that change what your
installer does today. A key this build does not read fails the build instead of being ignored, so
the configuration in front of you is the one your setup behaves by; the keys it refuses are listed
at the end.

## Product identity

| Setting | Type | Effect |
| --- | --- | --- |
| `project.name` | string | Product name; also the default file description |
| `project.version` | string | Your product version, may carry a release suffix |
| `project.file_version` | string | Windows file/product version, 1-4 numeric parts |
| `project.description` | string | Optional file description, defaults to `<project.name> Installer` |
| `project.output_name` | string | Internal name, defaults to `project.name` |
| `project.publisher` | string | Company name in file properties |
| `project.copyright` | string | Copyright text in file properties |

If you omit `project.file_version`, the builder uses `project.version` for the Windows version
resource, and it must then be numeric.

## Output files

| Setting | Type | Effect |
| --- | --- | --- |
| `output.installer_name` | string | Setup file name when `--output` is not passed |
| `output.installer_icon` | string | Optional ICO for the setup file, taskbar, and Alt+Tab |
| `output.uninstaller_name` | string | File name of the embedded uninstaller, defaults to `uninst.exe`; must be a bare file name |
| `output.uninstaller_icon` | string | Optional ICO for the generated uninstaller |

You write these paths relative to the project folder.

## Install behaviour

| Setting | Type | Effect |
| --- | --- | --- |
| `install.default_path` | string | Initial install directory shown on the first page |
| `install.required_space_mb` | integer | Required space in MiB: the install stops before writing anything when the destination drive has less free space; an XML `value-source` binding can show the same number |
| `install.exe_name` | string | The application executable the payload must contain; installation stops before deploying anything if it is missing |
| `install.require_admin` | bool | Ask Windows for administrator rights before the setup starts, defaults to `false` |
| `install.kill_process_on_install` | bool | Close running copies of the product before installing; failure aborts the install |
| `install.kill_process_on_uninstall` | bool | Close running copies of the product before uninstalling |
| `install.detect_running_process` | bool | Combined with the two switches above; any of them closes the process |
| `registry.uninstall_key` | string | Uninstall entry location, `HKCU` or `HKLM` only; an existing key is never overwritten |
| `links.*` | string | URLs a layout reaches through `[label](key)` markup or `action="open_url:key"` |

## Shortcuts and autostart

| Setting | Type | Effect |
| --- | --- | --- |
| `shortcuts.desktop_shortcut` | bool | Allow a desktop shortcut |
| `shortcuts.desktop_default` | bool | Default state of `chkShotcut`, defaults to `true` |
| `shortcuts.start_menu` | bool | Create product and uninstall entries in the Start menu |
| `shortcuts.start_menu_folder` | string | Start menu subfolder, defaults to `project.name`; only folders this install created and emptied are removed |
| `autostart.enabled` | bool | Allow an autostart entry |
| `autostart.default` | bool | Default state of `chkAutoRun`, defaults to `false` |
| `autostart.registry_key` | string | Autostart key, defaults to `HKCU\...\CurrentVersion\Run` |
| `autostart.registry_value_name` | string | Autostart value name, defaults to `project.name` |

Whether the checkboxes exist at all is up to your layout: if `chkShotcut` or `chkAutoRun` is
absent, the installer falls back to the corresponding default above. It records both entries while
installing, so removal restores the machine to its previous state.

## Files, languages, and pages

| Setting | Type | Effect |
| --- | --- | --- |
| `resources.layouts_dir` | string | XML directory to package, defaults to `layouts` |
| `resources.assets_dir` | string | Image directory to package, defaults to `assets` |
| `resources.locales_dir` | string | Language JSON directory, defaults to `locales` |
| `resources.payload_file` | string | Required; path to the ZIP or 7z payload |
| `resources.tools_dir` | string | Optional; directory of helper programs to bundle, read back by `get_tools_dir()` |
| `localization.default_locale` | string | Language used at startup, defaults to `zh-CN` |
| `localization.supported_locales` | array | Languages you intend to ship; the build reports any entry without a matching JSON file |
| `wizard.pages[].layout` | string | Install pages, walked with `action="next"` and `action="back"` |
| `wizard.pages[].role` | string | Optional: `progress` marks the page a task reports on, `finish` the page it ends on |
| `wizard.uninstall_pages[].layout` | string | Uninstall pages, walked the same way |
| `wizard.uninstall_pages[].role` | string | The same two roles for the uninstaller |

Without a `role`, the second page reports and the last one finishes, which is what the three
pages an ordinary project declares mean. A page may also carry `id` and `title`: the first names
it for your own reading, and the second is the heading a report shows it under.

## Interface

| Setting | Type | Effect |
| --- | --- | --- |
| `ui.dpi_aware` | bool | Enable DPI awareness and layout scaling, defaults to `true` |
| `ui.dpi_threshold` | integer | DPI at which `@2x` images are preferred, defaults to `144` |
| `ui.dialog_layout` | string | Layout used for the confirmation dialog, defaults to `layouts/msgBox.xml` |

The builder also writes `ui.dpi_aware` into the setup's application manifest, so Windows knows the
window scales itself rather than rescaling a blurry bitmap of it. With it on, the manifest asks for
per-monitor awareness: move the window onto a display with a different scaling factor and the
runtime lays it out again for that display, so text and artwork stay sharp. With it off the manifest
declares `unaware`, and the shell scales the window instead.

`ui.dpi_threshold` decides when the runtime prefers `@2x` artwork. It applies to whichever DPI is
in effect, so moving the window to another display re-evaluates it.

`ui.dialog_layout` names the layout the runtime draws inside the window for questions such as "exit
the installer?", for notices the user has to acknowledge, and for what a project script says through
`show_message`, `show_error` and `ask_yes_no`. The dialog therefore wears the product's own skin and
cannot end up behind the installer. If your project ships no such layout there is no question: the
close button then exits immediately, and a script's message falls back to a system message box. See
[page layout](XML_LAYOUT_GUIDE.md#dialogs).

## Administrator rights

`install.require_admin` decides the setup's `requestedExecutionLevel`:

```json
"install": { "require_admin": true }
```

- `true` writes `requireAdministrator`, so Windows shows the UAC consent prompt before the setup
  starts and the window runs elevated. Choose this when the install path is under `Program Files`.
- `false` (the default) writes `asInvoker`: no prompt, and the same rights the user already has.
  Choose this for a per-user install under `%LOCALAPPDATA%`.

The builder generates the embedded uninstaller from the same configuration, so it asks for the same
level. Otherwise the uninstall entry could not undo an elevated install.

## User data on uninstall

`uninstall.data_paths` lists the folders your product owns. The uninstaller deletes them only when
the user clears the keep-data checkbox, and it ignores entries that do not expand to a location
under `%APPDATA%` or `%LOCALAPPDATA%`, so unrelated files are never removed. If your layout has no
`chkReserveData`, user data is always kept.

```json
"uninstall": {
  "data_paths": ["%APPDATA%\\MyApp", "%LOCALAPPDATA%\\MyApp"]
}
```

## Payload format

The payload is your application's files, already compressed as ZIP or 7z. The builder detects the
format from the file signature, not the extension: `PK` selects the ZIP runtime and
`37 7A BC AF 27 1C` selects the 7z runtime. Any other format fails the build.

## Custom install and uninstall steps

Add `scripts/install.rhai` or `scripts/uninstall.rhai` to replace the built-in steps. See the
[script API](SCRIPT_API.md).

## Unattended runs

You can run your install or uninstall with no window at all, which is what a software deployment
tool needs. Your project has to say so first:

```json
"advanced": {
  "silent_mode_support": true,
  "uninstall_mode_support": true
}
```

With the switches on, the same executables accept `--silent`:

```powershell
MyApp_Setup.exe --silent --dir "%LOCALAPPDATA%\MyApp"
MyApp_Setup.exe --silent
uninst.exe --silent
```

- `--silent` installs or uninstalls without opening anything. A silent run never shows a dialog,
  because a dialog would wait for a click that never comes.
- `--dir` chooses the install directory for that run and wins over `install.default_path`. Both
  accept environment variables, which are expanded before use.
- The setup refuses any other option, so a mistyped flag stops the run instead of installing into
  the configured default.
- `silent_mode_support` and `uninstall_mode_support` are separate, so a product can allow
  unattended installs without allowing unattended removal.
- Because there are no checkboxes to read, the installer follows `shortcuts.desktop_default` and
  `autostart.default` for shortcuts and autostart, and always keeps user data on uninstall.

A windowless run reports progress to whoever started it: it writes to the console instead of
painting a window, and a failure sets a non-zero exit code with the reason on standard error.

## Minimum configuration

```json
{
  "project": {
    "name": "MyApp",
    "version": "2026.9.22-rel.1",
    "file_version": "2026.9.22",
    "publisher": "Example Company",
    "copyright": "Copyright 2026 Example Company"
  },
  "output": {
    "installer_name": "MyApp_Setup.exe",
    "installer_icon": "assets/logo.ico"
  },
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "payload_file": "payload/app.7z"
  },
  "localization": { "default_locale": "zh-CN" },
  "install": { "default_path": "C:\\Program Files\\MyApp", "require_admin": true },
  "ui": { "dpi_aware": true, "dpi_threshold": 144 },
  "wizard": {
    "pages": [
      { "id": "config", "layout": "layouts/configpage.xml" }
    ]
  }
}
```

## Refused settings

A setting that is accepted and then ignored is worse than one that is missing: the project reads
as if it worked, and the setup ships without it. The build therefore refuses a configuration that
holds a key it does not read, and names the setting that does the job instead.

| Refused key | What to do instead |
| --- | --- |
| `install.append_to_path` | nothing adds a directory to PATH yet |
| `install.mutex_name` | a setup does not yet refuse to run while another copy installs the same product |
| `registry.install_path_key` | the install path is not written into a registry value |
| `registry.help_link` | put the URL under `links` and open it with `action="open_url:<key>"` |
| `resources.installer_icon` | use `output.installer_icon` |
| `output.installer_stub`, `output.uninstaller_stub` | the runtimes come from the build's `--stubs` directory |
| `validation.*` | no build or runtime step reads it; `install.required_space_mb` is the one check that runs |
| `wizard.update_pages` | an upgrade replays `wizard.pages` |
| `advanced.update_mode_support` | an upgrade is decided by what is already installed at the destination |
| `advanced.launch_app_after_install` | put `action="launch_app"` on the finish page |
| `localization.show_language_selector` | the language list comes from the `Select` control in the layout |
| `ui.window_width`, `ui.window_height`, `ui.expanded_height` | the window size comes from `<Page width height>` in the layout |
| `ui.window_corner_radius` | the corner radius comes from `<Page border-radius>` in the layout |

`advanced.silent_mode_support` and `advanced.uninstall_mode_support` are read, and are described
under [unattended runs](#unattended-runs).

Any other key inside the sections above is refused as well, so a misspelled setting fails the build
instead of doing nothing. Two things are outside that rule: `links` is a table your project names
itself, and a section this build does not know at all is left alone, because a script reads it back
through `get_config_value`.

See [production status](PRODUCTION_STATUS.md) for the full picture.
