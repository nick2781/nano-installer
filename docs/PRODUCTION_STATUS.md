# 当前生产状态

**结论：当前 native 分支不能用于生产发布。**

已经验证：

- builder 与 runtime stub 分离；setup 不包含 builder。
- ZIP/7z 文件头识别与对应 stub 选择。
- 独立 LZMA 与 ZIP/Deflate backend 已链接，并通过真实归档 SHA-256 解压 smoke test。
- uninstaller 使用独立 package 和 `wizard.uninstall_pages` 入口，不链接 archive backend。
- CLI 与 Windows 10+ GUI 复用 core build/inspection API，GUI 依赖未进入 runtime stubs。
- 完整 payload、项目资源和带 icon/VERSIONINFO/卸载 UI bundle 的自包含 uninstaller 进入单 EXE。
- 安装按钮可通过匹配的 stub 解压 ZIP/7z payload，在新目录部署文件和 uninstaller，写入
  manifest 与卸载注册表；卸载按钮按 manifest 删除文件，不递归删除未知用户文件。
- 目标目录存在本项目先前安装时按升级处理：替换文件、清理旧版遗留文件，失败时回滚到
  先前版本。
- 首屏 XML 位图与 Unicode locale 文字原生渲染。
- 无边框圆角、拖动、最小化和关闭。
- 单一 Win7 SP1+ x64 release 和 PE import audit。

生产阻塞项：

1. 项目 Rhai 脚本执行尚未接入。`scripts/install.rhai` 与 `scripts/uninstall.rhai` 会被
   打包进 setup，但 runtime 不会执行，安装步骤仍由内置流程完成。
2. Authenticode 双重签名尚未接入，缺少签名证书。
3. 尚未通过真实 Win7 SP1 VM 端到端验收。

已完成的门禁（保留在此以便对照）：

- runtime 按偏移索引读取 bundle，启动不再整包载入 payload。
- 升级、旧文件清理与失败回滚已实现。
- 卸载会终止运行中的产品进程，按 manifest 清理文件与快捷方式，并按 `uninstall.data_paths`
  处理用户数据（`chkReserveData` 默认保留）；运行中的 uninstaller 安排在重启时删除。
- 安装与卸载会创建快捷方式和自启动项，二者都可通过回滚日志还原。
- 页面切换与任务进度已实现：安装和卸载切到各自的进度页，实时更新进度条与步骤文案，
  结束后切到完成页，完成页的 `launch_app` 会启动刚部署的 EXE。
- 流式布局已覆盖 VBox、HBox、Content 的嵌套摆放，以及 `padding`、`margin`、百分比尺寸、
  `justify-content` 与 `align-items`。

完成这些门禁后，生产项目才能按照 `examples/TapTap` 的结构接入。
