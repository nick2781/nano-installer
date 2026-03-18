# 项目结构

`nano-installer` 当前按“工具链 + 运行时 + 示例工程 + 发布文档”四层组织。

## 顶层目录

```text
nano-installer/
├── installer/     # CLI、核心库、安装器/卸载器 stub
├── examples/      # 示例工程（如 TapTap、TapTap-v2）
├── docs/          # 对外发布文档（docsify）
├── assets/        # 仓库级共享图标资源
├── tools/         # 随产品分发的辅助工具（如 7za）
├── scripts/       # smoke、诊断、辅助脚本
└── templates/     # init 命令使用的模板
```

## 核心代码

`installer/` 是产品主体，按职责再拆成三块：

- `installer/cli/`
  - `nano-installer.exe`
  - 提供 `init`、`build`、`validate`、`langpack`
  - 同时包含面向验证工具的 `harness snapshot`、`harness lint-resources`
- `installer/lib/`
  - 共享核心库
  - 包含配置、资源、安装/卸载引擎、XML 布局、UI 渲染、本地化
- `installer/stubs/`
  - `lzma/`：安装器运行时 stub
  - `uninst/`：卸载器运行时 stub

## 示例工程

`examples/` 保存可直接构建的安装器样板工程：

- `examples/TapTap/`
  - 原始样板
- `examples/TapTap-v2/`
  - 当前较新的视觉和交互基线

每个示例工程通常包含：

```text
examples/<Project>/
├── installer_config.json
├── layouts/
├── assets/
├── locales/
├── payload/
├── .build/    # 构建中间产物
└── dist/      # 最终 setup 输出
```

## 文档

`docs/` 是对外发布面，按 docsify 组织：

- `docs/index.html`
- `docs/_sidebar.md`
- `docs/_navbar.md`
- `docs/README.md`

保留的核心文档包括：

- 架构概览
- 配置参考
- XML 布局 DSL
- 本地化
- 开发指南
- 测试与验证

## 构建边界

最重要的工程约束如下：

- 修改 `installer/**`
  - 必须先重编 runtime/stub
  - 然后再重新 `build --project ...`
- 只修改 `examples/**`
  - 只需要重建对应示例工程

详见 [AGENTS.md](../AGENTS.md)。

## 产物约定

典型发布产物：

- `target/release/nano-installer.exe`
- `target/release/lzma-x64-unicode.exe`
- `target/release/uninst.exe`
- `examples/<Project>/dist/*_Setup.exe`

`1x/@2x` 资源通过运行时 DPI 逻辑自动切换，布局 XML 默认只写 `1x` 路径。
