<h1 align="center"><img src="../assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="../en/README.md">English</a> | <b>简体中文</b> | <a href="../README.md">全部文档</a></p>

把安装包交给用户，他们双击一个 exe 就装好了你的产品。Nano Installer 把一个装着配置、素材和
应用文件的目录，变成单个 Windows 安装程序：内置你自己的 Logo、你自己的界面、你自己写的文案，
不需要宿主的安装框架，用户机器上也不需要预装任何东西。

> **状态：** 早期实现，尚不适合对外发布产品。安装动作会写入文件和注册表，请在一次性虚拟机中测试。

## 它给你什么

| | |
| --- | --- |
| 只需交付一个文件 | 一个 exe，内置你配置的图标、版本信息和品牌素材 |
| 用户机器零依赖 | 干净的 Windows 7 SP1 x64 及以上即可运行 |
| 你的界面，不是我们的 | 用 XML 描述页面和控件，换上自己的背景图和按钮图 |
| 需要多少语言都行 | 内置 11 种界面语言，也可以增加自己的 JSON 语言文件 |
| 升级不会出错 | 重复运行安装包即原地升级，中途失败会自动回到升级前的状态 |
| 卸载不留痕迹 | 卸载只回收自己装下的内容，用户数据默认保留 |
| 权限按需申请 | 需要管理员权限时由配置决定，安装包自己向 Windows 申请 |
| 适配你的流程 | 日常点可视化界面；CI 里用命令行驱动同一套引擎 |

## 上手

```powershell
.\scripts\build.ps1
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

接着看[快速开始](QUICK_START.md)，或改用[可视化构建](GUI.md)操作同一套引擎。

## 指南

- [快速开始](QUICK_START.md) - 生成第一个安装包，并在虚拟机中试用
- [可视化构建](GUI.md) - Windows 10+ 编写工具
- [配置参考](CONFIG_REFERENCE.md) - 产品信息、安装行为、输出文件名
- [页面布局](XML_LAYOUT_GUIDE.md) - 页面、控件、流式布局、链接与动作
- [多语言](LOCALIZATION.md) - 发布带译文的安装包
- [自定义步骤](SCRIPT_API.md) - 用 Rhai 编写安装与卸载逻辑
- [当前生产状态](PRODUCTION_STATUS.md) - 现在能做什么、还差什么才能发布

## 技术说明

- [架构](ARCHITECTURE.md)
- [项目结构](PROJECT_STRUCTURE.md)
- [Windows 兼容性](WINDOWS_COMPATIBILITY.md)
- [构建与发布](BUILD_AND_RELEASE.md)
- [测试计划](TEST_PLAN.md)
