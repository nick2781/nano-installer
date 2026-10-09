# Windows compatibility

Everything you ship targets `x86_64-win7-windows-msvc`, and the minimum supported system is Windows 7 SP1
x64. That means no 32-bit build, and no ANSI build to keep in step.

The visual builder, `nano-installer-gui-x64.exe`, runs on Windows 10 x64 and later because it uses
eframe/egui. It only creates installers, though, and the setup it produces is still built for that
minimum version.

That minimum version has been through a real run: a build 7601 virtual machine, with a setup the
current builder produced. Every build also checks the PE imports, which proves that nothing a setup
calls was added after Windows 7, while the run on the machine proves those calls work. What it reached
and what it did not is in [Production status](PRODUCTION_STATUS.md); to walk the same ground yourself,
see [Running the acceptance](#running-the-acceptance).

## What the runtime relies on

- Win32 Unicode APIs: windows, controls, and text all go through them.
- WIC to decode PNG, and GDI to blend the result with alpha.
- The registry, shell, and process APIs, all of which Windows 7 SP1 has. The 32- and 64-bit views of a
  registry key are read with `KEY_WOW64_32KEY` and `KEY_WOW64_64KEY`, while what a command wrote is
  decoded in the system ANSI code page and converted back with `MultiByteToWideChar`. Both have been
  there since Windows XP.
- The service control manager. A service a script installs is an ordinary service on the machine, so
  installing, changing, stopping and deleting one all go through this interface. Every Windows this
  project supports shipped it long before Windows 7.
- IMM32, for the IME composition and candidate windows of a text field. Windows 7 ships it.
- WinHTTP, to download a dependency a project asks for. Over https it turns on TLS 1.1/1.2, but on
  Windows 7 SP1 that also needs the system's own schannel to support TLS 1.2, which means update
  KB3140245 or later. Without it the download fails instead of falling back to plain text.

Release builds use the pinned `nightly-2025-11-08` toolchain, `rust-src`,
`-Z build-std=std,panic_abort`, a statically linked CRT, and `panic=abort`.

## How compatibility is checked

Every build checks the PE imports of the builder, the three runtimes, and each setup you generate. If
an API that only Windows 8 or Windows 10 has appears, the build fails, which proves these files do not
statically depend on a newer system.

A static check cannot prove an installer works, so there is also a run on a real machine, and it has
happened. On build 7601, PNG decoding, text rendering, mouse input, window behaviour, extraction,
installation and uninstallation were all reached, and the item-by-item results are in
[Production status](PRODUCTION_STATUS.md). What that run did not reach is window dragging,
multi-monitor scaling, cursor shapes, and installing or removing a service. The screen reader item got
as far as the accessibility information, which read the same as Windows 11, but Narrator or NVDA was
never actually running and listening. Target machines should have update KB3033929 (SHA-2 code signing
support) installed, and one whose project downloads a dependency also needs KB3140245 or later.

## Running the acceptance

The paragraphs above state the requirement; this is how to meet it. The point is to leave evidence
somebody can check, not a memory.

**Prepare.** A clean Windows 7 SP1 x64 machine or virtual machine, build 7601, with update KB3033929
installed. The run installs software, so take a checkpoint first. If the project downloads a
dependency, the machine also needs KB3140245 or later, or that step is recorded as not reached.
Building the setup needs no toolchain: take the builder and the three runtimes from a release, as the
[quick start](QUICK_START.md) describes, put them in one folder, and point the builder at the project.

```powershell
.\nano-installer-native-x64.exe build --project <your project>
```

Copy that one setup to the Windows 7 machine. A project makes a better subject than an empty one when
it exercises the pages, ships more than one language and PNG artwork, carries a payload, and creates a
shortcut and an uninstall flow; `examples/TapTap` with a payload of your own is the shape this
documentation is validated with.

**Observe, in this order.** Each line is something this repository promises:

1. Run it as a standard user. The window opens without a title bar and with rounded corners, and the
   background bitmap and the logo are drawn (PNG decoding, alpha blending, GDI); the text reads in the
   project's language, and the pointer turns into a hand over anything clickable.
2. Drag the window by its background, minimize it, restore it.
3. Switch the language if the project ships more than one, and watch the page redraw.
4. Walk every page, including a progress page that names its steps. If the finish page is set to start
   the program, click that too.
5. After installing, look at what the machine now has: the files, the shortcuts (desktop, Start menu,
   autostart), the uninstall entry, and the manifest file the uninstaller will read.
6. If the project ships `scripts/install.rhai`, check what that script did.
7. Uninstall from the uninstall entry. The product's files and shortcuts should be gone, user data
   should stay unless the box was cleared, and an empty installation folder should be removed.
8. With a screen reader running (Narrator or NVDA), have it read the page, the focus, and a field as
   you type in it.
9. If the project installs a service, check that it exists, starts, and is gone after the uninstall.

**Record the result.** Write down the system version (`winver` shows "Version 6.1 (Build 7601: Service
Pack 1)"), the setup's version or tag, the project it was built from, whether it ran as administrator,
and the result of each item above: what actually happened, or which item was not reached and why. Then
go back to the repository:

- add what you saw to the Verified list in `docs/{en,zh-CN}/PRODUCTION_STATUS.md`, and say which
  release this run covers;
- add a `已验证` entry below the marker in `CHANGELOG.md` with the system version, the release it
  covers, and what it did not reach.

A run does not have to be repeated for every release; repeat it when something it exercised changes:
the runtime, a kind of page, the manifest file, the uninstaller, a Win32 call. Say which release it
covers. It cannot speak for every Windows 7 installation either: 32-bit Windows 7 and ARM64 are not
supported at all, and a machine without SP1 is outside what these files claim.
