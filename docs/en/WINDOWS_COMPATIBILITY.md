# Windows compatibility

Everything you ship targets `x86_64-win7-windows-msvc`, with Windows 7 SP1 x64 as the minimum
supported system. There are no 32-bit or ANSI variants to keep in sync.

The visual builder (`nano-installer-gui-x64.exe`) runs on Windows 10 x64 and later because it uses
eframe/egui. It only creates installers; the setup it produces still runs on Windows 7 SP1.

## What the runtime relies on

- Win32 Unicode APIs for windows, controls, and text
- WIC for PNG decoding and GDI for alpha-blended drawing
- Registry, shell, and process APIs that exist on Windows 7 SP1
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

Static checks do not prove the installer works. You can only claim formal Windows 7 SP1 x64 support
after a run on a clean machine covering PNG decoding, text rendering, mouse input, window
behaviour, extraction, installation, and uninstallation. Those machines should also have update
KB3033929 (SHA-2 code signing support) installed.
