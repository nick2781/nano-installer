<h1 align="center"><img src="assets/nano-technology.png" width="48" height="48" alt="Nano Installer icon"> Nano Installer</h1>

<p align="center"><a href="README.md">English</a> | <b>简体中文</b></p>

配置驱动的 Windows x64 安装包构建器。命令行 builder 与它生成的每个 setup 共用同一套原生 Win32
runtime，基线为 Windows 7 SP1+ 与 Unicode。可选的编写 GUI 是独立的 Windows 10+ crate；
egui/eframe 不会进入任何 stub 或 setup。

> **状态：** 实验性的原生实现，不是可发布成品。安装行为只能在隔离虚拟机中验证；接入产品前请先阅读
> [生产状态](docs/PRODUCTION_STATUS.md)。

## Crate 与产物

| Crate | 产物 | 职责 |
| --- | --- | --- |
| `nano-installer-core` | 库 | 项目检查、bundle 组装、XML 布局解析、Win32 UI、安装/卸载编排 |
| `nano-installer-cli` | `nano-installer-native-x64.exe` | core 构建 API 的命令行前端 |
| `nano-installer-gui` | `nano-installer-gui-x64.exe` | 基于同一 core API 的 Windows 10+ egui 前端 |
| `nano-installer-stub-lzma` | `stubs/lzma-stub-native.exe` | 7z/LZMA 解包；只链接 `sevenz-rust` |
| `nano-installer-stub-zlib` | `stubs/zlib-stub-native.exe` | ZIP/Deflate 解包；只链接 `zip` |
| `nano-installer-uninstaller` | `stubs/uninst-stub-native.exe` | manifest 驱动的移除；不链接解包后端 |

所有对外发布的 EXE 都针对 `x86_64-win7-windows-msvc`。GUI 是唯一的 Windows 10+ 二进制，且从不嵌入
setup。

## 构建

环境要求：Windows x64、MSVC 与 Windows SDK（Visual Studio 2022 Build Tools），以及仓库固定的
`nightly-2025-11-08` 工具链，组件由 `rust-toolchain.toml` 声明。

```powershell
.\scripts\build.ps1
```

脚本以 `-Z build-std=std,panic_abort`、静态 CRT 与 `panic=abort` 为 `x86_64-win7-windows-msvc`
构建 CLI 与三个 stubs，用 `+crt-static` 按 host target 构建 GUI，用真实 ZIP 与 7z 归档做解包冒烟并
比对 SHA-256，执行 Win7 基线 PE 导入审计，最后输出：

```text
target/release/
  nano-installer-native-x64.exe
  nano-installer-gui-x64.exe
  stubs/
    lzma-stub-native.exe
    zlib-stub-native.exe
    uninst-stub-native.exe
```

`target/x86_64-win7-windows-msvc/` 与 `target/gui-build/` 是 Cargo 缓存，不是额外发布产物。发布体积在
每次构建后实测，属于发布指标，不在本文档维护。

## 构建项目

一个项目是包含 `installer_config.json`、XML 布局、位图资源、JSON 本地化文件、已压缩的 ZIP 或 7z
payload，以及可选 scripts 的目录。所有配置路径都相对于项目目录解析。

```powershell
.\target\release\nano-installer-native-x64.exe build `
  --project .\examples\TapTap `
  --output .\examples\TapTap\dist\TapTap_Setup.exe
```

- `--output` 缺省为项目内的 `dist/<output.installer_name>`。
- `--stubs <目录>` 覆盖同级 `stubs/` 查找；`NANO_INSTALLER_NATIVE_STUB_DIR` 覆盖两者。
- builder 读取 payload 签名、复制匹配的 stub、注入配置的 setup 图标与 `VERSIONINFO`、追加 bundle，
  最后追加自包含卸载器。它不会把自身复制进 setup，也不会生成中间 `skins.zip`。
- GUI 通过同一 core API 检查和构建，不启动 CLI 子进程。

`examples/TapTap` 是测试项目，不是已发布的 setup，也不是 GUI 的默认项目。它的 payload
`examples/TapTap/payload/app.7z` 未入库（`.gitignore` 排除 `*.7z`），构建前需要自行放入一个 7z
归档。构建它并审计内嵌卸载器的 Win7 导入与文件版本：

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

不要在普通工作站上执行其安装动作：它会写入文件并注册卸载表项，默认目标位于 `Program Files` 下且没有
自动 UAC 请求。

## 校验

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

发布脚本另外执行归档解包校验与 PE 导入审计。两者都不能替代在干净 Windows 7 SP1 虚拟机上的端到端
安装/卸载验证；其中一项 HKCU 写入测试在受限环境中被忽略。

## 运行时行为与待办

安装会暂存 payload，遇到同一项目的既有安装时按升级替换，并在 manifest 中记录已部署文件、
快捷方式与自启动项，随后注册卸载器。移除会先关闭产品进程，只删除 manifest 跟踪的文件，
并遵循数据保留选项。

仍未实现：任务进度与页面切换、Rhai 脚本执行、自动 UAC、代码签名，以及 Win7 虚拟机验收。

## 文档

- [快速开始](docs/QUICK_START.md)
- [项目结构](docs/PROJECT_STRUCTURE.md)
- [配置参考](docs/CONFIG_REFERENCE.md)
- [XML 布局指南](docs/XML_LAYOUT_GUIDE.md)
- [本地化](docs/LOCALIZATION.md)
- [GUI 构建工具](docs/GUI.md)
- [构建与发布](docs/BUILD_AND_RELEASE.md)
- [测试计划](docs/TEST_PLAN.md)
- [Native 架构](docs/NATIVE_ARCHITECTURE.md)
- [Windows 兼容性](docs/WINDOWS_COMPATIBILITY.md)
- [生产状态](docs/PRODUCTION_STATUS.md)
- [文档索引](docs/README.md)

## 权利与许可

- 本仓库的 Rust 源码以 [MIT License](LICENSE) 授权。
- `examples/TapTap` 仅用于校验。其中的 TapTap 商标、图片与文案归
  **易玩（上海）网络科技有限公司**及相关权利人所有，不属于 MIT 授权范围。
- 生成的 setup 与卸载器携带该项目配置的产品资源，按其产品自身的授权处理。
