# Test plan

## Automated checks

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
.\scripts\build.ps1 -Project examples\TapTap
```

Unit tests cover bundle roundtrip, payload embedding, button hit testing, temporary-directory
deployment, manifest writing, refusing to overwrite an existing directory, upgrades and stale-file
cleanup, failure rollback, and uninstall rules for shortcuts and user data. Layout tests cover
nested flow containers and spacing, percentage sizing, progress bar clipping, falling back from an
out-of-range page, and the binding between progress pages and status text. Script tests cover
deploying files and the manifest from a script, rolling back a failing script, replaying the
manifest through `run_tracked_uninstall`, and the library fallback when a script skips cleanup.

The build script audits PE imports for the builder, all three runtimes, the setup, and the embedded
uninstaller. One test writes to `HKCU`, so it is ignored by default in restricted environments;
run it explicitly inside an isolated VM.

## Manual checks

Start `examples/TapTap/dist/TapTap_Setup.exe` and confirm:

- The client area is 720x450 with no system title bar.
- Background, logo, tagline, and button images are visible with correct transparency.
- The install button and version text use the expected locale.
- The empty area at the top drags the window.
- Minimize and close respond.
- Chinese, English, and Russian text render without mojibake.

## Windows 7 SP1 gate

Before claiming Windows 7 support, verify WIC PNG decoding, GDI text, mouse input, and window
behaviour on a clean Windows 7 SP1 x64 VM. In an isolated VM, test ZIP and 7z extraction, the
manifest, the uninstall entry, and the uninstall button using a fresh directory; then check
installed files, preservation of user-created files, failure rollback, and uninstaller cleanup
after a restart.

Never test the TapTap install action on a daily workstation. When the default path is under
`Program Files`, start the setup as an administrator; this version does not request elevation.
To exercise a user-writable directory, change the example's read-only `install.default_path` and
repack the setup.

## Production cases that cannot pass yet

- Immediate uninstaller self-deletion; the file is currently removed at the next restart.
- Authenticode signing chain.

Upgrades, failure rollback, shortcuts, autostart, the keep-data option, page transitions, and
progress display are implemented but not yet accepted on a real Windows 7 VM.
