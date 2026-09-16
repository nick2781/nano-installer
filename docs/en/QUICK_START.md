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

Copy the setup into a fresh virtual machine. The sample sets `install.require_admin`, and its
default install path is under `Program Files`, so Windows asks for consent the moment you launch
it. Approve the prompt and the setup runs with the rights it needs.

What you can check today:

- Background, logo, tagline, and button images render with correct transparency.
- The window has no system title bar, can be dragged from an empty area, and minimizes and closes.
- The install button extracts the payload, writes files and shortcuts, and registers an uninstall
  entry. Progress is reported while it runs, and the finish page can launch the installed program.
- Install a second time to see the upgrade path, and use the uninstall entry to check removal and
  the keep-data option.
- Switch the language to confirm translated text renders correctly; the open menu also responds to
  Up, Down, Enter, and Escape.
- Hover the folder icon, the install button, and the agreement links: the pointer becomes a hand,
  and the agreement links open their configured pages in your browser.
- Click the close button: it asks the localized confirmation question from `close_confirm_message`
  before the window closes.
- Point the path field at a folder under `Program Files`: the elevation you approved at launch is
  what lets the install write there.
- Drag or double-click inside the path field to select text, then copy, paste, and undo with
  Ctrl+C, Ctrl+V, and Ctrl+Z; the folder icon next to it picks a directory and writes it back into
  the field.
- The installation directory is gone as soon as the uninstall finishes; a directory you added your
  own files to is kept.

To test a different install path, either click the folder icon next to the path field to pick a
directory, or change `install.default_path` in `examples/TapTap/installer_config.json` and rebuild
the setup. A picked directory replaces the configured default for that run and updates the free
space reading next to it.

Do not run the sample's install action on your workstation: it writes files and registry entries.
