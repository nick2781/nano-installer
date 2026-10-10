# Quick start

One project folder in, one setup executable out. Nothing to install first and nothing to compile. The
builder is one executable you download and run. The setup it writes runs on the user's machine without
a runtime of your own, because the program that does the work is already inside the setup.

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
release](https://github.com/nick2781/nano-installer/releases/latest) and put them in one folder; the
builder looks for the three runtimes beside itself and in a `stubs` directory beside it, and you can
point it elsewhere with `--stubs <directory>` or with the `NANO_INSTALLER_NATIVE_STUB_DIR` environment
variable.

One command checks a download: it compares the line `SHA256SUMS.txt` holds for that file:

```powershell
certutil -hashfile nano-installer-native-x64.exe SHA256
```

Nothing published here is code-signed yet, so Windows may warn about an unknown publisher the first
time one of these runs; [production status](PRODUCTION_STATUS.md) says what that does and does not
affect.

## 2. Point it at a project folder

A project folder holds `installer_config.json`, the page layout XML, one JSON file per language, your
artwork, and the payload — the archive that gets packed into the setup, zip or 7z alike. The
[configuration reference](CONFIG_REFERENCE.md) and the [page layout guide](XML_LAYOUT_GUIDE.md)
describe every file it may hold, and `examples/TapTap` in the repository is a complete one.

```powershell
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

The setup lands in `dist/<output.installer_name>` inside the project; add `--msi dist\MyProduct.msi`
to also wrap it in the package an estate deploys through Windows Installer, and [build and
release](BUILD_AND_RELEASE.md#installer-packages) says what that does.

To start from the example, clone the repository and point the builder at `examples\TapTap`. The
example's images and the archiver under `tools/` are Git LFS objects, so a source ZIP download arrives
with pointers instead; the payload at `examples\TapTap\payload\app.7z` is not stored at all, and you
have to put a 7z archive there before you build.

Prefer clicking? Take `nano-installer-gui-x64.exe` from the same release, start it, open the project
folder and press **Build setup**: it drives the same engine and writes the same setup. See the
[visual builder](GUI.md).

## 3. Try the setup in a VM

Copy the setup into a fresh virtual machine. The sample sets `install.require_admin`, and its default
install path is under `Program Files`, so Windows puts up an administrator prompt (UAC) the moment you
launch it; approve that prompt and the setup runs with the rights it needs.

What you can check today:

- Background, logo, tagline, and button images render correctly, and transparency is right.
- The window has no system title bar, but you can drag it from an empty area, and minimize and close
  both work.
- The install button extracts the payload, writes files and shortcuts, and leaves an uninstall entry
  in Windows' Programs and Features, with progress shown while it runs. The finish page can launch the
  installed program.
- Install a second time to walk the upgrade path, then use the uninstall entry to check removal and
  the keep-data option.
- Switch the language and check that translated text renders; an open menu also responds to Up, Down,
  Enter, and Escape.
- Hover the folder icon, the install button, and the agreement links: the pointer becomes a hand, and
  the agreement links open their configured pages in your browser.
- Click the close button: the confirmation appears inside the window, with the same skin as the
  installer, and it closes only after you confirm.
- Drag or double-click inside the path field to select text; copy, paste, and undo with Ctrl+C,
  Ctrl+V, and Ctrl+Z. The folder icon next to it picks a directory and writes it back into the field.
  An IME composes Chinese in the field and shows its candidate window.
- Point the path field at a folder under `Program Files` and the install can write there, because you
  approved the elevation at launch.
- The installation directory is gone as soon as the uninstall finishes, but a directory you put your
  own files in stays.

To test a different install path, click the folder icon next to the path field to pick a directory, or
change `install.default_path` in `examples/TapTap/installer_config.json` and rebuild the setup. A
directory you pick replaces the configured default for that run, and the free space number shown next
to it changes too.

Do not run the sample's install on your workstation: it writes files and registry entries.

## Building the tools instead

Nothing above needs a compiler; building the tools from source is how a contributor works on them:
[build and release](BUILD_AND_RELEASE.md) covers `scripts\build.ps1`, which produces the same five
executables under `target/release/`, and
[contributing](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md) covers preparing
a working tree and the checks a change has to pass.
