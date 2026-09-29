# Documentation map for agents

This page says which document answers which question, so a reader -- a person or an agent -- can go
straight to one file instead of walking the site.

**It has no Chinese twin, deliberately.** The pages it maps are paired (`docs/en/X.md` and
`docs/zh-CN/X.md` are the same document), but this one is read by the same agents that already read
`AGENTS.md` and `CONTRIBUTING.md`, which are English, and a translation would drift from the map it
points at. `docs/zh-CN/TEST_CASES.md` is the other document that carries one language on purpose.

## Take these first

| File | What it holds |
| --- | --- |
| [`/llms.txt`](llms.txt) | Every page, one line each, linked to the page's own Markdown file. Fetch this first |
| [`/llms-full.txt`](llms-full.txt) | The English pages in one document, for a reader that would rather fetch once. The test coverage table is left out, because it is a table to look things up in rather than reading, and `llms.txt` links it |
| [`/sitemap.xml`](sitemap.xml), [`/robots.txt`](robots.txt) | The same page list for crawlers |

Every page of this site is a Markdown file served beside the HTML viewer, as `text/markdown`. The HTML
is one viewer over all of them, so fetch the file itself. The Markdown is the source: nothing is
generated for the reader.

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
| What is tested, and what is only promised? | [Test plan](en/TEST_PLAN.md), [Test coverage](en/TEST_COVERAGE.md) | [测试计划](zh-CN/TEST_PLAN.md)、[测试覆盖](zh-CN/TEST_COVERAGE.md)、[用例说明](zh-CN/TEST_CASES.md) |

## In the repository

These sit outside the published site, so the links leave it:

- [README](https://github.com/nick2781/nano-installer/blob/main/README.md) — what the product is,
  what it does today, and what blocks a release.
- [Plugin ABI header](https://github.com/nick2781/nano-installer/blob/main/include/nano_plugin.h) —
  the three rules and the host table a third-party DLL is built against.
- [Examples](https://github.com/nick2781/nano-installer/blob/main/examples/README.md) — the three
  example projects and what each one proves. `examples/plugin-c/sample.c` is the ABI in C.
- [Contributing](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md) — the working
  tree, the checks a change has to pass, and the rules this repository keeps.
- [Repository conventions](https://github.com/nick2781/nano-installer/blob/main/AGENTS.md) — the
  layout, the build and test commands, and the change boundaries a coding agent has to respect.
- [Security policy](https://github.com/nick2781/nano-installer/blob/main/SECURITY.md) — private
  reporting, and what is knowingly not a vulnerability.
- [Changelog](https://github.com/nick2781/nano-installer/blob/main/CHANGELOG.md) — what changed in
  each release. Written in Chinese.

## Where the index comes from

`scripts/build_docs_index.ps1` writes `llms.txt`, `llms-full.txt`, `robots.txt` and `sitemap.xml`
from `scripts/docs_index.json`, and `-Verify` is the same code, so the writer and the check cannot
disagree. A new page therefore needs a line in that data file, titled exactly like the page's own
`# heading`; without one the check fails rather than letting the page vanish from the index. Whether
a page is in `llms-full.txt` is the group's answer, and a single page can override it: the site's
landing page, `README.md`, is written in both languages and is kept out of the English file that way.
