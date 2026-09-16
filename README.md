<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="README.zh-CN.md">简体中文</a> | <a href="docs/en/">Docs</a></p>

Hand someone a single `.exe` and they have your product installed. Nano Installer turns a folder of
configuration, artwork, and your packaged application into one Windows setup file with your own
logo, your own interface, and your own wording — no installer framework to host, and nothing for
your users to install first.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM. See
> [production status](docs/en/PRODUCTION_STATUS.md) before you plan a release.

## What you get

| | |
| --- | --- |
| One file to ship | A single setup `.exe` carrying your icon, version info, and branding |
| Nothing to install first | Runs on a clean Windows 7 SP1 x64 machine or later |
| Your interface, not ours | Pages and controls described in XML, using your own backgrounds and buttons |
| Every language you need | Eleven UI languages included; add your own in plain JSON |
| Upgrades that behave | Re-running the setup upgrades in place, and rolls back if anything fails |
| Clean removal | Uninstalling takes back exactly what it put down, and keeps user data by default |
| Clear, friendly screens | Live progress with the current step named, plus a finish page that can start your app |
| Fits your process | Click through the visual builder, or drive the same engine from a command line in CI |

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

## What your setup does today

- Installs and uninstalls from one executable, on Windows 7 SP1 x64 and later.
- Accepts a ZIP or 7z payload and detects which one by looking at the file itself, so you never
  pick a format setting.
- Shows live progress with the current step, then a finish page that can launch what it installed.
- Upgrades in place when run again, and returns the machine to its previous state if a step fails.
- Creates desktop, Start menu, and autostart entries only when the user asks for them, and removes
  them again on uninstall.
- Lets the user keep their data on uninstall unless they explicitly clear that option.
- Ships eleven UI languages that switch instantly, and lets a page click through to your terms of
  service or privacy policy.
- Lets you pick the install directory with the standard Windows folder chooser, and lets the user
  edit the path by hand: select with the mouse or a double click, copy and paste, and undo a typo.
- Asks a localized question before closing, so nobody loses an install by accident.
- Takes back the installation folder as soon as an uninstall finishes, so no empty directory is
  left behind; files your users added there keep the folder, as they should.
- Paints the colours, rounded corners, and outlines your layout asks for, and turns the pointer
  into a hand over anything clickable.
- Supports custom install and uninstall steps from a small scripting file when the built-in steps
  are not enough; a failing script rolls back and never leaves a half-installed product.
- Offers a visual builder for Windows 10 and later that produces byte-for-byte the same result as
  the command line.

## Not there yet

- Setups are not code-signed, so Windows SmartScreen warns about an unknown publisher.
- Setup files do not ask for administrator rights by themselves. One that writes to
  `Program Files` has to be started as an administrator.
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
