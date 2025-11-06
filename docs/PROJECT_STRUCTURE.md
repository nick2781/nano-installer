# 项目结构详解

> 本文档详细说明 nano-installer 的完整项目组织结构。

## 📑 目录

- [总体架构](#-总体架构)
- [目录结构](#-目录结构)
- [核心组件](#-核心组件)
- [构建流程](#-构建流程)
- [工作流程](#-工作流程)
- [配置系统](#-配置系统)
- [扩展指南](#-扩展指南)

---

## 🏗️ 总体架构

nano-installer 采用 **Cargo Workspace** 架构，包含 4 个独立的包：

```
┌─────────────────────────────────────────────────────────────┐
│                    Workspace Root                            │
│                   (nano-installer/)                          │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
   ┌────▼────┐        ┌────▼────┐        ┌────▼─────┐
   │   CLI   │        │   LIB   │        │  STUBS   │
   │ Package │───────>│ Package │<───────│ Packages │
   └─────────┘        └─────────┘        └──────────┘
        │                   │                   │
    nano-installer    核心功能库         ├─ lzma/
       .exe           (共享代码)          └─ uninst/
```

### 设计原则

1. **职责分离**：每个包专注于特定功能
2. **代码复用**：核心逻辑在 lib 中实现一次
3. **独立优化**：每个包可独立配置编译选项
4. **清晰依赖**：单向依赖，避免循环

---

## 📁 目录结构

### 完整结构树

```
nano-installer/                     # 项目根目录
│
├── Cargo.toml                      # Workspace 配置
├── rust-toolchain.toml             # Rust 工具链版本 (1.81)
├── README.md                       # 项目主文档
├── PROJECT_STRUCTURE.md            # 本文档
├── REFACTORING_SUMMARY.md          # 重构记录
├── CHANGELOG.md                    # 版本历史
│
├── installer/                      # 🔥 核心代码目录
│   ├── README.md                   # 核心组件说明
│   │
│   ├── cli/                        # CLI 工具包
│   │   ├── Cargo.toml              # 包配置
│   │   ├── build.rs                # 构建脚本（设置图标）
│   │   └── src/
│   │       └── main.rs             # CLI 入口 (~1000 行)
│   │
│   ├── lib/                        # 共享库包
│   │   ├── Cargo.toml              # 包配置
│   │   └── src/
│   │       ├── lib.rs              # 库入口
│   │       ├── common/             # 通用工具
│   │       │   ├── mod.rs
│   │       │   ├── config.rs       # 配置加载
│   │       │   ├── error.rs        # 错误处理
│   │       │   ├── platform.rs     # 平台特定代码
│   │       │   └── ...
│   │       ├── config/             # 配置管理
│   │       │   ├── mod.rs
│   │       │   ├── installer_config.rs  # 主配置结构
│   │       │   ├── validation.rs   # 配置验证
│   │       │   └── ...
│   │       ├── i18n/               # 国际化
│   │       │   ├── mod.rs
│   │       │   ├── bundle.rs       # 语言包管理
│   │       │   └── loader.rs       # 语言加载器
│   │       ├── installer/          # 安装逻辑
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs       # 安装引擎
│   │       │   ├── tasks.rs        # 安装任务
│   │       │   ├── windows/        # Windows 特定
│   │       │   │   ├── registry.rs # 注册表操作
│   │       │   │   └── shortcuts.rs # 快捷方式
│   │       │   └── ...
│   │       ├── installer_runtime/  # 运行时模式
│   │       │   ├── mod.rs
│   │       │   └── mode.rs         # 模式检测
│   │       ├── layout/             # XML 布局系统
│   │       │   ├── mod.rs
│   │       │   ├── xml_parser.rs   # XML 解析
│   │       │   ├── element.rs      # UI 元素
│   │       │   └── layout_tree.rs  # 布局树
│   │       ├── logger/             # 日志系统
│   │       │   └── mod.rs
│   │       ├── resources/          # 资源管理
│   │       │   ├── mod.rs
│   │       │   ├── bundle.rs       # 资源打包
│   │       │   ├── embedded.rs     # 资源嵌入
│   │       │   ├── loader.rs       # 资源加载
│   │       │   ├── manifest.rs     # 清单管理
│   │       │   └── payload.rs      # Payload 处理
│   │       ├── ui/                 # GUI 界面
│   │       │   ├── mod.rs
│   │       │   ├── egui_app.rs     # Egui 应用
│   │       │   ├── wizard.rs       # 向导流程
│   │       │   ├── layout_renderer.rs # 布局渲染
│   │       │   ├── style_engine.rs # 样式引擎
│   │       │   └── ...
│   │       └── uninstaller/        # 卸载逻辑
│   │           ├── mod.rs
│   │           ├── engine.rs
│   │           └── tasks.rs
│   │
│   └── stubs/                      # 运行时 stubs
│       ├── lzma/                   # 安装器 stub
│       │   ├── Cargo.toml
│       │   ├── build.rs            # 设置安装器图标
│       │   └── src/
│       │       └── main.rs         # 启动逻辑 (~100 行)
│       │
│       └── uninst/                 # 卸载器 stub
│           ├── Cargo.toml
│           ├── build.rs            # 设置卸载器图标
│           └── src/
│               └── main.rs         # 卸载逻辑 (~300 行, 纯 Win32)
│
├── assets/                         # 共享资源文件
│   ├── logo.ico                    # CLI 工具图标
│   ├── uninst.ico                  # 卸载器图标
│   ├── *.png                       # UI 图片资源
│   └── ...
│
├── templates/                      # 项目模板
│   ├── layouts/                    # 默认布局文件
│   │   ├── welcome.xml
│   │   ├── config.xml
│   │   ├── installing.xml
│   │   └── finish.xml
│   └── locales/                    # 默认语言文件
│       ├── en-US.json
│       └── zh-CN.json
│
├── examples/                       # 示例项目
│   └── TapTap/                     # TapTap 客户端示例
│       ├── installer_config.json
│       ├── layouts/
│       ├── assets/
│       ├── locales/
│       ├── payload/
│       │   └── app.7z
│       └── dist/
│           └── TapTap_Setup.exe    # 生成的安装器
│
├── docs/                           # 文档
│   ├── README.md                   # 文档索引
│   ├── ARCHITECTURE.md             # 架构设计
│   ├── API.md                      # API 参考
│   ├── UI_DESIGN.md                # UI 设计指南
│   ├── DEVELOPMENT.md              # 开发指南
│   ├── guides/                     # 使用指南
│   │   ├── QUICKSTART.md
│   │   └── ...
│   └── reference/                  # 参考文档
│       └── ...
│
├── scripts/                        # 构建脚本
│   ├── build.ps1                   # Windows 构建
│   ├── build.sh                    # Linux/macOS 构建
│   └── package.sh                  # 打包脚本
│
├── tests/                          # 集成测试
│   └── integration_test.rs
│
└── target/                         # 构建输出 (.gitignore)
    └── release/
        ├── nano-installer.exe      # CLI 工具 (598 KB)
        ├── lzma-x64-unicode.exe    # 安装器 stub (4.15 MB)
        └── uninst.exe              # 卸载器 stub (199 KB)
```

---

## 🔧 核心组件

### 1. CLI 工具 (`installer/cli/`)

**作用**：命令行工具，类似 NSIS 的 `makensis.exe`

**主要功能**：
- 初始化新项目（`init` 命令）
- 构建安装器（`build` 命令）
- 验证配置（`verify` 命令）
- 资源打包和嵌入
- 调用 stub 生成最终安装器

**输出**：`nano-installer.exe` (~598 KB)

**关键文件**：
- `src/main.rs`：CLI 入口，命令解析，构建流程
- `build.rs`：设置 CLI 工具图标

**依赖关系**：
```rust
[dependencies]
nano-installer-lib = { path = "../lib" }
clap = "4.5"  // CLI 参数解析
```

### 2. 共享库 (`installer/lib/`)

**作用**：所有核心功能的实现

**主要模块**：
- `config/`：配置解析、验证、管理
- `resources/`：资源打包、嵌入、加载
- `installer/`：安装逻辑、文件操作、注册表
- `uninstaller/`：卸载逻辑
- `ui/`：GUI 界面（Egui）
- `layout/`：XML 布局解析和渲染
- `i18n/`：国际化支持
- `logger/`：日志系统

**输出**：库文件（被其他包依赖）

**关键特性**：
- 导出为 `nano_installer` crate
- 包含所有可复用的代码
- 提供统一的 API

### 3. 安装器 Stub (`installer/stubs/lzma/`)

**作用**：实际的安装器可执行文件，类似 NSIS 的 `lzma-x86-unicode`

**主要功能**：
- 检测运行模式（安装/更新/修复）
- 初始化 Egui GUI
- 显示安装向导
- 解压 payload
- 调用安装引擎
- 生成卸载器

**输出**：`lzma-x64-unicode.exe` (~4.15 MB)

**关键文件**：
- `src/main.rs`：启动逻辑（~100 行）
- `build.rs`：设置安装器图标

**大小组成**：
```
lzma-x64-unicode.exe (4.15 MB)
├── 代码 + Rust 标准库    ~1.5 MB
├── Egui GUI 框架         ~2.0 MB
├── 其他依赖              ~0.5 MB
└── 资源嵌入空间          ~0.15 MB

最终安装器 (例如 TapTap_Setup.exe = 7.25 MB)
├── lzma-x64-unicode.exe  ~4.15 MB
└── 资源包                ~3.10 MB
    ├── 配置              ~10 KB
    ├── 布局              ~50 KB
    ├── 资产              ~500 KB
    ├── 语言包            ~100 KB
    ├── payload (7z)      ~2 MB
    └── uninst.exe        ~500 KB
```

### 4. 卸载器 Stub (`installer/stubs/uninst/`)

**作用**：轻量级卸载器，类似 NSIS 的 `uninst`

**主要功能**：
- 读取安装清单
- 显示确认对话框（Win32 MessageBox）
- 删除文件和目录
- 清理注册表
- 删除快捷方式
- 自删除

**输出**：`uninst.exe` (~199 KB)

**关键特性**：
- 纯 Win32 API（无 GUI 框架）
- 最小化依赖
- 快速启动

**关键文件**：
- `src/main.rs`：卸载逻辑（~300 行）
- `build.rs`：设置卸载器图标

---

## 🔨 构建流程

### Workspace 编译

```bash
# 编译所有组件
cargo build --release

# 输出
target/release/
├── nano-installer.exe      # CLI
├── lzma-x64-unicode.exe    # 安装器 stub
└── uninst.exe              # 卸载器 stub
```

### 单独编译

```bash
# 只编译 CLI
cargo build --release -p nano-installer-cli

# 只编译安装器 stub
cargo build --release -p nano-installer-lzma

# 只编译卸载器 stub
cargo build --release -p nano-installer-uninst

# 只编译库（用于开发）
cargo build --release -p nano-installer-lib
```

### 交叉编译（未来）

```bash
# x86 (32位)
cargo build --release --target i686-pc-windows-msvc

# ARM64
cargo build --release --target aarch64-pc-windows-msvc
```

---

## 🔄 工作流程

### 开发者视角

```
1. 安装 CLI
   cargo install --path installer/cli
   
2. 创建项目
   nano-installer init MyApp
   cd MyApp
   
3. 配置项目
   编辑 installer_config.json
   自定义 layouts/*.xml
   准备 payload/
   
4. 构建安装器
   nano-installer build
   
5. 测试
   .\dist\MyApp_Setup.exe
```

### CLI 构建流程

```
nano-installer build
    │
    ├─> 1. 加载配置
    │      installer_config.json
    │
    ├─> 2. 验证资源
    │      检查 layouts/, assets/, locales/, payload/
    │
    ├─> 3. 构建卸载器
    │      ├─ 复制 uninst.exe stub
    │      ├─ 打包卸载资源
    │      │  ├─ config
    │      │  ├─ layouts (uninstall相关)
    │      │  └─ locales
    │      └─ 追加到 stub 末尾
    │      输出: .build/uninst.exe (~565 KB)
    │
    ├─> 4. 打包资源
    │      ├─ config
    │      ├─ layouts (所有)
    │      ├─ assets (所有)
    │      ├─ locales (编译为 .pak)
    │      ├─ payload (7z)
    │      └─ uninst.exe
    │      输出: 资源包 (~3 MB)
    │
    ├─> 5. 构建安装器
    │      ├─ 复制 lzma-x64-unicode.exe stub
    │      └─ 追加资源包
    │      输出: dist/MyApp_Setup.exe (~7 MB)
    │
    └─> 6. 清理
           删除 .build/ 临时目录
```

### 最终用户视角

```
运行 MyApp_Setup.exe
    │
    ├─> 1. 启动 (lzma-x64-unicode.exe)
    │      检测资源、初始化 GUI
    │
    ├─> 2. 显示向导
    │      欢迎页 → 许可 → 路径选择
    │
    ├─> 3. 安装
    │      解压 payload → 复制文件
    │      创建快捷方式 → 写注册表
    │
    ├─> 4. 生成卸载器
    │      复制 uninst.exe 到安装目录
    │      写入卸载清单
    │
    └─> 5. 完成
           显示完成页，可选启动应用
```

---

## ⚙️ 配置系统

### 配置层次

```
1. Workspace 配置 (Cargo.toml)
   └─> 统一管理所有依赖版本
   
2. 包配置 (各包的 Cargo.toml)
   └─> 选择需要的依赖
   
3. 构建配置 (build.rs)
   └─> 设置图标、资源等
   
4. 项目配置 (installer_config.json)
   └─> 用户项目的配置
```

### Workspace Cargo.toml

```toml
[workspace]
members = [
    "installer/cli",
    "installer/lib",
    "installer/stubs/lzma",
    "installer/stubs/uninst",
]

[workspace.dependencies]
# 所有包共享这些依赖版本
serde = "1.0"
egui = "0.33"
# ...
```

### 图标配置

每个包的 `build.rs` 设置自己的图标：

```rust
// installer/cli/build.rs
winres::WindowsResource::new()
    .set_icon("../../assets/logo.ico")
    .compile()

// installer/stubs/lzma/build.rs
winres::WindowsResource::new()
    .set_icon("../../../assets/logo.ico")
    .compile()

// installer/stubs/uninst/build.rs
winres::WindowsResource::new()
    .set_icon("../../../assets/uninst.ico")
    .compile()
```

---

## 🚀 扩展指南

### 添加新的 stub

1. 创建新包：
```bash
mkdir -p installer/stubs/new-stub/src
```

2. 创建 Cargo.toml：
```toml
[package]
name = "nano-installer-new-stub"
version.workspace = true

[[bin]]
name = "new-stub"
path = "src/main.rs"

[dependencies]
nano-installer-lib = { path = "../../lib" }
```

3. 添加到 workspace：
```toml
# 根 Cargo.toml
[workspace]
members = [
    "installer/cli",
    "installer/lib",
    "installer/stubs/lzma",
    "installer/stubs/uninst",
    "installer/stubs/new-stub",  # 新增
]
```

### 支持多架构

修改 CLI 选择正确的 stub：

```rust
// installer/cli/src/main.rs
fn get_stub_path(arch: &str) -> PathBuf {
    match arch {
        "x86_64" => "lzma-x64-unicode.exe",
        "x86" => "lzma-x86-unicode.exe",
        "aarch64" => "lzma-arm64-unicode.exe",
        _ => panic!("Unsupported architecture"),
    }.into()
}
```

### 添加新的核心功能

在 `installer/lib/src/` 中添加新模块：

```rust
// lib/src/my_feature/mod.rs
pub fn my_function() {
    // 实现
}

// lib/src/lib.rs
pub mod my_feature;
```

CLI 和 stubs 都可以使用：

```rust
use nano_installer::my_feature::my_function;
```

---

## 📊 性能优化

### 编译优化

```toml
[profile.release]
opt-level = "z"      # 优化大小
lto = true           # 链接时优化
codegen-units = 1    # 单个代码生成单元
strip = true         # 剥离符号
panic = "abort"      # 减小 panic 代码
```

### 依赖优化

```toml
# 禁用不需要的特性
eframe = { version = "0.33", default-features = false, features = ["glow"] }
```

### 未来优化方向

1. **减小 Egui 框架大小**：
   - 移除未使用的功能
   - 考虑更轻量的 GUI 方案

2. **改进压缩**：
   - 使用更高压缩率的算法
   - 按需解压

3. **代码分割**：
   - 动态加载某些功能
   - 减少 stub 初始大小

---

## 📖 相关文档

- [README.md](../README.md) - 项目概述和快速开始
- [installer/README.md](../installer/README.md) - 核心代码详解
- [ARCHITECTURE.md](ARCHITECTURE.md) - 架构深入
- [DEVELOPMENT.md](DEVELOPMENT.md) - 开发指南
- [文档导航](README.md) - 完整文档列表

---

<div align="center">

**[⬆ 回到顶部](#项目结构详解)**

最后更新：2025-11-05

</div>
