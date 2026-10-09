# Documentation map for agents

This page tells you which document answers which question, so a reader -- a person or an agent -- can
go straight to the file that answers it. The whole site does not have to be walked.

**Every page has a twin.** The pages here come in pairs: `docs/en/X.md` and `docs/zh-CN/X.md` are the
same document, and the switch in the site's top bar is how a reader crosses between the two. No page
here is written in one language only. `scripts/audit_docs_languages.ps1` is what holds that: it fails
when a page in one tree has no page in the other, or when the switch's own list of pages has drifted
from the files beside it.

This map sits at the top of `docs/` rather than inside a tree, which makes it one of only two files
there; the other is the site's landing page, a chooser rather than a document. Its Chinese side is
[`zh-CN/AGENTS.md`](zh-CN/AGENTS.md).

## Take these first

| File | What it holds |
| --- | --- |
| [`/llms.txt`](llms.txt) | Every page, one line each, linked to the page's own Markdown file. Fetch this one first |
| [`/llms-full.txt`](llms-full.txt) | The English pages in one document, for a reader that would rather fetch once. The test coverage table is left out: it is a table to look things up in, not to read. `llms.txt` links it |
| [`/sitemap.xml`](sitemap.xml), [`/robots.txt`](robots.txt) | The same page list for crawlers |

Every page here is a Markdown file, published beside the HTML viewer as `text/markdown`; that HTML is
one viewer over all of them and nothing more. So fetch the file itself, because the Markdown is the
source and no page is generated for the reader.

## Which file answers which question

| Question | English | 中文 |
| --- | --- | --- |
| What is this, and can I ship with it yet? | [README](en/README.md), [Production status](en/PRODUCTION_STATUS.md) | [中文文档](zh-CN/README.md)、[当前生产状态](zh-CN/PRODUCTION_STATUS.md) |
| How do I build my first setup? | [Quick start](en/QUICK_START.md) | [快速开始](zh-CN/QUICK_START.md) |
| Which settings exist, and what do they change? | [Configuration reference](en/CONFIG_REFERENCE.md) | [配置参考](zh-CN/CONFIG_REFERENCE.md) |
| How do I write pages and controls? | [Page layout](en/XML_LAYOUT_GUIDE.md) | [页面布局](zh-CN/XML_LAYOUT_GUIDE.md) |
| How do I add or change a language? | [Languages](en/LOCALIZATION.md) | [多语言](zh-CN/LOCALIZATION.md) |
| How do I run my own install and uninstall steps? | [Custom steps](en/SCRIPT_API.md) | [自定义步骤](zh-CN/SCRIPT_API.md) |
| How do I ship a plugin, and what may it do? | [Plugin ABI](en/PLUGIN_API.md) | [插件 ABI](zh-CN/PLUGIN_API.md) |
| I am coming from NSIS: what becomes what? | [Migrating from NSIS](en/MIGRATION_FROM_NSIS.md) | [从 NSIS 迁移](zh-CN/MIGRATION_FROM_NSIS.md) |
| How does the installer work inside? | [Architecture](en/ARCHITECTURE.md) | [架构](zh-CN/ARCHITECTURE.md) |
| Which Windows versions, and which system calls? | [Windows compatibility](en/WINDOWS_COMPATIBILITY.md) | [Windows 兼容性](zh-CN/WINDOWS_COMPATIBILITY.md) |
| How is it built and released? | [Build and release](en/BUILD_AND_RELEASE.md), [Project layout](en/PROJECT_STRUCTURE.md) | [构建与发布](zh-CN/BUILD_AND_RELEASE.md)、[项目结构](zh-CN/PROJECT_STRUCTURE.md) |
| What is tested, and what is only promised? | [Test plan](en/TEST_PLAN.md), [Test coverage](en/TEST_COVERAGE.md), [Test cases](en/TEST_CASES.md) | [测试计划](zh-CN/TEST_PLAN.md)、[测试覆盖](zh-CN/TEST_COVERAGE.md)、[用例说明](zh-CN/TEST_CASES.md) |

## In the repository

These files sit outside the published site, so a link to any of them leaves the site:

- [README](https://github.com/nick2781/nano-installer/blob/main/README.md) — what the product is,
  what it does today, and what blocks a release.
- [Plugin ABI header](https://github.com/nick2781/nano-installer/blob/main/include/nano_plugin.h) —
  the three rules a plugin has to keep, and the host table a third-party DLL is built against.
- [Examples](https://github.com/nick2781/nano-installer/blob/main/examples/README.md) — the three
  example projects and what each one proves. `examples/plugin-c/sample.c` is the ABI in C.
- [Contributing](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md) — how to set
  up a working tree, the checks a change has to pass, and the rules this repository keeps.
- [Repository conventions](https://github.com/nick2781/nano-installer/blob/main/AGENTS.md) — the
  layout, the build and test commands, and the change boundaries a coding agent has to respect.
- [Security policy](https://github.com/nick2781/nano-installer/blob/main/SECURITY.md) — private
  reporting, and what is knowingly not a vulnerability.
- [Changelog](https://github.com/nick2781/nano-installer/blob/main/CHANGELOG.md) — what changed in
  each release. Written in Chinese.

## Where the index comes from

`scripts/build_docs_index.ps1` writes `llms.txt`, `llms-full.txt`, `robots.txt` and `sitemap.xml` from
`scripts/docs_index.json`, and `-Verify` runs the same code, so the writer and the check cannot
disagree. A new page therefore needs a line in that data file, with a title that matches the page's own
`# heading` exactly; without that line the check fails, rather than letting the page vanish from the
index.

Whether a page is in `llms-full.txt` is the group's answer, but a single page can override it: the
site's landing page, `README.md`, is a chooser between the two languages rather than a page to read,
and that override is what keeps it out. The case tables stay out for the same reason the coverage table
does -- they are tables to look things up in, not to read.

## How these pages are written

The map above says which page holds what. This section is about the writing itself, for a new
page and for an edit to an old one.

- **One idea per sentence, and vary the length.** A conclusion, a warning or a step can be short;
  background, conditions and consequences get the room they need, with their clauses joined by the
  commas they were missing. A page whose every sentence is twenty words long has been written badly.
- **Keep the connectives**: so, but, because, while, which is why. They carry the logic, and a
  paragraph stripped of them is a list wearing prose.
- **One paragraph, one point.** Say what happens, then the condition, the exception or the conse-
  quence; from the second sentence on, refer back to the subject (it, this, they) instead of naming
  it again.
- **Name things the way the platform does.** A quote in a log, a key in the registry, a page in the
  layout: use the name the documentation and the tooling already use.
- **Do not repeat a word into the next sentence**, and never let a parenthesis restate the word
  before it. "The application files (your application files, a zip or 7z archive)" is one idea told
  twice.
- **Facts, numbers, commands, paths and field names do not move.** Neither do headings, anchors or
  link targets: a rewrite changes wording only. Check with `scripts/audit_docs_languages.ps1` and
  `scripts/build_docs_index.ps1 -Verify` -- the index title has to match the page's own `# heading`
exactly.

The English and Chinese files are one document: same paragraphs, same order, same claim in each.
