# Documentation map for agents

This page says which document answers which question, so a reader -- a person or an agent -- can go
straight to that file instead of walking the whole site.

**Every page has a twin.** The published pages come in pairs: `site/src/content/docs/X.mdx` and
`site/src/content/zh-CN/X.mdx` are the same document, served at `/X/` and `/zh-CN/X/`. No page is written
in one language only, and `scripts/audit_docs_languages.ps1` holds that rule: it fails when a page in one
tree has no page in the other, and it pairs this map with `zh-CN/AGENTS.md` the same way.

The repository's own conventions put this map at the top of `docs/` rather than inside a content tree,
which makes it the only English file under `docs/`. Its Chinese side is
[`zh-CN/AGENTS.md`](zh-CN/AGENTS.md), and the same map is published as a page as well, at `/project/agents/` and
`/zh-CN/project/agents/`.

## Take these first

| File | What it holds |
| --- | --- |
| [`/llms.txt`](https://nick2781.github.io/nano-installer/llms.txt) | Every English page, one line each, linked to that page's own Markdown version. Fetch this one first |
| [`/llms-full.txt`](https://nick2781.github.io/nano-installer/llms-full.txt) | The whole documentation in one document, both languages in it, the case and coverage tables included |
| [`/zh-CN/llms.txt`](https://nick2781.github.io/nano-installer/zh-CN/llms.txt) | The Chinese pages, listed the same way |
| [`/sitemap-index.xml`](https://nick2781.github.io/nano-installer/sitemap-index.xml), [`/robots.txt`](https://nick2781.github.io/nano-installer/robots.txt) | The same page list for crawlers |

Fetch `<page>/index.md` when a model is the reader: every page is served as Markdown beside its HTML at
that path, and the page is rendered from that file. A reader working from the repository instead takes
the page's own file under `site/src/content/`.

## Which file answers which question

| Question | English | 中文 |
| --- | --- | --- |
| What is this, and can I ship with it yet? | [README](../site/src/content/docs/index.mdx), [Production status](../site/src/content/docs/project/status.mdx) | [中文文档](../site/src/content/zh-CN/index.mdx)、[当前生产状态](../site/src/content/zh-CN/project/status.mdx) |
| How do I build my first setup? | [Quick start](../site/src/content/docs/quick-start.mdx) | [快速开始](../site/src/content/zh-CN/quick-start.mdx) |
| Which settings exist, and what do they change? | [Configuration reference](../site/src/content/docs/reference/configuration.mdx) | [配置参考](../site/src/content/zh-CN/reference/configuration.mdx) |
| How do I write pages and controls? | [Page layout](../site/src/content/docs/guides/page-layout.mdx) | [页面布局](../site/src/content/zh-CN/guides/page-layout.mdx) |
| How do I add or change a language? | [Languages](../site/src/content/docs/guides/languages.mdx) | [多语言](../site/src/content/zh-CN/guides/languages.mdx) |
| How do I run my own install and uninstall steps? | [Custom steps](../site/src/content/docs/guides/custom-steps.mdx) | [自定义步骤](../site/src/content/zh-CN/guides/custom-steps.mdx) |
| How do I ship a plugin, and what may it do? | [Plugin ABI](../site/src/content/docs/guides/plugins.mdx) | [插件 ABI](../site/src/content/zh-CN/guides/plugins.mdx) |
| I am coming from NSIS: what becomes what? | [Migrating from NSIS](../site/src/content/docs/guides/migrating-from-nsis.mdx) | [从 NSIS 迁移](../site/src/content/zh-CN/guides/migrating-from-nsis.mdx) |
| How does the installer work inside? | [Architecture](../site/src/content/docs/project/architecture.mdx) | [架构](../site/src/content/zh-CN/project/architecture.mdx) |
| Which Windows versions, and which system calls? | [Windows compatibility](../site/src/content/docs/reference/windows-compatibility.mdx) | [Windows 兼容性](../site/src/content/zh-CN/reference/windows-compatibility.mdx) |
| How do I build a project without the command line? | [Visual builder](../site/src/content/docs/guides/visual-builder.mdx) | [可视化构建](../site/src/content/zh-CN/guides/visual-builder.mdx) |
| How is it built and released? | [Build and release](../site/src/content/docs/reference/build-and-release.mdx), [Project layout](../site/src/content/docs/reference/project-structure.mdx) | [构建与发布](../site/src/content/zh-CN/reference/build-and-release.mdx)、[项目结构](../site/src/content/zh-CN/reference/project-structure.mdx) |
| What is tested, and what is only promised? | [Test plan](../site/src/content/docs/project/testing.mdx), [Test coverage](../site/src/content/docs/project/coverage.mdx), [Test cases](../site/src/content/docs/project/cases.mdx) | [测试计划](../site/src/content/zh-CN/project/testing.mdx)、[测试覆盖](../site/src/content/zh-CN/project/coverage.mdx)、[用例说明](../site/src/content/zh-CN/project/cases.mdx) |

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

## Where the pages and the index come from

The published site is an Astro project built with Cloudflare Nimbus, and it lives under `site/`. Its pages
are `site/src/content/docs/**/*.mdx` and `site/src/content/zh-CN/**/*.mdx`, and `site/astro.config.ts` holds the
site's settings and the sidebar's two language groups. The build writes `site/dist/`, which is what GitHub
Pages publishes under `/nano-installer/`. The docs workflow installs the toolchain those files name,
builds in `site/`, and uploads that directory.

The same build writes the discovery files, so nobody maintains a data file for them: `llms.txt`,
`llms-full.txt`, `robots.txt`, a `sitemap-index.xml` sitemap, and a Markdown version and an OG card for
every page. A new page therefore needs its file under `site/src/content/`, a twin in the other tree, and a
line in the sidebar list. `title` in the frontmatter is what the sidebar and the index both call it, so
the page's heading is maintained in one place only. The generator and the data file it read are gone with
the docsify site, because the build cannot drift from the pages it writes those files from.

`.well-known` is the one thing the site cannot do where a client expects it. Nimbus writes
`/.well-known/ai-catalog.json` and `/.well-known/ard.json` and warns that they belong at the origin root,
while a GitHub Pages project site is served under `/nano-installer/`, so the origin root belongs to the
account rather than to this project. Putting them there needs a custom domain, which this project does
not have; the files ship under the project path and the build's warning stands.

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
  targets: a rewrite changes wording only. In the frontmatter only `description` may change -- it is the
  line a search result shows -- while `title` and `sidebar` stay, and a page's path is decided by where
  its file sits. Check with
  `scripts/audit_docs_languages.ps1` for the pairs, with the site's own build (`pnpm build` in `site/`)
  for the frontmatter -- that build does validate it -- and with `scripts/audit_docs_links.ps1` for the
  links, which reads the built site: `astro build` evaluates no authoring rule, so the integration's own
  link rule runs under `nimbus-docs lint`, and the anchor a link names exists only once its heading has
  been rendered.
- **The Chinese pages consult [Tech-Doc-Style-Chinese](https://github.com/fenng/Tech-Doc-Style-Chinese) as
  an external reference, not as a second standard.** Where the two disagree, this section wins: pronouns
  (it, this, these) and the hard-wrapped source lines are the two places it does.

The English and Chinese files are one document: same paragraphs, same order, same claim in each.
