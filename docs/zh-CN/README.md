<h1 align="center"><img src="../assets/nano-technology.png" width="48" height="48" align="texttop" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="../en/README.md">English</a> | <b>简体中文</b></p>

Nano Installer 把一个装着配置、图片和压缩包 payload 的目录，变成单个 Windows 安装程序 exe。
不需要在目标电脑上先装任何运行库：生成的 exe 自带界面、解压能力和卸载程序。

> **状态：** 早期实现，尚不适合对外发布产品。安装动作会写入文件和注册表，请在一次性虚拟机中测试。

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

```powershell
.\scripts\build.ps1
.\target\release\nano-installer-native-x64.exe build --project .\examples\TapTap
```

接着看[快速开始](QUICK_START.md)，或改用[可视化构建](GUI.md)操作同一套引擎。

## 指南

- [快速开始](QUICK_START.md) - 生成第一个安装包
- [可视化构建](GUI.md) - Windows 10+ 编写工具
- [配置参考](CONFIG_REFERENCE.md) - 当前真正生效的全部设置
- [页面布局](XML_LAYOUT_GUIDE.md) - 页面、控件、流式布局与动作
- [多语言](LOCALIZATION.md) - 发布带译文的安装包
- [自定义步骤](SCRIPT_API.md) - 用 Rhai 编写安装与卸载逻辑
- [当前生产状态](PRODUCTION_STATUS.md) - 已经能做什么、还差什么才能发布

## 技术说明

- [架构](ARCHITECTURE.md)
- [项目结构](PROJECT_STRUCTURE.md)
- [Windows 兼容性](WINDOWS_COMPATIBILITY.md)
- [构建与发布](BUILD_AND_RELEASE.md)
- [测试计划](TEST_PLAN.md)
