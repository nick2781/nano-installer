---
title: 项目结构
description: 这个仓库里 crates、项目目录、构建产物和文档站各在什么位置，以及哪些内容由构建器用你的产品资源填进去。
slug: PROJECT_STRUCTURE
sidebar:
  order: 1200
---

## 仓库

```text
crates/
├── nano-installer-core/        # 打包、XML 布局、WIC/GDI 渲染与共享运行时
├── nano-installer-cli/         # 命令行构建器
├── nano-installer-gui/         # Windows 10+ 可视化构建器
├── nano-installer-stub-lzma/   # 7z 运行时，只链接 sevenz-rust
├── nano-installer-stub-zlib/   # ZIP 运行时，只链接 zip/deflate
└── nano-installer-uninstaller/ # 卸载运行时，不链接解包后端
docs/
├── AGENTS.md                   # 这套文档的地图，给在仓库里读的人
└── zh-CN/AGENTS.md             # 同一张地图的中文版
examples/TapTap/                # 校验项目
examples/nsis-migration/        # 从 NSIS 迁过来的示例与它的对照脚本
examples/plugin-c/              # 插件 ABI 的 C 实现，只用 MSVC 就能编
scripts/                        # 构建、冒烟测试、PE 审计、文档审计
site/                           # 文档站：内容、配置与构建
```

原始运行时里不带产品资源：图标、版本信息和品牌素材都来自你的项目配置，由构建器注入，所以一套运行时就能给任意产品生成安装包。

从 NSIS 搬过来的项目，先看[从 NSIS 迁移](/zh-CN/MIGRATION_FROM_NSIS/)：那一页逐条说明每个构造在这里变成什么，而 `examples/nsis-migration/migrated/` 是照它写出来的一份工程，可以直接构建。

## 产品项目

```text
product-installer/
├── installer_config.json
├── layouts/                    # XML 页面
├── assets/                     # 背景、按钮、图标
├── locales/                    # 每种语言一个 JSON 文件
├── scripts/                    # 可选的安装/卸载逻辑
└── payload/app.7z              # 也可以是 ZIP 归档
```

配置里的每个路径都相对项目目录解析。构建器会打包三个目录，`layouts`、`assets` 和 `locales`；可选的 `scripts` 目录也一起打包，配置里指定的那一个应用文件同样打包进去。

## 构建产物

```text
target/release/
├── nano-installer-native-x64.exe
├── nano-installer-gui-x64.exe
└── stubs/
    ├── lzma-stub-native.exe
    ├── zlib-stub-native.exe
    └── uninst-stub-native.exe
```

构建时加上 `--msi`，安装包旁边会多出一个包，给企业按 MSI 分发。

`target/x86_64-win7-windows-msvc/` 只是 Cargo 的交叉编译缓存，不是另一套发布文件。示例安装包输出到 `examples/TapTap/dist/`，这些不算发布内容。

## 文档站

对外发布的站点是 `site/` 下的一个 [Astro](https://astro.build/) 工程，用 [Cloudflare Nimbus](https://nimbus-docs.com/) 构建。一页就是一个 Markdown 文件。`site/src/content/docs/X.md` 发布在 `/X/`，它在 `site/src/content/zh-CN/X.md` 里的对照页发布在 `/zh-CN/X/`。站点设置和侧栏都在 `site/astro.config.ts` 里，侧栏按语言分成两组。

`site/dist/` 是构建写出的产物，也是 GitHub Pages 发布的东西，挂在 `/nano-installer/` 下。这是项目站点，不是账号站点，所以配置里的 `base` 就是 `/nano-installer`；Astro 的产物本身留在 `dist/` 里，不会再嵌一层 base，工作流因此直接把 `site/dist` 原样上传。

每一页除了 HTML，还有一个 Markdown 版本，位置是 `<页面>/index.md`；构建同时写出不用浏览器的读者要用的那几个文件：

| 文件 | 里面是什么 |
| --- | --- |
| `/llms.txt` | 每一个英文页面一行，链接指向那一页自己的 Markdown 版本。`/zh-CN/llms.txt` 用同样的写法列中文页 |
| `/llms-full.txt` | 整份文档合成一份，两种语言都在里面，用例表和覆盖表也收了进来 |
| `/robots.txt` | 全部公开，sitemap 的地址也写在这里，就是 `/sitemap-index.xml` |
| `/sitemap-index.xml` | 同一份页面清单的 XML 版，拆成若干编号的 sitemap |
| `/og.png`、`/og/<页面>.png` | 站点和每一页各自的社交卡片 |

这些文件没有一个是手工维护的。每一个都由构建从内容树写出来，所以页面不可能从索引里漏掉，也不存在第二份清单要跟着改。新增一页，就是 `site/src/content/` 下多一个文件，加上 `site/astro.config.ts` 侧栏清单里的一行。

读者在站点根目录 `/` 切换语言。那里每种语言一张卡片，而侧栏只显示当前页面这一种语言的分组。有两项检查为这件事负责，文档工作流在发布前都会跑：一棵树里的页面在另一棵树里没有对应页，`scripts/audit_docs_languages.ps1` 就会失败；某一页的 frontmatter 校验不过，或者站内链接指向不存在的地方，构建本身就会失败。

`.well-known` 是这个站点唯一做不到位的地方。Nimbus 会写出给 agent 找路用的 `/.well-known/ai-catalog.json` 与 `/.well-known/ard.json`，并警告这两个文件应该放在源站根目录。GitHub Pages 项目站点挂在 `/nano-installer/` 下，源站根目录属于账号而不是这个项目。所以去 `https://nick2781.github.io/.well-known/` 找的客户端什么也找不到。要做得更好得有自定义域名，而本项目没有，因此这两个文件只发布在项目路径下，构建的警告就留在那里。

`docs/AGENTS.md` 是这套文档自己的地图，说清哪个问题看哪一页、两种语言怎么对应、机器可读的入口在哪。仓库把它放在 `docs/` 根目录下而不是某棵树里，因为仓库约定要求它在那里，中文版是 `docs/zh-CN/AGENTS.md`。同一张地图也作为页面发布，就是 `/AGENTS/` 与 `/zh-CN/AGENTS/`；别的页面都在另一棵树里有对照的一份。
