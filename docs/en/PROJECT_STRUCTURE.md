# Project layout

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
├── index.html                  # the docsify shell the site is served from
├── llms.txt                    # every page, listed for an agent to read
├── robots.txt, sitemap.xml     # the same page list for crawlers
├── en/                         # English documentation
└── zh-CN/                      # Chinese documentation
examples/TapTap/                # validation project
examples/nsis-migration/        # a project migrated off NSIS, with the script that checks it
examples/plugin-c/              # the plugin ABI written in C and built with MSVC alone
scripts/                        # build, smoke test, PE audit, docs index generator
```

The raw runtime stubs carry no product resources: icons, version info, and branding all come from your project configuration, injected by the builder, so one runtime can produce setups for any product.

A project leaving NSIS starts at [Migrating from NSIS](MIGRATION_FROM_NSIS.md): that page says what every construct becomes here, and `examples/nsis-migration/migrated/` is a buildable project written to it.

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

The builder resolves every configured path relative to the project folder. It collects three directories: `layouts`, `assets`, and `locales`, and the optional `scripts` directory is collected too, as is the payload you name in the configuration.

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

`target/x86_64-win7-windows-msvc/` is only Cargo's cross-target cache, not a second set of release files; example setups land in `examples/TapTap/dist/` and are not part of a release.

## Documentation site

The published site is a [docsify](https://docsify.js.org/) shell: `docs/index.html` renders Markdown in the browser, and it is the only HTML GitHub Pages serves. Every page is also a Markdown file published beside that shell, so the site reads without a browser too, while a coding agent has those files instead; the pages it renders are the sources, and nothing is generated for the reader.

Four files make the pages findable to something that cannot run the viewer. `scripts/build_docs_index.ps1` writes them from `scripts/docs_index.json`:

| File | What it holds |
| --- | --- |
| `docs/llms.txt` | Every page, one line each, linked to the page's own Markdown file, with a one-line description. Repository files outside the site get their own section |
| `docs/llms-full.txt` | The pages of the groups that ask to be in it, which is the English documentation, in one document. Each is introduced by its own address. The coverage table is left out: it is a table to look things up in rather than reading |
| `docs/robots.txt` | Everything public. The sitemap's address is written here too |
| `docs/sitemap.xml` | The same list as XML. Its entries are the Markdown files, not the viewer's `#/` addresses. A crawler cannot fetch a fragment |

`.\scripts\build_docs_index.ps1 -Verify` is the check, and it is the same code that writes them, so the two cannot disagree. Any of these fails the check: a page has no entry in `scripts/docs_index.json`, an entry names a file that is not there, a title or description is empty, a title disagrees with the page's own `# heading`, a repository file has moved, a page belongs to no group, a non-page name is written into the one-document file's exclusion list, or no page would be in that file at all. Adding a page means adding its line to that data file, in the same change; the group a page is in decides whether it joins `llms-full.txt`, and one page may override that. The landing page is a chooser between the two languages, not a page to read, and that is how it stays out.

The sidebar files are navigation rather than pages: `_sidebar.md` and `_navbar.md`, and the index leaves out every file whose name begins with an underscore.

A reader crosses between the two trees with the switch in the site's top bar, which is script in `docs/index.html` rather than a link written into every page: it states the language of the page it is on, picking the other one lands you on the same page, and the names it can pair are listed in that file. Two checks hold the list to the trees: `scripts/audit_docs_languages.ps1` compares the pairs with the files in `docs/en` and `docs/zh-CN`, and `scripts/check_language_switch.js` runs the switch itself against a stand-in for a browser, because a runner has none. The docs workflow runs both, beside the index check.

`docs/AGENTS.md` is the map of the documentation itself: it says which page answers which question, how the two languages relate, and where the machine-readable entry points are, and it sits at the top of `docs/` rather than inside a tree. Its Chinese side is `docs/zh-CN/AGENTS.md`, and every other page has its twin beside it in the other tree.
