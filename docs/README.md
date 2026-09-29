<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><b>简体中文</b> &middot; <a href="en/README.md">English</a></p>

## 中文

把一个项目目录做成一个 Windows 安装包。配置、素材和应用文件都放在同一个目录里，图标、页面和文案都由你决定；用户的机器上不用预装任何东西。

<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-zh-CN.png" alt="用 TapTap 示例构建出来的安装包第一页" width="640">

上面这张是 `examples/TapTap` 装出来的第一页，由 `scripts/capture_setup_snapshots.ps1` 在 192 dpi 下截图，并对照工程自己的版面逐项检查过——所以图上是运行时画出来的样子，不是效果图。

> **状态**：早期实现，尚不适合对外发布产品。安装动作会写入文件和注册表，请在一次性虚拟机里测试。安装包没有代码签名，Windows 7 支持也还没在真机上验收过，逐条记在[当前生产状态](zh-CN/PRODUCTION_STATUS.md)里。

先读[快速开始](zh-CN/QUICK_START.md)：构建一次示例安装包，再在虚拟机里点一遍。

| 指南 | 说明 |
| --- | --- |
| [快速开始](zh-CN/QUICK_START.md) | 构建第一个安装包，并在虚拟机里试用 |
| [可视化构建](zh-CN/GUI.md) | Windows 10 及以上的图形界面 |
| [配置参考](zh-CN/CONFIG_REFERENCE.md) | 产品信息、安装行为、输出文件名 |
| [页面布局](zh-CN/XML_LAYOUT_GUIDE.md) | 页面、控件、流式布局、链接与动作 |
| [多语言](zh-CN/LOCALIZATION.md) | 发布带译文的安装包 |
| [自定义步骤](zh-CN/SCRIPT_API.md) | 用 Rhai 编写安装与卸载逻辑 |
| [插件 ABI](zh-CN/PLUGIN_API.md) | 第三方 DLL 照着编的头文件，以及宿主借给它的服务 |
| [从 NSIS 迁移](zh-CN/MIGRATION_FROM_NSIS.md) | 哪些对应得上，哪些是故意不做的 |
| [当前生产状态](zh-CN/PRODUCTION_STATUS.md) | 现在能做什么、还差什么才能发布 |

技术说明（架构、项目结构、Windows 兼容性、构建与发布、测试计划、测试覆盖、用例说明）面向维护者，列在左侧。

本站每一页同时是一个 Markdown 文件，不用浏览器也能读：[llms.txt](llms.txt) 是全部页面的索引，[llms-full.txt](llms-full.txt) 把英文文档合成了一份。

仓库根目录的文件不在本站：[README](https://github.com/nick2781/nano-installer/blob/main/README.zh-CN.md)、[贡献指南](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md)、[安全策略](https://github.com/nick2781/nano-installer/blob/main/SECURITY.md)、[行为准则](https://github.com/nick2781/nano-installer/blob/main/CODE_OF_CONDUCT.md)。

---

## English

Hand someone a single `.exe` and your product is installed. The configuration, the artwork and your application files live in one folder; the icon, the pages and the wording are yours. Your users install nothing first.

<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-en-US.png" alt="The first page of a setup built from the TapTap example" width="640">

That is the first page of `examples/TapTap`, captured at 192 dpi by `scripts/capture_setup_snapshots.ps1` and checked against the project's own layout — so it is what the runtime drew, not a mock-up.

> **Status:** early implementation, not ready for production distribution. Install actions write files and registry entries, so test only inside a disposable VM. Neither a setup nor the installer package around it is code-signed, and Windows 7 support has not been accepted on a real machine yet; both are listed in the [production status](en/PRODUCTION_STATUS.md).

Start with the [quick start](en/QUICK_START.md): build a sample setup, then click through it in a VM.

| Guide | What it covers |
| --- | --- |
| [Quick start](en/QUICK_START.md) | Build your first setup and try it in a VM |
| [Visual builder](en/GUI.md) | The Windows 10+ app you build setups with |
| [Configuration reference](en/CONFIG_REFERENCE.md) | Product identity, install behaviour, output names |
| [Page layout](en/XML_LAYOUT_GUIDE.md) | Pages, controls, flow layout, links and actions |
| [Languages](en/LOCALIZATION.md) | Shipping translated installers |
| [Custom steps](en/SCRIPT_API.md) | Install and uninstall logic in Rhai |
| [Plugin ABI](en/PLUGIN_API.md) | The header a third-party DLL is built against, and what the host lends it |
| [Migrating from NSIS](en/MIGRATION_FROM_NSIS.md) | What maps onto what, and what is refused on purpose |
| [Production status](en/PRODUCTION_STATUS.md) | What works today and what blocks a release |

The technical notes — architecture, project layout, Windows compatibility, build and release, the test plan, test coverage — are written for maintainers and are listed in the sidebar.

Every page here is also a Markdown file, so the site reads without a browser: [llms.txt](llms.txt) indexes all of them, and [llms-full.txt](llms-full.txt) is the English documentation in one file.

Files at the repository root are not published here: [README](https://github.com/nick2781/nano-installer/blob/main/README.md), [contributing](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md), [security policy](https://github.com/nick2781/nano-installer/blob/main/SECURITY.md), [code of conduct](https://github.com/nick2781/nano-installer/blob/main/CODE_OF_CONDUCT.md).
