<h1 align="center"><img src="../assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>English</b> | <a href="../zh-CN/README.md">简体中文</a></p>

Nano Installer turns a folder of configuration, images, and a compressed payload into a single
Windows setup executable. No installer framework to host, and no runtime to install on the target
machine: the generated `.exe` carries its own UI, unpacking engine, and uninstaller.

> **Status:** early implementation, not ready for production distribution. Install actions write
> files and registry entries, so test only inside a disposable VM.

## What you get

| | |
| --- | --- |
| One self-contained setup | A single `.exe` with your icon, version info, and branding |
| Nothing to install on the target | Windows 7 SP1 x64 or later, no prerequisites |
| Your own interface | XML pages and controls, your background and button images |
| 11 languages out of the box | Ship translated text, or add your own locale files |
| Safe upgrades, clean removal | Re-installing upgrades in place with rollback; removal takes only what it installed |
| Automation when you need it | The visual builder for everyday use, a CLI for CI |

## Start

```powershell
.\scripts\build.ps1
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

Then read the [quick start](QUICK_START.md), or drive the same engine from the
[visual builder](GUI.md).

## Guides

- [Quick start](QUICK_START.md) - build your first setup
- [Visual builder](GUI.md) - the Windows 10+ authoring tool
- [Configuration](CONFIG_REFERENCE.md) - every setting that is in effect today
- [Page layout](XML_LAYOUT_GUIDE.md) - pages, controls, flow layout, actions
- [Languages](LOCALIZATION.md) - shipping translated installers
- [Custom steps](SCRIPT_API.md) - install/uninstall logic in Rhai
- [Production status](PRODUCTION_STATUS.md) - what works, what blocks a release

## Technical notes

- [Architecture](ARCHITECTURE.md)
- [Project layout](PROJECT_STRUCTURE.md)
- [Windows compatibility](WINDOWS_COMPATIBILITY.md)
- [Build and release](BUILD_AND_RELEASE.md)
- [Test plan](TEST_PLAN.md)
