import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import nimbus, {
  defineConfig as defineNimbusConfig,
} from "@cloudflare/nimbus-docs";
import { tableScroll } from "@cloudflare/nimbus-docs/markdown";
import type { SidebarConfigItem } from "@cloudflare/nimbus-docs/types";

// One section list per language. The English groups list primary-collection
// (`docs`) entry ids, which are the pages' paths inside the tree with a trailing
// `/index` dropped (`guides/index.mdx` is the entry `guides`, served at
// `/guides/`); the Chinese groups point at the same pages through the `zh-CN`
// collection's mount path. Both lists follow the same four sections in the same
// order, and every page's twin sits in the same place, so the two rails read in
// the same sequence. `src/lib/languages.ts` and the two catch-all routes keep a
// page from showing the other language's groups.
const sidebarItems = [
  // ---- English (docs) ----
  {
    label: "Get Started",
    items: [
      "index",
      { label: "Quick start", link: "/quick-start", badge: { text: "Start here", variant: "success" } },
      "guides/visual-builder",
    ],
  },
  {
    label: "Guides",
    items: [
      "guides",
      "guides/page-layout",
      "guides/custom-steps",
      "guides/languages",
      "guides/plugins",
      "guides/migrating-from-nsis",
      "guides/distribution",
      "guides/troubleshooting",
    ],
  },
  {
    label: "Reference",
    items: [
      "reference",
      { label: "Configuration", link: "/reference/configuration", badge: { text: "Reference", variant: "info" } },
      "reference/cli",
      "reference/build-and-release",
      "reference/project-structure",
      "reference/windows-compatibility",
    ],
  },
  {
    label: "Project",
    items: [
      "project",
      "project/status",
      "project/architecture",
      "project/testing",
      "project/coverage",
      "project/cases",
      "project/agents",
    ],
  },
  // ---- 中文 (zh-CN) ----
  {
    label: "开始使用",
    items: [
      { label: "概览", link: "/zh-CN/" },
      { label: "快速开始", link: "/zh-CN/quick-start", badge: { text: "从这里开始", variant: "success" } },
      { label: "可视化构建", link: "/zh-CN/guides/visual-builder" },
    ],
  },
  {
    label: "指南",
    items: [
      { label: "指南", link: "/zh-CN/guides" },
      { label: "页面布局", link: "/zh-CN/guides/page-layout" },
      { label: "自定义安装与卸载步骤", link: "/zh-CN/guides/custom-steps" },
      { label: "多语言", link: "/zh-CN/guides/languages" },
      { label: "插件 ABI", link: "/zh-CN/guides/plugins" },
      { label: "从 NSIS 迁移", link: "/zh-CN/guides/migrating-from-nsis" },
      { label: "发布你的产品", link: "/zh-CN/guides/distribution" },
      { label: "故障排查", link: "/zh-CN/guides/troubleshooting" },
    ],
  },
  {
    label: "参考",
    items: [
      { label: "参考", link: "/zh-CN/reference" },
      { label: "配置参考", link: "/zh-CN/reference/configuration", badge: { text: "参考", variant: "info" } },
      { label: "命令行参考", link: "/zh-CN/reference/cli" },
      { label: "构建与发布", link: "/zh-CN/reference/build-and-release" },
      { label: "项目结构", link: "/zh-CN/reference/project-structure" },
      { label: "Windows 兼容性", link: "/zh-CN/reference/windows-compatibility" },
    ],
  },
  {
    label: "项目",
    items: [
      { label: "项目", link: "/zh-CN/project" },
      { label: "当前生产状态", link: "/zh-CN/project/status" },
      { label: "架构", link: "/zh-CN/project/architecture" },
      { label: "测试计划", link: "/zh-CN/project/testing" },
      { label: "测试覆盖", link: "/zh-CN/project/coverage" },
      { label: "用例说明", link: "/zh-CN/project/cases" },
      { label: "给 agent 的文档地图", link: "/zh-CN/project/agents" },
    ],
  },
] satisfies SidebarConfigItem[];

// Where the site is served. The redirect destinations are composed from it, and
// `site` below carries the same URL: the repository's own audits read that line
// as text (`scripts/audit_docs_links.ps1` takes the origin from it), so the two
// are kept in step by hand when the site moves.
const siteUrl = "https://nick2781.github.io/nano-installer";

// Every page moved into a lowercase nested path, and no old address is left
// dangling: Astro writes a static redirect page for each key below, and Nimbus
// merges the same map into the platform redirects it emits with the build. The
// keys are the paths the pages used to be served at, uppercase name and all.
//
// A destination is written out in full, because Astro puts it into the redirect
// page exactly as it is written: a destination that starts at the origin root
// (`/`) would send a reader out of this project site instead of to the page it
// names. Nimbus leaves an absolute destination alone as well, so the same map
// serves both the redirect pages and `dist/_redirects`.
const redirects: Record<string, string> = {
  "/README/": `${siteUrl}/`,
  "/QUICK_START/": `${siteUrl}/quick-start/`,
  "/XML_LAYOUT_GUIDE/": `${siteUrl}/guides/page-layout/`,
  "/SCRIPT_API/": `${siteUrl}/guides/custom-steps/`,
  "/LOCALIZATION/": `${siteUrl}/guides/languages/`,
  "/PLUGIN_API/": `${siteUrl}/guides/plugins/`,
  "/GUI/": `${siteUrl}/guides/visual-builder/`,
  "/MIGRATION_FROM_NSIS/": `${siteUrl}/guides/migrating-from-nsis/`,
  "/CONFIG_REFERENCE/": `${siteUrl}/reference/configuration/`,
  "/BUILD_AND_RELEASE/": `${siteUrl}/reference/build-and-release/`,
  "/PROJECT_STRUCTURE/": `${siteUrl}/reference/project-structure/`,
  "/WINDOWS_COMPATIBILITY/": `${siteUrl}/reference/windows-compatibility/`,
  "/PRODUCTION_STATUS/": `${siteUrl}/project/status/`,
  "/ARCHITECTURE/": `${siteUrl}/project/architecture/`,
  "/TEST_PLAN/": `${siteUrl}/project/testing/`,
  "/TEST_COVERAGE/": `${siteUrl}/project/coverage/`,
  "/TEST_CASES/": `${siteUrl}/project/cases/`,
  "/AGENTS/": `${siteUrl}/project/agents/`,
  "/zh-CN/README/": `${siteUrl}/zh-CN/`,
  "/zh-CN/QUICK_START/": `${siteUrl}/zh-CN/quick-start/`,
  "/zh-CN/XML_LAYOUT_GUIDE/": `${siteUrl}/zh-CN/guides/page-layout/`,
  "/zh-CN/SCRIPT_API/": `${siteUrl}/zh-CN/guides/custom-steps/`,
  "/zh-CN/LOCALIZATION/": `${siteUrl}/zh-CN/guides/languages/`,
  "/zh-CN/PLUGIN_API/": `${siteUrl}/zh-CN/guides/plugins/`,
  "/zh-CN/GUI/": `${siteUrl}/zh-CN/guides/visual-builder/`,
  "/zh-CN/MIGRATION_FROM_NSIS/": `${siteUrl}/zh-CN/guides/migrating-from-nsis/`,
  "/zh-CN/CONFIG_REFERENCE/": `${siteUrl}/zh-CN/reference/configuration/`,
  "/zh-CN/BUILD_AND_RELEASE/": `${siteUrl}/zh-CN/reference/build-and-release/`,
  "/zh-CN/PROJECT_STRUCTURE/": `${siteUrl}/zh-CN/reference/project-structure/`,
  "/zh-CN/WINDOWS_COMPATIBILITY/": `${siteUrl}/zh-CN/reference/windows-compatibility/`,
  "/zh-CN/PRODUCTION_STATUS/": `${siteUrl}/zh-CN/project/status/`,
  "/zh-CN/ARCHITECTURE/": `${siteUrl}/zh-CN/project/architecture/`,
  "/zh-CN/TEST_PLAN/": `${siteUrl}/zh-CN/project/testing/`,
  "/zh-CN/TEST_COVERAGE/": `${siteUrl}/zh-CN/project/coverage/`,
  "/zh-CN/TEST_CASES/": `${siteUrl}/zh-CN/project/cases/`,
  "/zh-CN/AGENTS/": `${siteUrl}/zh-CN/project/agents/`,
};

const nimbusConfig = defineNimbusConfig({
  // This site is served as a GitHub Pages project site, so the origin includes
  // the repository path. It is the same URL as `siteUrl` above, written out
  // because the repository's audits read this line as text. `base` below makes
  // the built pages and every generated link live under `/nano-installer`.
  site: "https://nick2781.github.io/nano-installer",
  title: "Nano Installer",
  // The repository README's own wording.
  description:
    "Put your configuration, artwork and application files in a project folder, and Nano Installer builds them into one Windows installer",
  locale: "en",
  github: "https://github.com/nick2781/nano-installer",
  socialImageAlt: "Nano Installer documentation preview",
  sidebar: {
    items: sidebarItems,
  },
});

export default defineConfig({
  // nimbus:adapter
  output: "static",
  // GitHub Pages serves this repository at https://nick2781.github.io/nano-installer/,
  // so the site is a sub-path deploy. Astro's `base` feeds
  // `import.meta.env.BASE_URL`, which nimbus-docs' `withBase()`/`stripBase()`
  // and the authored-link normalizer already thread through every generated
  // URL; the starter's link helpers do the same. The build output itself stays
  // in `dist/` — Astro does not nest it under the base — so `dist/` is what a
  // Pages workflow publishes.
  base: "/nano-installer",
  // The old uppercase addresses, kept alive: Astro writes one redirect page per
  // entry of the map above (see it for why the destinations are absolute).
  redirects,
  // Tailwind v4 via its Vite plugin (the integration Astro recommends for
  // Tailwind v4 — replaces the PostCSS plugin, which doesn't build under
  // Astro 7's Vite 8 bundler).
  vite: {
    plugins: [tailwindcss()],
  },
  // Hover-prefetch link targets so full-page navigations feel instant without
  // a client-side router.
  prefetch: {
    prefetchAll: true,
    defaultStrategy: "hover",
  },
  markdown: {
    shikiConfig: {
      // Shiki ships no Rhai grammar, so every ```rhai fence fell back to
      // plaintext with a warning. Rhai's syntax is Rust's (fn/let/if/return,
      // `//` comments, `"` strings); aliasing it to `rust` highlights those
      // blocks instead. Nimbus sets its own `langAlias` too, and Astro merges
      // the two key by key, so this entry survives.
      langAlias: { rhai: "rust" },
    },
  },
  integrations: [
    nimbus(nimbusConfig, {
      // Authoring rules are opt-in by design — your repo, your taste. The
      // two below are the load-bearing pair: frontmatter has to validate
      // against the content schema for the page to render properly, and
      // broken internal links are 404s for your readers. Add the others
      // (heading hierarchy, code-block language, style, etc.) when you're
      // ready to enforce them — see `nimbus-docs lint --help`.
      rules: {
        "nimbus/frontmatter-shape": "error",
        "nimbus/internal-link": "error",
      },
      // Wrap wide tables so they scroll instead of overflowing the page
      // (styled by `.nb-table-scroll` in src/styles/prose.css).
      markdown: {
        hastPlugins: [tableScroll()],
      },
    }),
  ],
});
