<h1 align="center"><img src="https://nick2781.github.io/nano-installer/assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="https://nick2781.github.io/nano-installer/#/zh-CN/README.md">简体中文</a> | <a href="https://nick2781.github.io/nano-installer/">All docs</a></p>

Hand someone a single `.exe` and your product is installed. You keep the configuration, the artwork
and your application files in one folder, and Nano Installer turns it into one Windows setup file
with your logo, your pages and your wording. Your users install nothing first.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM.

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
- [Production status](PRODUCTION_STATUS.md) - what works today and what blocks a release

## Technical notes

- [Architecture](ARCHITECTURE.md)
- [Project layout](PROJECT_STRUCTURE.md)
- [Windows compatibility](WINDOWS_COMPATIBILITY.md)
- [Build and release](BUILD_AND_RELEASE.md)
- [Test plan](TEST_PLAN.md)
