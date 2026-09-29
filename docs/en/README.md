<h1 align="center"><img src="https://nick2781.github.io/nano-installer/assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI status"></a></p>

Hand someone a single `.exe` and your product is installed. You keep the configuration, the artwork
and your application files in one folder. Nano Installer turns that folder into one Windows setup
file with your logo, your pages and your wording, and your users install nothing first.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM; neither a setup nor the
> installer package around it is code-signed, and Windows 7 support has not been accepted on a
> real machine yet. See [production status](PRODUCTION_STATUS.md).

<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-en-US.png" alt="The first page of a setup built from the TapTap example" width="640">

That is the first page of `examples/TapTap`, captured at 192 dpi by
`scripts/capture_setup_snapshots.ps1`, which photographs every page the example declares and
checks each one against the project's own layout. The file is 1440x900 for a page laid out as
720x450: two pixels per layout pixel, so it stays sharp on a display that scales, and it is the
page as the runtime drew it rather than a mock-up.

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
| Update packages | `--delta-from` ships only the files whose bytes changed, and the setup checks what is already on the machine before it writes |
| Screen readers | A reader in another process is told the page, the focus, the value of a field as it is typed, the words a running task publishes, and the card a question is drawn on |
| Plugins | A third party extends a setup with a DLL built against [`include/nano_plugin.h`](PLUGIN_API.md), called from the project's script |
| GUI, or a command line | Click through the visual builder, or drive the same engine from CI |

## Start

```powershell
.\scripts\build.ps1
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

Then read the [quick start](QUICK_START.md) to build your first setup and try it in a VM, or drive the
same engine from the [visual builder](GUI.md).

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
