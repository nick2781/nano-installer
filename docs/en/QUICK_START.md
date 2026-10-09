# Quick start

One project folder in, one setup executable out. Nothing to install first and nothing to compile. The
builder is one executable you download and run. The setup it writes runs on the user's machine and
needs none of your runtimes. A runtime here is the program inside the setup that does the work.

## 1. Download the builder

Every release publishes the executables a build needs, each with its digest:

| File | What it is |
| --- | --- |
| `nano-installer-native-x64.exe` | the command-line builder |
| `nano-installer-gui-x64.exe` | the visual builder, for a Windows 10+ desktop |
| `lzma-stub-native.exe` | the runtime a setup carries for a 7z payload |
| `zlib-stub-native.exe` | the runtime a setup carries for a zip payload |
| `uninst-stub-native.exe` | the uninstaller a setup carries |
| `SHA256SUMS.txt` | one digest per file above |

Take them from the [latest
release](https://github.com/nick2781/nano-installer/releases/latest) and put them in one folder. The
builder looks for the three runtimes beside itself, and in a `stubs` directory beside itself. Point it
elsewhere with `--stubs <directory>`, or with the `NANO_INSTALLER_NATIVE_STUB_DIR` environment
variable.

One command checks a download. It compares the line `SHA256SUMS.txt` holds for that file:

```powershell
certutil -hashfile nano-installer-native-x64.exe SHA256
```

Nothing published here is code-signed yet. Windows may warn about an unknown publisher the first time
one of these runs. [production status](PRODUCTION_STATUS.md) says what that does and does not affect.

## 2. Point it at a project folder

A project folder holds `installer_config.json`, the page layout XML, one JSON file per language, your
artwork, and the payload (your application files, as a zip or a 7z archive). The [configuration
reference](CONFIG_REFERENCE.md) and the [page layout guide](XML_LAYOUT_GUIDE.md) describe every file
it may hold. `examples/TapTap` in the repository is a complete one.

```powershell
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

The setup lands in `dist/<output.installer_name>` inside the project. Add `--msi dist\MyProduct.msi`
to also wrap it in the package an estate deploys through Windows Installer. [build and
release](BUILD_AND_RELEASE.md#installer-packages) says what that does.

To start from the example, clone the repository and point the builder at `examples\TapTap`. Its images
and the archiver under `tools/` are Git LFS objects, so a source ZIP download arrives without them. Its
payload at `examples\TapTap\payload\app.7z` is not stored at all. Put a 7z archive there before you
build it.

Prefer clicking? Take `nano-installer-gui-x64.exe` from the same release, start it, open the project
folder and press **Build setup**. It drives the same engine and writes the same setup. See the
[visual builder](GUI.md).

## 3. Try the setup in a VM

Copy the setup into a fresh virtual machine. The sample sets `install.require_admin`, and its default
install path is under `Program Files`. Windows puts up an administrator prompt (UAC) the moment you
launch it. Approve the prompt and the setup runs with the rights it needs.

What you can check today:

- Background, logo, tagline, and button images render correctly, and transparency is right.
- The window has no system title bar. Drag it from an empty area, minimize it, and close it.
- The install button extracts the payload, writes files and shortcuts, and leaves an uninstall entry
  in Windows' Programs and Features. Progress shows while it runs. The finish page can launch the
  installed program.
- Install a second time to walk the upgrade path. Use the uninstall entry to check removal and the
  keep-data option.
- Switch the language and check that translated text renders. An open menu also responds to Up, Down,
  Enter, and Escape.
- Hover the folder icon, the install button, and the agreement links: the pointer becomes a hand. The
  agreement links open their configured pages in your browser.
- Click the close button: the confirmation appears inside the window, with the same skin as the
  installer. It closes only after you confirm.
- Drag or double-click inside the path field to select text. Copy, paste, and undo with Ctrl+C,
  Ctrl+V, and Ctrl+Z. The folder icon next to it picks a directory and writes it back into the field.
  An IME composes Chinese in the field and shows its candidate window.
- Point the path field at a folder under `Program Files` and the install can write there. That works
  because you approved the elevation at launch.
- The installation directory is gone as soon as the uninstall finishes. A directory you put your own
  files in stays.

To test a different install path, click the folder icon next to the path field to pick a directory. Or
change `install.default_path` in `examples/TapTap/installer_config.json` and rebuild the setup. A
directory you pick replaces the configured default for that run. The free space number shown next to
it changes too.

Do not run the sample's install on your workstation. It writes files and registry entries.

## Building the tools instead

Nothing above needs a compiler. Building the tools from source is how a contributor works on them:
[build and release](BUILD_AND_RELEASE.md) covers `scripts\build.ps1`, which produces the same five
executables under `target/release/`, and
[contributing](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md) covers preparing
a working tree and the checks a change has to pass.
