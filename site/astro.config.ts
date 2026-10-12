import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import nimbus, {
  defineConfig as defineNimbusConfig,
} from "@cloudflare/nimbus-docs";
import { tableScroll } from "@cloudflare/nimbus-docs/markdown";

// One section list per language. The English groups list primary-collection
// (`docs`) entry ids; the Chinese groups point at the same pages through the
// `zh-CN` collection's mount path. Both lists follow the same seven sections in
// the same order, and each page's `sidebar.order` matches its twin's, so the two
// rails read in the same sequence. `src/lib/sidebar-language.ts` and the two
// catch-all routes keep a page from showing the other language's groups.
const sidebarItems = [
  // ---- English (docs) ----
  { label: "Overview", items: ["README", "PRODUCTION_STATUS"] },
  { label: "Get started", items: ["QUICK_START"] },
  {
    label: "Guides",
    items: [
      "XML_LAYOUT_GUIDE",
      "SCRIPT_API",
      "LOCALIZATION",
      "PLUGIN_API",
      "GUI",
      "MIGRATION_FROM_NSIS",
    ],
  },
  { label: "Reference", items: ["CONFIG_REFERENCE"] },
  {
    label: "Concepts",
    items: ["ARCHITECTURE", "PROJECT_STRUCTURE", "WINDOWS_COMPATIBILITY"],
  },
  {
    label: "Testing and status",
    items: ["TEST_PLAN", "TEST_COVERAGE", "TEST_CASES"],
  },
  { label: "Release", items: ["BUILD_AND_RELEASE"] },
  // ---- 中文 (zh-CN) ----
  {
    label: "总览",
    items: [
      { label: "中文文档", link: "/zh-CN/README" },
      { label: "当前生产状态", link: "/zh-CN/PRODUCTION_STATUS" },
    ],
  },
  { label: "快速开始", items: [{ label: "快速开始", link: "/zh-CN/QUICK_START" }] },
  {
    label: "指南",
    items: [
      { label: "页面布局", link: "/zh-CN/XML_LAYOUT_GUIDE" },
      { label: "自定义安装与卸载步骤", link: "/zh-CN/SCRIPT_API" },
      { label: "多语言", link: "/zh-CN/LOCALIZATION" },
      { label: "插件 ABI", link: "/zh-CN/PLUGIN_API" },
      { label: "可视化构建", link: "/zh-CN/GUI" },
      { label: "从 NSIS 迁移", link: "/zh-CN/MIGRATION_FROM_NSIS" },
    ],
  },
  { label: "参考", items: [{ label: "配置参考", link: "/zh-CN/CONFIG_REFERENCE" }] },
  {
    label: "概念",
    items: [
      { label: "架构", link: "/zh-CN/ARCHITECTURE" },
      { label: "项目结构", link: "/zh-CN/PROJECT_STRUCTURE" },
      { label: "Windows 兼容性", link: "/zh-CN/WINDOWS_COMPATIBILITY" },
    ],
  },
  {
    label: "测试与状态",
    items: [
      { label: "测试计划", link: "/zh-CN/TEST_PLAN" },
      { label: "测试覆盖", link: "/zh-CN/TEST_COVERAGE" },
      { label: "用例说明", link: "/zh-CN/TEST_CASES" },
    ],
  },
  { label: "发布", items: [{ label: "构建与发布", link: "/zh-CN/BUILD_AND_RELEASE" }] },
];

const nimbusConfig = defineNimbusConfig({
  // This site is served as a GitHub Pages project site, so the origin includes
  // the repository path. `base` below makes the built pages and every generated
  // link live under `/nano-installer`.
  site: "https://nick2781.github.io/nano-installer",
  title: "Nano Installer",
  // The repository README's own wording.
  description:
    "Put your configuration, artwork and application files in a project folder, and Nano Installer builds them into one Windows installer",
  locale: "en",
  github: null,
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
