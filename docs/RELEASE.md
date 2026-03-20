# 发布指南

本文档描述 `nano-installer` 当前的发布面和建议发布流程。

## 发布内容

当前仓库对外发布的内容包括：

- CLI 工具
- installer runtime / stubs
- docsify 文档站
- 示例工程：`TapTap`、`TapTap-v2`

## 文档发布

文档站目录在 [docs](/D:/taptap-pc/nano-installer/docs)。

当前文档站基于 `docsify`：

- 首页：[README.md](/D:/taptap-pc/nano-installer/docs/README.md)
- 侧边栏：[docs/_sidebar.md](/D:/taptap-pc/nano-installer/docs/_sidebar.md)
- 顶部导航：[docs/_navbar.md](/D:/taptap-pc/nano-installer/docs/_navbar.md)
- 入口页：[docs/index.html](/D:/taptap-pc/nano-installer/docs/index.html)

CI 中已有独立 docs workflow，可用于发布文档站。

## Release 发布

当前 release pipeline 已覆盖：

- 基础 CI
- docs 文档发布
- release 版本发布

对应 workflow：

- [.github/workflows/ci.yml](/D:/taptap-pc/nano-installer/.github/workflows/ci.yml)
- [.github/workflows/docs.yml](/D:/taptap-pc/nano-installer/.github/workflows/docs.yml)
- [.github/workflows/release.yml](/D:/taptap-pc/nano-installer/.github/workflows/release.yml)

## 发布前检查

建议至少执行：

1. `cargo test -p nano-installer-lib --lib`
2. `cargo test -p nano-installer-cli`
3. `cargo build --release -p nano-installer-lzma -p uninst -p nano-installer-cli`
4. `cargo run --release -p nano-installer-cli -- build --project examples\\TapTap-v2`
5. `cargo run -p nano-installer-cli -- harness lint-resources --project examples\\TapTap-v2 --format text`

如果修改了 `installer/**`，必须先重编 release stubs/runtime，再重打 setup。

如果只修改了 `examples/**`，只需要重新 build 对应示例工程。

## 对外口径

发布时建议统一口径为：

- `nano-installer` 是现代化 Windows 安装器框架
- 配置负责通用能力
- XML 布局 DSL 负责界面
- 脚本负责产品业务逻辑

不再以“NSIS 迁移工具”作为主要对外定位。
