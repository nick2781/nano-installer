# Code signing policy

Nothing this project publishes is signed today. [SignPath Foundation](https://signpath.org), which
signs open source projects for free, declined this project's application, because it signs only
projects that already carry a verifiable reputation. This project does not have one yet, and no
certificate has been bought either, so a setup that asks for administrator rights still shows an
unknown publisher.

What exists instead is a rehearsal of the flow. `.github/workflows/sigstore-flow.yml` is manual-only,
and it signs one file per run with a ten-minute certificate that Sigstore's Fulcio issues to the
workflow's own identity, then verifies it. The signature is a separate file, so no executable is
modified and nothing is attached to a release; [Build and
release](site/src/content/docs/reference/build-and-release.mdx) describes what that does and does not prove.

## What would be signed

The executables on the [releases page](https://github.com/nick2781/nano-installer/releases), built by
`.github/workflows/release.yml` from the tagged commit:

| File | What it is |
| --- | --- |
| `nano-installer-native-x64.exe` | the command line builder |
| `nano-installer-gui-x64.exe` | the visual builder |

A setup that a project builds with this framework belongs to that project rather than to this one: it
carries the building project's product name, version and company, and the pipeline that ships that
product signs it, if it is signed at all. This project does not sign such a setup and cannot vouch for
it.

The signed binaries carry no third-party or proprietary material. The example project under
`examples/` holds branded artwork that its owner licenses separately from this repository's MIT
licence; none of it is part of any released artifact, and the release build neither compiles nor
embeds it.

## Who

| Role | Who |
| --- | --- |
| Authors (committers) | [nick2781](https://github.com/nick2781) |
| Reviewers | [nick2781](https://github.com/nick2781); every change reaches `main` through a pull request whose head has to pass both suite jobs |
| Approvers | [nick2781](https://github.com/nick2781); every release needs manual approval before it is signed |

## Privacy

This program will not transfer any information to other networked systems unless specifically
requested by the user or the person installing or operating it.

One case deserves naming, because it is the installer's only network access at all. A project can
declare a dependency for its setup to download, and that setup then fetches exactly the URL the
project named, over https, and nothing else. A setup that declares no dependency never opens a
connection. This framework itself sends nothing anywhere, and there is no telemetry.

## Checking a signature

`scripts/verify_signing.ps1 -Setup <file>` reads a setup and the uninstaller embedded in it, reports
who signed each one and whether it carries a timestamp, and fails when the chain is untrusted or the
timestamp is missing.
