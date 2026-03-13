# nano-installer

<div align="center">

**现代化的 Windows 安装器生成工具**

[![Rust](https://img.shields.io/badge/rust-1.81%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2B-blue)](https://www.microsoft.com/windows)

*灵感源自 NSIS，采用 Rust 和现代技术栈重新打造*

[快速开始](#-快速开始) • [特性](#-特性) • [架构](#-架构) • [文档](#-文档)

</div>

---

## 📖 简介

nano-installer 是一个现代化的 Windows 安装器生成工具，采用 Rust 语言开发，提供：

- 🎨 **现代 UI** - 基于 Egui 的原生界面
- ⚙️ **配置驱动** - JSON 配置 + XML 布局系统
- 🌍 **多语言** - 内置 11+ 种语言支持
- 🚀 **高性能** - Rust 保证的安全性和性能
- 🔧 **易扩展** - 清晰的模块化架构

### 与 NSIS 的对比

| 特性 | NSIS | nano-installer |
|------|------|----------------|
| 编译器 | makensis.exe | nano-installer.exe |
| 配置格式 | .nsi 脚本 | JSON + XML |
| UI 框架 | Win32 | Egui (现代化) |
| 语言 | C/汇编 | Rust |
| 安装器大小 | ~300 KB | ~4 MB* |
| 类型安全 | ❌ | ✅ |
| 现代化 | ⚠️ | ✅ |

*大小差异主要来自 GUI 框架，未来将持续优化

## ✨ 特性

### 核心功能
- ✅ 图形化安装向导（欢迎、许可、路径选择、安装、完成）
- ✅ 自定义 XML 布局系统
- ✅ 多语言支持（运行时切换）
- ✅ 7z 压缩的 payload
- ✅ 注册表操作
- ✅ 快捷方式创建
- ✅ 卸载程序生成

### 开发体验
- 📦 一键初始化项目模板
- 🔄 热重载配置（无需重新编译）
- 📝 JSON Schema 验证
- 🎯 清晰的错误提示
- 📚 完整的文档和示例

## 🚀 快速开始

### 安装

#### 从源码编译
```bash
git clone https://github.com/your-org/nano-installer.git
cd nano-installer
cargo build --release

# 安装到系统
cargo install --path installer/cli
```

#### 使用预编译版本
从 [Releases](https://github.com/your-org/nano-installer/releases) 下载最新版本。

### 创建第一个安装器

```bash
# 1. 初始化新项目
nano-installer init MyApp

# 2. 进入项目目录
cd MyApp

# 3. 准备你的应用程序文件
# 推荐直接放到 files/ 目录；build 时会自动打包成 payload/app.7z

# 4. 编辑配置（可选）
# 编辑 installer_config.json 自定义安装器

# 5. 构建安装器
nano-installer build

# 6. 测试安装器
cd dist
.\MyApp_Setup.exe
```

生成的 `MyApp_Setup.exe` 是一个独立的安装器，包含所有资源和应用程序文件。

## 📁 项目结构

```
MyApp/                          # 你的安装器项目
├── installer_config.json       # 主配置文件
├── layouts/                    # XML 布局文件
│   ├── welcome.xml
│   ├── config.xml
│   ├── installing.xml
│   └── finish.xml
├── assets/                     # 资源文件（图片、图标）
│   ├── logo.ico
│   └── background.png
├── locales/                    # 语言文件
│   ├── en-US.json
│   └── zh-CN.json
├── files/                      # 应用程序文件（推荐）
├── payload/                    # 可选：预先打好的归档或 build 自动生成
│   └── app.7z
└── dist/                       # 构建输出
    └── MyApp_Setup.exe         # 最终的安装器
```

## 🏗️ 架构

nano-installer 采用模块化的 Workspace 架构：

```
nano-installer/
└── installer/
    ├── cli/                    # 命令行工具
    │   └── nano-installer.exe  # 类似 makensis.exe
    ├── lib/                    # 核心库
    │   └── 所有核心功能实现
    └── stubs/                  # 运行时 stubs
        ├── lzma/               # 安装器 stub
        │   └── lzma-x64-unicode.exe
        └── uninst/             # 卸载器 stub
            └── uninst.exe
```

### 工作流程

```
开发者                      最终用户
   │                           │
   ├─ nano-installer init      │
   ├─ 编辑配置和资源           │
   ├─ nano-installer build     │
   │                           │
   └─ MyApp_Setup.exe ────────→ 运行安装器
                               ├─ 显示 GUI
                               ├─ 解压文件
                               ├─ 创建快捷方式
                               └─ 生成卸载器
```

详细架构说明请查看 [PROJECT_STRUCTURE.md](docs/PROJECT_STRUCTURE.md)。

## 📝 配置示例

### installer_config.json

```json
{
  "product": {
    "name": "MyApp",
    "version": "1.0.0",
    "publisher": "My Company",
    "website": "https://example.com"
  },
  "install": {
    "default_path": "$PROGRAMFILES\\MyApp",
    "required_space": 100,
    "mutex_name": "MyApp_Installer_Mutex"
  },
  "ui": {
    "title": "MyApp Setup",
    "icon": "assets/logo.ico",
    "default_locale": "en-US"
  },
  "resources": {
    "layouts_dir": "layouts",
    "assets_dir": "assets",
    "locales_dir": "locales",
    "installer_icon": "assets/logo.ico",
    "uninstaller_icon": "assets/uninst.ico"
  },
  "payload": {
    "file": "payload/app.7z",
    "type": "7z-or-zip"
  }
}
```

### 布局示例 (welcome.xml)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<layout>
  <window width="600" height="400">
    <vbox padding="20">
      <image src="logo.png" width="200" height="200"/>
      <text id="welcome_title" font-size="24" bold="true"/>
      <text id="welcome_message" font-size="14"/>
      <spacer/>
      <hbox>
        <button id="btn_cancel" text="Cancel"/>
        <button id="btn_next" text="Next" primary="true"/>
      </hbox>
    </vbox>
  </window>
</layout>
```

## 📚 文档

### 核心文档
- [PROJECT_STRUCTURE.md](docs/PROJECT_STRUCTURE.md) - 项目结构详解
- [installer/README.md](installer/README.md) - 核心代码说明
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - 架构设计

### 指南
- [快速开始指南](docs/guides/QUICKSTART.md) - 5 分钟上手
- [配置参考](docs/API.md) - 完整的配置选项
- [布局系统](docs/UI_DESIGN.md) - XML 布局指南
- [多语言支持](docs/LOCALIZATION_UPDATE.md) - 国际化指南

### 开发文档
- [架构设计](docs/ARCHITECTURE.md) - 深入理解架构
- [开发指南](docs/DEVELOPMENT.md) - 贡献代码
- [更新日志](CHANGELOG.md) - 版本历史

## 🎯 示例项目

### TapTap 客户端安装器

查看 [examples/TapTap](examples/TapTap/) 目录，这是一个完整的实际案例：

```bash
cd examples/TapTap
../../target/release/nano-installer build
```

生成的 `dist/TapTap_Setup.exe` 包含：
- 完整的图形化安装向导
- 11 种语言支持
- 自定义 UI 布局
- 应用程序打包（7z 压缩）
- 卸载程序

## 🔧 命令行参考

```bash
# 初始化新项目
nano-installer init <project-name> [--output <dir>]

# 构建安装器
nano-installer build [--release] [--output <name>]

# 验证配置
nano-installer verify

# 显示帮助
nano-installer --help
```

## 🛠️ 开发

### 环境要求
- Rust 1.81+
- Windows 10+ (开发和目标平台)
- 可选：7-Zip (用于手动压缩 payload)

### 构建项目

```bash
# 克隆仓库
git clone https://github.com/your-org/nano-installer.git
cd nano-installer

# 构建所有组件
cargo build --release

# 运行测试
cargo test

# 构建示例
cd examples/TapTap
../../target/release/nano-installer build
```

### 项目结构

```bash
# 查看 workspace 成员
cargo metadata --no-deps | jq '.workspace_members'

# 编译单个组件
cargo build --release -p nano-installer-cli
cargo build --release -p nano-installer-lzma
cargo build --release -p nano-installer-uninst
```

## 🚀 路线图

### v0.2.0 (计划中)
- [ ] 大小优化：lzma stub < 1 MB
- [ ] 多架构支持：x86, x64, ARM64
- [ ] 插件系统
- [ ] 自定义主题

### v0.3.0 (规划中)
- [ ] GUI 构建器
- [ ] 在线更新支持
- [ ] 数字签名集成
- [ ] macOS 支持 (.dmg)

### v1.0.0 (长期目标)
- [ ] Linux 支持 (.deb, .rpm)
- [ ] CI/CD 集成工具
- [ ] 完整的插件生态

## 📊 性能指标

### 构建性能
- 初始化项目：< 1 秒
- 构建小型安装器 (< 10 MB)：2-5 秒
- 构建大型安装器 (> 100 MB)：10-30 秒

### 运行时性能
- 安装器启动：< 0.5 秒
- UI 响应：< 16ms (60 FPS)
- 文件解压：取决于文件大小，通常 10-50 MB/s

### 大小对比 (TapTap 示例)
- 源文件：~150 MB
- 7z 压缩后：~140 MB
- 最终安装器：~144 MB (stub + 资源 + payload)
- 压缩率：93%

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md)（待创建）了解详情。

### 贡献方式
- 🐛 报告 Bug
- 💡 提出新功能
- 📝 改进文档
- 🔧 提交 Pull Request

## 📄 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- 灵感来源：[NSIS](https://nsis.sourceforge.io/)
- GUI 框架：[egui](https://github.com/emilk/egui)
- 压缩：[7-Zip](https://www.7-zip.org/)

## 📞 联系方式

- 问题反馈：[GitHub Issues](https://github.com/your-org/nano-installer/issues)
- 讨论：[GitHub Discussions](https://github.com/your-org/nano-installer/discussions)
- 邮件：your-email@example.com

---

<div align="center">

**[⬆ 回到顶部](#nano-installer)**

Made with ❤️ by the nano-installer team

</div>
