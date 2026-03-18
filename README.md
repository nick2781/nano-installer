# nano-installer

现代化的 Windows 安装器框架。

`nano-installer` 使用 Rust 构建，提供配置驱动的安装器构建流程、原生 GUI、国际化、多页面 XML 布局 DSL，以及可嵌入式的 payload / uninstaller 打包能力。

## 特性

- 现代化 Windows 安装器 UI
- JSON 配置 + XML 布局 DSL
- 自动 DPI 资源切换（`1x/@2x`）
- 多语言支持
- 内置 `7za` 解压链路
- 卸载器生成
- 示例工程：`TapTap`、`TapTap-v2`

## 快速开始

```bash
git clone https://github.com/Nick2781/nano-installer.git
cd nano-installer
cargo build --release
```

创建项目：

```bash
target/release/nano-installer.exe init MyApp
cd MyApp
target/release/nano-installer.exe build
```

示例构建：

```bash
target/release/nano-installer.exe build --project examples/TapTap-v2
```

输出安装器：

- `examples/TapTap-v2/dist/TapTapV2_Setup.exe`

## 文档

发布文档位于 [docs/README.md](docs/README.md)。

核心入口：

- [架构概览](docs/ARCHITECTURE.md)
- [配置参考](docs/CONFIG_REFERENCE.md)
- [XML 布局指南](docs/XML_LAYOUT_GUIDE.md)
- [XML Schema](docs/XML_SCHEMA.md)
- [本地化](docs/LOCALIZATION.md)
- [开发指南](docs/DEVELOPMENT.md)
- [测试与验证](docs/TEST_PLAN.md)

## 仓库结构

```text
nano-installer/
├── installer/        # CLI、runtime、stub、核心库
├── examples/         # 示例工程
├── docs/             # 发布文档（docsify）
├── tools/            # 随产品分发的辅助工具
└── assets/           # 仓库级共享资源
```

## 示例

- [examples/TapTap](examples/TapTap/)
- [examples/TapTap-v2](examples/TapTap-v2/)

## 开发说明

- 修改 `installer/**` 后，必须先重编 stub/runtime，再重打 setup。
- 只修改 `examples/**` 的资源、布局、文案或配置时，只需要重新 build 对应示例。

详细规则见 [AGENTS.md](AGENTS.md)。

## 许可证

MIT
