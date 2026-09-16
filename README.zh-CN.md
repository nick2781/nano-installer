<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="README.md">English</a> | <b>简体中文</b> | <a href="docs/zh-CN/">文档</a></p>

把安装包交给用户，他们双击一个 exe 就装好了你的产品。Nano Installer 把一个装着配置、素材和
应用文件的目录，变成单个 Windows 安装程序：内置你自己的 Logo、你自己的界面、你自己写的文案，
不需要宿主的安装框架，用户机器上也不需要预装任何东西。

> **状态：** 早期实现，尚不适合对外发布产品。安装动作会写入文件和注册表，请在一次性虚拟机中
> 测试；正式规划发布前请先读[当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md)。

## 它给你什么

| | |
| --- | --- |
| 只需交付一个文件 | 一个 exe，内置你配置的图标、版本信息和品牌素材 |
| 用户机器零依赖 | 干净的 Windows 7 SP1 x64 及以上即可运行 |
| 你的界面，不是我们的 | 用 XML 描述页面和控件，换上自己的背景图和按钮图 |
| 需要多少语言都行 | 内置 11 种界面语言，也可以增加自己的 JSON 语言文件 |
| 升级不会出错 | 重复运行安装包即原地升级，中途失败会自动回到升级前的状态 |
| 卸载不留痕迹 | 卸载只回收自己装下的内容，用户数据默认保留 |
| 过程清晰友好 | 实时进度会说明当前在做什么，完成页可直接启动刚装好的应用 |
| 适配你的流程 | 日常点可视化界面；CI 里用命令行驱动同一套引擎 |

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

## 现在装出来是什么样

- 安装与卸载合并在一个可执行文件里，支持 Windows 7 SP1 x64 及以上。
- 支持 ZIP 与 7z 两种压缩包，按文件本身自动识别，不需要你在配置里做选择。
- 安装过程显示实时进度和当前步骤，结束后的完成页可以直接启动刚装好的应用。
- 再次运行安装包按原地升级处理；某一步失败会把机器恢复到安装前的状态。
- 桌面快捷方式、开始菜单项和开机自启动都由用户勾选，卸载时一并清理。
- 用户数据默认保留，只有用户主动取消勾选才会删除。
- 内置 11 种界面语言，切换即时生效；页面上的服务协议、隐私政策可以直接点击打开。
- 安装目录可以用 Windows 标准的文件夹选择框来选，也可以手动改：鼠标拖选或双击选词、复制粘贴、
  写错了撤销一步。
- 关闭窗口前会先问一句本地化确认问题，避免误操作打断安装。
- 卸载完成后立刻回收安装目录，不留空文件夹；用户自己放进去的文件会让目录保留下来。
- 会绘制布局里声明的颜色、圆角和描边，鼠标移到可点击的控件上会变成手型。
- 内置步骤不够用时，可以用一份小脚本自定义安装与卸载流程；脚本出错会回滚，不会留下装到一半的产品。
- 提供 Windows 10 及以上的可视化构建界面，产物与命令行完全一致。

## 目前还没有

- 安装包尚未签名，Windows SmartScreen 会提示「未知发布者」。
- 安装包不会自动申请管理员权限。要写入 `Program Files` 的安装包需要以管理员身份启动。
- Windows 7 兼容性目前由每次构建的系统调用自动审计保证，尚未在真实 Windows 7 SP1 机器上完成
  最终验收。

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
