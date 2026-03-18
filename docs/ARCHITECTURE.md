# nano-installer 架构概览

## 概述

`nano-installer` 采用“CLI + runtime stub + 资源分段打包”的架构：

- CLI 负责读取项目配置、验证资源、打包安装器
- 安装器 stub 负责运行时启动、加载资源、执行安装流程
- 卸载器 stub 负责读取卸载信息并执行清理
- 布局、图片、语言包和 payload 作为资源随 setup 一起分发

## 构建链路

典型构建流程如下：

```text
nano-installer.exe build --project <Project>
  ├─ 读取 installer_config.json
  ├─ 校验 layouts / assets / locales / payload
  ├─ 生成卸载器资源
  ├─ 打包 UI 资源与 payload
  ├─ 复制安装器 stub
  └─ 产出 <Project>_Setup.exe
```

## 运行时链路

安装器运行时主要分成三层：

1. `installer_runtime`
   - 决定当前运行模式
   - 初始化窗口、资源和安装流程
2. `ui + layout`
   - 解析 XML 布局
   - 渲染页面、控件和多语言文案
3. `installer / uninstaller / resources`
   - 解压 payload
   - 执行文件、快捷方式、注册表操作
   - 写入或读取卸载信息

## 目录对应关系

```text
installer/
├── cli/          # 构建工具
├── lib/          # 共享核心库
└── stubs/
   ├── lzma/      # 安装器 runtime stub
   └── uninst/    # 卸载器 runtime stub
```

## UI 与资源

UI 由两部分组成：

- JSON 配置
  - 控制项目基础信息、路径、行为、资源目录
- XML 布局 DSL
  - 控制页面结构、控件位置、图片、文字和交互动作

资源默认按 `1x` 路径声明，运行时会根据 DPI 自动映射到 `@2x` 资源。

## 验证与回归

当前工程把验证分成三层：

- library snapshot / verification tooling
  - 用于检查布局矩形、语言切换、动作分发、资源引用
- script smoke
  - 用于验证真实 EXE 主链路
- manual review
  - 用于最终视觉验收

## 关键工程边界

- 修改 `installer/**`
  - 需要先重编 runtime/stub，再重打 setup
- 只修改 `examples/**`
  - 只需要重建对应示例工程

详见 [AGENTS.md](../AGENTS.md)。
