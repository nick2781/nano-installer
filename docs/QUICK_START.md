# 快速开始

## 环境

- Windows x64
- Visual Studio 2022 Build Tools（MSVC、Windows SDK）
- Rust `nightly-2025-11-08`
- `rust-src` 组件

首次准备：

```powershell
rustup toolchain install nightly-2025-11-08 --component rust-src,rustfmt,clippy
```

## 构建 TapTap example

在仓库根目录执行：

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

脚本会构建 Win7 SP1+ CLI/stubs、Windows 10+ GUI，根据 payload 文件头选择 stub，生成
`examples/TapTap/dist/TapTap_Setup.exe`，并审计 builder、三个 stubs 和 setup 的 PE 导入。

## 使用 GUI

```powershell
.\target\release\nano-installer-gui-x64.exe
```

GUI 与 CLI 共用 core 项目检查和构建 API，可选择项目/输出路径、查看配置摘要和 DPI 资源
警告，并异步生成 setup。GUI 需要 Windows 10+，但生成物仍支持 Windows 7 SP1+。

## 手动启动

```powershell
.\examples\TapTap\dist\TapTap_Setup.exe
```

当前可验证原生位图/文字、无边框圆角窗口、拖动、最小化和关闭，以及安装/卸载的页面流转
与进度显示。安装按钮会真的解压并写入文件、快捷方式和注册表：只能在测试 VM 内、指定一个
尚不存在的测试安装目录操作；不要在本机对 TapTap 生产环境执行。目标目录已有同一项目的
安装时按升级处理。项目带有 `scripts/` 时，安装与卸载步骤由其中的 Rhai 脚本决定，
见 [脚本 API](SCRIPT_API.md)。
TapTap 默认路径在 `Program Files`；测试该路径必须在隔离 VM 以管理员身份启动 setup，
当前版本不会自动弹出 UAC。当前示例 XML 的安装目录是只读文本；如需测试用户可写目录，
先把 `examples/TapTap/installer_config.json` 中的 `install.default_path` 改为 VM 内尚不存在的
目录，随后只用 builder 重建 TapTap setup，再在 VM 中测试。不要以为点击文件夹图片就能改路径。

## 直接调用 builder

已有完整 release 目录时可以直接调用：

```powershell
.\target\release\nano-installer-native-x64.exe build `
  --project .\examples\TapTap `
  --output .\examples\TapTap\dist\TapTap_Setup.exe `
  --stubs .\target\release\stubs
```

builder 默认从自身同级的 `stubs/` 查找 stub，也可通过
`NANO_INSTALLER_NATIVE_STUB_DIR` 指定目录。
