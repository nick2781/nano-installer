<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="README.md">English</a> | <b>简体中文</b> | <a href="docs/zh-CN/">文档</a></p>

Nano Installer 把一个装着配置、图片和压缩包 payload 的目录，变成单个 Windows 安装程序 exe。
不需要在目标电脑上先装任何运行库：生成的 exe 自带界面、解压能力和卸载程序。

> **状态：** 早期实现，尚不适合对外发布产品。安装动作会写入文件和注册表，请在一次性虚拟机中
> 测试；正式规划发布前请先读[当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md)。

## 它给你什么

| | |
| --- | --- |
| 单个自包含安装包 | 一个 exe，内置你配置的图标、版本信息和品牌素材 |
| 目标机器零依赖 | Windows 7 SP1 x64 及以上，无需额外安装任何东西 |
| 你设计的界面 | 用 XML 排布页面和控件，使用自己的背景图和按钮图 |
| 开箱支持 11 种语言 | 直接换上译文，或增加自己的语言文件 |
| 升级安全、卸载干净 | 重复安装即原地升级并可回滚；卸载只删除自己装下的内容 |
| 需要时可自动化 | 日常使用点可视化界面；命令行供 CI 批量构建 |

## 上手

需要 Windows x64、MSVC 构建工具，以及本仓库固定的 Rust 工具链。

```powershell
# 1. 构建工具（每个检出做一次）
.\scripts\build.ps1

# 2. 用项目目录生成安装包
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

安装包输出到项目内的 `dist/<installer_name>`。更喜欢点界面？启动
`target/release/nano-installer-gui-x64.exe`，在里面选择项目目录即可。

完整步骤见[快速开始](docs/zh-CN/QUICK_START.md)。

## 一个项目目录长这样

```
MyApp/
  installer_config.json     产品名、版本、安装路径、快捷方式、输出文件名
  layouts/                  XML 页面：欢迎、进度、完成、卸载
  assets/                   背景、按钮、图标（含 1x 与 @2x 两档）
  locales/                  每种语言一个 JSON 文件
  scripts/                  可选的安装/卸载逻辑
  payload/app.7z            你的应用文件，ZIP 或 7z 压缩包
```

配置里的路径都相对项目目录解析，项目可以整体搬移。最快的起点是复制 `examples/TapTap`，
再替换里面的内容。

## 现在已经可以

- 安装程序与卸载程序合成单个可执行文件，支持 Windows 7 SP1 x64 及以上。
- ZIP 与 7z payload，按文件头自动识别。
- 安装、升级、失败回滚与卸载，带进度页；完成页可直接启动刚装好的应用。
- 桌面快捷方式、开始菜单项和开机自启动，安装时可勾选，卸载时一并清理。
- 卸载默认保留用户数据，用户取消勾选「保留数据」才会删除。
- 11 种语言的界面文案，运行时即可切换。
- 内置步骤不够用时，可用 [Rhai 脚本](docs/zh-CN/SCRIPT_API.md)自定义安装与卸载步骤。
- Windows 10 及以上的可视化构建界面，与命令行共用同一套构建引擎。

## 目前还没有

- 安装包尚未签名，Windows SmartScreen 会提示「未知发布者」。
- 不会自动申请管理员权限。要写入 `Program Files` 的安装包需要以管理员身份启动。
- Windows 7 兼容性目前靠静态导入检查保证，尚未在真实 Windows 7 SP1 机器上完整跑通。

## 文档

使用指南：[快速开始](docs/zh-CN/QUICK_START.md) &middot;
[可视化构建](docs/zh-CN/GUI.md) &middot;
[配置参考](docs/zh-CN/CONFIG_REFERENCE.md) &middot;
[页面布局](docs/zh-CN/XML_LAYOUT_GUIDE.md) &middot;
[多语言](docs/zh-CN/LOCALIZATION.md) &middot;
[自定义步骤](docs/zh-CN/SCRIPT_API.md)

技术说明：[架构](docs/zh-CN/ARCHITECTURE.md) &middot;
[构建与发布](docs/zh-CN/BUILD_AND_RELEASE.md) &middot;
[Windows 兼容性](docs/zh-CN/WINDOWS_COMPATIBILITY.md) &middot;
[测试计划](docs/zh-CN/TEST_PLAN.md) &middot;
[当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md) &middot;
[项目结构](docs/zh-CN/PROJECT_STRUCTURE.md)

英文文档见 [docs/en](docs/en/)。

## 权利与许可

- 本仓库的 Rust 源码以 [MIT License](LICENSE) 授权。
- `examples/TapTap` 仅用于校验。其中的 TapTap 商标、图片与文案归
  **易玩（上海）网络科技有限公司**及相关权利人所有，不属于 MIT 授权范围。
- 你生成的安装包携带你自己配置的产品资源，按你产品的授权处理。
