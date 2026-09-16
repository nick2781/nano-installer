# Quick start

This walkthrough builds one installer from a sample project and runs it in a virtual machine.

## 1. Prepare your machine

- Windows x64 with MSVC and the Windows SDK (Visual Studio 2022 Build Tools)
- Rust with the toolchain pinned in this repository, including the `rust-src` component

```powershell
rustup toolchain install nightly-2025-11-08 --component rust-src,rustfmt,clippy
```

## 2. Build the tools

From the repository root:

```powershell
.\scripts\build.ps1
```

The script builds the builder, the two runtimes, and the uninstaller runtime, checks that real ZIP
and 7z archives unpack correctly, and audits every produced binary against the Windows 7 baseline.
The result is `target/release/`.

## 3. Build a sample setup

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

The example writes its own payload, so it produces `examples/TapTap/dist/TapTap_Setup.exe`. The
payload `examples/TapTap/payload/app.7z` is not stored in the repository; put a 7z archive there
before building.

Prefer a visual workflow? Run `target/release/nano-installer-gui-x64.exe`, open the project
folder, and press **Build setup**. The result is identical because the GUI drives the same build
engine. See the [visual builder](GUI.md).

## 4. Try the setup in a VM

Copy the setup into a fresh virtual machine. The sample's default install path is under
`Program Files`, so start it as an administrator; this version does not request elevation on its
own.

What you can check today:

- Background, logo, tagline, and button images render with correct transparency.
- The window has no system title bar, can be dragged from an empty area, and minimizes and closes.
- The install button extracts the payload, writes files and shortcuts, and registers an uninstall
  entry. Progress is reported while it runs, and the finish page can launch the installed program.
- Install a second time to see the upgrade path, and use the uninstall entry to check removal and
  the keep-data option.
- Switch the language to confirm translated text renders correctly.

To test a different install path, change `install.default_path` in
`examples/TapTap/installer_config.json` and rebuild the setup. The path shown on the first page is
read-only in this version, so clicking the folder image does not open a picker yet.

Do not run the sample's install action on your workstation: it writes files and registry entries.
