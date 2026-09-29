<h1 align="center"><img src="https://nick2781.github.io/nano-installer/assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI 状态"></a></p>

你交付的只有一个 exe，用户双击就能装上你的产品。配置、素材和应用文件放在同一个目录里，Nano
Installer 把它打成一份 Windows 安装包；图标、界面和文案都由你决定，用户的机器上不用预装任何东西。

> **状态：** 早期实现，尚不适合对外发布产品。安装动作会写入文件和注册表，请在一次性虚拟机中测试。
> 安装包本身没有代码签名，Windows 7 支持也还没在真机上验收过，逐条见[当前生产状态](PRODUCTION_STATUS.md)。

<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-zh-CN.png" alt="用 TapTap 示例构建出来的安装包第一页" width="640">

上面这张是 `examples/TapTap` 装出来的第一页，由 `scripts/capture_setup_snapshots.ps1` 在 192 dpi 下
截取：脚本会把示例声明的每一页都拍下来，并拿工程自己的版面逐项核对。版面是 720x450，截图是 1440x900，
一个版面像素对应两个图像像素，所以在会缩放的屏幕上依然清晰。图上是运行时画出来的样子，不是效果图。

## 它给你什么

| | |
| --- | --- |
| 只需交付一个文件 | 一个 exe，内置你配置的图标、版本信息和品牌素材 |
| 干净的机器就能装 | Windows 7 SP1 x64 及以上，用户机器零依赖 |
| 页面和控件归你 | 用 XML 描述，换上自己的背景图和按钮图 |
| 内置 11 种界面语言 | 也可以添加自己的 JSON 语言文件 |
| 升级与回滚 | 重复运行安装包即原地升级，中途失败会自动回到升级前的状态 |
| 卸载 | 卸载只回收自己装下的内容，用户数据默认保留 |
| 管理员权限 | 配置要求时才向 Windows 申请 |
| 进度与完成页 | 实时进度会说明当前在做什么，完成页可直接启动刚装好的应用 |
| 更新包 | `--delta-from` 只发字节变了的文件；安装时先核对机器上已有的那些，再动手写 |
| 读屏 | 另一个进程里的读屏会被告知当前是哪个页面、焦点在哪、正在输入的字符、任务发布的状态，以及提问画在哪张卡片上 |
| 插件 | 第三方按 [`include/nano_plugin.h`](PLUGIN_API.md) 编一个 DLL，脚本就能调用它 |
| 界面或命令行 | 日常点可视化界面；CI 里用命令行驱动同一套引擎 |

## 上手

```powershell
.\scripts\build.ps1
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

接着看[快速开始](QUICK_START.md)，照着生成第一个安装包并在虚拟机里试一遍；也可以改用
[可视化构建](GUI.md)，它操作的是同一套引擎。

## 指南

- [快速开始](QUICK_START.md) - 生成第一个安装包，并在虚拟机中试用
- [可视化构建](GUI.md) - Windows 10 及以上的图形界面
- [配置参考](CONFIG_REFERENCE.md) - 产品信息、安装行为、输出文件名
- [页面布局](XML_LAYOUT_GUIDE.md) - 页面、控件、流式布局、链接与动作
- [多语言](LOCALIZATION.md) - 发布带译文的安装包
- [自定义步骤](SCRIPT_API.md) - 用 Rhai 编写安装与卸载逻辑
- [插件 ABI](PLUGIN_API.md) - 第三方 DLL 照着编的头文件，以及宿主借给它的服务
- [从 NSIS 迁移](MIGRATION_FROM_NSIS.md) - 哪些对应得上，哪些是故意不做的
- [当前生产状态](PRODUCTION_STATUS.md) - 现在能做什么、还差什么才能发布

## 技术说明

- [架构](ARCHITECTURE.md)
- [项目结构](PROJECT_STRUCTURE.md)
- [Windows 兼容性](WINDOWS_COMPATIBILITY.md)
- [构建与发布](BUILD_AND_RELEASE.md)
- [测试计划](TEST_PLAN.md)

---

读英文版：[English documentation](../en/README.md)。
