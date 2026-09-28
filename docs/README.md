<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="en/">English documentation</a> &middot; <a href="zh-CN/">中文文档</a></p>

你交付的只有一个 exe，用户双击就能装上你的产品。配置、素材和应用文件都放在同一个目录里，Nano
Installer 把它打包成一个 Windows 安装包。图标、界面和文案都由你决定。用户的机器上不用预装任何
东西。

Hand someone a single `.exe` and your product is installed. You keep the configuration, the artwork
and your application files in one folder. Nano Installer turns that folder into one Windows setup
file with your logo, your pages and your wording, and your users install nothing first.

> **状态 / Status:** 早期实现，尚不适合对外发布产品。安装动作请只在一次性虚拟机中测试；
> 未签名的安装包、尚未在真机上验收的 Windows 7 支持这些已知缺口，逐条记在
> [当前生产状态](zh-CN/PRODUCTION_STATUS.md)里。
>
> Early implementation, not ready for production distribution. Test install actions only in a
> disposable VM. The missing code signature and the Windows 7 acceptance that has not happened
> on a real machine are listed one by one in the
> [production status](en/PRODUCTION_STATUS.md).

<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-en-US.png" alt="A setup built from the TapTap example, first page" width="640">

上面这张是 `examples/TapTap` 装出来的第一页，由 `scripts/capture_setup_snapshots.ps1` 在 96 dpi 下截取，页面本身经过工程自己的版面核对。The picture above is the first page of `examples/TapTap`, captured at
96 dpi by `scripts/capture_setup_snapshots.ps1` and checked against the project's own layout.

## 从这里开始 / Start here

想要快速看到结果，先读[快速开始](zh-CN/QUICK_START.md)：构建一次示例安装包，然后在虚拟机里点一遍。
想了解产品现在能做什么、还差什么才能发布，看[当前生产状态](zh-CN/PRODUCTION_STATUS.md)。

Start with the [quick start](en/QUICK_START.md) to build a sample setup and click through it in a
VM. The [production status](en/PRODUCTION_STATUS.md) page lists what works today and what still
blocks a release.

| 中文 | English |
| --- | --- |
| [快速开始](zh-CN/QUICK_START.md) | [Quick start](en/QUICK_START.md) |
| [可视化构建](zh-CN/GUI.md) | [Visual builder](en/GUI.md) |
| [配置参考](zh-CN/CONFIG_REFERENCE.md) | [Configuration](en/CONFIG_REFERENCE.md) |
| [页面布局](zh-CN/XML_LAYOUT_GUIDE.md) | [Page layout](en/XML_LAYOUT_GUIDE.md) |
| [多语言](zh-CN/LOCALIZATION.md) | [Languages](en/LOCALIZATION.md) |
| [自定义步骤](zh-CN/SCRIPT_API.md) | [Custom steps](en/SCRIPT_API.md) |
| [从 NSIS 迁移](zh-CN/MIGRATION_FROM_NSIS.md) | [Migrating from NSIS](en/MIGRATION_FROM_NSIS.md) |
| [插件 ABI](zh-CN/PLUGIN_API.md) | [Plugin ABI](en/PLUGIN_API.md) |
| [当前生产状态](zh-CN/PRODUCTION_STATUS.md) | [Production status](en/PRODUCTION_STATUS.md) |

## 技术说明 / Technical notes

架构、构建发布与测试计划按语言归档，内容面向维护者而非产品接入方。

Architecture, build and release, and the test plan live under each language tree and are written for
maintainers rather than for product onboarding.

| 中文 | English |
| --- | --- |
| [架构](zh-CN/ARCHITECTURE.md) | [Architecture](en/ARCHITECTURE.md) |
| [项目结构](zh-CN/PROJECT_STRUCTURE.md) | [Project layout](en/PROJECT_STRUCTURE.md) |
| [Windows 兼容性](zh-CN/WINDOWS_COMPATIBILITY.md) | [Windows compatibility](en/WINDOWS_COMPATIBILITY.md) |
| [构建与发布](zh-CN/BUILD_AND_RELEASE.md) | [Build and release](en/BUILD_AND_RELEASE.md) |
| [测试计划](zh-CN/TEST_PLAN.md) | [Test plan](en/TEST_PLAN.md) |

本页只发布 `docs/`，所以仓库根目录的文件得回 GitHub 上看：[README（English）](https://github.com/nick2781/nano-installer/blob/main/README.md)、[README（简体中文）](https://github.com/nick2781/nano-installer/blob/main/README.zh-CN.md)、[贡献指南](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md)、[安全策略](https://github.com/nick2781/nano-installer/blob/main/SECURITY.md)。

Only `docs/` is published as this site, so the files at the repository root live on GitHub:
[English README](https://github.com/nick2781/nano-installer/blob/main/README.md), [Chinese README](https://github.com/nick2781/nano-installer/blob/main/README.zh-CN.md), [contributing guide](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md), [security policy](https://github.com/nick2781/nano-installer/blob/main/SECURITY.md).
