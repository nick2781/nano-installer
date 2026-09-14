# nano-installer

`nano-installer` 是一个配置驱动的 Windows 安装器框架。构建工具把产品配置、XML
布局、语言包、图片、Rhai 脚本、payload 和卸载器封装为单个 setup；安装器和卸载器
共享同一套 Rust 运行时。

## 当前状态

- 正式运行平台：Windows 10/11 x64
- 兼容构建：Windows 7 SP1 x64，已通过编译和 PE 导入审计，等待 VM 验收
- 构建平台：Windows x64 + MSVC Build Tools
- CI 工具链：Rust 1.91.1；workspace MSRV 为 1.88
- 示例工程：`examples/TapTap`

`examples/TapTap` 中的 TapTap 品牌和相关资源归易玩（上海）网络科技有限公司及相关
权利人所有，仅用于本工具的开发、测试和兼容性验证，不属于本项目开源许可范围。

Windows 7 目前不在正式支持范围。原因、证据和可选技术路线见
[Windows 兼容性](docs/WINDOWS_COMPATIBILITY.md)。

## 组件

| 组件 | 产物 | 职责 |
| --- | --- | --- |
| `installer/cli` | `nano-installer.exe` | 初始化、校验和打包项目 |
| `installer/stubs/lzma` | `lzma-x64.exe` | 7z/LZMA 安装器 stub |
| `installer/stubs/zlib` | `zlib-x64.exe` | ZIP/Deflate 安装器 stub |
| `installer/stubs/uninst` | `uninst-x64.exe` | 卸载器 stub |
| `installer/native` | `nano-installer-native-x64.exe` + native stubs | 实验中的原生 Win32 layout 构建与运行工具 |
| `installer/lib` | Rust library | 配置、资源、UI、安装和卸载引擎 |

发布工具包中的 CLI 和所选 stubs 应放在同一目录。CLI 也支持通过
`NANO_INSTALLER_STUB_DIR` 指定 stub 目录。

运行时 stub 全部使用 Unicode Windows API 和 UTF-16 字符串，不提供 ANSI 版本。
当前只发布 Windows x64 版本；文件名保留 `x64`，明确表示目标架构。

## 构建

```powershell
cargo build --locked --release -p nano-installer-lzma
cargo build --locked --release -p nano-installer-zlib
cargo build --locked --release -p uninst
cargo build --locked --release -p nano-installer-cli

.\target\release\nano-installer.exe `
  build --project .\examples\TapTap --release
```

也可以使用统一脚本：

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

可用 `-Compression lzma|zlib` 选择预编译 stub。

三个 stub 必须分别执行 `cargo build`。在同一次 invocation 中同时选择三个包会合并共享库
features，使每个 stub 都链接两种 payload 后端，失去按压缩算法拆分的意义。

Win7 SP1 x64 兼容构建见 [Windows 兼容性](docs/WINDOWS_COMPATIBILITY.md)。

输出位于 `examples/TapTap/dist/TapTap_Setup.exe`。

## 制作产品安装器

先阅读[生产接入指南](docs/PRODUCTION_INTEGRATION.md)。产品工程由以下内容组成：

```text
installer_config.json
assets/
layouts/
locales/
scripts/       # 可选
payload/app.7z
```

配置表达通用安装能力，XML 表达界面与交互，Rhai 脚本只承载产品特有副作用。

## 文档

- [文档入口](docs/README.md)
- [架构](docs/ARCHITECTURE.md)
- [生产接入](docs/PRODUCTION_INTEGRATION.md)
- [配置参考](docs/CONFIG_REFERENCE.md)
- [XML 布局](docs/XML_LAYOUT_GUIDE.md)
- [本地化](docs/LOCALIZATION.md)
- [开发](docs/DEVELOPMENT.md)
- [测试](docs/TEST_PLAN.md)
- [发布](docs/RELEASE.md)

修改 `installer/**` 后必须先重编四个 release 组件，再重新打 setup；只修改产品项目的
配置、资源、布局或语言文件时，可以直接重打对应 setup。详细约束见 [AGENTS.md](AGENTS.md)。

## License

MIT
