# nano-installer 核心代码

> 本目录包含 nano-installer 的所有核心实现代码。

## 📑 目录

- [概览](#-概览)
- [目录结构](#-目录结构)
- [组件详解](#-组件详解)
- [构建指南](#-构建指南)
- [开发指南](#-开发指南)

---

## 🎯 概览

`installer/` 目录采用 **Cargo Workspace** 架构，包含 4 个相互协作的包：

```
installer/
├── cli/          命令行工具    (nano-installer.exe    ~598 KB)
├── lib/          核心功能库    (共享代码)
└── stubs/
    ├── lzma/     安装器 stub    (lzma-x64-unicode.exe ~4.15 MB)
    └── uninst/   卸载器 stub    (uninst.exe           ~199 KB)
```

### 组件对应关系

| 组件 | 可执行文件 | 大小 | 说明 |
|------|------------|------|------|
| CLI | `nano-installer.exe` | ~500 KB | 编译/打包工具 |
| Installer Stub | `lzma-x64-unicode.exe` | ~4 MB | 安装器 stub |
| Uninstaller Stub | `uninst.exe` | ~200 KB | 卸载器 stub |

---

## 📁 目录结构

### 完整树形图

```
installer/
│
├── README.md                    # 本文档
│
├── cli/                         # CLI 工具包
│   ├── Cargo.toml               # 包配置
│   ├── build.rs                 # 构建脚本（图标）
│   └── src/
│       └── main.rs              # CLI 入口 (~1000 行)
│                                # 功能：init, build, verify
│
├── lib/                         # 共享库包
│   ├── Cargo.toml               # 包配置
│   └── src/
│       ├── lib.rs               # 库入口（导出所有模块）
│       │
│       ├── common/              # 通用工具（400+ 行）
│       │   ├── mod.rs
│       │   ├── cli.rs           # CLI 辅助
│       │   ├── config.rs        # 配置加载
│       │   ├── error.rs         # 错误类型
│       │   ├── mutex.rs         # 互斥锁
│       │   ├── path_validation.rs # 路径验证
│       │   ├── platform.rs      # 平台检测
│       │   ├── process.rs       # 进程管理
│       │   └── result.rs        # Result 类型
│       │
│       ├── config/              # 配置管理（800+ 行）
│       │   ├── mod.rs
│       │   ├── installer_config.rs  # 主配置结构
│       │   ├── wizard_config.rs     # 向导配置
│       │   ├── validation.rs        # 配置验证
│       │   └── tests.rs             # 单元测试
│       │
│       ├── i18n/                # 国际化（300+ 行）
│       │   ├── mod.rs
│       │   ├── bundle.rs        # 语言包管理
│       │   ├── langpack.rs      # .pak 格式
│       │   └── loader.rs        # 语言加载
│       │
│       ├── installer/           # 安装逻辑（1500+ 行）
│       │   ├── mod.rs
│       │   ├── engine.rs        # 安装引擎
│       │   ├── state.rs         # 安装状态
│       │   ├── tasks.rs         # 安装任务
│       │   ├── extractor.rs     # 文件解压
│       │   ├── launcher.rs      # 应用启动
│       │   ├── registry.rs      # 注册表（通用）
│       │   ├── shortcuts.rs     # 快捷方式（通用）
│       │   └── windows/         # Windows 特定实现
│       │       ├── mod.rs
│       │       ├── elevation.rs # 提权
│       │       ├── registry.rs  # 注册表实现
│       │       └── shortcuts.rs # 快捷方式实现
│       │
│       ├── installer_runtime/   # 运行时模式（200+ 行）
│       │   ├── mod.rs           # 模式检测和初始化
│       │   └── mode.rs          # 安装模式枚举
│       │
│       ├── layout/              # XML 布局系统（1200+ 行）
│       │   ├── mod.rs
│       │   ├── xml_parser.rs    # XML 解析器
│       │   ├── element.rs       # UI 元素定义
│       │   └── layout_tree.rs   # 布局树构建
│       │
│       ├── logger/              # 日志系统（200+ 行）
│       │   └── mod.rs           # 日志配置和管理
│       │
│       ├── resources/           # 资源管理（1000+ 行）
│       │   ├── mod.rs
│       │   ├── bundle.rs        # 资源打包
│       │   ├── embedded.rs      # 资源嵌入检测
│       │   ├── loader.rs        # 资源加载
│       │   ├── manifest.rs      # 安装清单
│       │   ├── payload.rs       # Payload 处理
│       │   ├── runtime.rs       # 运行时资源
│       │   └── types.rs         # 资源类型定义
│       │
│       ├── ui/                  # GUI 界面（2000+ 行）
│       │   ├── mod.rs
│       │   ├── egui_app.rs      # Egui 主应用
│       │   ├── egui_app_xml.rs  # XML 驱动的 UI
│       │   ├── wizard.rs        # 向导流程管理
│       │   ├── layout_renderer.rs # 布局渲染器
│       │   ├── style_engine.rs  # 样式引擎
│       │   ├── assets.rs        # 资产管理
│       │   ├── dpi_handler.rs   # DPI 处理
│       │   ├── message_box.rs   # 消息框
│       │   └── styles/
│       │       └── mod.rs       # 样式定义
│       │
│       └── uninstaller/         # 卸载逻辑（600+ 行）
│           ├── mod.rs
│           ├── engine.rs        # 卸载引擎
│           ├── tasks.rs         # 卸载任务
│           └── windows/
│               └── mod.rs       # Windows 特定实现
│
└── stubs/                       # 运行时 stubs
    │
    ├── lzma/                    # 安装器 stub
    │   ├── Cargo.toml
    │   ├── build.rs             # 设置安装器图标
    │   └── src/
    │       └── main.rs          # 启动逻辑 (~100 行)
    │                            # 功能：
    │                            # - 检测运行模式
    │                            # - 初始化 GUI
    │                            # - 调用安装引擎
    │
    └── uninst/                  # 卸载器 stub
        ├── Cargo.toml
        ├── build.rs             # 设置卸载器图标
        └── src/
            └── main.rs          # 卸载逻辑 (~300 行)
                                 # 功能：
                                 # - 读取清单
                                 # - 显示确认框
                                 # - 删除文件/注册表
                                 # - 自删除
```

---

## 🔧 组件详解

### 1. CLI 工具 (`cli/`)

#### 职责
- 项目初始化
- 配置验证
- 资源打包
- 安装器构建

#### 主要函数

```rust
// CLI 命令
enum Commands {
    Init { name, output },    // 初始化项目
    Build { release, output }, // 构建安装器
    Verify,                    // 验证配置
}

// 核心构建流程
fn build_installer(project_dir, config, output_name) {
    1. validate_resources()          // 验证资源
    2. build_uninstaller_exe()       // 构建卸载器
    3. pack_resources()              // 打包资源
    4. append_to_stub()              // 追加到 stub
    5. cleanup_temp_files()          // 清理临时文件
}
```

#### 配置文件

**Cargo.toml**:
```toml
[package]
name = "nano-installer-cli"

[[bin]]
name = "nano-installer"
path = "src/main.rs"

[dependencies]
nano-installer-lib = { path = "../lib" }
clap.workspace = true
anyhow.workspace = true
# ...
```

**build.rs**:
```rust
// 设置 CLI 工具图标
winres::WindowsResource::new()
    .set_icon("../../assets/logo.ico")
    .compile()
```

---

### 2. 共享库 (`lib/`)

#### 职责
提供所有核心功能的实现，被 CLI 和 stubs 共同使用。

#### 主要模块

##### `common/` - 通用工具
```rust
// 配置加载
pub fn load_config(path: &Path) -> Result<InstallerConfig>;

// 平台检测
pub fn is_windows_10_or_later() -> bool;

// 路径验证
pub fn validate_install_path(path: &Path) -> Result<()>;
```

##### `config/` - 配置管理
```rust
// 主配置结构
pub struct InstallerConfig {
    pub product: ProductInfo,
    pub install: InstallConfig,
    pub ui: UIConfig,
    pub resources: ResourcesConfig,
    pub payload: PayloadConfig,
}

// 配置验证
pub fn validate_config(config: &InstallerConfig) -> Result<()>;
```

##### `installer/` - 安装逻辑
```rust
// 安装引擎
pub struct InstallEngine {
    config: InstallerConfig,
    state: InstallState,
}

impl InstallEngine {
    pub fn new(config: InstallerConfig) -> Self;
    pub fn install(&mut self) -> Result<()>;
    pub fn rollback(&mut self) -> Result<()>;
}

// 主要功能
- 文件解压和复制
- 注册表写入
- 快捷方式创建
- 服务安装
```

##### `ui/` - GUI 界面
```rust
// Egui 应用
pub struct InstallerApp {
    wizard: Wizard,
    config: InstallerConfig,
    engine: InstallEngine,
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame);
}

// 向导流程
pub enum WizardPage {
    Welcome,
    Language,
    License,
    InstallPath,
    Installing,
    Finish,
}
```

##### `resources/` - 资源管理
```rust
// 资源打包
pub struct ResourceBundle {
    pub items: Vec<ResourceItem>,
}

impl ResourceBundle {
    pub fn new() -> Self;
    pub fn add_config(&mut self, data: Vec<u8>);
    pub fn add_layout(&mut self, name: String, data: Vec<u8>);
    pub fn add_asset(&mut self, name: String, data: Vec<u8>);
    pub fn write_to_file(&self, path: &Path) -> Result<()>;
}

// 资源嵌入
pub fn has_embedded_resources() -> bool;
pub fn extract_embedded_resources() -> Result<ResourceBundle>;
```

#### 配置文件

**Cargo.toml**:
```toml
[package]
name = "nano-installer-lib"

[lib]
name = "nano_installer"
path = "src/lib.rs"

[dependencies]
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
eframe.workspace = true
egui.workspace = true
# ... 所有核心依赖
```

---

### 3. 安装器 Stub (`stubs/lzma/`)

#### 职责
实际的安装器可执行文件，包含完整的 GUI 和安装逻辑。

#### 主要流程

```rust
fn main() -> Result<()> {
    // 1. 检查资源
    if !has_embedded_resources() {
        bail!("No embedded resources found");
    }
    
    // 2. 加载配置
    let config = load_config_from_resources()?;
    
    // 3. 检测运行模式
    let mode = determine_mode(&config);
    
    // 4. 启动 GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 400.0])
            .with_resizable(false),
        ..Default::default()
    };
    
    eframe::run_native(
        &config.ui.title,
        options,
        Box::new(|cc| Box::new(InstallerApp::new(cc, config, mode))),
    )?;
    
    Ok(())
}

fn determine_mode(config: &InstallerConfig) -> InstallerMode {
    // 检查命令行参数、文件名、已安装版本等
    // 返回: Install, Update, 或 Repair
}
```

#### 配置文件

**Cargo.toml**:
```toml
[package]
name = "nano-installer-lzma"

[[bin]]
name = "lzma-x64-unicode"
path = "src/main.rs"

[dependencies]
nano-installer-lib = { path = "../../lib" }
anyhow.workspace = true
```

**build.rs**:
```rust
// 设置安装器图标
winres::WindowsResource::new()
    .set_icon("../../../assets/logo.ico")
    .compile()
```

#### 大小分析

```
lzma-x64-unicode.exe (4.15 MB)
├── Rust 运行时 + 标准库        ~1.0 MB
├── Egui + eframe 框架          ~2.0 MB
├── nano-installer-lib 代码     ~0.8 MB
├── Windows API 绑定            ~0.2 MB
└── 其他依赖                    ~0.15 MB
```

---

### 4. 卸载器 Stub (`stubs/uninst/`)

#### 职责
轻量级卸载器，使用纯 Win32 API 实现。

#### 主要流程

```rust
fn main() -> Result<()> {
    // 1. 读取卸载清单
    let manifest_path = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("uninstall_manifest.json");
    
    let manifest: UninstallManifest = 
        serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
    
    // 2. 显示确认对话框（Win32 MessageBox）
    let response = unsafe {
        MessageBoxW(
            None,
            &format!("确定要卸载 {} 吗？", manifest.product_name),
            w!("卸载确认"),
            MB_YESNO | MB_ICONQUESTION,
        )
    };
    
    if response != IDYES {
        return Ok(());
    }
    
    // 3. 执行卸载
    uninstall(&manifest)?;
    
    // 4. 删除自身
    self_delete()?;
    
    Ok(())
}

fn uninstall(manifest: &UninstallManifest) -> Result<()> {
    // 删除文件
    for file in &manifest.files_to_remove {
        std::fs::remove_file(file)?;
    }
    
    // 删除目录
    for dir in &manifest.directories_to_remove {
        std::fs::remove_dir_all(dir)?;
    }
    
    // 清理注册表
    for key in &manifest.registry_keys_to_remove {
        remove_registry_key(key)?;
    }
    
    // 删除快捷方式
    for shortcut in &manifest.shortcuts_to_remove {
        std::fs::remove_file(shortcut)?;
    }
    
    Ok(())
}

fn self_delete() -> Result<()> {
    // 创建批处理文件来删除自身
    let exe_path = std::env::current_exe()?;
    let batch_path = std::env::temp_dir().join("uninstall.bat");
    
    std::fs::write(
        &batch_path,
        format!(
            "@echo off\n\
             :retry\n\
             del /f /q \"{}\"\n\
             if exist \"{}\" goto retry\n\
             del \"%~f0\"\n",
            exe_path.display(),
            exe_path.display()
        ),
    )?;
    
    // 执行批处理
    unsafe {
        CreateProcessW(
            None,
            &format!("cmd.exe /c \"{}\"", batch_path.display()),
            // ... 其他参数
        )?;
    }
    
    Ok(())
}
```

#### 配置文件

**Cargo.toml**:
```toml
[package]
name = "nano-installer-uninst"

[[bin]]
name = "uninst"
path = "src/main.rs"

[dependencies]
# 最小化依赖
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true

[target.'cfg(windows)'.dependencies]
windows.workspace = true  # 仅 Win32 API
```

**build.rs**:
```rust
// 设置卸载器图标
winres::WindowsResource::new()
    .set_icon("../../../assets/uninst.ico")
    .compile()
```

#### 优化要点

1. **无 GUI 框架**：使用 Win32 MessageBox
2. **最小依赖**：只需要 serde_json 和 Windows API
3. **快速启动**：无需初始化复杂的运行时

---

## 🔨 构建指南

### 构建所有组件

```bash
# 在项目根目录
cargo build --release

# 输出
target/release/
├── nano-installer.exe      # CLI 工具
├── lzma-x64-unicode.exe    # 安装器 stub
└── uninst.exe              # 卸载器 stub
```

### 单独构建

```bash
# CLI 工具
cargo build --release -p nano-installer-cli

# 共享库（用于开发和测试）
cargo build --release -p nano-installer-lib

# 安装器 stub
cargo build --release -p nano-installer-lzma

# 卸载器 stub
cargo build --release -p nano-installer-uninst
```

### 开发模式（更快的编译）

```bash
# 使用 debug 模式
cargo build

# 只检查语法（不生成二进制）
cargo check

# 运行测试
cargo test -p nano-installer-lib
```

---

## 💻 开发指南

### 添加新功能到 lib

1. 在 `lib/src/` 创建新模块：
```bash
mkdir -p lib/src/my_feature
touch lib/src/my_feature/mod.rs
```

2. 实现功能：
```rust
// lib/src/my_feature/mod.rs
pub fn my_function() {
    // 实现
}
```

3. 在 `lib.rs` 中导出：
```rust
// lib/src/lib.rs
pub mod my_feature;
```

4. 在 CLI 或 stubs 中使用：
```rust
use nano_installer::my_feature::my_function;
```

### 修改 CLI 行为

编辑 `cli/src/main.rs`：

```rust
// 添加新命令
enum Commands {
    // ... 现有命令
    NewCommand {
        #[arg(short, long)]
        option: String,
    },
}

// 实现命令
fn handle_new_command(option: &str) -> Result<()> {
    // 实现
}
```

### 优化二进制大小

1. **移除未使用的特性**：
```toml
[dependencies]
eframe = { 
    version = "0.33", 
    default-features = false,
    features = ["glow"]  # 只启用需要的
}
```

2. **启用更激进的优化**：
```toml
[profile.release]
opt-level = "z"      # 优化大小（而不是速度）
lto = true           # 链接时优化
codegen-units = 1    # 单个代码生成单元
strip = true         # 剥离符号
```

3. **使用 `cargo-bloat` 分析**：
```bash
cargo install cargo-bloat
cargo bloat --release -p nano-installer-lzma --crates
```

### 调试技巧

1. **启用日志**：
```rust
// lib/src/logger/mod.rs 已配置 tracing

// 在代码中使用
use tracing::{info, warn, error, debug};

debug!("Debug message");
info!("Info message");
warn!("Warning message");
error!("Error message");
```

2. **设置日志级别**：
```bash
# Windows
$env:RUST_LOG="debug"
cargo run --release

# Linux/macOS
RUST_LOG=debug cargo run --release
```

3. **使用 VS Code 调试**：
```json
// .vscode/launch.json
{
    "type": "lldb",
    "request": "launch",
    "name": "Debug CLI",
    "cargo": {
        "args": [
            "build",
            "--bin=nano-installer",
            "--package=nano-installer-cli"
        ]
    },
    "args": ["build"],
    "cwd": "${workspaceFolder}/examples/TapTap"
}
```

### 测试

```bash
# 运行所有测试
cargo test

# 运行特定包的测试
cargo test -p nano-installer-lib

# 运行特定测试
cargo test --test integration_test

# 查看测试输出
cargo test -- --nocapture
```

---

## 📊 性能指标

### 编译时间

| 组件 | Debug | Release |
|------|-------|---------|
| lib | ~30s | ~2m |
| cli | ~10s | ~30s |
| lzma stub | ~15s | ~1m |
| uninst stub | ~5s | ~15s |
| **Total** | ~1m | ~4m |

*在 Ryzen 7 5800X, 32GB RAM, NVMe SSD 上测试*

### 运行时性能

| 指标 | 值 |
|------|-----|
| CLI 启动 | < 0.1s |
| 配置加载 | < 0.05s |
| 资源打包 (10MB) | ~1s |
| 安装器启动 | < 0.5s |
| UI 渲染 (60 FPS) | ~16ms/frame |
| 文件解压 (100MB) | ~2-5s |

### 内存使用

| 组件 | 峰值内存 |
|------|----------|
| CLI | ~50 MB |
| 安装器 (Egui) | ~80 MB |
| 卸载器 | ~10 MB |

---

## 🔗 相关链接

- [项目主文档](../README.md)
- [项目结构详解](../docs/PROJECT_STRUCTURE.md)
- [架构设计](../docs/ARCHITECTURE.md)
- [开发指南](../docs/DEVELOPMENT.md)
- [文档导航](../docs/README.md)

---

<div align="center">

**[⬆ 回到顶部](#nano-installer-核心代码)**

最后更新：2025-11-05

</div>
