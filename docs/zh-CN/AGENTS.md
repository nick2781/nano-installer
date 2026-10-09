# 给 agent 的文档地图

这一页告诉你哪个问题该看哪份文档，所以不管是人还是 agent，都能直接找到需要的那一份，不用把整个
站点走一遍。

**每一页都有对照的另一语言版本。** 这里的文档都成对：`docs/en/X.md` 与 `docs/zh-CN/X.md` 是
同一份文档，而读者靠站点顶栏的那个开关在两棵树之间切换。这里没有哪一页只写一种语言。
`scripts/audit_docs_languages.ps1` 守着这一点：一棵树里的页面在另一棵树里没有对应页，它失败；开关
那份页名清单和旁边的文件对不上，它也会失败。

这张地图放在 `docs/` 根目录下而不是某棵树里，所以根目录只有两个文件，另一个是站点首页——那是选
语言的入口页，不是文档。中文版就是本页，英文版是 [`../AGENTS.md`](../AGENTS.md)。

## 先拿这几份

| 文件 | 里面是什么 |
| --- | --- |
| [`/llms.txt`](../llms.txt) | 每一页一行，链接指向那一页自己的 Markdown 文件。先取这一份 |
| [`/llms-full.txt`](../llms-full.txt) | 英文页面合成一份，给只想取一次文件的读者。测试覆盖表没收进来：那是拿来查的表，不是拿来读的。链接在 `llms.txt` 里 |
| [`/sitemap.xml`](../sitemap.xml)、[`/robots.txt`](../robots.txt) | 给爬虫的同一份页面清单 |

本站每一页都是 Markdown 文件，和 HTML 查看器并排发布，以 `text/markdown` 提供；那份 HTML 只是
覆盖全部页面的一个查看器。所以直接取文件本身就好，因为 Markdown 才是原始文档：没有哪一页是为读者
另外生成的。

## 哪个问题看哪个文件

| 问题 | English | 中文 |
| --- | --- | --- |
| 这是什么，现在能不能拿来发布？ | [README](../en/README.md), [Production status](../en/PRODUCTION_STATUS.md) | [中文文档](README.md)、[当前生产状态](PRODUCTION_STATUS.md) |
| 怎么装出第一个安装包？ | [Quick start](../en/QUICK_START.md) | [快速开始](QUICK_START.md) |
| 有哪些设置，各自改什么？ | [Configuration reference](../en/CONFIG_REFERENCE.md) | [配置参考](CONFIG_REFERENCE.md) |
| 页面和控件怎么写？ | [Page layout](../en/XML_LAYOUT_GUIDE.md) | [页面布局](XML_LAYOUT_GUIDE.md) |
| 怎么增加或修改一种语言？ | [Languages](../en/LOCALIZATION.md) | [多语言](LOCALIZATION.md) |
| 怎么运行自己的安装与卸载步骤？ | [Custom steps](../en/SCRIPT_API.md) | [自定义步骤](SCRIPT_API.md) |
| 怎么带一个插件，它允许做什么？ | [Plugin ABI](../en/PLUGIN_API.md) | [插件 ABI](PLUGIN_API.md) |
| 从 NSIS 过来，什么对应什么？ | [Migrating from NSIS](../en/MIGRATION_FROM_NSIS.md) | [从 NSIS 迁移](MIGRATION_FROM_NSIS.md) |
| 安装器内部是怎么走的？ | [Architecture](../en/ARCHITECTURE.md) | [架构](ARCHITECTURE.md) |
| 支持哪些 Windows 版本，用了哪些系统调用？ | [Windows compatibility](../en/WINDOWS_COMPATIBILITY.md) | [Windows 兼容性](WINDOWS_COMPATIBILITY.md) |
| 怎么构建、怎么发布？ | [Build and release](../en/BUILD_AND_RELEASE.md), [Project layout](../en/PROJECT_STRUCTURE.md) | [构建与发布](BUILD_AND_RELEASE.md)、[项目结构](PROJECT_STRUCTURE.md) |
| 测了什么，什么只是承诺？ | [Test plan](../en/TEST_PLAN.md), [Test coverage](../en/TEST_COVERAGE.md), [Test cases](../en/TEST_CASES.md) | [测试计划](TEST_PLAN.md)、[测试覆盖](TEST_COVERAGE.md)、[用例说明](TEST_CASES.md) |

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
- [代码签名政策](https://github.com/nick2781/nano-installer/blob/main/CODE_SIGNING.md)：签名由谁提供、签哪些文件、
  维护者是谁。
- [变更日志](https://github.com/nick2781/nano-installer/blob/main/CHANGELOG.md)：每个版本改了什么。

## 这份索引是怎么来的

`scripts/build_docs_index.ps1` 从 `scripts/docs_index.json` 写出 `llms.txt`、`llms-full.txt`、
`robots.txt` 和 `sitemap.xml`，而 `-Verify` 用的是同一份代码，所以写和查不会对不上。新增一页因此
要在那份数据文件里加一行，标题还得和页面自己的 `# 标题` 一字不差；少了这一行，检查就会失败，不会
让这一页悄悄从索引里消失。

某一页收不收进 `llms-full.txt`，由它所在的分组决定，但单页也能覆盖：站点首页是选语言的入口页，
不是文档，它就是靠这一点排除在英文合成文件之外的。两张用例表也不收，理由和覆盖表一样：它们是
拿来查的表，不是拿来通读的。

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
  `scripts/audit_docs_languages.ps1` 与 `scripts/build_docs_index.ps1 -Verify` 核对——索引里的标题必须和
  页面自己的 `# 标题` 一字不差。
- 中英两份是同一篇文档：段落顺序、每段讲的事必须对应。

