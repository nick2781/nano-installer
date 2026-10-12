---
title: Project layout
description: 'Where the crates, the project folders, the build outputs and the documentation site sit, and which of them the builder fills with your product resources.'
slug: PROJECT_STRUCTURE
sidebar:
  order: 1200
---

## Repository

```text
crates/
├── nano-installer-core/        # bundling, XML layout, WIC/GDI rendering, shared runtime
├── nano-installer-cli/         # command-line builder
├── nano-installer-gui/         # Windows 10+ visual builder
├── nano-installer-stub-lzma/   # 7z runtime, links only sevenz-rust
├── nano-installer-stub-zlib/   # ZIP runtime, links only zip/deflate
└── nano-installer-uninstaller/ # uninstaller runtime, no archive backend
docs/
├── AGENTS.md                   # the map of the documentation, for a reader in the repository
└── zh-CN/AGENTS.md             # the same map in Chinese
examples/TapTap/                # validation project
examples/nsis-migration/        # a project migrated off NSIS, with the script that checks it
examples/plugin-c/              # the plugin ABI written in C and built with MSVC alone
scripts/                        # build, smoke test, PE audit, documentation audits
site/                           # the documentation site: its content, its config and its build
```

The raw runtime stubs carry no product resources: icons, version info, and branding all come from your project configuration, injected by the builder, so one runtime can produce setups for any product.

A project leaving NSIS starts at [Migrating from NSIS](/MIGRATION_FROM_NSIS/): that page says what every construct becomes here, and `examples/nsis-migration/migrated/` is a buildable project written to it.

## A product project

```text
product-installer/
├── installer_config.json
├── layouts/                    # XML pages
├── assets/                     # backgrounds, buttons, icons
├── locales/                    # one JSON file per language
├── scripts/                    # optional install/uninstall logic
└── payload/app.7z              # or a ZIP archive
```

The builder resolves every configured path relative to the project folder. It collects three directories, `layouts`, `assets` and `locales`; the optional `scripts` directory is collected too, as is the payload you name in the configuration.

## Build outputs

```text
target/release/
├── nano-installer-native-x64.exe
├── nano-installer-gui-x64.exe
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

A build asked for `--msi` writes one more package beside the setup it wrapped, for an estate to deploy as MSI.

`target/x86_64-win7-windows-msvc/` is only Cargo's cross-target cache, not a second set of release files. Example setups land in `examples/TapTap/dist/`, and they are not part of a release.

## Documentation site

The published site is an [Astro](https://astro.build/) project built with [Cloudflare Nimbus](https://nimbus-docs.com/), and it lives under `site/`. A page is a Markdown file. `site/src/content/docs/X.md` is served at `/X/`, and its twin in `site/src/content/zh-CN/X.md` at `/zh-CN/X/`. `site/astro.config.ts` holds the site's settings and the sidebar, one group list per language.

`site/dist/` is what the build writes and what GitHub Pages publishes, under `/nano-installer/`. This is a project site rather than an account site, so `base` in that config is `/nano-installer`; Astro keeps the output itself in `dist/` instead of nesting it under the base, and that is why the workflow uploads `site/dist` as it is.

Every page is served as Markdown beside its HTML, at `<page>/index.md`, and the build writes the files a reader that is not a browser needs:

| File | What it holds |
| --- | --- |
| `/llms.txt` | Every English page, one line each, linked to that page's own Markdown version. `/zh-CN/llms.txt` lists the Chinese pages the same way |
| `/llms-full.txt` | The whole documentation in one document, both languages in it, the case and coverage tables included |
| `/robots.txt` | Everything public, and the sitemap's address, which is `/sitemap-index.xml` |
| `/sitemap-index.xml` | The same page list as XML, split into numbered sitemaps |
| `/og.png`, `/og/<page>.png` | A social card for the site and for every page |

None of them is maintained by hand. Each one is written from the content tree during the build, so a page cannot go missing from the index, and there is no second list to keep in step. A new page is its file under `site/src/content/`, plus a line in the sidebar list in `site/astro.config.ts`.

A reader crosses between the two languages at the site root, `/`. It is a chooser with one card per tree, and the sidebar shows only the language of the page being read. Three checks answer for this, and the docs workflow runs them around its build. `scripts/audit_docs_languages.ps1` fails when a page in one tree has no page in the other, and `scripts/audit_docs_sidebar.ps1` fails when a page is missing from the sidebar or a sidebar row points at no page. The build itself fails when a page's frontmatter does not validate. Links come last, because an anchor exists only once its heading has been rendered: `scripts/audit_docs_links.ps1` reads the built site, and the integration's own link rule cannot do that job here, since it reads only `.mdx` while these pages are `.md`.

`.well-known` is the one thing this site cannot do where a client expects it. Nimbus writes the agent-discovery files, `/.well-known/ai-catalog.json` and `/.well-known/ard.json`, and warns that they belong at the origin root. A GitHub Pages project site is served under `/nano-installer/`, so the origin root belongs to the account rather than to this project. A client that looks at `https://nick2781.github.io/.well-known/` therefore finds nothing. Doing better needs a custom domain, which this project does not have, so the files ship under the project path and the build's warning stands.

`docs/AGENTS.md` is the map of the documentation itself. It says which page answers which question, how the two languages relate, and where the machine-readable entry points are. The repository keeps it at the top of `docs/` rather than inside a tree, because the repository's conventions ask for it there, and its Chinese side is `docs/zh-CN/AGENTS.md`. The same map is published as a page, at `/AGENTS/` and `/zh-CN/AGENTS/`, and every other page has its twin beside it in the other tree.
