---
title: 给 agent 的文档地图
description: 这一页告诉你哪个问题该看哪份文档，所以不管是人还是 agent，都能直接找到需要的那一份，不用把整个站点走一遍。
slug: AGENTS
sidebar:
  order: 250
---

这一页告诉你哪个问题该看哪份文档，所以不管是人还是 agent，都能直接找到需要的那一份，不用把整个
站点走一遍。

**每一页都有对照的另一语言版本。** 页面都成对：`site/src/content/docs/X.md` 与
`site/src/content/zh-CN/X.md` 是同一份文档，分别发布在 `/X/` 和 `/zh-CN/X/`。这里没有哪一页只写
一种语言，`scripts/audit_docs_languages.ps1` 守着这一点：一棵树里的页面在另一棵树里没有对应页，它
就失败。

这张地图本身就是这些对子之一。仓库里以文件形式存着同一张地图，放在 `docs/` 根目录下，因为仓库约定
要求它在那里：`docs/AGENTS.md` 与 `docs/zh-CN/AGENTS.md` 就是 agent 在检出目录里读的那两份。本页
的英文版是 [Documentation map for agents](/AGENTS/)。

## 先拿这几份

| 文件 | 里面是什么 |
| --- | --- |
| [`/llms.txt`](/llms.txt) | 每一个英文页面一行，链接指向那一页自己的 Markdown 版本。先取这一份 |
| [`/llms-full.txt`](/llms-full.txt) | 整份文档合成一份，两种语言都在里面，用例表和覆盖表也收了进来，给只想取一次文件的读者 |
| [`/zh-CN/llms.txt`](/zh-CN/llms.txt) | 中文页面，同样一行一页 |
| [`/sitemap-index.xml`](/sitemap-index.xml)、[`/robots.txt`](/robots.txt) | 给爬虫的同一份页面清单 |

每一页除了 HTML，还有一个 Markdown 版本，位置是 `<页面>/index.md`，页面就是照着它渲染出来的。
所以读者是模型时直接取那份文件：HTML 只是同一份源的渲染结果，仅此而已。

## 哪个问题看哪个文件

| 问题 | English | 中文 |
| --- | --- | --- |
| 这是什么，现在能不能拿来发布？ | [README](/README/), [Production status](/PRODUCTION_STATUS/) | [中文文档](/zh-CN/README/)、[当前生产状态](/zh-CN/PRODUCTION_STATUS/) |
| 怎么装出第一个安装包？ | [Quick start](/QUICK_START/) | [快速开始](/zh-CN/QUICK_START/) |
| 有哪些设置，各自改什么？ | [Configuration reference](/CONFIG_REFERENCE/) | [配置参考](/zh-CN/CONFIG_REFERENCE/) |
| 页面和控件怎么写？ | [Page layout](/XML_LAYOUT_GUIDE/) | [页面布局](/zh-CN/XML_LAYOUT_GUIDE/) |
| 怎么增加或修改一种语言？ | [Languages](/LOCALIZATION/) | [多语言](/zh-CN/LOCALIZATION/) |
| 怎么运行自己的安装与卸载步骤？ | [Custom steps](/SCRIPT_API/) | [自定义步骤](/zh-CN/SCRIPT_API/) |
| 怎么带一个插件，它允许做什么？ | [Plugin ABI](/PLUGIN_API/) | [插件 ABI](/zh-CN/PLUGIN_API/) |
| 从 NSIS 过来，什么对应什么？ | [Migrating from NSIS](/MIGRATION_FROM_NSIS/) | [从 NSIS 迁移](/zh-CN/MIGRATION_FROM_NSIS/) |
| 安装器内部是怎么走的？ | [Architecture](/ARCHITECTURE/) | [架构](/zh-CN/ARCHITECTURE/) |
| 支持哪些 Windows 版本，用了哪些系统调用？ | [Windows compatibility](/WINDOWS_COMPATIBILITY/) | [Windows 兼容性](/zh-CN/WINDOWS_COMPATIBILITY/) |
| 不动命令行怎么构建？ | [Visual builder](/GUI/) | [可视化构建](/zh-CN/GUI/) |
| 怎么构建、怎么发布？ | [Build and release](/BUILD_AND_RELEASE/), [Project layout](/PROJECT_STRUCTURE/) | [构建与发布](/zh-CN/BUILD_AND_RELEASE/)、[项目结构](/zh-CN/PROJECT_STRUCTURE/) |
| 测了什么，什么只是承诺？ | [Test plan](/TEST_PLAN/), [Test coverage](/TEST_COVERAGE/), [Test cases](/TEST_CASES/) | [测试计划](/zh-CN/TEST_PLAN/)、[测试覆盖](/zh-CN/TEST_COVERAGE/)、[用例说明](/zh-CN/TEST_CASES/) |

## 仓库里的文件

这些文件不在发布的站点里，所以点开链接就会离开本站：

- [README](https://github.com/nick2781/nano-installer/blob/main/README.zh-CN.md)：这是什么、现在能做什么、
  发布还差什么。
- [插件 ABI 头文件](https://github.com/nick2781/nano-installer/blob/main/include/nano_plugin.h)：插件要守的
  三条规矩，还有第三方 DLL 照着编的那张宿主表。
- [示例](https://github.com/nick2781/nano-installer/blob/main/examples/README.md)：三个示例工程，各自证明
  什么。`examples/plugin-c/sample.c` 是用 C 写的 ABI。
- [贡献指南](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md)：怎么准备一份工作副本，
  一个改动要过哪些检查，这个仓库守哪些规矩。
- [仓库约定](https://github.com/nick2781/nano-installer/blob/main/AGENTS.md)：目录结构、构建与测试命令，
  还有写代码的 agent 要守住的改动边界。
- [安全策略](https://github.com/nick2781/nano-installer/blob/main/SECURITY.md)：怎么私下上报，哪些
  已知问题不算漏洞。
- [代码签名政策](https://github.com/nick2781/nano-installer/blob/main/CODE_SIGNING.md)：为什么现在没有签名、
  会签哪些文件、维护者是谁。
- [变更日志](https://github.com/nick2781/nano-installer/blob/main/CHANGELOG.md)：每个版本改了什么。
- [文档地图](https://github.com/nick2781/nano-installer/blob/main/docs/zh-CN/AGENTS.md)：本页作为文件放在
  `docs/` 根目录下的那一份，给仓库里的读者看。

## 页面和索引是怎么来的

站点是 `site/` 下的一个 Astro 工程，用 Cloudflare Nimbus 构建。页面就是
`site/src/content/docs/` 与 `site/src/content/zh-CN/` 里的 Markdown 文件，构建产物写在 `dist/`，
GitHub Pages 发布的正是这个目录，挂在 `/nano-installer/` 下。

给爬虫和 agent 用的文件由同一次构建产生，不靠谁维护的那份清单：`llms.txt`、`llms-full.txt`、
`robots.txt`、`sitemap-index.xml` 站点地图，以及每一页的 Markdown 版本和 OG 图。所以新增一页只要在
`site/src/content/` 里放上文件、在 `site/astro.config.ts` 的侧栏清单里加一行，别的都不需要。索引是
从页面写出来的，不可能和页面脱节。

## 这些页面怎么写

上面说的是哪一页放什么。这一节说的是写的时候怎么落笔，改稿、新写一页都照它来。

- **一句一件事，但句子要长短交错。** 结论、警告、步骤可以短；背景、条件、因果该长，把关系紧密的从句用
  「，」「而」「所以」接起来。一页里每句话都在 20 字上下，就说明写坏了。
- **连接词要留着**：所以、但、而、因为、这样一来、也就是说。它们承载逻辑；删干净之后，段子里只剩并列。
- **一段讲一个完整的意思。** 先说要干什么，再补条件、例外或后果；从第二句起用指代（它、这、这些）承接，
  不要每句重新报一次主语。
- **用中文计算机领域的通用说法**，不要英文直译：运行时、布局、清单文件、静默安装、就地升级、管理员权限、
  实机测试、已覆盖/未覆盖；Windows 那个地方叫「程序和功能」，不叫「卸载登记项」。
- **同一个词别连着出现**；括号里的补充不能与前面的词重复——「应用文件（你的应用文件，zip 或 7z 归档）」
  这种同义反复要改成一句话。
- **事实、数字、命令、路径、字段名不许动**：标题、锚点、链接目标也要原样，改写只动措辞。改完用
  `scripts/audit_docs_languages.ps1` 核对两棵树是否成对，用站点自己的构建（在 `site/` 里跑
  `pnpm build`）核对 frontmatter 与站内链接——frontmatter 校验不过、或者站内链接指向不存在的地方，
  构建就会失败。
- **中文页参考 [Tech-Doc-Style-Chinese](https://github.com/fenng/Tech-Doc-Style-Chinese) 这份外部规范，
  但它只是参考，不是第二套标准**：两者冲突时以本节为准，用指代（它、这、这些）承接、源文件保持硬
  换行，这两处就是例子。

中英两份是同一篇文档：段落顺序、每段讲的事必须对应。
