# 快速开始指南

## 5 分钟快速体验

### 前提条件
- Rust 1.75+ 已安装
- 在 macOS 上开发（Windows 上实际构建和测试）

### 步骤 1: 构建语言包

```bash
cd /Users/nick/work/nano-installer
cargo run --bin langpack-builder -- locales dist/locales
```

预期输出：
```
Building language packs...
Input directory: locales
Output directory: dist/locales

Processing: en-US -> en-US.pak
  ✓ Built successfully (XXX bytes)
...
```

### 步骤 2: 测试多语言系统

运行测试：
```bash
cargo test
```

### 步骤 3: 查看项目结构

```bash
tree -L 2 src/
```

### 步骤 4: 准备一个测试 Payload

创建一个简单的测试应用：
```bash
mkdir -p test-app
echo "Hello from test app" > test-app/readme.txt
```

压缩为 7z（需要 7-Zip）：
```bash
7z a test-app.7z test-app/
```

### 步骤 5: 构建安装器（Windows 上）

在 Windows 环境：
```powershell
.\scripts\build.ps1
```

### 步骤 6: 打包 Payload 到安装器

```bash
.\scripts\package.sh test-app.7z TestInstaller.exe
```

### 步骤 7: 测试静默安装

```cmd
TestInstaller.exe /S /D=C:\TestApp
```

## 当前可用功能

✅ **多语言系统**
```bash
cargo run --bin langpack-builder -- locales dist/locales
cargo test test_language_pack
```

✅ **命令行参数解析**
```bash
cargo run --bin installer -- --help
cargo run --bin uninstaller -- --help
```

✅ **配置系统**
查看 `src/common/config.rs` 中的默认配置

## 下一步

### 等待设计稿
收到 UI 设计稿后，可以开始实现：
1. GPUI 组件
2. 安装向导界面
3. 进度显示

### 开发新功能
参考文档：
- `docs/DEVELOPMENT.md` - 开发指南
- `docs/API.md` - API 文档
- `.spec/` - 技术规范

### 添加新语言
1. 复制 `locales/en-US.json` 为新语言文件
2. 翻译所有键值
3. 在 `src/i18n/mod.rs` 中添加语言代码
4. 重新构建语言包

## 常见问题

**Q: 在 macOS 上如何测试 Windows 功能？**
A: Windows 特定代码使用条件编译，在 macOS 上会被跳过。实际测试需要在 Windows 环境。

**Q: GPUI 依赖如何处理？**
A: 当前 Cargo.toml 引用了 GPUI 的 git 仓库，首次构建会下载。

**Q: 如何修改默认配置？**
A: 编辑 `src/common/config.rs` 中的 `InstallerConfig::default()`。

## 调试技巧

启用详细日志：
```bash
RUST_LOG=debug cargo run --bin installer
```

查看生成的语言包：
```bash
xxd dist/locales/en-US.pak | head -20
```

检查依赖树：
```bash
cargo tree
```

## 项目状态

查看 `PROJECT_STATUS.md` 了解：
- ✅ 已完成的功能
- 🚧 进行中的工作
- ⏳ 待完成的任务
- ⚠️ 已知问题

