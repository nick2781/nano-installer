<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="README.zh-CN.md">简体中文</a> | <a href="docs/en/">Docs</a></p>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a></p>

One folder in, one setup `.exe` out. Keep your configuration, artwork and application files in a
project folder; Nano Installer builds them into a single Windows installer carrying your logo, your
pages and your wording. Nothing to host, and nothing your users install first.

> **Early implementation, not ready for production distribution.** Test in a disposable VM: an
> install writes files and registry entries. Setups are unsigned, so Windows warns about an unknown
> publisher. **Requires Windows 7 SP1 x64 or later.** See
> [production status](docs/en/PRODUCTION_STATUS.md).

<img src="assets/setup-welcome-en-US.png" alt="First page of a setup built from the TapTap example: the product logo, a tagline, the installation options, and an Install Now button" width="720">

The first page of `examples/TapTap`, built and photographed by `scripts/capture_setup_snapshots.ps1`:
what the runtime drew, not a mock-up.

## Build your first setup

Nothing to install and nothing to compile — every
[release](https://github.com/nick2781/nano-installer/releases/latest) publishes the builder, the
runtimes it needs and a digest for each file.

```powershell
# 1. Download the release files into one folder
# 2. Point the builder at a project folder
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

The setup lands in `dist/<installer_name>`. Prefer clicking? `nano-installer-gui-x64.exe` from the
same release drives the same engine. Full walkthrough:
[quick start](docs/en/QUICK_START.md).

## What you get

| | |
| --- | --- |
| One file to ship | A single setup `.exe` with your icon, version info and branding |
| A clean machine | No runtime and no framework: the setup carries what it needs |
| Your pages | XML layouts drawn with your own backgrounds and buttons |
| Eleven UI languages | Built in, and one JSON file adds another |
| Progress and finish pages | The current step is named live, and the finish page can start what it installed |
| Upgrade, rollback, update packages | Re-running upgrades in place, a failed step rolls back, and `--delta-from` ships only what changed |
| Uninstall | Takes back what it wrote, and keeps user data by default |
| Administrator rights | Asked of Windows only when the project says so |
| Scripts and plugins | Rhai install and uninstall steps, plus DLLs built against [`include/nano_plugin.h`](docs/en/PLUGIN_API.md) |
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
[`examples/TapTap`](examples/README.md) and replace what is inside — the shape of the configuration,
not the artwork, which belongs to its rights holders.

## Documentation

[Quick start](docs/en/QUICK_START.md) &middot; [Configuration](docs/en/CONFIG_REFERENCE.md) &middot;
[Page layout](docs/en/XML_LAYOUT_GUIDE.md) &middot; [Languages](docs/en/LOCALIZATION.md) &middot;
[Custom steps](docs/en/SCRIPT_API.md) &middot; [Plugins](docs/en/PLUGIN_API.md) &middot;
[Visual builder](docs/en/GUI.md) &middot; [Migrating from NSIS](docs/en/MIGRATION_FROM_NSIS.md)

Site: **https://nick2781.github.io/nano-installer/**, and Chinese pages live in
[`docs/zh-CN`](docs/zh-CN/). Every page is also a Markdown file, so
[`docs/llms.txt`](docs/llms.txt) indexes them for an agent and
[`docs/llms-full.txt`](docs/llms-full.txt) is the English documentation in one file. Architecture,
build and release, Windows compatibility, the test plan and the production status are in
[`docs/en`](docs/en/) for maintainers.

## Contributing, security, licence

[CONTRIBUTING.md](CONTRIBUTING.md) has the working tree, the checks a change passes and the rules this
repository keeps, including that work reaches `main` through a pull request.
[Issue forms](.github/ISSUE_TEMPLATE) ask for the version and the run log, because a report without
them costs a round trip. Security problems go through
[private vulnerability reporting](SECURITY.md), not an issue; that page also lists what is knowingly
not a vulnerability here. Everyone in this repository is held to
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). The Rust source is [MIT licensed](LICENSE), while the
trademarks, images and copy of `examples/TapTap` belong to 易玩（上海）网络科技有限公司 and are not
MIT-licensed — the screenshots in `assets/` are captures of that example.
