# Configuration reference

Every project has one `installer_config.json`. This page lists the settings that really change what
your installer does. A key this build does not read fails the build; it is never quietly ignored.
What you write is what your setup does. The keys it refuses are at the end.

## Product identity

| Setting | Type | Effect |
| --- | --- | --- |
| `project.name` | string | Product name, and the default file description |
| `project.version` | string | Your product version, may carry a release suffix |
| `project.file_version` | string | Windows file/product version, 1-4 numeric parts |
| `project.description` | string | Optional file description, defaults to `<project.name> Installer` |
| `project.output_name` | string | Internal name, defaults to `project.name` |
| `project.publisher` | string | Company name in file properties |
| `project.copyright` | string | Copyright text in file properties |

If you leave `project.file_version` out, the Windows version resource uses `project.version`. That
value must then be plain numbers.

## Output files

| Setting | Type | Effect |
| --- | --- | --- |
| `output.installer_name` | string | Setup file name when `--output` is not passed |
| `output.installer_icon` | string | Optional ICO for the setup file, taskbar and Alt+Tab |
| `output.uninstaller_name` | string | File name of the embedded uninstaller, defaults to `uninst.exe`. A bare file name only, no folder |
| `output.uninstaller_icon` | string | Optional ICO written into the generated uninstaller |

You write these paths relative to the project folder.

## Commands that run on the finished build

Once the setup and the embedded uninstaller are written, the builder can hand each file to a command
of the project's own. When the build was asked for `--msi`, the installer package an estate deploys
gets the same treatment. Signing is what this is usually for. The first two settings are NSIS's
`!finalize` and `!uninstfinalize`. The package needs signing too: Group Policy and Intune deploy that
file, and a machine checks its signature.

| Setting | Type | Effect |
| --- | --- | --- |
| `finalize.uninstaller` | string | Command to run on the uninstaller while it is still a file of its own, before the setup embeds it |
| `finalize.installer` | string | Command to run on the setup once it is complete |
| `finalize.package` | string | Command to run on the installer package once it is written; only a build asked for `--msi` writes one |

`%1` in the command stands for the full path of that file:

```json
"finalize": {
  "uninstaller": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\"",
  "installer": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\"",
  "package": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign.ps1 -File \"%1\""
}
```

Each command runs on its own, and every line it prints goes into the build log. A non-zero exit code
stops the build. A refused uninstaller never reaches the setup. A refused setup is removed from disk.
A refused package is removed too, so the next step of a pipeline does not pick it up. Signing
rewrites the file, so the size the build reports is measured on disk after the command.

The builder signs nothing itself and touches no certificate. The certificate, the timestamp service
and the key ring belong to the pipeline. A setting holding nothing but whitespace is refused, or a
project would believe its setup was signed.

## Install behaviour

| Setting | Type | Effect |
| --- | --- | --- |
| `install.default_path` | string | Initial install directory shown on the first page |
| `install.required_space_mb` | integer | Space needed, in MiB. The install stops before writing anything when the destination drive has less free space; an XML `value-source` can show the same number |
| `install.exe_name` | string | The application EXE your application files must contain. If it is missing, the install stops and deploys nothing |
| `install.require_admin` | bool | Ask Windows for administrator rights before the setup starts, defaults to `false` |
| `install.kill_process_on_install` | bool | Close running copies of the product before installing; failure aborts the install |
| `install.kill_process_on_uninstall` | bool | Close running copies of the product before uninstalling |
| `install.detect_running_process` | bool | Combined with the two switches above; any of them closes the process |
| `registry.uninstall_key` | string | Where the uninstall entry in Programs and Features lives, `HKCU` or `HKLM` only; an existing key is never overwritten |
| `links.*` | string | URLs a layout opens through `[label](key)` markup or `action="open_url:key"` |

## Shortcuts and autostart

| Setting | Type | Effect |
| --- | --- | --- |
| `shortcuts.desktop_shortcut` | bool | Allow a desktop shortcut |
| `shortcuts.desktop_default` | bool | Default state of `chkShotcut`, defaults to `true` |
| `shortcuts.start_menu` | bool | Create product and uninstall entries in the Start menu |
| `shortcuts.start_menu_folder` | string | Start menu subfolder, defaults to `project.name`. Only folders this install created and left empty are removed |
| `autostart.enabled` | bool | Allow an autostart entry |
| `autostart.default` | bool | Default state of `chkAutoRun`, defaults to `false` |
| `autostart.registry_key` | string | Autostart key, defaults to `HKCU\...\CurrentVersion\Run` |
| `autostart.registry_value_name` | string | Autostart value name, defaults to `project.name` |

Whether the checkboxes exist is up to your layout. With no `chkShotcut` or `chkAutoRun`, the installer
uses the defaults above. It records both entries while installing, so an uninstall puts the machine
back the way it was.

## Files, languages, and pages

| Setting | Type | Effect |
| --- | --- | --- |
| `resources.layouts_dir` | string | XML directory to package, defaults to `layouts` |
| `resources.assets_dir` | string | Image directory to package, defaults to `assets` |
| `resources.locales_dir` | string | Language JSON directory, defaults to `locales` |
| `resources.payload_file` | string | Required; path to the ZIP or 7z application files |
| `resources.tools_dir` | string | Optional; directory of helper programs to bundle, read back by `get_tools_dir()` |
| `resources.plugins_dir` | string | Optional; directory of plugin DLLs to bundle, called as `plugin_call("dll::function", ...)`, see [Plugin ABI](PLUGIN_API.md). Every DLL in it is checked for shape at build time |
| `localization.default_locale` | string | Language used at startup, defaults to `zh-CN` |
| `localization.supported_locales` | array | Languages you intend to ship. The build warns about an entry with no matching JSON file |
| `wizard.pages[].layout` | string | Install pages. Walk them with `action="next"` and `action="back"`, or let `scripts/pages.rhai` choose the next one |
| `wizard.pages[].role` | string | Optional: `progress` marks the page a task reports on, `finish` the page it ends on |
| `wizard.uninstall_pages[].layout` | string | Uninstall pages, walked the same way |
| `wizard.uninstall_pages[].role` | string | The same two roles for the uninstaller |

Without a `role`, the second page reports and the last one finishes. That is what the three pages an
ordinary project declares mean. A page may also carry `id` and `title`. A page hook addresses a page
by its `id`, which has to be unique inside one list. Two pages answering to one id are refused at
build time. `title` is the heading a report shows it under.

## Interface

| Setting | Type | Effect |
| --- | --- | --- |
| `ui.dpi_aware` | bool | Enable DPI awareness and layout scaling, defaults to `true` |
| `ui.dpi_threshold` | integer | DPI at which `@2x` images are preferred, defaults to `144` |
| `ui.dialog_layout` | string | Layout used for the confirmation dialog, defaults to `layouts/msgBox.xml` |

The builder also writes `ui.dpi_aware` into the setup's application manifest file. Windows then knows
the window scales itself, rather than rescaling a blurry bitmap of it. With it on, the manifest file
asks for per-monitor awareness. Move the window onto a display with a different scaling factor and
the runtime lays it out again for that display, so text and artwork stay sharp. With it off the
manifest file declares `unaware`, and the shell scales the window instead.

`ui.dpi_threshold` decides when the runtime prefers `@2x` artwork. It looks only at the DPI in
effect, not at the display. Move the window to another display and it decides again.

`ui.dialog_layout` names the layout the runtime draws inside the window. Questions such as "exit the
installer?", notices the user has to acknowledge, and what a project script says through
`show_message`, `show_error` and `ask_yes_no` all use it. The dialog then wears the product's own skin
and cannot end up behind the installer. If your project ships no such layout, there is no question at
all: the close button exits immediately, and a script's message falls back to a system message box.
See [page layout](XML_LAYOUT_GUIDE.md#dialogs).

## Administrator rights

`install.require_admin` decides the setup's `requestedExecutionLevel`:

```json
"install": { "require_admin": true }
```

- `true` writes `requireAdministrator`. Windows shows the UAC prompt before the setup starts and the
  window runs elevated. Choose it when the install path is under `Program Files`.
- `false` (the default) writes `asInvoker`: no prompt, and the same rights the user already has.
  Choose it for a per-user install under `%LOCALAPPDATA%`.

The builder generates the embedded uninstaller from the same configuration, so it asks for the same
level. Otherwise the uninstall entry in Programs and Features could not undo an elevated install.

## User data on uninstall

`uninstall.data_paths` lists the folders your product owns. The uninstaller deletes them only when the
user clears the keep-data checkbox. It ignores entries that do not expand to a location under
`%APPDATA%` or `%LOCALAPPDATA%`, so unrelated files are never removed. With no `chkReserveData` in
your layout, user data is always kept.

```json
"uninstall": {
  "data_paths": ["%APPDATA%\\MyApp", "%LOCALAPPDATA%\\MyApp"]
}
```

## Payload format

Your application files are the product's own files, already compressed as ZIP or 7z. The builder
detects the format from the file signature, not the extension: `PK` selects the ZIP runtime and
`37 7A BC AF 27 1C` selects the 7z runtime. Any other format fails the build.

## Components

`resources.payload_file` is the base set of application files every run installs. To let the user pick
part of the product as well, cut it into components with `components.items`:

```json
"components": {
  "items": [
    { "id": "core", "payload": "payload/core.7z", "required": true },
    { "id": "docs", "payload": "payload/docs.7z" },
    { "id": "samples", "payload": "payload/samples.7z", "default": true }
  ]
}
```

| Setting | Type | Effect |
| --- | --- | --- |
| `components.items[].id` | string | Component name. A checkbox of the same id on a page chooses it, and a script asks with `is_component_selected()` |
| `components.items[].payload` | string | The component's own ZIP or 7z archive |
| `components.items[].default` | bool | Whether it installs when the page has no such checkbox, a silent run included; defaults to `false` |
| `components.items[].required` | bool | A required component cannot be cleared on the page and installs either way; defaults to `false` |

Whether a component installs is decided in this order: `required` installs it whatever else says. A
checkbox of its id on the page answers for it, written as in the
[page layout](XML_LAYOUT_GUIDE.md#scrolling-containers). A page without that checkbox lets `default`
answer, a silent run included. One setup can install different things in a window and under
`--silent`, which is what `default` is for.

Components are the parts beside the base application files, so every component's archive has to match
the same format: one runtime unpacks the whole setup, and mixing ZIP with 7z fails the build. At install
time the base set lands first and the components follow into the same directory. Two archives that
carry one relative path fail there and then, rather than overwriting each other in the order the
project declares them. The build also refuses a component without an `id` or a `payload`, one whose
`id` or `payload` repeats another's, one whose `payload` is `resources.payload_file`, and one that
writes `required: true` beside `default: false`, since a required component never reads the default.

## Dependencies

The VC++ runtimes, the WebView2 runtime, a .NET Framework version: these are not part of the product.
They are installed once on the machine and shared by everything on it. `dependencies.items` declares
them: how to tell whether the machine has one, and which program puts it there when it does not.

```json
"dependencies": {
  "items": [
    {
      "id": "vcredist_x64",
      "detect": {
        "registry": {
          "key": "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64",
          "name": "Installed",
          "equals": "1"
        }
      },
      "payload": "payload/vc_redist.x64.exe",
      "arguments": ["/install", "/quiet", "/norestart"],
      "required": true
    },
    {
      "id": "webview2",
      "detect": {
        "file": "%ProgramFiles(x86)%\\Microsoft\\EdgeWebView\\Application\\msedgewebview2.exe"
      },
      "download": {
        "url": "https://go.microsoft.com/fwlink/?linkid=2124703",
        "sha256": "e5f5a4b0b1b7c0d4a4f0d2a0f9c1e8b6d3a7c2f4b8e6d1a3c5f7b9d0e2a4c6f8"
      },
      "arguments": ["/silent", "/install"]
    }
  ]
}
```

| Setting | Type | Effect |
| --- | --- | --- |
| `dependencies.items[].id` | string | Dependency name. A script asks with `dependency_installed()` and `install_dependency()` |
| `dependencies.items[].detect` | object | How the machine is asked. `file` or `registry`, one of the two |
| `dependencies.items[].payload` | string | The installer shipped inside the setup; it has to be an `.exe` |
| `dependencies.items[].download` | object | The installer fetched at install time, described below |
| `dependencies.items[].arguments` | array | What the installer is run with; nothing by default |
| `dependencies.items[].required` | bool | A required dependency stops the install when it cannot be installed; defaults to `false` |

`detect` is written one of two ways, and one rule answers one question:

- `{ "file": "%ProgramFiles(x86)%\\...\\msedgewebview2.exe" }`: the machine has it when this file
  is there. Environment variables in the path are expanded first.
- `{ "registry": { ... } }`: read the registry. `key` is the key to open, under `HKCU` or `HKLM`. A
  key may also name a view: `HKLM32` is the copy of `HKLM` a 32-bit program sees, `HKLM64` the copy a
  64-bit program reads. That is how a rule asks after a runtime installed for the other width. `name`
  is the value to read; without one, the rule only asks whether the key exists. With a `name`, the
  value can also be compared: `equals` is an exact match, `at_least` compares the dot-separated
  numbers. The value is read as text whether the machine stored text or a dword, so the `1` the VC++
  runtimes write, the version the WebView2 runtime writes and the number .NET Framework records are
  all comparable. A missing part in `at_least` counts as zero, which makes `14.0.1` and `14.0.1.0`
  the same version.

`payload` and `download` are alternatives. A bundled installer is collected into the setup by the
build, and unpacked to a temporary directory before it runs. A download is fetched while the setup
runs. It is checked against `sha256` before anything is executed. A file that does not match is
deleted rather than run, which is why `sha256` is required. Compute it with something like
`certutil -hashfile <file> SHA256`. A URL whose last segment is an executable keeps that name.
Anything else is named after the dependency's id with an `.exe` suffix, because Windows only runs what
it recognizes as a program.

Dependencies are handled before the application files. The built-in flow checks each one, installs the
ones that are missing, and unpacks only once they are all in place. A dependency that cannot be
installed stops the install when the project marks it `required`, and says why; nothing of the product
has been written at that point. One that is not required is noted in the log and the run carries on.
An installer that reports `1638`, meaning a newer version is already installed, or `3010`/`1641`,
meaning it worked and Windows wants a restart, counts as success. Any other non-zero exit code is a
failure. The status text the built-in flow publishes comes from the locale key
`status.dependencies`.

A dependency is not uninstalled with the product. It belongs to the machine, and other products use
it too.

A project that ships `scripts/install.rhai` keeps the built-in flow out of the way. The script decides
when to check and whether to install, through `dependency_installed()` and `install_dependency()`,
which read the same declaration. See the [script API](SCRIPT_API.md#dependencies-and-downloads).

## Custom install and uninstall steps

Add `scripts/install.rhai` or `scripts/uninstall.rhai` to replace the built-in steps. See the
[script API](SCRIPT_API.md). A `scripts/pages.rhai` replaces no step: it decides which page follows
each Next, see [page hooks](SCRIPT_API.md#page-hooks).

## Unattended runs

You can run your install or uninstall with no window at all, which is what a software deployment tool
needs. Your project has to say so first:

```json
"advanced": {
  "silent_mode_support": true,
  "uninstall_mode_support": true
}
```

With the switches on, the same executables accept `--silent`:

```powershell
MyApp_Setup.exe --silent --dir "%LOCALAPPDATA%\MyApp"
MyApp_Setup.exe --silent --log "%TEMP%\MyApp-setup.log"
MyApp_Setup.exe --silent
uninst.exe --silent
```

- `--silent` installs or uninstalls without opening anything. A silent run never shows a dialog,
  because a dialog would wait for a click that never comes.
- `--dir` chooses the install directory for that run and wins over `install.default_path`. Both
  accept environment variables, expanded before use.
- `--log <file>` chooses the file this run logs to. Without it the log lands in the `nano-installer`
  directory of the temporary directory, named after the setup image, the moment, and which task this
  is. It holds the machine, the product and the directory, every step the run took, and every line a
  project script wrote. A run that fails writes the log's whole path to standard error, which is the
  path an administrator collects the file from. Unattended deployments usually point it at a place
  they gather logs from anyway.
- The setup refuses any other option, so a mistyped flag stops the run instead of installing into the
  configured default.
- `silent_mode_support` and `uninstall_mode_support` are separate, so a product can allow unattended
  installs without allowing unattended removal.
- Because there are no checkboxes to read, the installer follows `shortcuts.desktop_default` and
  `autostart.default`. User data is always kept on uninstall.

A silent run reports progress to whoever started it: it writes to the console instead of painting
a window. A failure sets a non-zero exit code, with the reason on standard error.

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

A setting that is accepted and then ignored is worse than one that is missing: the project reads as if
it worked, and the setup ships without it. The build refuses a configuration holding a key it does not
read, and names the setting that does the job instead.

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
