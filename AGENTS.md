# Repository Guidelines

## Project Layout

- `crates/nano-installer-core`: shared bundle, builder, XML layout, and Win32 runtime code.
- `crates/nano-installer-cli`: `nano-installer-native-x64.exe` project builder.
- `crates/nano-installer-gui`: Windows 10+ visual project builder; the only crate allowed to use eframe/egui.
- `crates/nano-installer-stub-lzma`: 7z/LZMA installer runtime only.
- `crates/nano-installer-stub-zlib`: ZIP/Deflate installer runtime only.
- `crates/nano-installer-uninstaller`: uninstaller runtime without archive backends.
- `examples/TapTap`: validation project; its branded resources belong to 易玩（上海）网络科技有限公司 and are not MIT-licensed.
- `docs`: the map of the documentation for agents, one file per language (`AGENTS.md` and `zh-CN/AGENTS.md`).
- `scripts`: release build and PE compatibility checks.
- `site`: the published documentation site: its content, its configuration and its build.

## Build And Test

- Format: `cargo fmt --all -- --check`
- Test: `cargo test --locked --workspace`
- Setup end-to-end suite: `.\scripts\run_e2e_setup.ps1`
- Lint: `cargo clippy --locked --workspace --all-targets -- -D warnings`
- Release: `.\scripts\build.ps1`
- Release plus TapTap setup: `.\scripts\build.ps1 -Project examples\TapTap`
- Backend smoke test: `.\scripts\smoke_backends.ps1`
- Example-only rebuild: `.\target\release\nano-installer-native-x64.exe build --project examples\TapTap`
- Setup screenshots: `.\scripts\capture_setup_snapshots.ps1`
- Release smoke test: `.\scripts\smoke_release.ps1 -Tag v<tag>` (checks a published release; `-SkipUi` on a machine with no desktop)

All published executables target `x86_64-win7-windows-msvc`. Do not introduce a Win32, ANSI, or separate Windows 10 artifact.

## Change Boundaries

- Changes under `crates/**` or `scripts/build.ps1` require rebuilding the builder and all three stubs before rebuilding a setup.
- Changes limited to `examples/**` require only an example setup rebuild.
- Keep LZMA dependencies out of the ZIP and uninstaller crates, ZIP dependencies out of the LZMA and uninstaller crates, and archive dependencies out of the uninstaller.
- Keep eframe, egui, winit, and graphics backends confined to `nano-installer-gui`; they must not enter core, CLI, or stubs.
- Keep raw stubs free of icons and product resources. The builder injects project resources into generated executables.
- Do not edit or commit generated files under `target/` or example `dist/` directories.

## Branches

`main` is the release line and is closed to direct pushes: a change reaches it through a pull
request, whose head has to pass the two suite jobs, and the branch's history can be neither
force-pushed nor deleted. None of that is bypassable, so a commit made on `main` and pushed will be
refused -- branch first, always. The rules and how each one binds are in
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## Documentation

- `docs/AGENTS.md` is the map: which page answers which question, how the two languages relate, and where the machine-readable entry points are. Read it before searching the docs.
- The published site is the Astro project under `site/`, built with Cloudflare Nimbus. Pages are `site/src/content/docs/X.md`, served at `/X/`, and `site/src/content/zh-CN/X.md`, served at `/zh-CN/X/`; `site/dist/` is what GitHub Pages publishes under `/nano-installer/`. The build writes `llms.txt`, `llms-full.txt`, `robots.txt`, `sitemap-index.xml` and a Markdown version of every page from the content tree, so there is no index data file to keep in step -- the generator and its data file went with the docsify site.
- Pages come in pairs, and nothing is written in one language only. The site root is the chooser between the two trees, and each tree is one Nimbus collection. The documentation map is published as a page as well, and the repository keeps it at the top of `docs/` -- `docs/AGENTS.md` paired with `docs/zh-CN/AGENTS.md` -- because the repository's conventions ask for it there. `scripts/audit_docs_languages.ps1` fails when a page in one content tree has no page in the other, or when one side of the map is missing.
- A new page is its file under `site/src/content/`, its twin in the other tree, and a line in the sidebar list in `site/astro.config.ts`. `title` in the frontmatter is what the sidebar and the index call it, so the page's own heading is not written a second time anywhere.

## Verification

- Run the Win7 PE import audit for the builder, every stub, and each generated setup.
- Test archive extraction with real ZIP and 7z files after changing a backend.
- Treat generated setup size and per-stub size as release metrics; update documentation when they materially change.
- Capture the example setup's first page with `.\scripts\capture_setup_snapshots.ps1`, which checks it against the project's own layout; it needs a desktop session, and reading the PNGs with a model needs one running locally.
- Preserve unrelated worktree changes in the working tree.
