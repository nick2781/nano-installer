# Security policy

## What this project is, and where the risk is

Nano Installer builds and runs Windows installers. A setup it produces writes files and registry
values. It may install a service or a dependency. For a project that ships plugins, it loads a DLL
into its own process. A setup built with `install.require_admin` runs elevated.

Those steps should only happen when they are asked for. If one of them happens anyway, or a setup can
be talked into writing somewhere it was not asked to, that is a security report worth making.

The parts in scope:

- the runtime stubs and the uninstaller (`crates/nano-installer-stub-*`, `crates/nano-installer-uninstaller`),
- the shared runtime, including the bundle reader, the update-package verification and the plugin host (`crates/nano-installer-core`),
- the builders (`crates/nano-installer-cli`, `crates/nano-installer-gui`),
- the scripts under `scripts/`, which are what a release is cut with.

## How to report

Prefer GitHub's private reporting. On this repository, open **Security → Report a vulnerability**
(`https://github.com/nick2781/nano-installer/security/advisories/new`). Private vulnerability
reporting is enabled, so your report reaches the maintainer without becoming public.

If you cannot use that form, open a regular issue. Say only that you have a security report and how to
reach you. Leave the details out. A private channel can then be opened.

Please include:

- the version or tag, and the commit if you built from source,
- the Windows version and whether the run was elevated,
- what you ran, and the smallest project or setup that shows it (a setup built by this repository is ideal; the run log at `%TEMP%\nano-installer\` often names the step),
- what an attacker gets out of it, and what they need to have first.

## What to expect

This is a small project maintained in someone's own time. There is no response-time promise and no bug
bounty. Reports are read. A fix ships in the next release once the report is understood. If a report
turns out to be a documented limitation rather than a vulnerability, the reply will say which document
says so.

## Not vulnerabilities

These are known and written down, so you do not need to report them:

- **No code signing.** Neither a setup nor the MSI package around it is signed, so SmartScreen warns
  about an unknown publisher. See [`docs/en/PRODUCTION_STATUS.md`](docs/en/PRODUCTION_STATUS.md).
- **Elevation is the project's choice.** `install.require_admin` asks Windows for administrator
  rights. A setup built that way runs elevated by design.
- **An installer writes what it was configured to write.** Files, registry values, shortcuts,
  services, environment variables and file associations are the job. The manifest is what an
  uninstall takes back.
- **The example's assets.** `examples/TapTap` carries third-party trademarks and images under their
  own terms. That is a licensing matter rather than a security one.
