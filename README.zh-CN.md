<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="README.md">English</a> | <b>简体中文</b> | <a href="docs/zh-CN/">文档</a></p>

<p align="center"><a href="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml"><img src="https://github.com/nick2781/nano-installer/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI 状态"></a></p>

你交付的只有一个 exe，用户双击就能装上你的产品。配置、素材和应用文件都放在同一个目录里，Nano
Installer 把它打包成一个 Windows 安装包。图标、界面和文案都由你决定。用户的机器上不用预装任何
东西。

> **状态：** 早期实现，尚不适合对外发布产品。安装动作会写入文件和注册表，请在一次性虚拟机中
> 测试；正式规划发布前请先读[当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md)。

<img src="assets/setup-welcome-zh-CN.png" alt="用 TapTap 示例构建出的安装包首屏：产品 logo、一句标语、安装选项，以及「立即安装」按钮" width="720">

这张图就是 `examples/TapTap` 的首屏，由 `scripts/capture_setup_snapshots.ps1` 在 200% 显示缩放下拍下来——
版面 720x450、图 1440x900，所以在会缩放的屏幕上不会糊。脚本会把示例真的构建出来，再逐页截图并对照
工程自己的布局检查，所以图上是运行时画出来的样子，不是效果图。

## 它给你什么

| | |
| --- | --- |
| 只需交付一个文件 | 一个 exe，内置你配置的图标、版本信息和品牌素材 |
| 干净的机器就能装 | Windows 7 SP1 x64 及以上，用户机器零依赖 |
| 页面和控件归你 | 用 XML 描述，换上自己的背景图和按钮图 |
| 内置 11 种界面语言 | 也可以另外增加自己的 JSON 语言文件 |
| 升级与回滚 | 重复运行安装包即原地升级，中途失败会自动回到升级前的状态 |
| 更新包 | `--delta-from` 只发字节变了的文件；安装时先核对机器上已有的那些，再动手写 |
| 卸载 | 卸载只回收自己装下的内容，用户数据默认保留 |
| 管理员权限 | 配置要求时才向 Windows 申请 |
| 进度与完成页 | 实时进度会说明当前在做什么，完成页可直接启动刚装好的应用 |
| 读屏 | 另一个进程里的读屏被告知：这一页是什么、焦点在哪、字段里正在敲进去的值、任务报出来的那句话与进度条、提问卡片——走 MSAA，也走 `IDispatch`（按名字问的客户程序拿到同样的答案） |
| 插件 | 第三方按 [`include/nano_plugin.h`](docs/zh-CN/PLUGIN_API.md) 编一个 DLL，脚本就能调用它 |
| 界面或命令行 | 日常点可视化界面；CI 里用命令行驱动同一套引擎 |

## 上手

需要 Windows x64、MSVC 构建工具，以及本仓库固定的 Rust 工具链。请用 `git clone` 取代码，并先跑一次
`git lfs install`：图标、示例图片和 `tools/` 下的压缩工具都存在 Git LFS 里，直接下载源码 ZIP 拿到的是
指针文件而不是它们本身。

```powershell
# 1. 构建工具（每个检出做一次）
.\scripts\build.ps1

# 2. 用项目目录生成安装包
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

生成的安装包放在项目的 `dist/<installer_name>` 里。不想敲命令的话，启动
`target/release/nano-installer-gui-x64.exe`，在里面选项目目录也一样。

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

配置里的路径都相对项目目录，所以项目可以整个搬走。上手最快的办法是复制 `examples/TapTap` 的配置骨架，
再把里面的内容换成你自己的——素材本身不属于你，它归示例里的权利人。`examples/` 下三个例子各能证明
什么，见 [examples/README.md](examples/README.md)。

## 现在装出来是什么样

- 一个可执行文件同时负责安装和卸载，Windows 7 SP1 x64 及以上都能跑。
- 应用文件给 ZIP 或 7z 都行：安装包自己读文件头判断，不需要你在配置里选。
- 用户安装时能看到实时进度和当前步骤，完成页还能直接启动刚装好的应用。
- 再运行一次安装包就是原地升级；某一步失败，机器会回到升级前的状态。
- 桌面快捷方式、开始菜单项和开机自启动只在用户勾选时创建，卸载时一并清理。
- 卸载默认保留用户数据，只有用户自己取消勾选才会删。
- 内置 11 种界面语言，切换即时生效；页面上的服务协议、隐私政策点一下就能打开。
- 安装目录可以让用户用 Windows 自带的文件夹选择框来挑，也可以手动改路径：鼠标拖选或双击选词、
  复制粘贴、写错了撤销一步。
- 中日韩文字用正规的输入法组合窗输入，候选框跟在光标下面。
- 项目在配置里打开 `install.require_admin` 后，安装包会自己向 Windows 要管理员权限，用户不用
  再右键「以管理员身份运行」。
- 关闭窗口前，安装包会先问一句本地化确认问题。
- 卸载一结束就删掉安装目录；用户往里面放过文件的话，目录会留下。
- 布局里写的颜色、圆角和描边都会画出来，鼠标移到能点的地方会变成手型。
- 内置步骤不够用时，可以用一份小脚本自己写安装和卸载流程；脚本出错会回滚，不会留下装到一半的
  产品。
- 工程可以带上自己的插件：一个按 [`include/nano_plugin.h`](docs/zh-CN/PLUGIN_API.md) 编出来的 64 位
  DLL 随安装包发出去，脚本用 `plugin_call("dll::function", [...])` 调用它；插件经宿主写下的东西照样
  进清单，卸载收得回来。
- 页面不只是画出来，还会被读给读屏听：焦点、字段里正在敲进去的值、任务报出来的话与进度条、提问
  卡片；按成员名通过 `IDispatch` 来问的客户程序，拿到的答案与走 `MSAA` 的逐项一致。
- 构建时多给一个 `--msi <文件>`，可以把做完的安装包封成更大规模部署用的那个包：`.msi` 通过
  Windows Installer 无窗口装上产品、也能把它卸干净，新版本的包会升级旧版包装出来的那一份。
- Windows 10 及以上还有可视化构建界面，生成的安装包和命令行一模一样。

## 目前还没有

- 安装包与外面那层 MSI 都没有签名，Windows SmartScreen 会提示「未知发布者」；MSI 装给整台
  机器时需要提权的会话。
- 每次构建都会审计用到的系统调用，Windows 7 兼容性靠它保证，但还没有在真实的 Windows 7 SP1
  机器上做过最终验收。

## 文档

文档站点：**https://nick2781.github.io/nano-installer/**

站点只是那些 Markdown 文件的一个浏览器视图，所以不用浏览器也能读：
[`docs/llms.txt`](docs/llms.txt) 是写给 agent 的索引，[`docs/llms-full.txt`](docs/llms-full.txt)
是英文文档合成的一份。

使用指南：[快速开始](docs/zh-CN/QUICK_START.md) &middot;
[可视化构建](docs/zh-CN/GUI.md) &middot;
[配置参考](docs/zh-CN/CONFIG_REFERENCE.md) &middot;
[页面布局](docs/zh-CN/XML_LAYOUT_GUIDE.md) &middot;
[多语言](docs/zh-CN/LOCALIZATION.md) &middot;
[自定义步骤](docs/zh-CN/SCRIPT_API.md) &middot;
[插件 ABI](docs/zh-CN/PLUGIN_API.md) &middot;
[从 NSIS 迁移](docs/zh-CN/MIGRATION_FROM_NSIS.md)

技术说明：[架构](docs/zh-CN/ARCHITECTURE.md) &middot;
[构建与发布](docs/zh-CN/BUILD_AND_RELEASE.md) &middot;
[Windows 兼容性](docs/zh-CN/WINDOWS_COMPATIBILITY.md) &middot;
[测试计划](docs/zh-CN/TEST_PLAN.md) &middot;
[当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md) &middot;
[项目结构](docs/zh-CN/PROJECT_STRUCTURE.md)

每个 release 里的 5 个可执行文件旁边都带一份 `SHA256SUMS.txt`，下载之后可以用
`sha256sum -c SHA256SUMS.txt` 核对。

英文文档见 [docs/en](docs/en/)。

## 参与

怎么把工作区装起来、一个改动必须过哪几条检查、这个仓自己有哪些规矩，都写在
[CONTRIBUTING.md](CONTRIBUTING.md) 里。缺陷、插件问题、功能请求、使用提问各有一张
[issue 表单](.github/ISSUE_TEMPLATE)；每张都问版本号和运行日志，因为少了这两样就得来回一轮。

安全漏洞请不要开 issue，走私有上报，见 [SECURITY.md](SECURITY.md)；那份文件也列出哪些在这里
**不算**漏洞，比如没有代码签名。大家在这里该守什么规矩，见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## 权利与许可

- 本仓库的 Rust 源码以 [MIT License](LICENSE) 授权。
- `examples/TapTap` 仅用于校验。其中的 TapTap 商标、图片与文案归
  **易玩（上海）网络科技有限公司**及相关权利人所有，不属于 MIT 授权范围；`assets/` 下的截图就是
  这个示例的截图，同样适用。
- 你生成的安装包携带你自己配置的产品资源，按你产品的授权处理。
