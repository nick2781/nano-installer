---
title: Documentation map for agents
description: This map says which document answers which question, so a person or an agent can go straight to the right file instead of walking the whole site.
slug: AGENTS
sidebar:
  order: 250
---

This page says which document answers which question, so a reader -- a person or an agent -- can go
straight to that file instead of walking the whole site.

**Every page has a twin.** The pages come in pairs: `site/src/content/docs/X.md` and
`site/src/content/zh-CN/X.md` are the same document, served at `/X/` and `/zh-CN/X/`. No page is written
in one language only, and `scripts/audit_docs_languages.ps1` holds that rule: it fails when a page in one
tree has no page in the other.

This map is one of those pairs. The repository keeps the same map as files rather than as pages: the
repository's own conventions ask for it at the top of `docs/`, where `docs/AGENTS.md` and
`docs/zh-CN/AGENTS.md` are what a coding agent reads from a checkout. The Chinese side of this page is
[给 agent 的文档地图](/zh-CN/AGENTS/).

## Take these first

| File | What it holds |
| --- | --- |
| [`/llms.txt`](/llms.txt) | Every English page, one line each, linked to that page's own Markdown version. Fetch this one first |
| [`/llms-full.txt`](/llms-full.txt) | The whole documentation in one document, both languages, with the case and coverage tables in it, for a reader that would rather fetch once |
| [`/zh-CN/llms.txt`](/zh-CN/llms.txt) | The Chinese pages, listed the same way |
| [`/sitemap-index.xml`](/sitemap-index.xml), [`/robots.txt`](/robots.txt) | The same page list for crawlers |

Fetch `<page>/index.md` when a model is the reader. Every page is served as Markdown beside its HTML at
that path, and the page is rendered from that file, so the HTML is one rendering of the same source and
nothing more.

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

The site is an Astro project under `site/`, built with Cloudflare Nimbus. Its pages are the Markdown
files in `site/src/content/docs/` and `site/src/content/zh-CN/`, and the build writes them to `dist/`,
which is what GitHub Pages publishes under `/nano-installer/`.

The same build writes the discovery files, so nobody maintains a list of them: `llms.txt`,
`llms-full.txt`, `robots.txt`, a `sitemap-index.xml` sitemap, and a Markdown version and an OG image for
every page. A new page therefore needs its file under `site/src/content/` and a line in the sidebar list
in `site/astro.config.ts`, and nothing else. The index cannot drift from the pages, because the build
writes it from them.

## How these pages are written

The map above says which page holds what. This section is the writing standard itself, for a new page and
for an edit to an old one.

- **Every sentence has a subject and a verb.** A colon followed by a run of nouns is not a sentence yet;
  write it out.
- **Put the conclusion or the core action first.** The condition, the scope and the exception follow it.
- **One idea per sentence, and vary the length.** A step or a warning can be short, while a condition or
  a reason needs its clause joined on -- because, so, while, which is why -- instead of a row of
  fragments.
- **Keep the connectives.** They carry the logic. What goes is the filler: "has already been run
  through", "what is more", "it is worth noting", "in the case of".

Structure counts as much as the sentences:

- Keep a paragraph to three to five sentences and to one idea. A paragraph carrying three points becomes
  three paragraphs.
- Write steps as an ordered list, one step per item.
- Write parallel values, limits and error codes as a bulleted list or a table.
- Give a limit and its consequence a sentence each ("applies only under X", "overwrites Y") instead of
  burying them in a clause.
- Explain a name once on a page. When it has to come up again, say it in one sentence and point at that
  section.
- Do not repeat a word into the next sentence, and do not let a parenthesis restate the word before it:
  "The application files (your application files, a zip or 7z archive)" is one idea told twice.
- Name things the way the platform does. A quote in a log, a key in the registry, a page in the layout:
  use the name the documentation and the tooling already use.

A rewrite is not a shorter page. The conditions and the reasons keep the room they need; what goes is
the repetition and the filler.

Here is the same sentence in three versions:

- **The original (wordy).** The lowest supported version has already been run through once: Windows 7
  SP1 x64, build 7601, in a temporary virtual machine; a setup built by the current builder was installed
  there once, uninstalled once and upgraded once, plus two silent runs.
- **Over-compressed (also wrong, and it reads like a telegram).** Verified on Windows 7 SP1 x64 (build
  7601) in a temporary virtual machine: install, uninstall and upgrade once each, two silent installs.
- **The target.** The lowest supported version has been verified on a real machine. The test used a setup
  built by the current builder, and completed an install, an uninstall and an upgrade on a temporary
  virtual machine running Windows 7 SP1 x64 (build 7601), plus two more silent installs.

When the rewrite is done, a native speaker of that language should read it once and think "a person wrote
this carefully", not "a machine compressed the notes". If a sentence reads awkwardly, put the longer
version back.

- **Facts, numbers, commands, paths and field names do not move.** Neither do headings, anchors or link
  targets: a rewrite changes wording only. In the frontmatter, `description` may change -- it is the line
  a search result shows -- while `title`, `slug` and `sidebar` stay. Check with
  `scripts/audit_docs_languages.ps1` for the pairs, with the site's own build (`pnpm build` in `site/`)
  for the frontmatter -- that build does validate it -- and with `scripts/audit_docs_links.ps1` for the
  links, which reads the built site, because the integration's own link rule reads only `.mdx` and a dead
  link between two `.md` pages is not what fails the build.
- **The Chinese pages consult [Tech-Doc-Style-Chinese](https://github.com/fenng/Tech-Doc-Style-Chinese) as
  an external reference, not as a second standard.** Where the two disagree, this section wins: pronouns
  (it, this, these) and the hard-wrapped source lines are the two places it does.

The English and Chinese files are one document: same paragraphs, same order, same claim in each.
