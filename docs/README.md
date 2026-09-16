<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="en/README.md">English documentation</a> &middot; <a href="zh-CN/README.md">中文文档</a></p>

把安装包交给用户，他们双击一个 exe 就装好了你的产品。Nano Installer 把一个装着配置、素材和
应用文件的目录，变成单个 Windows 安装程序：内置你自己的 Logo、界面和文案，用户机器上不需要
预装任何东西。

Hand someone a single `.exe` and they have your product installed. Nano Installer turns a folder of
configuration, artwork, and your packaged application into one Windows setup file with your own
logo, your own interface, and your own wording, and nothing for your users to install first.

> **状态 / Status:** 早期实现，尚不适合对外发布产品。安装动作请只在一次性虚拟机中测试。
>
> Early implementation, not ready for production distribution. Test install actions only in a
> disposable VM.

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

仓库说明见 [README.md](../README.md)（English）与 [README.zh-CN.md](../README.zh-CN.md)（简体中文）。

The repository README is available in [English](../README.md) and [简体中文](../README.zh-CN.md).
