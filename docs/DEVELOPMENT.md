# 开发指南

本文档面向想要**参与 nano-installer 开发**或**修改源代码**的开发者。

如果你只是想**使用 nano-installer 制作安装程序**，请查看：
- [主文档](../README.md) - 快速开始
- [配置参考](CONFIG_REFERENCE.md) - 如何配置
- [示例项目](../examples/TapTap/README.md) - 完整示例

---

## 📖 目录

- [开发环境设置](#开发环境设置)
- [项目结构](#项目结构)
- [构建和测试](#构建和测试)
- [代码风格](#代码风格)
- [贡献流程](#贡献流程)

## 开发环境设置

### 系统要求

- **Rust**: 1.81 或更高版本
- **操作系统**: Windows 10+ (开发和测试)
- **工具**: Git, Visual Studio Build Tools (Windows)

### 安装 Rust

```bash
# Windows
winget install Rustlang.Rustup

# 或访问 https://rustup.rs/
```

### 克隆项目

```bash
git clone https://github.com/yourusername/nano-installer.git
cd nano-installer
```

### 安装依赖

```bash
# 检查依赖
cargo check

# 第一次构建（会下载依赖）
cargo build
```

## 项目结构

```text
nano-installer/
├── installer/
│   ├── cli/                    # nano-installer CLI
│   ├── lib/                    # runtime / layout / installer / uninstaller 核心库
│   └── stubs/                  # installer / uninstaller stub
├── examples/
│   ├── TapTap/                 # v1 示例工程
│   └── TapTap-Global/          # 海外版发布级示例工程
├── docs/                       # docsify 文档站
├── scripts/                    # smoke、诊断、辅助脚本
├── templates/                  # init 生成模板
└── tools/                      # 随产品分发的辅助工具（如 7za.exe）
```

### 核心模块说明

| 模块 | 功能 | 关键文件 |
|------|------|----------|
| `installer/lib/src/config` | 配置解析和验证 | `installer_config.rs`, `validation.rs` |
| `installer/lib/src/layout` | XML 布局 DSL 与 Taffy 桥接 | `xml_parser.rs`, `taffy_bridge.rs` |
| `installer/lib/src/ui` | egui UI 和布局渲染 | `egui_app_xml.rs`, `layout_renderer.rs` |
| `installer/lib/src/resources` | payload / bundle / 7za 资源链路 | `payload.rs`, `bundle.rs` |
| `installer/lib/src/scripting` | Rhai 脚本 API 与上下文 | `engine.rs`, `context.rs` |
| `installer/lib/src/uninstaller` | 卸载引擎 | `engine.rs` |

## 构建和测试

### 开发构建

```bash
# 构建所有二进制文件
cargo build
```

### Release 构建

```bash
# 重编 release stubs / runtime
cargo build --release -p nano-installer-lzma -p uninst -p nano-installer-cli

# 构建 TapTap 国内版示例安装器
cargo run --release -p nano-installer-cli -- build --project examples/TapTap

# 构建 TapTap 海外版示例安装器
cargo run --release -p nano-installer-cli -- build --project examples/TapTap-Global
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test config

# 显示输出
cargo test -- --nocapture
```

### 测试安装器

```bash
# 使用示例项目测试
cd examples/TapTap
../../target/debug/installer.exe --config installer_config.json
```

### 代码检查

```bash
# 检查编译错误
cargo check

# 代码格式化
cargo fmt

# Lint 检查
cargo clippy

# 修复简单问题
cargo clippy --fix
```

## 代码风格

### Rust 风格指南

遵循标准 Rust 风格：

```rust
// ✅ 好的命名
struct InstallerConfig { ... }
fn load_config() -> Result<Config> { ... }
let user_name = "Alice";

// ❌ 不好的命名
struct installerconfig { ... }
fn LoadConfig() -> Result<Config> { ... }
let userName = "Alice";
```

### 注释规范

```rust
/// 加载配置文件
/// 
/// # Arguments
/// 
/// * `path` - 配置文件路径
/// 
/// # Returns
/// 
/// 成功返回 `InstallerConfig`，失败返回错误
pub fn load_config(path: &Path) -> Result<InstallerConfig> {
    // 实现细节注释
    let content = fs::read_to_string(path)?;
    // ...
}
```

### 错误处理

```rust
// ✅ 使用 Result 和 ?
pub fn do_something() -> Result<()> {
    let config = load_config("config.json")?;
    process(config)?;
    Ok(())
}

// ❌ 避免 unwrap (除非在测试中)
let config = load_config("config.json").unwrap(); // 不推荐
```

### 文档

所有公开 API 都应该有文档注释：

```rust
/// 安装器配置
/// 
/// 从 `installer_config.json` 加载的完整配置。
/// 
/// # Example
/// 
/// ```rust
/// let config = InstallerConfig::load("config.json")?;
/// ```
pub struct InstallerConfig {
    /// 项目信息
    pub project: ProjectConfig,
    // ...
}
```

## 贡献流程

### 1. Fork 和克隆

```bash
# Fork 项目到你的 GitHub
# 然后克隆你的 fork
git clone https://github.com/your-username/nano-installer.git
cd nano-installer
```

### 2. 创建分支

```bash
# 从 master 创建特性分支
git checkout -b feature/my-awesome-feature

# 或修复分支
git checkout -b fix/bug-description
```

### 3. 开发

```bash
# 进行修改
# 编写测试
# 运行测试
cargo test

# 格式化代码
cargo fmt

# 检查 lint
cargo clippy
```

### 4. 提交

```bash
# 添加修改
git add .

# 提交（使用清晰的提交信息）
git commit -m "Add XML validation for Image elements"
```

**提交信息规范：**
- `feat: 添加新功能`
- `fix: 修复 bug`
- `docs: 更新文档`
- `style: 代码格式化`
- `refactor: 重构代码`
- `test: 添加测试`
- `chore: 构建/工具相关`

### 5. 推送和 PR

```bash
# 推送到你的 fork
git push origin feature/my-awesome-feature
```

然后在 GitHub 上创建 Pull Request。

### PR 检查清单

在提交 PR 前确认：

- [ ] 代码通过 `cargo test`
- [ ] 代码通过 `cargo clippy`
- [ ] 代码已格式化 (`cargo fmt`)
- [ ] 添加了必要的测试
- [ ] 更新了相关文档
- [ ] 提交信息清晰明确

## 调试技巧

### 启用日志

```rust
// 在代码中添加日志
use tracing::{info, warn, error, debug};

info!("Loading config from {:?}", path);
debug!("Config content: {:?}", config);
```

```bash
# 运行时设置日志级别
RUST_LOG=debug cargo run --bin installer
RUST_LOG=nano_installer=trace cargo run --bin installer
```

### 使用 Rust 调试器

Visual Studio Code + rust-analyzer:

1. 安装 CodeLLDB 扩展
2. 设置断点
3. F5 启动调试

### 性能分析

```bash
# 使用 flamegraph
cargo install flamegraph
cargo flamegraph --bin installer
```

## 常见任务

### 添加新的 XML 元素

1. 在 `src/layout/element.rs` 中添加元素类型到 `ElementType` 枚举
2. 在 `src/ui/layout_renderer.rs` 中实现渲染逻辑
3. 在 `docs/XML_SCHEMA.md` 中添加文档
4. 添加测试

### 添加新的配置选项

1. 在 `src/config/installer_config.rs` 中添加字段
2. 更新 `examples/TapTap/installer_config.json` 示例
3. 在 `docs/CONFIG_REFERENCE.md` 中添加文档
4. 在 `docs/CONFIG_REFERENCE.md` 中更新对应配置说明

### 添加新语言

1. 在 `examples/TapTap/locales/` 中创建 `<locale>.json`
2. 测试语言加载和显示
3. 更新 `docs/LOCALIZATION.md`

## 技术栈

- **UI**: egui 0.33 (即时模式 GUI)
- **图片**: image 0.25
- **压缩**: 7za.exe (外部工具)
- **XML 解析**: quick-xml 0.36
- **JSON 解析**: serde_json 1.0
- **Windows API**: windows 0.58
- **日志**: tracing + tracing-subscriber

## 发布流程

1. 更新版本号 (`Cargo.toml`)
2. 更新 `CHANGELOG.md`
3. 创建 Git tag: `git tag v0.1.0`
4. 推送 tag: `git push --tags`
5. 构建 release: `cargo build --release`
6. 创建 GitHub Release

## 获取帮助

- **文档问题**: 提交 Issue 并标记 `documentation`
- **Bug 报告**: 提交 Issue 并标记 `bug`
- **功能请求**: 提交 Issue 并标记 `enhancement`
- **讨论**: 使用 GitHub Discussions

## 资源链接

- [Rust 官方文档](https://doc.rust-lang.org/)
- [egui 文档](https://docs.rs/egui/)
- [quick-xml 文档](https://docs.rs/quick-xml/)
- [windows-rs 文档](https://microsoft.github.io/windows-docs-rs/)

---

感谢你对 nano-installer 的贡献！ 🎉
