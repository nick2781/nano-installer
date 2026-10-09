<h1 align="center"><img src="https://nick2781.github.io/nano-installer/assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI 状态"></a></p>

你交付的只有一个 exe，用户双击就能装上你的产品。把配置、素材和应用文件放进一个目录，Nano Installer
就会把它们打成一份 Windows 安装包；图标、界面和文案都由你定，而用户的机器不用预装任何东西。

> **状态：** 可以拿来构建和发布产品。安装包和 MSI 包没有代码签名，Windows 会提示「未知发布者」，
> 公开分发前请在流水线里自己签。**需要 Windows 7 SP1 x64 及以上**。它调用了什么，见
> [Windows 兼容性](WINDOWS_COMPATIBILITY.md)；逐条状态见[当前生产状态](PRODUCTION_STATUS.md)。

<p align="center">
<img src="https://github.com/nick2781/nano-installer/raw/main/assets/setup-welcome-zh-CN.png" alt="用 TapTap 示例构建出来的安装包第一页" width="640">
</p>

上面这张是 `examples/TapTap` 的第一页，由 `scripts/capture_setup_snapshots.ps1` 构建后拍下来——是运行时
画出来的样子，不是效果图。

## 它给你什么

| | |
| --- | --- |
| 只需交付一个文件 | 一个安装包 exe，内置你的图标、版本信息和品牌素材 |
| 干净的机器就能装 | 不用先装运行环境，也不依赖框架：要用的东西都在安装包里 |
| 页面归你 | 用 XML 写页面布局，配自己的背景图和按钮图 |
| 内置 11 种界面语言 | 再加一种语言就是一个 JSON 文件 |
| 进度与完成页 | 进度页实时说明当前在做什么，完成页则能直接启动刚装好的应用 |
| 升级、回滚与更新包 | 重跑就是原地升级，某一步失败会回滚；`--delta-from` 只发字节变了的文件 |
| 卸载 | 只回收自己写下的东西，用户数据默认保留 |
| 管理员权限 | 只在配置要求时才向 Windows 申请 |
| 脚本与插件 | 用 Rhai 写安装和卸载步骤，插件则是照 [`include/nano_plugin.h`](PLUGIN_API.md) 编的 DLL |
| 读屏可用 | 页面、焦点、输入的字符和实时进度都会播报，既走 MSAA，也走 `IDispatch` |
| 界面或命令行 | Windows 10 及以上的可视化构建器，或让 CI 驱动同一套引擎 |
| 还能出 MSI | `--msi` 把安装包封成 MSI，供企业用 Windows Installer 分发 |

## 上手

从[最新发布](https://github.com/nick2781/nano-installer/releases/latest)取构建器，那三个运行时也要一起
取，因为构建器会从自己旁边找它们。然后让它对着项目目录运行：

```powershell
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

接着看[快速开始](QUICK_START.md)，生成第一个安装包，再在虚拟机里试一遍；也可以改用
[可视化构建](GUI.md)，它操作的是同一套引擎。从源码构建工具是贡献者的事，见[构建与发布](BUILD_AND_RELEASE.md)。

## 指南

- [快速开始](QUICK_START.md) - 生成第一个安装包，并在虚拟机中试用
- [可视化构建](GUI.md) - Windows 10 及以上的图形界面
- [配置参考](CONFIG_REFERENCE.md) - 产品信息、安装行为、输出文件名
- [页面布局](XML_LAYOUT_GUIDE.md) - 页面、控件、流式布局、链接与动作
- [多语言](LOCALIZATION.md) - 发布带译文的安装包
- [自定义步骤](SCRIPT_API.md) - 用 Rhai 编写安装与卸载逻辑
- [插件 ABI](PLUGIN_API.md) - 第三方 DLL 照着编的头文件，还有宿主借给它的服务
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
