<h1 align="center"><img src="../assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="../zh-CN/README.md">简体中文</a> | <a href="../README.md">All docs</a></p>

Hand someone a single `.exe` and they have your product installed. Nano Installer turns a folder of
configuration, artwork, and your packaged application into one Windows setup file with your own
logo, your own interface, and your own wording — no installer framework to host, and nothing for
your users to install first.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM.

## What you get

| | |
| --- | --- |
| One file to ship | A single setup `.exe` carrying your icon, version info, and branding |
| Nothing to install first | Runs on a clean Windows 7 SP1 x64 machine or later |
| Your interface, not ours | Pages and controls described in XML, using your own backgrounds and buttons |
| Every language you need | Eleven UI languages included; add your own in plain JSON |
| Upgrades that behave | Re-running the setup upgrades in place, and rolls back if anything fails |
| Clean removal | Uninstalling takes back exactly what it put down, and keeps user data by default |
| Rights that fit | Ask for administrator rights only when your product needs them, straight from the config |
| Fits your process | Click through the visual builder, or drive the same engine from a command line in CI |

## Start

```powershell
.\scripts\build.ps1
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

Then read the [quick start](QUICK_START.md), or drive the same engine from the
[visual builder](GUI.md).

## Guides

- [Quick start](QUICK_START.md) - build your first setup, then try it in a VM
- [Visual builder](GUI.md) - the Windows 10+ authoring tool
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
