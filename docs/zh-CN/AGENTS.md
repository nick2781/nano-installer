# 给 agent 的文档地图

这一页说明哪个问题该看哪份文档。人和 agent 都能直接打开那一份，不用走遍整个站点。

**每一页都有对照的另一语言版本。** 发布的页面都成对：`site/src/content/docs/X.mdx` 与
`site/src/content/zh-CN/X.mdx` 是同一份文档，分别发布在 `/X/` 和 `/zh-CN/X/`。这里没有哪一页只写
一种语言，`scripts/audit_docs_languages.ps1` 守着这条规矩：一棵树里的页面在另一棵树里没有对应页，
它就失败；它同样把本页与 `zh-CN/AGENTS.md` 配成一对。

仓库约定要求这张地图放在 `docs/` 根目录下，而不是放进某棵内容树里，所以 `docs/` 下只有这一个英文
文件。中文版是 [`zh-CN/AGENTS.md`](zh-CN/AGENTS.md)，同一张地图也作为页面发布，就是 `/project/agents/` 与
`/zh-CN/project/agents/`。

## 先拿这几份

| 文件 | 里面是什么 |
| --- | --- |
| [`/llms.txt`](https://nick2781.github.io/nano-installer/llms.txt) | 每一个英文页面一行，链接指向那一页自己的 Markdown 版本。先取这一份 |
| [`/llms-full.txt`](https://nick2781.github.io/nano-installer/llms-full.txt) | 整份文档合成一份，两种语言都在里面，用例表和覆盖表也收了进来 |
| [`/zh-CN/llms.txt`](https://nick2781.github.io/nano-installer/zh-CN/llms.txt) | 中文页面，同样一行一页 |
| [`/sitemap-index.xml`](https://nick2781.github.io/nano-installer/sitemap-index.xml)、[`/robots.txt`](https://nick2781.github.io/nano-installer/robots.txt) | 给爬虫的同一份页面清单 |

读者是模型时，直接取 `<页面>/index.md`。每一页除了 HTML，都在这个位置放一份 Markdown 版本，页面
就是照着它渲染出来的。手里是仓库的读者，取 `site/src/content/` 下那一页自己的文件。

## 哪个问题看哪个文件

| 问题 | English | 中文 |
| --- | --- | --- |
| 这是什么，现在能不能拿来发布？ | [README](../../site/src/content/docs/index.mdx), [Production status](../../site/src/content/docs/project/status.mdx) | [中文文档](../../site/src/content/zh-CN/index.mdx)、[当前生产状态](../../site/src/content/zh-CN/project/status.mdx) |
| 怎么装出第一个安装包？ | [Quick start](../../site/src/content/docs/quick-start.mdx) | [快速开始](../../site/src/content/zh-CN/quick-start.mdx) |
| 构建和部署支持哪些参数？ | [Command line](../../site/src/content/docs/reference/cli.mdx) | [命令行参考](../../site/src/content/zh-CN/reference/cli.mdx) |
| 产品如何签名、部署和验收？ | [Distribution](../../site/src/content/docs/guides/distribution.mdx) | [发布你的产品](../../site/src/content/zh-CN/guides/distribution.mdx) |
| 构建或安装失败时从哪里开始？ | [Troubleshooting](../../site/src/content/docs/guides/troubleshooting.mdx) | [故障排查](../../site/src/content/zh-CN/guides/troubleshooting.mdx) |
| 有哪些设置，各自改什么？ | [Configuration reference](../../site/src/content/docs/reference/configuration.mdx) | [配置参考](../../site/src/content/zh-CN/reference/configuration.mdx) |
| 页面和控件怎么写？ | [Page layout](../../site/src/content/docs/guides/page-layout.mdx) | [页面布局](../../site/src/content/zh-CN/guides/page-layout.mdx) |
| 怎么增加或修改一种语言？ | [Languages](../../site/src/content/docs/guides/languages.mdx) | [多语言](../../site/src/content/zh-CN/guides/languages.mdx) |
| 怎么运行自己的安装与卸载步骤？ | [Custom steps](../../site/src/content/docs/guides/custom-steps.mdx) | [自定义步骤](../../site/src/content/zh-CN/guides/custom-steps.mdx) |
| 怎么带一个插件，它允许做什么？ | [Plugin ABI](../../site/src/content/docs/guides/plugins.mdx) | [插件 ABI](../../site/src/content/zh-CN/guides/plugins.mdx) |
| 从 NSIS 过来，什么对应什么？ | [Migrating from NSIS](../../site/src/content/docs/guides/migrating-from-nsis.mdx) | [从 NSIS 迁移](../../site/src/content/zh-CN/guides/migrating-from-nsis.mdx) |
| 安装器内部是怎么走的？ | [Architecture](../../site/src/content/docs/project/architecture.mdx) | [架构](../../site/src/content/zh-CN/project/architecture.mdx) |
| 支持哪些 Windows 版本，用了哪些系统调用？ | [Windows compatibility](../../site/src/content/docs/reference/windows-compatibility.mdx) | [Windows 兼容性](../../site/src/content/zh-CN/reference/windows-compatibility.mdx) |
| 不动命令行怎么构建？ | [Visual builder](../../site/src/content/docs/guides/visual-builder.mdx) | [可视化构建](../../site/src/content/zh-CN/guides/visual-builder.mdx) |
| 怎么构建、怎么发布？ | [Build and release](../../site/src/content/docs/reference/build-and-release.mdx), [Project layout](../../site/src/content/docs/reference/project-structure.mdx) | [构建与发布](../../site/src/content/zh-CN/reference/build-and-release.mdx)、[项目结构](../../site/src/content/zh-CN/reference/project-structure.mdx) |
| 测了什么，什么只是承诺？ | [Test plan](../../site/src/content/docs/project/testing.mdx), [Test coverage](../../site/src/content/docs/project/coverage.mdx), [Test cases](../../site/src/content/docs/project/cases.mdx) | [测试计划](../../site/src/content/zh-CN/project/testing.mdx)、[测试覆盖](../../site/src/content/zh-CN/project/coverage.mdx)、[用例说明](../../site/src/content/zh-CN/project/cases.mdx) |

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

## 页面和索引是怎么来的

对外发布的站点是 `site/` 下的一个 Astro 工程，用 Cloudflare Nimbus 构建。页面就是
`site/src/content/docs/**/*.mdx` 与 `site/src/content/zh-CN/**/*.mdx`，站点设置和侧栏的两组语言条目都在
`site/astro.config.ts` 里。构建产物写在 `site/dist/`，GitHub Pages 发布的正是这个目录，挂在
`/nano-installer/` 下；文档工作流按这些文件里写的版本装好工具链，在 `site/` 里构建，然后上传那个
目录。

给爬虫和 agent 用的文件也由同一次构建写出，没有人另外维护一份数据文件：`llms.txt`、`llms-full.txt`、
`robots.txt`、`sitemap-index.xml` 站点地图，以及每一页的 Markdown 版本和 OG 卡片。所以新增一页只要
在 `site/src/content/` 里放上文件、在另一棵树里放上对照页、再在侧栏清单里加一行。frontmatter 里的
`title` 同时是侧栏和索引对它的称呼，页面标题只需要维护一处。生成器和它读的那份数据文件随 docsify
站点一起退役了，因为构建是从页面写出这些文件的，两者不可能脱节。

`.well-known` 是这个站点唯一做不到位的地方。Nimbus 会写出 `/.well-known/ai-catalog.json` 与
`/.well-known/ard.json`，并警告这两个文件应该放在源站根目录；而 GitHub Pages 项目站点挂在
`/nano-installer/` 下，源站根目录属于账号而不是这个项目。要放过去得有自定义域名，本项目没有，因此
这两个文件只发布在项目路径下，构建的警告就留在那里。

## 这些页面怎么写

上面说的是哪一页放什么。这一节就是本站的写作标准，新写一页、改一页都照它来。

- **句子要有主语和动词。** 一句话要是写成冒号后面堆一串名词，那还不算句子，得把它说完整。
- **结论或核心动作放最前面。** 条件、范围、例外跟在它后面。
- **一句讲一件事，长短交错。** 步骤和警告可以短，条件和因果该长，用「因为」「所以」「而」把从句接上，
  不要一律切成三五个字的碎句。
- **连接词留着。** 它们承载逻辑；该删的只是口水，比如「已经……过一遍」「另外还有」「值得一提的是」
  「需要注意的是」「在……的情况下」。

结构上的要求和句子同等重要：

- 一段不超过三到五句，一段只讲一个意思。原来一段里装了三件事，就拆成三段。
- 步骤写成有序列表，一条一步。
- 并列的取值、限制、错误码写成无序列表或表格。
- 限制与后果各自独立成句（「只在 X 下生效」「会覆盖 Y」），不要塞进长句的从句里。
- 同一个字段或概念在一页里只解释一次。需要重复时用一句话带过，并指向那一段。
- 同一个词不要连着出现，括号里的补充也不要重复前面的词——「应用文件（你的应用文件，zip 或 7z 归档）」
  这种同义反复要改成一句话。
- 用中文计算机领域的通用说法，不要英文直译：运行时、布局、清单文件、静默安装、就地升级、管理员权限、
  实机测试、已覆盖/未覆盖；Windows 那个地方叫「程序和功能」，不叫「卸载登记项」。

改写不是把页面改短。条件和因果该占的地方留着；要删的是重复和口水。

下面这句话有三个版本，可以拿它当判断尺度：

- **原句（啰嗦）**：最低支持的版本已经实机跑过一遍：内部版本 7601 的 Windows 7 SP1 x64，装在临时
  虚拟机里；当前构建器生成的安装包在那儿装了一次、卸了一次、升级了一次，另外还有两次静默运行。
- **过度压缩（同样是错的，像电报体）**：已在 Windows 7 SP1 x64（内部版本 7601）的临时虚拟机上验证：
  安装、卸载、升级各一次，静默安装两次。
- **目标**：最低支持的版本已经实机验证过。测试用的是当前构建器生成的安装包，在一台装有 Windows 7
  SP1 x64（内部版本 7601）的临时虚拟机上完成了安装、卸载和升级，另外还做了两次静默安装。

改完之后，一个中文母语的工程师读一遍，应该觉得这是「有人好好写的文档」，而不是「机器压缩过的要点」。
如果他觉得哪句读着别扭，即使更短，也要改回去。

- **事实、数字、命令、路径、字段名不许动。** 标题、锚点、链接目标也要原样，改写只动措辞。frontmatter
  里只有 `description` 可以改，它就是搜索结果里显示的那句话；`title` 与 `sidebar` 不动，页面路径由文件
  所在的位置决定，frontmatter 里没有这个字段。改完用
  `scripts/audit_docs_languages.ps1` 核对两棵树是否成对，用站点自己的构建（在 `site/` 里跑
  `pnpm build`）核对 frontmatter——那份校验会拦下不合格的 frontmatter；站内链接交给
  `scripts/audit_docs_links.ps1`，它读的是构建产物，因为 `astro build` 不跑任何 authoring 规则，
  链接指向的锚点只有标题渲染出来之后才存在。
- **中文页参考 [Tech-Doc-Style-Chinese](https://github.com/fenng/Tech-Doc-Style-Chinese) 这份外部规范，
  但它只是参考，不是第二套标准**：两者冲突时以本节为准，用指代（它、这、这些）承接、源文件保持硬
  换行，这两处就是例子。

中英两份是同一篇文档：段落顺序、每段讲的事必须对应。
