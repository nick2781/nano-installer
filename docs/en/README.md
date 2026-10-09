<h1 align="center"><img src="https://nick2781.github.io/nano-installer/assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a></p>

You ship one `.exe`, and a double-click installs your product. Keep the configuration, the artwork and
your application files in one folder, and Nano Installer turns it into one Windows setup; the icon,
the pages and the wording are yours, and the user's machine needs nothing installed first.

> **Status:** ready to build and ship with. Setups and MSI packages are unsigned, so Windows warns about
> an unknown publisher; sign them in your own pipeline before distributing publicly. **Requires Windows 7
> SP1 x64 or later.** What a setup calls is in [Windows compatibility](WINDOWS_COMPATIBILITY.md);
> item-by-item status is in [production status](PRODUCTION_STATUS.md).

<p align="center">
<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-en-US.png" alt="The first page of a setup built from the TapTap example" width="640">
</p>

That is the first page of `examples/TapTap`, built and photographed by
`scripts/capture_setup_snapshots.ps1`; this is how the runtime drew it, not a mock-up.

## What you get

| | |
| --- | --- |
| One file to ship | A single setup `.exe` with your icon, version info and branding |
| A clean machine | No runtime to install and no framework: the setup carries what it needs |
| Your pages | XML layouts with your own backgrounds and buttons |
| Eleven UI languages | Eleven built in, and one JSON file adds another |
| Progress and finish pages | The progress page names the current step, and the finish page can start what it installed |
| Upgrade, rollback, update packages | Re-run to upgrade in place, and a failed step rolls back; `--delta-from` ships only what changed |
| Uninstall | Takes back what it wrote, and user data stays by default |
| Administrator rights | Asked of Windows only when the project says so |
| Scripts and plugins | Install and uninstall steps in Rhai, and plugins as DLLs built against [`include/nano_plugin.h`](PLUGIN_API.md) |
| Screen readers | The page, the focus, typed text and live progress are announced, over MSAA as well as `IDispatch` |
| GUI or command line | The Windows 10+ visual builder, or the same engine driven from CI |
| Also a package | `--msi` wraps the setup for an estate that deploys through Windows Installer |

## Start

Take the builder from the [latest release](https://github.com/nick2781/nano-installer/releases/latest),
and the three runtimes as well, because the builder looks for them beside itself. Then point it at a
project folder:

```powershell
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

Then read the [quick start](QUICK_START.md), make your first setup, and try it in a VM; or drive the
same engine from the [visual builder](GUI.md). Building the tools from source is for contributors; see
[build and release](BUILD_AND_RELEASE.md).

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
