<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="README.md">English</a> | <b>简体中文</b> | <a href="docs/zh-CN/">文档</a></p>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI 状态"></a></p>

一个项目目录进去，一个安装包 exe 出来。配置、素材、应用文件都放进去，Nano Installer 就把它们打成一个
Windows 安装包，图标、页面、文案都由你定；不用自己搭服务，用户的机器上也不用先装东西。

> **早期实现，尚不适合对外发布产品。** 安装会写入文件和注册表，所以请在临时虚拟机里测试；安装包又没有
> 代码签名，Windows 会提示「未知发布者」。**需要 Windows 7 SP1 x64 及以上。** 详见
> [当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md)。

<img src="assets/setup-welcome-zh-CN.png" alt="用 TapTap 示例构建出来的安装包第一页：产品 logo、一句标语、安装选项，以及「立即安装」按钮" width="720">

这是 `examples/TapTap` 的第一页，由 `scripts/capture_setup_snapshots.ps1` 构建后拍下来，是运行时真画出来的
样子，不是效果图。

## 装出第一个安装包

不用先装什么，也不用自己编译：每次[发布](https://github.com/nick2781/nano-installer/releases/latest)都会
发出构建器、它要用的运行时，还有每个文件一行的摘要。

```powershell
# 1. 把一次发布的文件下到同一个目录
# 2. 让构建器对着项目目录跑
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

安装包落在项目的 `dist/<installer_name>` 里。想点界面的话，同一次发布里的
`nano-installer-gui-x64.exe` 走同一套引擎；完整步骤见[快速开始](docs/zh-CN/QUICK_START.md)。

## 它给你什么

| | |
| --- | --- |
| 只需交付一个文件 | 一个安装包 exe，图标、版本信息和品牌素材都在里面 |
| 干净的机器就能装 | 不用先装运行时，也不用框架：要用的东西安装包自己带着 |
| 页面归你 | 用 XML 布局画页面，配你自己的背景图和按钮图 |
| 内置 11 种界面语言 | 再加一种语言，就是一个 JSON 文件 |
| 进度与完成页 | 进度页实时报出当前在做什么，完成页则能直接启动刚装好的应用 |
| 升级、回滚与更新包 | 重跑就是原地升级，某一步失败会回滚；`--delta-from` 只发字节变了的文件 |
| 卸载 | 只收回自己写下的东西，用户数据默认保留 |
| 管理员权限 | 只在工程要求时才向 Windows 申请 |
| 脚本与插件 | 安装与卸载步骤用 Rhai 写，插件则是照 [`include/nano_plugin.h`](docs/zh-CN/PLUGIN_API.md) 编的 DLL |
| 读屏可用 | 页面、焦点、输入的文字和实时进度都会播报，既走 MSAA，也走 `IDispatch` |
| 界面或命令行 | 要界面就用 Windows 10+ 的可视化构建器，要 CI 就用同一套引擎 |
| 还能出 MSI | `--msi` 把安装包封成 MSI，给走 Windows Installer 分发的环境用 |

## 一个项目目录长这样

```
MyApp/
  installer_config.json     产品名、版本、安装路径、快捷方式、输出文件名
  layouts/                  XML 页面：欢迎、进度、完成、卸载
  assets/                   背景图、按钮图、图标（1x 与 @2x）
  locales/                  每种语言一个 JSON 文件
  scripts/                  可选的安装与卸载逻辑
  payload/app.7z            你的应用文件，zip 或 7z
```

配置里的路径都相对这个目录，工程搬哪儿都能跑。想最快开始，就复制
[`examples/TapTap`](examples/README.md) 再把里面的东西换掉；要换的是配置的写法，素材本身属于它的权利人。

## 文档

[快速开始](docs/zh-CN/QUICK_START.md) &middot; [配置参考](docs/zh-CN/CONFIG_REFERENCE.md) &middot;
[页面布局](docs/zh-CN/XML_LAYOUT_GUIDE.md) &middot; [多语言](docs/zh-CN/LOCALIZATION.md) &middot;
[自定义步骤](docs/zh-CN/SCRIPT_API.md) &middot; [插件 ABI](docs/zh-CN/PLUGIN_API.md) &middot;
[可视化构建](docs/zh-CN/GUI.md) &middot; [从 NSIS 迁移](docs/zh-CN/MIGRATION_FROM_NSIS.md)

站点是 **https://nick2781.github.io/nano-installer/**，英文页在 [`docs/en`](docs/en/)；每一页同时是一个
Markdown 文件，其中 [`docs/llms.txt`](docs/llms.txt) 是给 agent 的索引，
[`docs/llms-full.txt`](docs/llms-full.txt) 把英文文档合成一份。架构、构建与发布、Windows 兼容性、
测试计划、生产状态这些维护者文档在 [`docs/zh-CN`](docs/zh-CN/) 里。

## 参与、安全、许可

怎么准备一份工作副本、一个改动要过哪些检查，见[贡献指南](CONTRIBUTING.md)；这个仓库守的规矩也在那里，
包括改动必须经 PR 进入 `main`。[issue 表单](.github/ISSUE_TEMPLATE)会要版本号和运行日志，缺了就得
多一个来回。安全问题走[私下上报](SECURITY.md)，不要开 issue，那一页也列了哪些是已知的、不算漏洞。
这里对每个人的要求写在[行为准则](CODE_OF_CONDUCT.md)。Rust 源码是 [MIT 许可](LICENSE)；`examples/TapTap`
里的商标、图片和文案归易玩（上海）网络科技有限公司所有，不是 MIT 许可，`assets/` 下的截图就来自那个
示例。
