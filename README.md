<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="README.zh-CN.md">简体中文</a> | <a href="docs/en/">Docs</a></p>

Nano Installer turns a folder of configuration, images, and a compressed payload into a single
Windows setup executable. No installer framework to host, no runtime to install on the target
machine: the generated `.exe` carries its own UI, unpacking engine, and uninstaller.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM. See
> [production status](docs/en/PRODUCTION_STATUS.md) before you plan a release.

## What you get

| | |
| --- | --- |
| One self-contained setup | A single signed-ready `.exe` with your icon, version info, and branding |
| No prerequisites on the target | Windows 7 SP1 x64 or later; nothing else to install |
| Your own interface | Arrange pages and controls with XML, using your own background and button images |
| 11 languages out of the box | Ship translated UI text, or add your own locale files |
| Safe upgrades and clean removal | Re-installing upgrades in place with rollback; uninstalling removes only what it installed |
| Automation when you need it | The optional GUI covers everyday use; a CLI drives builds in CI |

## Get started

You need Windows x64 with the MSVC build tools, plus the Rust toolchain pinned in this repository.

```powershell
# 1. Build the tools (once per checkout)
.\scripts\build.ps1

# 2. Build a setup from a project folder
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

The setup lands in `dist/<installer_name>` inside your project. Prefer clicking to typing? Launch
`target/release/nano-installer-gui-x64.exe` and pick the project folder there.

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

Every path in the configuration is relative to the project folder, so a project stays portable.
Start by copying `examples/TapTap` and replacing its content.

## What is supported today

- Setup and uninstaller in a single executable, targeting Windows 7 SP1 x64 and later.
- ZIP and 7z payloads, detected automatically from the file signature.
- Install, upgrade, rollback, and uninstall, with progress pages and a finish page that can launch
  the installed application.
- Desktop shortcuts, Start menu entries, and autostart, each optional at install time and cleaned
  up on removal.
- User data kept by default on uninstall, unless the user clears the keep-data option.
- Localized UI text for 11 languages, switchable at runtime.
- Custom install and uninstall steps written in [Rhai](docs/en/SCRIPT_API.md) when the built-in
  steps are not enough.
- A visual builder for Windows 10 and later that shares the same build engine as the CLI.

## Not there yet

- Installers are not code-signed; Windows SmartScreen will warn about an unknown publisher.
- No automatic elevation prompt. An installer writing to `Program Files` must be started as an
  administrator.
- Windows 7 support is verified by static import checks, not yet by a full run on a real
  Windows 7 SP1 machine.

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
