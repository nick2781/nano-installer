# Configuration reference

Every project has one `installer_config.json`. This page lists the settings that change what your
installer does today. Settings that are accepted but not yet acted on are listed at the end, so you
never have to guess whether a field is live.

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

If `project.file_version` is omitted, `project.version` is used for the Windows version resource;
it must then be numeric.

## Output files

| Setting | Type | Effect |
| --- | --- | --- |
| `output.installer_name` | string | Setup file name when `--output` is not passed |
| `output.installer_icon` | string | Optional ICO for the setup file, taskbar, and Alt+Tab |
| `output.uninstaller_name` | string | File name of the embedded uninstaller, defaults to `uninst.exe`; must be a bare file name |
| `output.uninstaller_icon` | string | Optional ICO for the generated uninstaller |

Paths are relative to the project folder.

## Install behaviour

| Setting | Type | Effect |
| --- | --- | --- |
| `install.default_path` | string | Initial install directory shown on the first page |
| `install.required_space_mb` | integer | Required space in MiB, shown through an XML `value-source` binding |
| `install.exe_name` | string | The application executable the payload must contain; installation stops before deploying anything if it is missing |
| `install.kill_process_on_install` | bool | Close running copies of the product before installing; failure aborts the install |
| `install.kill_process_on_uninstall` | bool | Close running copies of the product before uninstalling |
| `install.detect_running_process` | bool | Combined with the two switches above; any of them closes the process |
| `registry.uninstall_key` | string | Uninstall entry location, `HKCU` or `HKLM` only; an existing key is never overwritten |

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

Whether the checkboxes exist at all is up to your layout: if `chkShotcut` or `chkAutoRun` is absent,
the corresponding default above is used. Both entries are recorded while installing, so removal
restores the machine to its previous state.

## Files, languages, and pages

| Setting | Type | Effect |
| --- | --- | --- |
| `resources.layouts_dir` | string | XML directory to package, defaults to `layouts` |
| `resources.assets_dir` | string | Image directory to package, defaults to `assets` |
| `resources.locales_dir` | string | Language JSON directory, defaults to `locales` |
| `resources.payload_file` | string | Required; path to the ZIP or 7z payload |
| `localization.default_locale` | string | Language used at startup, defaults to `zh-CN` |
| `wizard.pages[].layout` | string | Install pages: first is the welcome page, second shows progress, last is the finish page |
| `wizard.update_pages[].layout` | string | Reserved update page list; the install flow uses `wizard.pages` |
| `wizard.uninstall_pages[].layout` | string | Uninstall pages, switched in the same order |

## Interface

| Setting | Type | Effect |
| --- | --- | --- |
| `ui.dpi_aware` | bool | Enable DPI awareness and layout scaling, defaults to `true` |
| `ui.dpi_threshold` | integer | DPI at which `@2x` images are preferred, defaults to `144` |

## User data on uninstall

`uninstall.data_paths` lists the folders a product owns. They are deleted on uninstall only when
the user clears the keep-data checkbox, and entries that do not expand to a location under
`%APPDATA%` or `%LOCALAPPDATA%` are ignored, so unrelated files are never removed. If your layout
has no `chkReserveData`, user data is always kept.

```json
"uninstall": {
  "data_paths": ["%APPDATA%\\MyApp", "%LOCALAPPDATA%\\MyApp"]
}
```

## Payload format

The payload is your application's files, already compressed as ZIP or 7z. The format is detected
from the file signature, not the extension: `PK` selects the ZIP runtime and `37 7A BC AF 27 1C`
selects the 7z runtime. Any other format fails the build.

## Custom install and uninstall steps

Add `scripts/install.rhai` or `scripts/uninstall.rhai` to replace the built-in steps. See the
[script API](SCRIPT_API.md).

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
  "ui": { "dpi_aware": true, "dpi_threshold": 144 },
  "wizard": {
    "pages": [
      { "id": "config", "layout": "layouts/configpage.xml" }
    ]
  }
}
```

## Accepted but not yet in effect

These settings are packaged into the setup but are not read at runtime. Do not treat their
presence as a working feature:

- `install.*` and `registry.*` fields other than the ones listed above
- `links.*`, `validation.*`, `advanced.*`
- `localization.supported_locales` and `localization.show_language_selector`; the language list
  and its visibility come from the XML `Select` control
- `advanced.update_mode_support`; upgrades are detected from an existing installation in the
  destination, not from this switch

See [production status](PRODUCTION_STATUS.md) for the full picture.
