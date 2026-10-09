# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

That certificate is issued to SignPath Foundation rather than to this repository, so SignPath
Foundation is the publisher of the signed binaries. For every release, SignPath.io verifies
that the file it signs was produced by this repository's own automated build from the tagged
commit.

## What is signed

The executables on the [releases page](https://github.com/nick2781/nano-installer/releases),
built by `.github/workflows/release.yml` from the tagged commit:

| File | What it is |
| --- | --- |
| `nano-installer-native-x64.exe` | the command line builder |
| `nano-installer-gui-x64.exe` | the visual builder |

A setup that a project builds with this framework is that project's binary rather than this
project's: it carries the building project's product name, version and company, and it is
signed, if at all, by the pipeline that ships that product. This project does not sign such a
setup and cannot vouch for it.

The signed binaries carry no third-party or proprietary material. The example project under
`examples/` holds branded artwork that its owner licenses separately from this repository's
MIT licence; none of it is part of any released artifact, and the release build neither
compiles nor embeds it.

## Who

| Role | Who |
| --- | --- |
| Authors (committers) | [nick2781](https://github.com/nick2781) |
| Reviewers | [nick2781](https://github.com/nick2781); every change reaches `main` through a pull request whose head has to pass both suite jobs |
| Approvers | [nick2781](https://github.com/nick2781); every release needs manual approval before it is signed |

## Privacy

This program will not transfer any information to other networked systems unless specifically
requested by the user or the person installing or operating it.

One case deserves naming because it is the installer's only network access at all. A project
can declare a dependency for its setup to download, and then that setup fetches exactly the
URL the project named, over https, and nothing else; a setup that declares no dependency never
opens a connection. This framework itself sends nothing anywhere, and there is no telemetry.

## Checking a signature

`scripts/verify_signing.ps1 -Setup <file>` reads a setup and the uninstaller embedded in it,
reports who signed each one and whether it carries a timestamp, and fails when the chain is
untrusted or the timestamp is missing.