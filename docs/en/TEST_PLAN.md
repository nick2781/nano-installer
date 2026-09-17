# Test plan

## Automated checks

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
.\scripts\build.ps1 -Project examples\TapTap
```

The setup-level suite builds a setup and runs it, so build the runtime executables first:

```powershell
cargo build -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller
$env:NANO_INSTALLER_E2E_REQUIRE_STUBS = "1"
cargo test -p nano-installer-core --test e2e_setup
```

Unit tests cover bundle roundtrip, payload embedding, button hit testing, temporary-directory
deployment, manifest writing, refusing to overwrite an existing directory, upgrades and stale-file
cleanup, failure rollback, and uninstall rules for shortcuts and user data. Layout tests cover
nested flow containers and spacing, percentage sizing, progress bar clipping, falling back from an
out-of-range page, and the binding between progress pages and status text. Script tests cover
deploying files and the manifest from a script, rolling back a failing script, replaying the
manifest through `run_tracked_uninstall`, and the library fallback when a script skips cleanup.

The build script audits PE imports for the builder, all three runtimes, the setup, and the embedded
uninstaller. It also reads the manifest resource back out of the generated setup and the embedded
uninstaller, and fails if the declared execution level or DPI behaviour differs from what the
project configuration resolves to. One test writes to `HKCU`, so it is ignored by default in
restricted environments; run it explicitly inside an isolated VM.

`crates/nano-installer-core/tests/e2e_setup.rs` covers the seams the unit tests cannot see. A change
that packs the wrong layout, drops the payload, or forgets a resource passes every unit test and
still produces a setup that cannot install, because the defects live between two binaries. That is
why the suite writes its own project, builds a setup from it with the real builder, and then runs
that setup and the uninstaller it deployed.

- Build product: the footer the runtime reads, a payload appended behind a real PE, the version and
  manifest resources, and the runtime that matches the payload format.
- Install: the payload lands on disk and comes back byte for byte, the manifest lists what was
  written, the uninstall entry points at the deployed uninstaller, a configured `%TEMP%` path is
  expanded, `--dir` beats the configured path, and an unopted or mistyped run installs nothing.
- Upgrade: a second install over the first drops the files the new payload no longer ships and
  leaves a file the payload does not own alone.
- Uninstall: the product, the registration, and the directory go; a directory holding a file the
  user added survives.

Every case generates its fixture, carries no product payload and no third-party assets, installs
below the temporary directory, and registers under a registry key naming only that case, so parallel
cases cannot see each other and repeated runs do not collide. A case that fails still cleans up
after itself. Without the runtime stubs the suite prints a skip and passes, so a job that validates
setups sets `NANO_INSTALLER_E2E_REQUIRE_STUBS=1` to turn that skip into a failure.

## Manual checks

Start `examples/TapTap/dist/TapTap_Setup.exe` and confirm:

- The client area is 720x450 with no system title bar.
- Background, logo, tagline, and button images are visible with correct transparency.
- The install button and version text use the expected locale.
- The empty area at the top drags the window.
- Minimize and close respond.
- Chinese, English, and Russian text render without mojibake.
- The path field supports drag selection, double-click word selection, copy/paste, and undo, and
  the folder icon picks a directory and writes it back into the field.
- An IME composition window appears at the caret and the candidate list sits just below it while
  typing Chinese or Japanese.
- The installation directory is gone as soon as the uninstall finishes, while files the user added
  are still kept.

## Windows 7 SP1 gate

Before you claim Windows 7 support, verify WIC PNG decoding, GDI text, mouse input, and window
behaviour on a clean Windows 7 SP1 x64 VM. In an isolated VM, test ZIP and 7z extraction, the
manifest, the uninstall entry, and the uninstall button using a fresh directory; then check
installed files, preservation of user-created files, failure rollback, and that the installation
directory is gone as soon as the uninstall finishes and the cleaner copy in the temporary directory
has exited.

Never test the TapTap install action on a daily workstation. The example sets
`install.require_admin`, so a correct build shows the UAC consent prompt before the setup window
appears; declining it must leave the machine untouched. To exercise a build that should never
prompt, clear `install.require_admin` and confirm the same setup starts without a consent prompt. To
exercise a user-writable directory, change the example's read-only `install.default_path` and
repack the setup.

## Production cases that cannot pass yet

- Authenticode signing chain.

Upgrades, failure rollback, shortcuts, autostart, the keep-data option, page transitions, and
progress display are implemented but not yet accepted on a real Windows 7 VM.
