<h1 align="center"><img src="https://nick2781.github.io/nano-installer/assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a></p>

Hand someone a single `.exe` and your product is installed. You keep the configuration, the artwork
and your application files in one folder. Nano Installer turns that folder into one Windows setup
file with your logo, your pages and your wording, and your users install nothing first.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM; neither a setup nor the
> installer package around it is code-signed. **Requires Windows 7 SP1 x64 or later**, and what a
> setup calls there is in [Windows compatibility](WINDOWS_COMPATIBILITY.md).
> See [production status](PRODUCTION_STATUS.md).

<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-en-US.png" alt="The first page of a setup built from the TapTap example" width="640">

That is the first page of `examples/TapTap`, built and photographed by
`scripts/capture_setup_snapshots.ps1`: the page as the runtime drew it, not a mock-up.

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
| Scripts and plugins | Rhai install and uninstall steps, plus DLLs built against [`include/nano_plugin.h`](PLUGIN_API.md) |
| Screen readers | The page, the focus, typed text and live progress are announced, over MSAA and `IDispatch` |
| GUI or command line | The Windows 10+ visual builder, or the same engine driven from CI |
| Also a package | `--msi` wraps the setup for an estate that deploys through Windows Installer |

## Start

Take the builder, and the three runtimes it looks for beside itself, from the [latest
release](https://github.com/nick2781/nano-installer/releases/latest), then point it at a project
folder:

```powershell
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

Then read the [quick start](QUICK_START.md) to make your first setup and try it in a VM, or drive the
same engine from the [visual builder](GUI.md). Building the tools from source is for contributors, and
[build and release](BUILD_AND_RELEASE.md) covers it.

## Guides

- [Quick start](QUICK_START.md) - build your first setup, then try it in a VM
- [Visual builder](GUI.md) - the Windows 10+ app you build setups with
- [Configuration](CONFIG_REFERENCE.md) - product identity, install behaviour, output names
- [Page layout](XML_LAYOUT_GUIDE.md) - pages, controls, flow layout, links, actions
- [Languages](LOCALIZATION.md) - shipping translated installers
- [Custom steps](SCRIPT_API.md) - install/uninstall logic in Rhai
- [Plugin ABI](PLUGIN_API.md) - the header a third-party DLL is built against, and what the host lends it
- [Migrating from NSIS](MIGRATION_FROM_NSIS.md) - what maps onto what, and what is refused on purpose
- [Production status](PRODUCTION_STATUS.md) - what works today and what blocks a release

## Technical notes

- [Architecture](ARCHITECTURE.md)
- [Project layout](PROJECT_STRUCTURE.md)
- [Windows compatibility](WINDOWS_COMPATIBILITY.md)
- [Build and release](BUILD_AND_RELEASE.md)
- [Test plan](TEST_PLAN.md)

---

Read this in Chinese: [中文文档](../zh-CN/README.md).
