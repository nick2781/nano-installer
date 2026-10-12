---
title: Documentation map for agents
description: This page tells you which document answers which question, so a reader -- a person or an agent -- can go straight to the file that answers it.
slug: AGENTS
sidebar:
  order: 250
---

This page tells you which document answers which question, so a reader -- a person or an agent -- can
go straight to the file that answers it. The whole site does not have to be walked.

**Every page has a twin.** The pages come in pairs: `site/src/content/docs/X.md` and
`site/src/content/zh-CN/X.md` are the same document, served at `/X/` and `/zh-CN/X/`. No page is written
in one language only, and `scripts/audit_docs_languages.ps1` is what holds that: it fails when a page in
one tree has no page in the other.

This map is one of those pairs. The repository keeps the same map as files rather than as pages, at the
top of `docs/` because the repository's own conventions ask for it there: `docs/AGENTS.md` and
`docs/zh-CN/AGENTS.md` are what a coding agent reads from a checkout. The Chinese side of this page is
[给 agent 的文档地图](/zh-CN/AGENTS/).

## Take these first

| File | What it holds |
| --- | --- |
| [`/llms.txt`](/llms.txt) | Every English page, one line each, linked to that page's own Markdown version. Fetch this one first |
| [`/llms-full.txt`](/llms-full.txt) | The whole documentation in one document, both languages, with the case and coverage tables in it, for a reader that would rather fetch once |
| [`/zh-CN/llms.txt`](/zh-CN/llms.txt) | The Chinese pages, listed the same way |
| [`/sitemap-index.xml`](/sitemap-index.xml), [`/robots.txt`](/robots.txt) | The same page list for crawlers |

Every page is served as Markdown beside its HTML, at `<page>/index.md`, and that file is what the page
is rendered from. So fetch it when a model is the reader: the HTML is one rendering of the same source
and nothing more.

## Which file answers which question

| Question | English | 中文 |
| --- | --- | --- |
| What is this, and can I ship with it yet? | [README](/README/), [Production status](/PRODUCTION_STATUS/) | [中文文档](/zh-CN/README/)、[当前生产状态](/zh-CN/PRODUCTION_STATUS/) |
| How do I build my first setup? | [Quick start](/QUICK_START/) | [快速开始](/zh-CN/QUICK_START/) |
| Which settings exist, and what do they change? | [Configuration reference](/CONFIG_REFERENCE/) | [配置参考](/zh-CN/CONFIG_REFERENCE/) |
| How do I write pages and controls? | [Page layout](/XML_LAYOUT_GUIDE/) | [页面布局](/zh-CN/XML_LAYOUT_GUIDE/) |
| How do I add or change a language? | [Languages](/LOCALIZATION/) | [多语言](/zh-CN/LOCALIZATION/) |
| How do I run my own install and uninstall steps? | [Custom steps](/SCRIPT_API/) | [自定义步骤](/zh-CN/SCRIPT_API/) |
| How do I ship a plugin, and what may it do? | [Plugin ABI](/PLUGIN_API/) | [插件 ABI](/zh-CN/PLUGIN_API/) |
| I am coming from NSIS: what becomes what? | [Migrating from NSIS](/MIGRATION_FROM_NSIS/) | [从 NSIS 迁移](/zh-CN/MIGRATION_FROM_NSIS/) |
| How does the installer work inside? | [Architecture](/ARCHITECTURE/) | [架构](/zh-CN/ARCHITECTURE/) |
| Which Windows versions, and which system calls? | [Windows compatibility](/WINDOWS_COMPATIBILITY/) | [Windows 兼容性](/zh-CN/WINDOWS_COMPATIBILITY/) |
| How do I build a project without the command line? | [Visual builder](/GUI/) | [可视化构建](/zh-CN/GUI/) |
| How is it built and released? | [Build and release](/BUILD_AND_RELEASE/), [Project layout](/PROJECT_STRUCTURE/) | [构建与发布](/zh-CN/BUILD_AND_RELEASE/)、[项目结构](/zh-CN/PROJECT_STRUCTURE/) |
| What is tested, and what is only promised? | [Test plan](/TEST_PLAN/), [Test coverage](/TEST_COVERAGE/), [Test cases](/TEST_CASES/) | [测试计划](/zh-CN/TEST_PLAN/)、[测试覆盖](/zh-CN/TEST_COVERAGE/)、[用例说明](/zh-CN/TEST_CASES/) |

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
- [Code signing policy](https://github.com/nick2781/nano-installer/blob/main/CODE_SIGNING.md) — why nothing is signed
  yet, what would be, and who maintains the project.
- [Changelog](https://github.com/nick2781/nano-installer/blob/main/CHANGELOG.md) — what changed in
  each release. Written in Chinese.
- [Documentation map](https://github.com/nick2781/nano-installer/blob/main/docs/AGENTS.md) — this
  page as a file, kept at the top of `docs/` for a reader inside the repository.

## Where the pages and the index come from

The site is an Astro project under `site/`, built with Cloudflare Nimbus. The pages are the Markdown
files in `site/src/content/docs/` and `site/src/content/zh-CN/`, the build writes `dist/`, and that
directory is what GitHub Pages publishes under `/nano-installer/`.

The discovery files come from the same build rather than from a list someone maintains: `llms.txt`,
`llms-full.txt`, `robots.txt`, a `sitemap-index.xml` sitemap, and a Markdown version and an OG image for
every page. So a new page needs its file under `site/src/content/` and a line in the sidebar list in
`site/astro.config.ts`, and nothing else. The index cannot drift from the pages, because it is written
from them.

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
  link targets: a rewrite changes wording only. Check with `scripts/audit_docs_languages.ps1` for the
  pairs and with the site's own build (`pnpm build` in `site/`) for the frontmatter and the links --
  a page whose frontmatter does not validate, or whose internal link goes nowhere, fails the build.
- **The Chinese pages consult [Tech-Doc-Style-Chinese](https://github.com/fenng/Tech-Doc-Style-Chinese)
  as an external reference, not as a second standard.** Where the two disagree, this section wins:
  pronouns (it, this, these) and the hard-wrapped source lines are the two places it does.

The English and Chinese files are one document: same paragraphs, same order, same claim in each.
