# 开发文档

## 快速开始

### 环境要求

- Rust 1.75+
- Windows 7 SP1+ (64位) 用于测试
- 7-Zip 或其他 7z 压缩工具

### 构建步骤

1. **克隆仓库**
```bash
git clone <repository-url>
cd nano-installer
```

2. **构建语言包**
```bash
cargo run --bin langpack-builder -- locales dist/locales
```

3. **构建安装器和卸载器**
```bash
cargo build --release
```

或使用构建脚本：
```bash
# Linux/macOS
./scripts/build.sh

# Windows
.\scripts\build.ps1
```

### 项目结构

```
nano-installer/
├── src/
│   ├── bin/                  # 可执行文件
│   │   ├── installer.rs      # 安装器入口
│   │   ├── uninstaller.rs    # 卸载器入口
│   │   └── langpack_builder.rs # 语言包构建工具
│   ├── common/               # 通用功能
│   ├── i18n/                 # 多语言系统
│   ├── installer/            # 安装逻辑
│   ├── uninstaller/          # 卸载逻辑
│   ├── resources/            # 资源管理
│   ├── logger/               # 日志系统
│   ├── ui/                   # UI 模块
│   └── lib.rs
├── locales/                  # 语言文件（JSON）
├── scripts/                  # 构建脚本
├── .spec/                    # 技术规范
└── dist/                     # 输出目录
```

## 开发指南

### 添加新语言

1. 在 `locales/` 目录创建新的 JSON 文件，如 `fr.json`
2. 复制 `en-US.json` 的所有键，翻译对应的值
3. 在 `src/i18n/mod.rs` 中添加语言代码到 `SUPPORTED_LOCALES`
4. 重新构建语言包

### 添加新的安装任务

1. 在 `src/installer/tasks.rs` 中实现 `InstallTask` trait
2. 在 `src/bin/installer.rs` 的 `run_silent_install` 函数中添加任务

### 修改 UI

等待设计稿后使用 GPUI 实现：
- 页面：`src/ui/pages/`
- 组件：`src/ui/components/`
- 样式：`src/ui/styles/`

### 测试

```bash
# 运行所有测试
cargo test

# 测试特定模块
cargo test --lib i18n

# 测试语言包构建
cargo test langpack
```

### 调试

启用详细日志：
```bash
RUST_LOG=debug cargo run --bin installer
```

## 打包流程

### 1. 准备 Payload

将要安装的应用程序打包为 7z：
```bash
7z a app.7z your-app-files/
```

### 2. 嵌入 Payload

使用打包脚本：
```bash
./scripts/package.sh app.7z MyAppSetup.exe
```

### 3. 代码签名（可选）

```bash
./scripts/sign.sh MyAppSetup.exe cert.pfx password
```

## 常见问题

### Q: GPUI 在 Windows 7 上无法运行？
A: 确保安装了最新的显卡驱动，GPUI 需要 DirectX 11+ 支持。

### Q: 语言包构建失败？
A: 检查 JSON 文件格式是否正确，所有语言文件必须包含相同的键。

### Q: 如何在非 Windows 系统上开发？
A: 大部分代码可以在任何平台编译，但 Windows 特定功能（注册表、快捷方式）需要条件编译。

### Q: 静默安装不工作？
A: 检查是否提供了必要的参数，某些路径可能需要管理员权限。

## 贡献

欢迎贡献代码、报告问题或提出建议！

## 许可证

待定

