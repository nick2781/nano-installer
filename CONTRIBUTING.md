# Contributing

Thanks for looking. This is a Windows installer framework: one project folder in, one self-contained
`.exe` out. Bugs, plugins and documentation fixes are all welcome, and issues may be written in
English or Chinese.

Two documents are worth reading before a change: [`docs/en/BUILD_AND_RELEASE.md`](docs/en/BUILD_AND_RELEASE.md)
for what the builds are, and [`docs/en/TEST_PLAN.md`](docs/en/TEST_PLAN.md) for how a change is
verified. Everything below is the short version.

## Getting set up

You need Windows x64 with the MSVC build tools. The Rust toolchain is pinned in
`rust-toolchain.toml`, so `rustup` picks it up on its own. Clone with `git clone` and run
`git lfs install` once — the icons, the example's images and the archiver under `tools/` are Git LFS
objects, and a source ZIP download arrives without them.

```powershell
.\scripts\build.ps1                     # builder, GUI and the three runtime stubs
.\scripts\run_tests.ps1                 # the whole suite
```

## What a change has to pass

Run these before opening a pull request; CI runs the same ones.

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
.\scripts\run_tests.ps1
.\scripts\run_e2e_setup.ps1             # builds the stubs and the sample plugin, then the setup-level cases
.\scripts\audit_script_encoding.ps1     # scripts must be pure ASCII or carry a BOM
.\scripts\audit_case_descriptions.ps1   # every case has a row in the case table of each language
.\scripts\build_docs_index.ps1 -Verify  # every documentation page has a line in scripts/docs_index.json
.\scripts\audit_docs_languages.ps1     # both language trees hold the same pages, and the language switch knows them
node scripts/check_language_switch.js   # the switch in the site's top bar moves a page to the same page in the other tree
.\scripts\verify_reports.ps1            # a run's report survives a run that did nothing
.\scripts\audit_workflow_images.ps1     # every job names the image it runs on, and a release is audited on the image it is built on
```

A change to what a setup *does* also updates the documents that promise it: `CHANGELOG.md`,
`docs/{en,zh-CN}/TEST_COVERAGE.md` (the counts, and the row for the behaviour), and
`docs/{en,zh-CN}/TEST_CASES.md` (one row per case — the audit refuses a case without one, in either
language). Every page exists in both languages and a change to one side is a change to both: the
site switches a reader between `docs/en/X.md` and `docs/zh-CN/X.md`, and the documentation map is
paired with `docs/zh-CN/AGENTS.md`. `scripts/audit_docs_languages.ps1` refuses a page that only one
tree carries.

## Branches, and what `main` protects

`main` is the release line: every tag is cut from it, and the CI runs on a commit are the evidence the
release gate reads back. Three rules are enforced on it, and none of them can be bypassed, this
repository's administrator included:

- **Its history.** `main` cannot be force-pushed and cannot be deleted. A repository ruleset with no
  exemption says so, and both refusals were observed on a scratch ref before it was applied here.
- **A pull request.** Work reaches `main` through one, because the suite has to be green on the commit
  that lands and a direct push cannot be checked before it exists. No approval is required: this is a
  one-maintainer repository, so the rule is about the checks rather than about a second reader.
- **The suite.** `Native Win7+ Build` and `Setup End to End` are the required checks, and the branch
  has to be up to date with `main` before the pull request can merge. A branch is deleted once it has
  merged, so the branch list stays a list of work in flight.

So the shape of a change is: branch, pull request, two green jobs, merge. That is the whole of it --
there is no short path for a small fix, which is what makes the two jobs mean something.

## What a change has to respect

- **The release baseline.** Every published executable targets `x86_64-win7-windows-msvc` with
  Windows 7 SP1 x64 as the minimum. A new system call has to pass
  `scripts/audit_win7_imports.ps1`, and there is no second artifact for newer Windows.
- **The crate boundaries.** LZMA stays out of the ZIP stub and the uninstaller, ZIP stays out of the
  LZMA stub and the uninstaller, and archive dependencies stay out of the uninstaller. `eframe`,
  `egui`, `winit` and the graphics backends live in `nano-installer-gui` and nowhere else.
- **The plugin ABI.** `include/nano_plugin.h` is a promise to people outside this repository. The
  host structure only ever grows — `struct_size` and `abi_version` are its first two fields and
  always will be — and the three parameters of a plugin function never change.
  `NANO_PLUGIN_ABI_VERSION` moves only for a change that breaks a plugin built against an older
  header, and such a change needs the reasoning written down in the changelog.
- **The conduct file stays canonical.** GitHub recognises `CODE_OF_CONDUCT.md` as the Contributor
  Covenant only while the text is the template with the contact placeholder replaced by something
  short — at the time of writing, one account link. Rewriting the enforcement sentence around it, or
  adding a section for this project's own notes, is enough for the community profile to fall back to
  an unrecognised file. The notes that used to sit at the end of that file live in this document.
- **Configuration keys are validated.** The builder refuses a key it does not read, so adding one
  means adding it to `crates/nano-installer-core/src/config.rs` and to
  `docs/{en,zh-CN}/CONFIG_REFERENCE.md` in the same change.
- **The documentation index.** The published site is a viewer over Markdown files, and
  `docs/llms.txt` is how something that cannot run the viewer finds them. A new page under `docs/`
  therefore needs a line in `scripts/docs_index.json` — title and one-line description — and that
  title has to read exactly like the page's own `# heading`, because the index is what a reader and
  an agent quote back. `.\scripts\build_docs_index.ps1` writes the three index files;
  `-Verify` is the check.

## Reporting

- **Bugs and feature requests**: open an issue; the forms ask for what a maintainer would otherwise
  have to ask for. The run log at `%TEMP%\nano-installer\` is usually the fastest evidence — a failed
  install leaves one behind and names it.
- **Security**: do not open a public issue. See [`SECURITY.md`](SECURITY.md).
- **Conduct**: what is expected of everyone here is in [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md), which is the Contributor Covenant 2.1 text as
  written. Two things this project adds on top of it: English and Chinese are both welcome —
  nobody is asked to write in a language they are guessing at — and the argument stays about
  the code rather than the person. "This reads the registry without checking the view" is a
  review; "you clearly do not know Win32" is not. A direct message or an issue reaches the
  maintainer; anything that has to stay out of public view — including a report about the
  maintainer, since there is nobody else here to read it — can go to GitHub's
  [report abuse](https://github.com/contact/report-abuse) form instead.

## Licence

Contributions are under the MIT licence in [`LICENSE`](LICENSE). `examples/TapTap` is a validation
project whose trademarks, images and copy belong to their rights holders and are not MIT-licensed;
do not copy that material into a new example.
