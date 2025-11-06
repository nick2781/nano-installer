# Workspace 重构实施计划

## 目标

将 nano-installer 从单一包结构重构为 Workspace 结构，完全对标 NSIS 架构。

## 现状

```
nano-installer/
├── Cargo.toml (单一包，3 个 [[bin]])
├── build.rs (无法为不同 binary 设置不同图标)
└── src/
    ├── lib.rs
    └── bin/
        ├── nano-installer.rs (双重身份：CLI + 安装器)
        ├── runtime-stub.rs
        └── uninstaller-stub.rs
```

**问题**：
- ❌ 图标无法分别设置
- ❌ `nano-installer.exe` 有不必要的双重身份
- ❌ 不符合 NSIS 架构

## 目标结构

```
nano-installer/
├── Cargo.toml (Workspace 配置)
├── nano-installer/ (CLI 工具)
├── lzma-x64-unicode/ (安装器 stub)
└── uninst/ (卸载器 stub)
```

## 实施步骤

### 阶段 1：创建 Workspace 结构

**步骤 1.1：创建目录结构**
```bash
mkdir -p nano-installer/src
mkdir -p lzma-x64-unicode/src
mkdir -p uninst/src
```

**步骤 1.2：移动现有代码**
- `src/` → `nano-installer/src/`
- `src/bin/nano-installer.rs` → `nano-installer/src/main.rs`
- `src/bin/runtime-stub.rs` → `lzma-x64-unicode/src/main.rs`
- `src/bin/uninstaller-stub.rs` → `uninst/src/main.rs`

**步骤 1.3：删除 `bin/` 目录**
```bash
rm -rf nano-installer/src/bin/
```

---

### 阶段 2：配置 Workspace

**步骤 2.1：创建根 `Cargo.toml`**

```toml
[workspace]
members = [
    "nano-installer",
    "lzma-x64-unicode",
    "uninst",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.81"
authors = ["Your Name <your.email@example.com>"]

[workspace.dependencies]
# 共享依赖版本
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
# ... 其他共享依赖
```

**步骤 2.2：创建 `nano-installer/Cargo.toml`**

```toml
[package]
name = "nano-installer"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true

[[bin]]
name = "nano-installer"
path = "src/main.rs"

[dependencies]
# 从 workspace 继承
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
# ... 所有需要的依赖

[build-dependencies]
winres = "0.1"

[lib]
name = "nano_installer"
path = "src/lib.rs"
```

**步骤 2.3：创建 `lzma-x64-unicode/Cargo.toml`**

```toml
[package]
name = "lzma-x64-unicode"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "lzma-x64-unicode"
path = "src/main.rs"

[dependencies]
nano-installer = { path = "../nano-installer" }
serde.workspace = true
anyhow.workspace = true
# ... GUI 相关依赖

[build-dependencies]
winres = "0.1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

**步骤 2.4：创建 `uninst/Cargo.toml`**

```toml
[package]
name = "uninst"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "uninst"
path = "src/main.rs"

[dependencies]
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
windows = { version = "0.58", features = [...] }

[build-dependencies]
winres = "0.1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

---

### 阶段 3：创建 build.rs

**步骤 3.1：`nano-installer/build.rs`**

```rust
fn main() {
    #[cfg(target_os = "windows")]
    {
        let _ = winres::WindowsResource::new()
            .set_icon("../assets/nano-installer.ico")
            .compile();
    }
}
```

**步骤 3.2：`lzma-x64-unicode/build.rs`**

```rust
fn main() {
    #[cfg(target_os = "windows")]
    {
        let _ = winres::WindowsResource::new()
            .set_icon("../assets/stub.ico")
            .compile();
    }
}
```

**步骤 3.3：`uninst/build.rs`**

```rust
fn main() {
    #[cfg(target_os = "windows")]
    {
        let _ = winres::WindowsResource::new()
            .set_icon("../assets/uninstall.ico")
            .compile();
    }
}
```

---

### 阶段 4：修改源代码

**步骤 4.1：修改 `nano-installer/src/main.rs`**

删除双重身份逻辑：

```rust
// ❌ 删除这段代码
fn main() {
    if nano_installer::resources::has_embedded_resources() {
        run_as_installer()  // ← 删除
    } else {
        run_as_cli()
    }
}

// ✅ 改为纯 CLI
fn main() {
    if let Err(e) = run_as_cli() {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}

fn run_as_cli() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { .. } => cmd_init(...),
        Commands::Build { .. } => cmd_build(...),
        Commands::Validate { .. } => cmd_validate(...),
        Commands::Langpack { .. } => cmd_langpack(...),
    }
}
```

**步骤 4.2：修改 `nano-installer/src/main.rs` 的 `cmd_build()`**

更新 stub 查找路径：

```rust
fn build_uninstaller_exe(...) -> Result<PathBuf> {
    let stub_paths = vec![
        PathBuf::from("target/release/uninst.exe"),
        PathBuf::from("../../target/release/uninst.exe"),
    ];
    
    let stub = stub_paths.iter()
        .find(|p| p.exists())
        .context("uninst.exe not found. Please run: cargo build --release -p uninst")?;
    
    // ...
}

fn build_installer_exe(...) -> Result<()> {
    let stub_paths = vec![
        PathBuf::from("target/release/lzma-x64-unicode.exe"),
        PathBuf::from("../../target/release/lzma-x64-unicode.exe"),
    ];
    
    let stub = stub_paths.iter()
        .find(|p| p.exists())
        .context("lzma-x64-unicode.exe not found. Please run: cargo build --release -p lzma-x64-unicode")?;
    
    // ...
}
```

**步骤 4.3：`lzma-x64-unicode/src/main.rs`**

保持简洁：

```rust
use anyhow::Result;

fn main() -> Result<()> {
    // 初始化日志
    let _ = nano_installer::logger::init(None, "installer", true);
    
    // 初始化运行时资源
    nano_installer::resources::RuntimeResources::init()?;
    
    // 检测模式并运行
    let mode = nano_installer::installer_runtime::InstallerMode::detect();
    nano_installer::installer_runtime::run_installer(mode)?;
    
    Ok(())
}
```

**步骤 4.4：`uninst/src/main.rs`**

保持现有代码不变（已经是纯 Win32 实现）。

---

### 阶段 5：测试和验证

**步骤 5.1：清理并重新编译**

```bash
cargo clean
cargo build --release
```

**步骤 5.2：验证输出文件**

```bash
ls -lh target/release/*.exe

预期输出：
nano-installer.exe      ~4.5 MB
lzma-x64-unicode.exe    ~4.2 MB
uninst.exe              ~200 KB
```

**步骤 5.3：验证图标**

在 Windows 资源管理器中检查三个文件的图标是否正确。

**步骤 5.4：测试构建流程**

```bash
cd examples/TapTap
../../target/release/nano-installer.exe build
```

**步骤 5.5：测试安装器**

```bash
cd examples/TapTap/dist
.\TapTap_Setup.exe
```

验证：
- GUI 启动正常
- 安装流程正常
- uninst.exe 被正确复制到安装目录
- 图标显示正确

**步骤 5.6：测试卸载器**

运行安装目录中的 `uninst.exe`，验证卸载功能。

---

## 潜在问题和解决方案

### 问题 1：路径问题

**问题**：代码中有硬编码的路径引用。

**解决方案**：
- 使用相对路径
- 使用 `env!("CARGO_MANIFEST_DIR")` 获取项目根目录

### 问题 2：依赖冲突

**问题**：Workspace 成员之间的依赖版本冲突。

**解决方案**：
- 使用 `[workspace.dependencies]` 统一版本
- 检查 `Cargo.lock` 确保一致性

### 问题 3：构建脚本路径

**问题**：`build.rs` 中的图标路径可能不正确。

**解决方案**：
```rust
// 使用相对于项目根的路径
let icon_path = "../assets/stub.ico";
let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join(icon_path);
```

### 问题 4：资源查找

**问题**：运行时查找嵌入资源可能失败。

**解决方案**：
- 确保 `resources::RuntimeResources::init()` 正确实现
- 测试从不同目录运行安装器

---

## 验收标准

### 功能验收

- [x] 三个独立的可执行文件编译成功
- [ ] 每个文件有正确的图标
- [ ] `nano-installer build` 成功生成安装器
- [ ] 安装器 GUI 正常启动
- [ ] 安装流程完整执行
- [ ] uninst.exe 正确提取到安装目录
- [ ] 卸载功能正常工作

### 代码质量

- [ ] 无编译警告
- [ ] 无 clippy 警告
- [ ] 代码结构清晰
- [ ] 文档完整

### 性能验收

- [ ] 编译时间 < 5 分钟（release mode）
- [ ] 安装器启动时间 < 2 秒
- [ ] 卸载器体积 < 300 KB

---

## 回滚计划

如果重构失败，使用 git 回滚：

```bash
git checkout <previous-commit>
```

或者保留备份分支：

```bash
git checkout -b workspace-refactor
# 进行重构
# 如果失败：
git checkout main
```

---

## 时间估算

| 阶段 | 预计时间 |
|------|---------|
| 阶段 1：创建结构 | 30 分钟 |
| 阶段 2：配置 Workspace | 1 小时 |
| 阶段 3：创建 build.rs | 30 分钟 |
| 阶段 4：修改代码 | 2 小时 |
| 阶段 5：测试验证 | 1 小时 |
| **总计** | **5 小时** |

---

## 参考资料

- [Cargo Workspace 文档](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [winres crate 文档](https://docs.rs/winres/)
- [NSIS 源码](https://sourceforge.net/projects/nsis/)

