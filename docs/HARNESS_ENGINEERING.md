# Verification Tooling

> 这份文档描述的是验证工具，不是完整意义上的 harness engineering。
> 如果讨论的是面向 AI agent 的整体工程改造，请看 [AGENT_ENGINEERING.md](/D:/taptap-pc/nano-installer/docs/AGENT_ENGINEERING.md)。

## Goal

把 UI/布局验证拆成三层，而不是所有问题都直接丢给真窗口和人工截图：

1. `library snapshot`
   - 直接驱动 `InstallerApp` / `LayoutRenderer`
   - 注入语言、页面动作、布局 fixture
   - 导出布局矩形、控件状态、弹窗状态
2. `script smoke`
   - 启动真实 EXE
   - 验证安装/卸载/静默模式主链路
3. `manual visual review`
   - 只负责最后的像素级体验确认

## Current Tooling Surface

库内 snapshot/harness 实现在 [test_harness.rs](/D:/taptap-pc/nano-installer/installer/lib/src/ui/test_harness.rs)：

- `UiHarness::from_project_dir(...)`
  - 从项目目录加载 `installer_config.json`
  - 使用文件系统资源源，而不是嵌入 bundle
- `snapshot_page(...)`
  - 返回 Taffy 布局快照
- `dispatch_action(...)`
  - 直接驱动 XML action 分发
- `switch_language(...)`
  - 直接验证运行时语言切换
- `has_pending_close_confirmation()`
  - 验证消息框状态，而不是靠截图猜

CLI 诊断入口在 `nano-installer harness snapshot`：

- 直接从项目目录加载页面
- 可注入 `--locale`
- 可串行执行多个 `--action`
- 可输出 JSON / text
- 适合把 UI 问题先缩到“结构化布局证据”

资源基线入口在 `nano-installer harness lint-resources`：

- 扫描 `layouts/*.xml` 和 `installer_config.json` 引用到的图片资源
- 遵守 `resources.layouts_dir / assets_dir / locales_dir`，不再默认写死 `layouts/assets/locales`
- 检查资源是否缺失、是否可解码、是否有合理的 `@2x` 对应项
- 对小图标额外检查 alpha 可视区域是否明显偏心
- 适合在换资源、换皮肤、接设计稿前先把“脏资源”挡在外面

构建链路也会复用这条基线：

- `nano-installer build` 前置执行同一套资源校验
- 如果资源 lint 有 `error`，构建直接失败

示例：

```powershell
nano-installer harness snapshot `
  --project examples/TapTap `
  --mode install `
  --locale ru `
  --action toggle_panel:moreconfiginfo:show `
  --page config

nano-installer harness lint-resources `
  --project examples/TapTap `
  --format text
```

## Resource Provider Split

`InstallerApp` 不再硬绑 `RuntimeResources`，而是通过 `UiResourceProvider` 读取：

- `RuntimeUiResourceProvider`
  - 真正安装器运行时使用
- `FilesystemUiResourceProvider`
  - harness / fixture / 本地工程目录测试使用

这条边界在 [resource_provider.rs](/D:/taptap-pc/nano-installer/installer/lib/src/ui/resource_provider.rs)。

## Recommended Workflow

修 UI 问题时按这个顺序：

1. 先在 verification tooling 里复现
2. 用 snapshot/lint 锁定布局盒子、控件状态、语言切换结果
   - 必要时先跑 `nano-installer harness snapshot ...`
   - 资源替换前先跑 `nano-installer harness lint-resources ...`
3. 再跑 PowerShell smoke
4. 最后做肉眼确认

## Non-Goals

- verification tooling 不替代真实 EXE smoke
- verification tooling 不负责像素级 screenshot diff
- verification tooling 不模拟 OS 对话框、UAC、窗口管理器行为
