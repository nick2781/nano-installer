# nano-installer 重构状态

## 重构目标

将 `nano-installer` 简化为单一的 CLI 工具，生成独立的安装器和卸载器 exe。

## 已完成 ✅

### 1. 简化二进制结构
- ✅ `Cargo.toml` 只保留 `nano-installer` 一个 bin
- ✅ 删除 `builder.rs`、`langpack-builder.rs`、`uninstaller.rs`
- ✅ `build.rs` 只为 `nano-installer.exe` 设置图标

### 2. CLI 工具功能
- ✅ `nano-installer init` - 创建新项目
- ✅ `nano-installer validate` - 验证配置
- ✅ `nano-installer langpack` - 构建语言包
- ✅ `nano-installer build` - 框架代码（待实现完整功能）

### 3. 模板系统
- ✅ 创建 `templates/` 目录
- ✅ 基础布局模板（welcome, config, installing, finish）
- ✅ 语言模板（en-US, zh-CN）
- ✅ `init` 命令使用 `include_str!` 嵌入模板

### 4. 项目清理
- ✅ 移除嵌入语言包逻辑（`loader.rs`）
- ✅ 移除 `embedded-locales` feature
- ✅ 简化 `build.rs`，不再构建语言包

## 待实现 ⚠️

### 1. 资源嵌入机制
**问题**：如何将用户项目的资源嵌入到生成的安装器 exe 中？

**当前方案**：
- 使用 `rust-embed` crate
- 在 `build` 命令中动态生成 Rust 代码
- 编译生成的代码以创建最终的 exe

**步骤**：
1. 在 `nano-installer build` 时：
   - 创建临时 Rust 项目
   - 使用 `rust-embed` 嵌入用户项目的资源
   - 链接 `nano-installer` 的安装器引擎库
   - 编译生成 `ProjectName_Setup.exe`

2. 修改 `src/bin/installer.rs` 为库代码：
   - 移动到 `src/installer_runtime/` 或类似
   - 提供 `run_installer()` 入口函数
   - 支持从嵌入资源运行

### 2. 安装器引擎改造
**当前状态**：`src/bin/installer.rs` 是一个独立的 bin

**需要改为**：
1. 将其改为库代码（`src/installer_runtime/mod.rs`）
2. 提供统一的入口点：
   ```rust
   pub fn run_installer(mode: InstallerMode) -> Result<()>
   ```
3. `mode` 可以是：
   - `Install` - 安装模式
   - `Uninstall` - 卸载模式
   - `Update` - 更新模式

### 3. 资源加载改造
**当前状态**：从文件系统加载资源（`layouts/`, `assets/`, `locales/`）

**需要改为**：
1. 优先从嵌入资源加载
2. 回退到文件系统（用于开发/调试）
3. 使用 `ResourceLoader` trait：
   ```rust
   pub trait ResourceLoader {
       fn load_layout(&self, name: &str) -> Result<String>;
       fn load_asset(&self, name: &str) -> Result<Vec<u8>>;
       fn load_locale(&self, locale: &str) -> Result<Vec<u8>>;
   }
   
   pub struct EmbeddedResourceLoader { /* rust-embed */ }
   pub struct FileSystemResourceLoader { /* 文件系统 */ }
   ```

### 4. 构建流程实现
**`nano-installer build` 需要实现**：

```rust
fn build_installer_exe() -> Result<()> {
    // 1. 创建临时构建目录
    let temp_dir = create_temp_build_dir()?;
    
    // 2. 生成 main.rs
    generate_installer_main(&temp_dir, project_config)?;
    
    // 3. 生成 Cargo.toml
    generate_cargo_toml(&temp_dir)?;
    
    // 4. 复制资源到 temp_dir/resources/
    copy_project_resources(&temp_dir, project_dir)?;
    
    // 5. 链接 nano-installer 库
    add_dependency_to_nano_installer(&temp_dir)?;
    
    // 6. 编译
    run_cargo_build(&temp_dir, release)?;
    
    // 7. 复制生成的 exe 到 dist/
    copy_output_exe(&temp_dir, dist_dir, output_name)?;
    
    // 8. 清理临时目录
    cleanup_temp_dir(&temp_dir)?;
    
    Ok(())
}
```

### 5. 卸载器生成
**选项 A（推荐）**：安装器和卸载器是同一个 exe
- 安装时将 `ProjectName_Setup.exe` 复制到安装目录并重命名为 `uninst.exe`
- 通过 exe 名称或参数判断运行模式

**选项 B**：独立生成卸载器
- 类似安装器的构建流程
- 嵌入卸载所需的最小资源集

## 编译测试

```powershell
# 清理并编译
cargo clean
cargo build --release --bin nano-installer

# 测试
.\target\release\nano-installer.exe --version
.\target\release\nano-installer.exe init TestProject
.\target\release\nano-installer.exe validate TestProject/installer_config.json
.\target\release\nano-installer.exe build --project TestProject
```

## 图标状态

当前 `nano-installer.exe` 应该已经有图标（`assets/nano-installer.ico`）。

如果没有，需要：
1. 确保 `assets/nano-installer.ico` 存在
2. 运行 `cargo clean`
3. 重新编译 `cargo build --release --bin nano-installer`

## 下一步行动

### 优先级 1（核心功能）
1. 将 `installer.rs` 改造为库代码
2. 实现资源嵌入机制（基于 rust-embed）
3. 实现 `build` 命令的完整流程

### 优先级 2（完善功能）
4. 支持从嵌入资源加载
5. 卸载器生成
6. 图标设置（从项目配置）

### 优先级 3（增强功能）
7. 更多模板选项
8. 自动化测试
9. CI/CD 集成
10. 文档完善

## 测试计划

### 单元测试
- [ ] CLI 参数解析
- [ ] 配置文件验证
- [ ] 语言包构建

### 集成测试
- [ ] `init` 命令创建完整项目
- [ ] `validate` 命令检测配置错误
- [ ] `build` 命令生成可运行的安装器
- [ ] 生成的安装器可以正常安装和卸载

### 端到端测试
- [ ] 完整流程：init → edit → build → install → uninstall

## 已知问题

1. ⚠️ PowerShell 终端无法使用（Cursor 配置问题）
   - 需要手动在外部终端运行命令

2. ⚠️ `build` 命令未完全实现
   - 当前只是占位代码
   - 需要实现资源嵌入和编译流程

3. ⚠️ 安装器仍然从文件系统加载资源
   - 需要改造为从嵌入资源加载

