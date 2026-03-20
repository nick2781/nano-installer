# TapTap Dual Example Design

**Date:** 2026-03-20

## Goal

将当前仓库中的 `TapTap-v2` 升级为正式的国内发布级样板，并新增一个完全独立的海外样板 `TapTap-Global`。同时删除旧的历史 `TapTap` 样板，避免仓库长期维持多套含义不清的示例工程。

## Confirmed Decisions

- 国内版目录使用 `examples/TapTap`
- 海外版目录使用 `examples/TapTap-Global`
- 删除旧的历史 `examples/TapTap`
- 删除 `examples/TapTap-v2`
- 国内与海外 **各自独立维护一整套 example**
- 不做共享 layout + 覆盖资源的结构
- 本轮需要补齐以下 7 类页面：
  - `config`
  - `installing`
  - `finish`
  - `uninstall_confirm`
  - `uninstalling`
  - `uninstall_finish`
  - `msgBox`

## Architecture

example 层只表达“产品样板”，不表达“产品族条件系统”。

运行时仍然保持一套公共 installer framework：
- XML DSL
- i18n
- DPI-aware asset lookup
- payload extraction
- scripting API

但 example 层不再做“国内/海外差异共享抽象”。每个样板都各自拥有：
- `installer_config.json`
- `layouts/`
- `assets/`
- `locales/`
- `scripts/`
- `payload/`
- `README.md`

这样后续设计继续分叉时，不需要把业务差异塞回 runtime 条件判断，也不会把样板工程做成难以理解的共享树。

## Scope

### In scope

- 将 `TapTap-v2` 内容迁移为新的正式 `examples/TapTap`
- 新增 `examples/TapTap-Global`
- 基于最新 Figma 补齐国内和海外的 7 类页面
- 更新 docs / tests / CI / release workflow 中的 example 路径与命名
- 删除旧 `TapTap` 和 `TapTap-v2`

### Out of scope

- 不修改卸载业务逻辑
- 不新增新的 runtime 业务能力，除非页面无法由现有 DSL 表达
- 不将国内/海外差异重新抽象回共享条件系统

## Page Mapping

### Domestic: `examples/TapTap`

以当前已收好的 `TapTap-v2` 为基础，继续对齐最新设计：
- 配置页
- 安装中页
- 完成页
- 卸载确认页
- 卸载中页
- 卸载完成页
- 消息框

### Global: `examples/TapTap-Global`

以国内版目录为母版复制一套，再逐页替换：
- slogan / title assets
- 安装中背景
- 完成页文案
- 卸载页资源
- 海外 locale 与语言选择资源

## Validation

完成后需要满足：

- `examples/TapTap` 可成功 build
- `examples/TapTap-Global` 可成功 build
- 两个 example 的 `harness lint-resources` 通过
- 国内版至少肉眼确认 `config / installing / finish`
- 海外版至少肉眼确认 `config / installing / finish`
- 卸载三页与 `msgBox` 能正常加载，且不出现明显布局错误

## Migration Notes

- 所有 docs 中对 `TapTap-v2` 的引用需要切换为：
  - `TapTap`（国内）
  - `TapTap-Global`（海外）
- 旧 `TapTap` 样板删除后，测试与文档不再保留其路径
- 如果有仅用于旧样板的资源、脚本或构建说明，需同步清理
