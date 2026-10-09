<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="README.zh-CN.md">简体中文</a> | <a href="docs/en/">Docs</a></p>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a></p>

One folder in, one setup `.exe` out. Put your configuration, artwork and application files in a
project folder. Nano Installer builds them into one Windows installer. The logo, the pages and the
wording are yours. Nothing to host, and nothing your users install first.

> **Early implementation, not ready for production distribution.** Test in a disposable VM: an
> install writes files and registry entries. Setups are unsigned, so Windows warns about an unknown
> publisher. **Requires Windows 7 SP1 x64 or later.** See
> [production status](docs/en/PRODUCTION_STATUS.md).

<img src="assets/setup-welcome-en-US.png" alt="First page of a setup built from the TapTap example: the product logo, a tagline, the installation options, and an Install Now button" width="720">

The first page of `examples/TapTap`, built and photographed by `scripts/capture_setup_snapshots.ps1`.
The runtime drew this, and it is not a mock-up.

## Build your first setup

Nothing to install, nothing to compile. Every
[release](https://github.com/nick2781/nano-installer/releases/latest) publishes the builder, the
runtimes it needs and a digest for each file.

```powershell
# 1. Download the release files into one folder
# 2. Point the builder at a project folder
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

The setup lands in `dist/<installer_name>`. Prefer clicking? `nano-installer-gui-x64.exe` from the
same release runs the same engine. Full walkthrough:
[quick start](docs/en/QUICK_START.md).

## What you get

| | |
| --- | --- |
| One file to ship | One setup `.exe`, with your icon, version info and branding inside |
| A clean machine | No runtime to install, and no framework. The setup carries what it needs |
| Your pages | XML layouts, with your own backgrounds and buttons |
| Eleven UI languages | Eleven built in. One JSON file adds another |
| Progress and finish pages | The progress page names the current step. The finish page can start what it installed |
| Upgrade, rollback, update packages | Re-run to upgrade in place. A failed step rolls back. `--delta-from` ships only what changed |
| Uninstall | Takes back what it wrote. Keeps user data by default |
| Administrator rights | Asked of Windows only when the project says so |
| Scripts and plugins | Install and uninstall steps in Rhai, plus DLLs built against [`include/nano_plugin.h`](docs/en/PLUGIN_API.md) |
| Screen readers | The page, the focus, typed text and live progress are announced, over MSAA and `IDispatch` |
| GUI or command line | The Windows 10+ visual builder, or the same engine driven from CI |
| Also a package | `--msi` wraps the setup for an estate that deploys through Windows Installer |

## A project folder

```
MyApp/
  installer_config.json     product name, version, install path, shortcuts, output name
  layouts/                  XML pages: welcome, progress, finish, uninstall
  assets/                   backgrounds, buttons, icons (1x and @2x variants)
  locales/                  one JSON file per language
  scripts/                  optional install and uninstall logic
  payload/app.7z            your application files, zipped or 7z-compressed
```

Paths are relative to the folder, so a project moves anywhere. The quickest start is to copy
[`examples/TapTap`](examples/README.md) and replace what is inside. Copy the shape of the
configuration, not the artwork, which belongs to its rights holders.

## Documentation

[Quick start](docs/en/QUICK_START.md) &middot; [Configuration](docs/en/CONFIG_REFERENCE.md) &middot;
[Page layout](docs/en/XML_LAYOUT_GUIDE.md) &middot; [Languages](docs/en/LOCALIZATION.md) &middot;
[Custom steps](docs/en/SCRIPT_API.md) &middot; [Plugins](docs/en/PLUGIN_API.md) &middot;
[Visual builder](docs/en/GUI.md) &middot; [Migrating from NSIS](docs/en/MIGRATION_FROM_NSIS.md)

Site: **https://nick2781.github.io/nano-installer/**, and Chinese pages live in
[`docs/zh-CN`](docs/zh-CN/). Every page is also a Markdown file:
[`docs/llms.txt`](docs/llms.txt) indexes them for an agent, and
[`docs/llms-full.txt`](docs/llms-full.txt) is the English documentation in one file. Maintainer
pages are in [`docs/en`](docs/en/): architecture, build and release, Windows compatibility, the test
plan, the production status.

## Contributing, security, licence

[CONTRIBUTING.md](CONTRIBUTING.md) has the working tree and the checks a change passes. It also has
the rules this repository keeps, including that work reaches `main` through a pull request.
[Issue forms](.github/ISSUE_TEMPLATE) ask for the version and the run log, because a report without
them costs a round trip. Send security problems through
[private vulnerability reporting](SECURITY.md), not an issue; that page also lists what is knowingly
not a vulnerability here. Everyone here is held to [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). The Rust
source is [MIT licensed](LICENSE). The trademarks, images and copy of `examples/TapTap` belong to
易玩（上海）网络科技有限公司 and are not MIT-licensed; the screenshots in `assets/` are captures of
that example.
