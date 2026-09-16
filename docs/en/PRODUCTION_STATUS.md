# Production status

**The current implementation is not ready for production distribution.** Install actions write
files and registry entries; validate them in a disposable virtual machine only.

## Verified

- The builder and the runtime are separate binaries; a setup never contains the builder.
- ZIP and 7z payloads are detected from the file signature and routed to the matching runtime.
- Both runtimes unpack real archives correctly, checked against expected SHA-256 hashes.
- The uninstaller uses its own runtime and page list and links no archive backend.
- The CLI and the Windows 10+ GUI drive the same build API; GUI dependencies stay out of the
  runtime.
- A setup carries the payload, project resources, and a self-contained uninstaller with its own
  icon and version resources.
- Installation extracts the payload, deploys files and the uninstaller into a new directory, writes
  the manifest and the uninstall entry; uninstall removes only what the manifest records.
- Reinstalling over an existing installation upgrades in place, drops files the new payload no
  longer ships, and restores the previous version if anything fails.
- The first page renders background bitmaps and Unicode text natively, in a borderless rounded
  window that can be dragged, minimized, and closed.
- Windows 7 SP1 x64 compatibility is checked by PE import audit for every artifact.
- Progress pages report live status, and the finish page can launch the deployed application.
- Shortcuts and autostart entries are created during install and restored on uninstall, including
  through rollback.
- Uninstall terminates a running product, removes shortcuts, and deletes declared user data only
  when the user clears the keep-data option.
- `scripts/install.rhai` and `scripts/uninstall.rhai` run in an embedded Rhai engine whose
  primitives reuse the built-in deployment, rollback, and manifest code; a failing script rolls
  back, and a script that skips manifest cleanup falls back to the library removal.
- Flow layout covers nested `VBox`, `HBox`, and `Content` containers with padding, margins,
  percentage sizing, `justify-content`, and `align-items`.

## Blocking a release

1. No code signing. Authenticode (including dual signing) is not wired up, and there is no
   certificate.
2. No acceptance run on a real Windows 7 SP1 machine.

## Known limitations

- Installers do not request elevation. Writing to `Program Files` requires starting the setup as an
  administrator.
- The running uninstaller removes its own file at the next restart rather than immediately.
- The runtime stubs embed the Rhai engine, which raised each stub from roughly 0.57-0.66 MB to
  about 1.8-1.9 MB.
- Link clicks, the folder picker, and Select keyboard handling are not implemented.

Once signing and the Windows 7 acceptance run are done, a product can be onboarded using the
`examples/TapTap` structure.
