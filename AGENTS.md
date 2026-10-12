# Repository Guidelines

## Project Layout

- `crates/nano-installer-core`: shared bundle, builder, XML layout, and Win32 runtime code.
- `crates/nano-installer-cli`: `nano-installer-native-x64.exe` project builder.
- `crates/nano-installer-gui`: Windows 10+ visual project builder; the only crate allowed to use eframe/egui.
- `crates/nano-installer-stub-lzma`: 7z/LZMA installer runtime only.
- `crates/nano-installer-stub-zlib`: ZIP/Deflate installer runtime only.
- `crates/nano-installer-uninstaller`: uninstaller runtime without archive backends.
- `examples/TapTap`: validation project; its branded resources belong to 易玩（上海）网络科技有限公司 and are not MIT-licensed.
- `docs`: the documentation map, one file per language (`AGENTS.md`, `zh-CN/AGENTS.md`).
- `scripts`: the release build, the audits a change passes, and the PE compatibility checks.
- `site`: the published documentation site.

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
- Keep LZMA out of the ZIP and uninstaller crates, ZIP out of the LZMA and uninstaller crates, and archive dependencies out of the uninstaller.
- Keep eframe, egui, winit and the graphics backends in `nano-installer-gui`; they must not enter core, CLI, or stubs.
- Keep raw stubs free of icons and product resources; the builder injects them into generated executables only.
- Do not edit or commit generated files under `target/` or example `dist/` directories.

## Branches

`main` is the release line, and it takes no direct push.

- Branch first: a commit pushed straight to `main` is refused.
- Reach it through a pull request whose head passes both suite jobs, `Native Win7+ Build` and `Setup End to End`.
- `main` can be neither force-pushed nor deleted.

None of this is bypassable; the rules are in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Documentation

- Read `docs/AGENTS.md` first: it maps each question to the page that answers it and names the machine-readable entry points.
- The site is the Astro project under `site/`, built with Cloudflare Nimbus, and `site/dist/` is what GitHub Pages publishes under `/nano-installer/`. English pages are `site/src/content/docs/<section>/<page>.mdx` at `/<section>/<page>/`, Chinese pages are `site/src/content/zh-CN/<section>/<page>.mdx` at `/zh-CN/<section>/<page>/`, and every section has an `index.mdx` landing page. `site/astro.config.ts` redirects each page's old uppercase address to where it lives now.
- Keep no index data by hand: the build writes `llms.txt`, `llms-full.txt`, `robots.txt`, `sitemap-index.xml` and a Markdown copy of every page from the content tree. The generator and its data file went with the docsify site.
- Write every page in both languages, and keep the map at the top of `docs/` (`docs/AGENTS.md`, `docs/zh-CN/AGENTS.md`). `scripts/audit_docs_languages.ps1` fails on a page whose twin is missing, or on a missing side of the map.
- A new page is its file under `site/src/content/`, its twin in the other tree, and a line in the sidebar list in `site/astro.config.ts`; the frontmatter `title` is what the sidebar and the index call it, so do not write the page's own heading again.
- `scripts/audit_docs_sidebar.ps1` holds the sidebar and the two content trees to each other. `scripts/audit_docs_links.ps1` reads the built site for a dead link or anchor, so it runs after the build.

## Verification

- Run the Win7 PE import audit for the builder, every stub, and each generated setup.
- Test archive extraction with real ZIP and 7z files after changing a backend.
- Treat generated setup size and per-stub size as release metrics; update documentation when they materially change.
- Capture the example setup's first page with `.\scripts\capture_setup_snapshots.ps1`, which checks it against the project's own layout. It needs a desktop session, and reading the PNGs with a model needs one running locally.
- Preserve unrelated worktree changes in the working tree.
