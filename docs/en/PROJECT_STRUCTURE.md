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

The raw runtime stubs carry no product resources. The builder injects icons, version info, and
branding from your project configuration, so one runtime can produce setups for any product.

A project leaving NSIS starts at [Migrating from NSIS](MIGRATION_FROM_NSIS.md), which says what every
construct becomes here; `examples/nsis-migration/migrated/` is a buildable project written to it.

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

The builder resolves every configured path relative to the project folder, and collects the whole
`layouts`, `assets`, and `locales` directories, the optional `scripts` directory, and the payload
you name in the configuration.

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

A build asked for `--msi` writes the installer package an estate deploys beside the setup
it wrapped.

`target/x86_64-win7-windows-msvc/` is Cargo's cross-target cache, not a second set of release
files. Example setups land in `examples/TapTap/dist/` and are not part of a release.

## Documentation site

The published site is a [docsify](https://docsify.js.org/) shell: `docs/index.html` renders Markdown
in the browser, and it is the only HTML GitHub Pages serves. Every page is also a Markdown file
published beside that shell, so the site reads without a browser as well -- which is what a coding
agent has instead. The pages it renders are the sources: nothing is generated for the reader.

Three files make the pages findable to something that cannot run the viewer, and
`scripts/build_docs_index.ps1` writes them from `scripts/docs_index.json`:

| File | What it holds |
| --- | --- |
| `docs/llms.txt` | Every page, one line each, linked to the page's own Markdown file with a one-line description; then the repository files outside the site that are worth knowing about |
| `docs/llms-full.txt` | The pages of the groups that ask to be in it -- the English documentation -- in one document, each introduced by its own address. The coverage table is left out because it is a table to look things up in rather than reading |
| `docs/robots.txt` | Everything public, and where the sitemap is |
| `docs/sitemap.xml` | The same list as XML. Its entries are the Markdown files rather than the viewer's fragment addresses, because a fragment is not a page a crawler can fetch |

`.\scripts\build_docs_index.ps1 -Verify` is the check, and it is the same code that writes them, so
the two cannot disagree. A page with no entry in `scripts/docs_index.json`, an entry naming a file
that is not there, an empty title or description, a title that disagrees with the page's own
`# heading`, a repository file that has moved, a page that belongs to no group, a name excluded from
the one-document file that is not a page, and no page that would be in that file at all are all
failures. Adding a page therefore means adding its line to that data file, in the same change -- and
it joins `llms-full.txt` because of the group it is in, not because a list was updated. One page may
override its group's answer, which is how the landing page stays out of it: that page is a chooser
between the two languages rather than a page to read, so it is not an English page.

The sidebar files (`_sidebar.md`, `_navbar.md`) are navigation rather than pages, and the index
leaves out every file whose name begins with an underscore for that reason.

A reader crosses between the two trees with the switch in the site's top bar, which is script in
`docs/index.html` rather than a link written into every page: it states the language of the page it
is on and moves to the same page in the other language. The names it can pair are listed in that
file, and two checks hold the list to the trees. `scripts/audit_docs_languages.ps1` compares the
pairs with the files in `docs/en` and `docs/zh-CN`, and `scripts/check_language_switch.js` runs the
switch itself against a stand-in for a browser, because a runner has none. The docs workflow runs
both, beside the index check.

`docs/AGENTS.md` is the map of the documentation itself: which page answers which question, how the
two languages relate, and where the machine-readable entry points are. It sits at the top of `docs/`
rather than inside a tree, and its Chinese side is `docs/zh-CN/AGENTS.md`; every other page has its
twin beside it in the other tree.
