# Quick start

One project folder in, one setup executable out. Nothing has to be installed first and nothing has to
be compiled: the builder is one executable you download, and the setup it writes runs on a machine
with none of your own runtimes on it.

## 1. Download the builder

Every release publishes the executables a build needs, beside their digests:

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
builder looks for the three runtimes beside itself, or in a `stubs` directory beside itself;
`--stubs <directory>` points it somewhere else, as does the `NANO_INSTALLER_NATIVE_STUB_DIR`
environment variable.

One command checks a download, against the line `SHA256SUMS.txt` holds for that file:

```powershell
certutil -hashfile nano-installer-native-x64.exe SHA256
```

Nothing published here is code-signed yet, so Windows may warn about an unknown publisher the first
time one of these runs; [production status](PRODUCTION_STATUS.md) says what that does and does not
affect.

## 2. Point it at a project folder

A project folder holds `installer_config.json`, the layout XML, one JSON file per language, your
artwork, and the payload: your application files, as a zip or a 7z archive. The [configuration
reference](CONFIG_REFERENCE.md) and the [page layout guide](XML_LAYOUT_GUIDE.md) describe every file
it may hold, and `examples/TapTap` in the repository is a complete one.

```powershell
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

The setup lands in `dist/<output.installer_name>` inside the project. Add `--msi dist\MyProduct.msi`
to also wrap it in the package an estate deploys through Windows Installer; [build and
release](BUILD_AND_RELEASE.md#installer-packages) says what that does.

To start from the example, clone the repository and point the builder at `examples\TapTap`. Its
images and the archiver under `tools/` are Git LFS objects, so a source ZIP download arrives without
them, and its payload at `examples\TapTap\payload\app.7z` is not stored at all: put a 7z archive
there before you build it.

Prefer clicking? Take `nano-installer-gui-x64.exe` from the same release, start it, open the project
folder and press **Build setup**. It drives the same engine and writes the same setup; see the
[visual builder](GUI.md).

## 3. Try the setup in a VM

Copy the setup into a fresh virtual machine. The sample sets `install.require_admin`, and its
default install path is under `Program Files`, so Windows asks for consent the moment you launch
it. Approve the prompt and the setup runs with the rights it needs.

What you can check today:

- Background, logo, tagline, and button images render with correct transparency.
- The window has no system title bar; you can drag it from an empty area, minimize it, and close it.
- The install button extracts the payload, writes files and shortcuts, and registers an uninstall
  entry. You see progress while it runs, and the finish page can launch the installed program.
- Install a second time to see the upgrade path, and use the uninstall entry to check removal and
  the keep-data option.
- Switch the language to confirm translated text renders correctly; the open menu also responds to
  Up, Down, Enter, and Escape.
- Hover the folder icon, the install button, and the agreement links: the pointer becomes a hand,
  and the agreement links open their configured pages in your browser.
- Click the close button: the confirmation appears inside the window, wearing the same skin as the
  installer, and only then does it close.
- Point the path field at a folder under `Program Files`: the elevation you approved at launch is
  what lets the install write there.
- Drag or double-click inside the path field to select text, then copy, paste, and undo with
  Ctrl+C, Ctrl+V, and Ctrl+Z; the folder icon next to it picks a directory and writes it back into
  the field.
- The installation directory is gone as soon as the uninstall finishes; a directory you added your
  own files to is kept.

To test a different install path, either click the folder icon next to the path field to pick a
directory, or change `install.default_path` in `examples/TapTap/installer_config.json` and rebuild
the setup. A directory you pick replaces the configured default for that run and updates the free
space reading next to it.

Do not run the sample's install action on your workstation: it writes files and registry entries.

## Building the tools instead

Nothing above needs a compiler. Building the tools from source is how a contributor works on them:
[build and release](BUILD_AND_RELEASE.md) covers `scripts\build.ps1`, which produces the same five
executables under `target/release/`, and
[contributing](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md) covers the
working tree and the checks a change has to pass.
