# This Nimbus docs site

> `CLAUDE.md` delegates here. Keep project instructions canonical in this file.

Astro-based docs for Nano Installer, built with Cloudflare Nimbus and published by GitHub Pages at
https://nick2781.github.io/nano-installer/. The `nimbus-docs` package handles content schemas,
sidebar/TOC, Markdown→text, build hooks, and the `nimbus-docs` CLI. Install dependencies first, then
run it as `npx @cloudflare/nimbus-docs <command>`: npx uses the project's installed version. Before an
install, npx fetches the latest release instead, which may differ from the site's version and give
different results. The scoped name keeps npx from fetching the unrelated `nimbus-docs` package.
Everything in `src/` is yours to edit.

Pages are MDX, and they come in pairs: `src/content/docs/<path>.mdx` is served at `/<path>/`, and
its twin `src/content/zh-CN/<path>.mdx` at `/zh-CN/<path>/`. The paths are lowercase and nested one
level per section (`guides/`, `reference/`, `project/`), each section's own `index.mdx` is its landing
page, and the two trees' roots are `/` and `/zh-CN/`. Nothing is written in one language only.

## File layout

```text
astro.config.ts              # nimbus + defineNimbusConfig: site, base, and the sidebar for both languages
nimbus.json                  # records the last reviewed Nimbus package version
wrangler.jsonc               # the optional Cloudflare preview path; publishing is GitHub Pages
src/
├── components.ts            # MDX globals — components every .mdx can use without an import
├── components/              # AgentDirective, Header, LanguageSwitch, Render, WebMCP + ui/<slug>/
├── content/
│   ├── docs/**/*.mdx        # the English tree, served at /, one section per directory
│   ├── zh-CN/**/*.mdx       # the Chinese tree, served at /zh-CN/, the same paths
│   └── partials/*.mdx       # shared MDX, pulled into a page with <Render file="..." />
├── content.config.ts        # registers docsCollection() for docs and zh-CN, plus partialsCollection()
├── layouts/                 # BaseLayout (NimbusHead), DocsLayout (sidebar/TOC/breadcrumbs)
├── lib/cn.ts                # Tailwind className merger
├── lib/languages.ts         # the two collections, their mount prefixes, and the per-language sidebar
├── pages/
│   ├── [...slug].astro          # English pages
│   ├── zh-CN/[...slug].astro    # Chinese pages
│   ├── [...slug]/index.md.ts    # Markdown version of every page, all collections
│   ├── [...slug]/index.mdx.ts   # authored source of every page, all collections
│   ├── [section]/llms.txt.ts    # per-section llms.txt
│   ├── 404.astro
│   ├── llms.txt.ts
│   ├── llms-full.txt.ts
│   ├── nimbus-api/coordinates.json.ts  # API citation manifest
│   ├── og.png.ts                # site-level OG card
│   ├── og/
│   │   ├── _og-card-config.ts   # shared OG theme tokens (underscore = not a route)
│   │   └── [...slug].ts         # per-page OG cards
│   └── robots.txt.ts
└── styles/                  # globals.css, prose.css
```

`src/content/partials/example.mdx` is the starter's sample partial, and no page renders it yet.
`src/pages/zh-CN/[...slug].astro` is why the Chinese tree has a route file of its own: Nimbus mounts a
non-primary collection at `/<collection name>`.

## Writing docs

Frontmatter validates against the schema from `docsCollection()` (`@cloudflare/nimbus-docs/content`).
Required: `title`. Every page here also carries `description` for the search result and the share
card, and `sidebar.order` for its place in the group. No page carries a `slug`: a page's path inside
its tree is its route, so `src/content/docs/guides/page-layout.mdx` is served at
`/guides/page-layout/` and `src/content/docs/guides/index.mdx` at `/guides/`.

```md
---
title: My page
description: One-line summary.
sidebar:
  order: 100
---

Content here. The page H1 comes from `title` — don't repeat it in the body.

## Section heading
```

Rules:

- **A page is `.mdx`, and MDX reads the body.** Components work in it: an import, a registered global such as `<Aside>`, or `<Render file="..." />`. Raw HTML keeps working too, which is how the section `index.mdx` landing pages draw their card grids. The cost is that MDX reads a bare `<` as the start of a tag and a bare `{` as the expression opener, so a literal one in prose is escaped as `\<` or `\{`; a code fence or an inline code span needs no escape.
- **Components are PascalCase.** Register one in `src/components.ts` to use it in every `.mdx` without an import, or import it in the file that uses it. A pre-build validator catches unresolved names with a "did you mean" hint.
- **Partials use `<Render file="..." />`.** Don't import `.mdx` directly. Shared content lives in `src/content/partials/<slug>.mdx`.
- **Icons use Nimbus's `Icon` + Phosphor.** `<Icon name="ph:<glyph>" class="w-4 h-4" />` from `@cloudflare/nimbus-docs/components/Icon.astro`. Glyphs: [phosphoricons.com](https://phosphoricons.com).
- **Don't remove `<AgentDirective />` from `BaseLayout.astro`.** It points agents at `/llms.txt`.
- **Write both trees in one change.** A page's twin is part of the change, and `sidebarItems` in `astro.config.ts` carries one group list per language.

## Adding things

| Goal | Action |
|---|---|
| New doc page | Create `src/content/docs/<page>.mdx` — or `src/content/docs/<section>/<page>.mdx` for a page inside a section, whose landing page is that section's `index.mdx` — create its twin in `src/content/zh-CN/`, then add a line for each to the matching group in `sidebarItems` in `astro.config.ts`. |
| Move a page | Move the file in both trees and add the old address to `redirects` in `astro.config.ts`, so the published URL it used to have keeps working. |
| New partial | Create `src/content/partials/<slug>.mdx`. Use via `<Render file="<slug>" />`. |
| UI from registry | `npx @cloudflare/nimbus-docs add <slug>`. Register in `src/components.ts` if used in MDX. |
| Feature recipe | `npx @cloudflare/nimbus-docs add <feature-slug> --print`. Prints the recipe for you to follow; it changes no files itself. |
| Check it builds | `pnpm build` in `site/`, then `npx @cloudflare/nimbus-docs check` — build-free preflight (env + structure + authoring + types). `--json` for an agent loop, `--fix` to repair what's safe. |
| Custom page route | Add a file under `src/pages/`. |
| Custom OG style | Edit `src/pages/og/_og-card-config.ts`. |
| Check for updates | `npx @cloudflare/nimbus-docs outdated` — starter files behind their tag + registry components behind. |
| Upgrade Nimbus | Update the package, then run `npx @cloudflare/nimbus-docs migrate --dry-run --diff`. Review every change and required manual step before applying. |
| Upgrade a starter file | `npx @cloudflare/nimbus-docs diff <file>` to review, `diff --apply <file>` to pull a clean upstream change. |
| Upgrade a registry component | `npx @cloudflare/nimbus-docs add <slug> --overwrite`, then review with `git diff`. |

Sätteri is Nimbus's default Markdown and MDX processor. Extend it using `markdown.mdastPlugins` for Markdown AST transformations or `markdown.hastPlugins` for HTML AST transformations. This site registers `tableScroll()` from `@cloudflare/nimbus-docs/markdown` that way, so a wide table scrolls instead of overflowing the page.
If the site replaces Sätteri with another processor, set `admonitions: false` and keep that processor's existing callout implementation.

List installable items: `npx @cloudflare/nimbus-docs list`.

## Agent interfaces

Read the release-matched guide at [`node_modules/@cloudflare/nimbus-docs/dist/docs-for-agents.md`](node_modules/@cloudflare/nimbus-docs/dist/docs-for-agents.md), especially for preview packages. Nimbus publishes discovery automatically; add project-owned Skills under `skills/<name>/SKILL.md`. Keep `<WebMCP />` in `BaseLayout.astro` for browser-agent search with default Pagefind. Markdown negotiation applies to request-rendered pages; static hosting and response headers follow the deployment target's setup.

## Upgrading Nimbus

Keep `nimbus.json` committed. Its `lastReviewedNimbusVersion` is the baseline Nimbus uses to select the versioned reviews crossed by a package upgrade; state-detected migrations come from the current project files. It is not a package pin and should not be edited by hand.

1. Update `@cloudflare/nimbus-docs` with the project's package manager.
2. Preview the complete plan with `npx @cloudflare/nimbus-docs migrate --dry-run --diff`. If no baseline exists yet, add `--from <previous-version>`.
3. Review every versioned entry and resolve each blocked/manual item.
4. Apply safe edits only with explicit consent: `npx @cloudflare/nimbus-docs migrate --yes`. Review the resulting diff, then rerun the preview.
5. When no migration remains, run `npx @cloudflare/nimbus-docs migrate --yes` again to record the completed review in `nimbus.json`.
6. Pull in starter fixes: `npx @cloudflare/nimbus-docs outdated` lists starter files that changed upstream. For each one, review it with `npx @cloudflare/nimbus-docs diff <file>`, and take a clean update with `diff --apply <file>`. Merge files you've edited by hand. `migrate` doesn't cover these.
7. Run the project's typecheck and production build, then run `npx @cloudflare/nimbus-docs check` again for post-build coverage.

Except for task-printing mode (`--print`), `migrate` exits nonzero while work or review remains; that is a pending-upgrade signal, not necessarily a command failure. Never skip versions by changing `nimbus.json` directly.

## Audit this site

The repository's own gate, in the order the docs workflow runs it:

- `.\scripts\audit_docs_languages.ps1` — every page in one tree has its twin, and the documentation map is a pair.
- `.\scripts\audit_docs_sidebar.ps1` — every page is listed once in its own language's groups, every row answers to a page, and no page is listed twice.
- `pnpm build` in `site/` — the build and the frontmatter check, because `nimbus/frontmatter-shape` and `nimbus/internal-link` are configured as errors. The same run writes `llms.txt`, `llms-full.txt`, `robots.txt`, the sitemap, and a Markdown version and an OG card for every page.
- `.\scripts\audit_docs_links.ps1` — reads `site/dist/` after the build. `astro build` evaluates no authoring rule, so the integration's own link rule runs under `nimbus-docs lint` instead; an anchor is checked here, because it exists only once its heading has been rendered.
- `pnpm typecheck` (`astro check`) and `pnpm lint:docs` (`nimbus-docs lint`) when a change touches types or the authored rules.

`.github/workflows/docs.yml` runs the two audits first, builds, audits the links, and publishes
`site/dist` to GitHub Pages on a push to `main` that touches `site/**`. `site` and `base` in
`astro.config.ts` are already the production pair, origin and repository path, so every published page
lives under `/nano-installer/`.

Then run `npx @cloudflare/nimbus-docs check --json`. It runs the environment, structural, authoring, and type checks build-free — config validity, `site` placeholder, route collisions, MDX component resolution, the lint rules, and a `tsc` type-check — and returns three top-level signals plus per-scope detail:

- **`status`** (`passed` | `failed` | `partial`) and **`readiness`** (`buildable` | `blocked` | `unknown`) are the primary signals. `status` is the whole-run verdict; `readiness` answers "does env + structure say it builds?". `ok` (=== zero errors) is kept for back-compat only.
- **`findings[{scope,code,severity,file,line,message,fixable,fix}]`** are problems we evaluated. Apply each `fix` (or `check --fix`).
- **`scopes[].notes[{code,reason,requiresBuild?,requiresInput?}]`** are checks we *couldn't* evaluate yet (e.g. types before a build). A note is never a finding and never carries a `fix` — you resolve it by making the missing thing exist (usually a build), not by `--fix`. `summary.notes` counts them.

Loop until `status !== "failed"` and no finding has a `fix` without `fix.requiresInput` — `summary.fixable` also counts fixes that need input (a placeholder `site`), which `--fix` can't apply for you. An error with no `fix` (such as `nimbus/dependencies-missing`) ends the loop too: do what its message says (install dependencies), then run `check` again, or stop and report it. A `partial` run with nothing left to fix is a **stop** (optionally build, then re-check), not a `--fix` retry. Exit is `1` only when `status` is `"failed"`. For full coverage (types + link-checking) run a build first, then `check` again. With server output, `check` stays `partial` with the note `nimbus/request-rendering-build-required` even after a build: it doesn't verify request-rendered pages, so a passing production build is the gate.

Then walk the categories below for what `check` doesn't cover yet — route-file existence, registry hygiene, the AI surface, post-build search, and Cloudflare config. Emit findings as:

```
- [error|warn|info] FILE:LINE — what + why + fix.
```

End with `Summary: N errors, N warnings.`

- **Config** — `astro.config.ts` calls `nimbus(defineNimbusConfig({ ... }))`; `site` is the production URL and `base` is `/nano-installer`; `editPattern` (if set) contains `{path}`; `output:` matches the deploy target.
- **Content** — `content.config.ts` registers `docsCollection()` for `docs` and, with `base: "zh-CN"`, for `zh-CN`, plus `partialsCollection()`; every page is an `.mdx` in one of the two trees; frontmatter validates.
- **Sidebar** — every sidebar ref resolves to a content entry; no orphans; no slug collisions. Both languages' groups live in one `sidebarItems` list and are filtered per page by `src/lib/languages.ts`, so read both lists.
- **MDX** — every PascalCase component in `*.mdx` is registered or imported; every `<Render file=...>` resolves; code-fence languages are valid.
- **Routes** — `[...slug].astro`, `zh-CN/[...slug].astro`, `llms.txt.ts`, `llms-full.txt.ts`, `robots.txt.ts`, `[...slug]/index.md.ts`, `[...slug]/index.mdx.ts`, `og.png.ts`, `og/[...slug].ts` all exist.
- **Registry hygiene** — every `src/components/ui/<slug>/` is either MDX-registered or imported in `src/`; transitive deps (`lib/cn.ts`, etc.) exist.
- **AI surface** — `<AgentDirective />` renders in `BaseLayout.astro`; doc `<head>` has `<link rel="alternate" type="text/markdown" ...>`.
- **Search** — `data-pagefind-body` is on the docs main wrapper; after a production build, `dist/pagefind/` exists with ≥1 indexed page.
- **Cloudflare** (the optional path only) — this repository publishes to GitHub Pages, and `wrangler.jsonc` is what `pnpm preview:cf` and `pnpm deploy` use: `name` and `compatibility_date` are set, and static output wants `assets.directory = "./dist"` with `assets.not_found_handling = "404-page"`. Server output (`@astrojs/cloudflare`): no `assets.directory` or `main`; the adapter writes the deploy config to `dist/server/wrangler.json` at build time, so check that file after a build.

## Don't

- Hand-add components under `src/components/ui/` that exists in the Nimbus registry — use `npx @cloudflare/nimbus-docs add <slug>` so deps resolve.
- Import `.mdx` files directly — use `<Render file="..." />`.
- Remove `<AgentDirective />` unless asked.
- Add or move a page without its twin and its sidebar line: `scripts/audit_docs_languages.ps1` and `scripts/audit_docs_sidebar.ps1` are what refuse that, and the docs workflow runs both.

## Project home

[nimbus-docs.com](https://nimbus-docs.com)
