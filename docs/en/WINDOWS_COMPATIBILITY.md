# Windows compatibility

Everything you ship targets `x86_64-win7-windows-msvc`, with Windows 7 SP1 x64 as the minimum
supported system. There are no 32-bit or ANSI variants to keep in sync.

The visual builder (`nano-installer-gui-x64.exe`) runs on Windows 10 x64 and later because it uses
eframe/egui. It only creates installers; the setup it produces is built to run on Windows 7 SP1.

**Windows 7 SP1 x64 has been through one acceptance run**: build 7601, Ultimate, in a disposable
virtual machine, with a setup built by the current builder. The baseline above is what every build
targets and what the import audit checks, so nothing a setup calls was added after Windows 7; that
run turned the argument about imports into a measurement, and what it covered and what it did not is
recorded in [Production status](PRODUCTION_STATUS.md). To walk the same ground on your own machine,
follow [Running the acceptance](#running-the-acceptance).

## What the runtime relies on

- Win32 Unicode APIs for windows, controls, and text
- WIC for PNG decoding and GDI for alpha-blended drawing
- Registry, shell, and process APIs that exist on Windows 7 SP1: the 32- and 64-bit views of a
  key are read with `KEY_WOW64_32KEY` and `KEY_WOW64_64KEY`, and what a command wrote is decoded in
  the machine's ANSI code page with `MultiByteToWideChar`, both of which have been there since
  Windows XP
- The service control manager, for the service a project's script installs, changes, starts and
  deletes: every Windows this project supports has shipped it since long before Windows 7
- IMM32 for the IME composition and candidate windows of a text field, which Windows 7 has shipped
  since release
- WinHTTP to fetch a dependency a project declares for download, with TLS 1.1/1.2 turned on for
  https (Windows 7 SP1 also needs the machine's own schannel to support TLS 1.2, which KB3140245 and
  later bring; without it the download fails rather than falling back to plain text)

Release builds use the pinned `nightly-2025-11-08` toolchain with `rust-src`,
`-Z build-std=std,panic_abort`, a statically linked CRT, and `panic=abort`.

## How compatibility is checked

Every build audits the PE imports of the builder, all three stubs, and each setup you generate, and
fails if a blocked Windows 8 or Windows 10 API appears, so the files cannot statically depend on a
newer system API.

Static checks do not prove the installer works, so there is an acceptance run on a real machine, and
it has happened: on build 7601, PNG decoding, text rendering, mouse input, window behaviour,
extraction, installation, and uninstallation were all reached, item by item, in
[Production status](PRODUCTION_STATUS.md). What that run did not reach is window dragging and
multi-monitor scaling, cursor shapes, and installing or removing a service; the screen reader item
reached the accessibility tree, which answered identically to Windows 11, but Narrator or NVDA was
never actually running and listened to. Machines should also have update KB3033929 (SHA-2 code
signing support) installed, and one whose setup downloads a dependency needs KB3140245 and later as
well.

## Running the acceptance

The paragraph above is the requirement; this is how it is met, so that a run leaves evidence rather
than an impression.

**Prepare.** A clean Windows 7 SP1 x64 machine or virtual machine, build 7601, with update KB3033929
installed, and a checkpoint to go back to afterwards, because the run installs software. If the
project declares a dependency it downloads, the machine also needs KB3140245 and later for TLS 1.2,
or that step is recorded as not covered. Building the setup needs no toolchain: take the builder and
the three runtimes from a release, as the [quick start](QUICK_START.md) describes, put them in one
folder, and point the builder at the project.

```powershell
.\nano-installer-native-x64.exe build --project <your project>
```

Then copy that one setup file to the Windows 7 machine. A project that exercises the pages, more than
one language, PNG artwork, a payload, a shortcut and an uninstall is a better subject than an empty
one; `examples/TapTap` with a payload of your own is the shape this documentation is validated with.

**Observe, in this order.** Each line is something a claim in this repository rests on:

1. Run it as a standard user: the window opens borderless with rounded corners, the background bitmap
   and the logo are drawn (PNG decoding, alpha blending, GDI), the text reads in the project's
   language, and the pointer becomes a hand over anything clickable.
2. Drag the window by its background, minimize it, restore it.
3. Switch the language if the project ships more than one, and watch the page redraw.
4. Walk every page, including a progress page that names its steps and the finish page starting the
   deployed program if the project configures that.
5. Install it, then look at the machine: the files, the shortcuts (desktop, Start menu, autostart),
   the uninstall entry, and the manifest the uninstaller will read.
6. If the project ships `scripts/install.rhai`, check what that script did as well.
7. Uninstall from the uninstall entry: the product's files and shortcuts are gone, user data is kept
   unless the box was cleared, and an empty installation folder is removed.
8. With a screen reader running (Narrator or NVDA), have it read the page, the focus, and a field as
   it is typed.
9. If the project installs a service, check that the service exists, starts, and is gone after the
   uninstall.

**Record.** Write down the build (`winver`: "Version 6.1 (Build 7601: Service Pack 1)"), the setup's
version or tag, the project it was built from, whether it ran elevated, and each observation above --
what happened, or that it was not covered and why. Then, in the repository:

- the Verified list in `docs/{en,zh-CN}/PRODUCTION_STATUS.md` gains what was observed, and the note at
  the top of that document stops saying that Windows 7 was never observed;
- `CHANGELOG.md` gains a verified entry (`已验证`) naming the build, the release the run covers, and
  what it did not cover.

One run does not have to be repeated for every release. Repeat it when something the run exercised
changes -- the runtime, a kind of page, the manifest, the uninstaller, a Win32 call -- and say which
release it covers. It cannot prove every Windows 7 installation either: 32-bit Windows 7 and ARM64
are not supported at all, and a machine without SP1 is not the baseline these artifacts claim.
