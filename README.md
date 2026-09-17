<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="README.zh-CN.md">简体中文</a> | <a href="docs/en/">Docs</a></p>

Hand someone a single `.exe` and your product is installed. You keep the configuration, the artwork
and your application files in one folder; Nano Installer turns that folder into one Windows setup
file with your logo, your pages and your wording. There is no framework to host, and nothing your
users install first.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM. See
> [production status](docs/en/PRODUCTION_STATUS.md) before you plan a release.

## What you get

| | |
| --- | --- |
| One file to ship | A single setup `.exe` carrying your icon, version info, and branding |
| Runs on a clean machine | Windows 7 SP1 x64 or later, with nothing to install first |
| Your pages and controls | Described in XML, using your own backgrounds and buttons |
| Eleven UI languages | Built in, and you can add more in plain JSON |
| Upgrades and rollback | Re-running the setup upgrades in place, and returns to the previous state if a step fails |
| Uninstall | Takes back what it put down, and keeps user data by default |
| Administrator rights | Requested from Windows when the config asks for them |
| Progress and finish pages | Live progress names the current step, and the finish page can start what it installed |
| GUI, or a command line | Click through the visual builder, or drive the same engine from CI |

## Get started

You need Windows x64 with the MSVC build tools, plus the Rust toolchain pinned in this repository.

```powershell
# 1. Build the tools (once per checkout)
.\scripts\build.ps1

# 2. Build a setup from a project folder
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

Your setup lands in `dist/<installer_name>` inside the project. If you would rather click than type,
start `target/release/nano-installer-gui-x64.exe` and pick the project folder there.

Full walkthrough: [Quick start](docs/en/QUICK_START.md).

## A project folder

```
MyApp/
  installer_config.json     product name, version, install path, shortcuts, output name
  layouts/                  XML pages: welcome, progress, finish, uninstall
  assets/                   backgrounds, buttons, icons (1x and @2x variants)
  locales/                  one JSON file per language
  scripts/                  optional install/uninstall logic
  payload/app.7z            your application files, zipped or 7z-compressed
```

Paths in the configuration are relative to the project folder, so you can move a project anywhere. The
quickest start is to copy `examples/TapTap` and replace what is inside.

## What your setup does today

- One executable installs and uninstalls, on Windows 7 SP1 x64 and later.
- Hand it a ZIP or a 7z payload. It reads the file itself and picks the matching runtime, so you never
  set a format option.
- Your users watch live progress with the current step named, and the finish page can launch what it
  installed.
- Run the same setup again and it upgrades in place. If a step fails, the machine goes back to the
  version it had.
- Desktop, Start menu and autostart entries are created only when the user asks for them, and cleaned
  up again on uninstall.
- Users keep their data on uninstall unless they clear that option themselves.
- Eleven UI languages switch instantly, and a page can link to your terms of service or privacy
  policy.
- Users pick the install directory with the standard Windows folder chooser, or type and edit the
  path: mouse or double-click selection, copy and paste, undo.
- East Asian text goes in through a proper IME composition window, with the candidate list under the
  caret.
- When a project sets `install.require_admin`, the setup asks Windows for administrator rights, so
  nobody has to right-click and choose Run as administrator.
- The setup asks a localized question before it closes the window.
- The installation folder disappears as soon as an uninstall finishes. If the user put files there,
  it stays.
- Colours, rounded corners and outlines are drawn as your layout asks for them, and the pointer turns
  into a hand over anything clickable.
- When the built-in steps are not enough, a small script file can drive install and uninstall. A
  failing script rolls back and never leaves a half-installed product.
- The visual builder for Windows 10 and later produces byte-for-byte the same result as the command
  line.

## Not there yet

- Setups are not code-signed, so Windows SmartScreen warns about an unknown publisher.
- Windows 7 support is checked automatically against the system calls each build uses, but has not
  yet been signed off on a real Windows 7 SP1 machine.

## Documentation

Product guides: [Quick start](docs/en/QUICK_START.md) &middot;
[Visual builder](docs/en/GUI.md) &middot;
[Configuration](docs/en/CONFIG_REFERENCE.md) &middot;
[Page layout](docs/en/XML_LAYOUT_GUIDE.md) &middot;
[Languages](docs/en/LOCALIZATION.md) &middot;
[Custom steps](docs/en/SCRIPT_API.md)

Technical notes: [Architecture](docs/en/ARCHITECTURE.md) &middot;
[Build and release](docs/en/BUILD_AND_RELEASE.md) &middot;
[Windows compatibility](docs/en/WINDOWS_COMPATIBILITY.md) &middot;
[Test plan](docs/en/TEST_PLAN.md) &middot;
[Production status](docs/en/PRODUCTION_STATUS.md) &middot;
[Project layout](docs/en/PROJECT_STRUCTURE.md)

Chinese documentation lives in [docs/zh-CN](docs/zh-CN/).

## Rights and license

- The Rust source in this repository is licensed under the [MIT License](LICENSE).
- `examples/TapTap` is a validation project. The TapTap trademarks, images, and copy in it belong
  to **易玩（上海）网络科技有限公司** and their respective rights holders; they are not
  MIT-licensed.
- Setups you generate carry the resources you configure for your own product, under your own
  licensing.
