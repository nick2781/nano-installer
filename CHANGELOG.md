# 更新日志

本文件记录 nano-installer 的版本变更历史。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。
版本号使用 CalVer（`YYYY.M.D`），tag 形如 `v2026.9.15`。

## [未发布]

### 变更

- 安装改为按偏移索引读取内嵌 bundle，启动不再把整个 payload 载入内存。
- 目标目录存在同一项目的先前安装时按升级处理：替换文件、清理旧版遗留文件，注册失败时
  回滚到先前版本。
- 安装按 `shortcuts.*` 与 `chkShotcut`/`chkAutoRun` 创建桌面、开始菜单快捷方式和开机
  自启动项；`autostart.*` 会先记录原值，失败时还原。
- 卸载会终止运行中的产品进程，按 manifest 清理快捷方式，并按 `uninstall.data_paths`
  删除用户数据；`chkReserveData` 未勾选保留数据时才会删除，默认保留。
- 示例 `examples/TapTap` 重新声明 `uninstall.data_paths`，卸载页补回 `chkReserveData`。
- 安装与卸载会切换到各自的进度页，实时更新进度条和步骤文案，任务结束后切到完成页；
  完成页的 `launch_app` 启动刚部署的 EXE。
- 布局引擎支持嵌套 `VBox`/`HBox`/`Content`，新增 `padding`、`margin`（含单边写法）、
  百分比尺寸、`justify-content` 与 `align-items`；`ProgressBar` 按百分比裁剪 `bar-image`。
- `value-source="status"` 让进度页显示运行时发布的 locale 键，`action="finish"` 与
  `action="launch_app"` 分别对应关闭窗口和启动已安装应用。
- 11 个 locale 补齐 `status.extracting`、`status.deploying`、`status.finishing`。
- 接入项目 Rhai 脚本：`scripts/install.rhai` 与 `scripts/uninstall.rhai` 存在时，安装与卸载
  步骤由脚本决定，原语复用内置流程的部署、回滚与 manifest 代码。脚本失败回滚本次改动；
  卸载脚本漏调 `run_tracked_uninstall` 时由库回退清理，产品不会残留。
- `set_status_key` 让脚本步骤文案走 locale，`set_status` 仍显示字面文本；11 个 locale 补齐
  `status.checking_processes`、`status.installing_uninstaller`、`status.creating_shortcuts`、
  `status.writing_registry` 与 `uninstall.status.cleaning_game_data`。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`（53 个 core 测试、12 个 GUI 测试、2 个 stub 测试）。
- `scripts/build.ps1` 全量构建、ZIP/7z backend smoke test 与 Win7 PE 导入审计。
- CLI 与 GUI 产物内嵌 16/24/32/48/64/128/256 七种尺寸图标，逐尺寸与各自 branding PNG
  缩放结果比对；三个原始 stub 无图标和版本资源。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。

## [2026.9.15]

该版本发布的是构建工具链本身，不含任何产品 setup。

### 变更

- 目录结构由 `installer/**` 迁移到 `crates/**`，crate 名统一为 `nano-installer-*`：
  `nano-installer-core`、`nano-installer-cli`、`nano-installer-gui`、
  `nano-installer-stub-lzma`、`nano-installer-stub-zlib`、`nano-installer-uninstaller`。
- 运行时基线收敛为单一 `x86_64-win7-windows-msvc`，最低 Windows 7 SP1 x64，不再提供
  单独的 Windows 10 产物。GUI 只作为 Windows 10+ 构建工具，不进入 setup。
- 解压由外部 `tools/7za.exe` 改为内嵌：7z/LZMA 与 ZIP/Deflate 各由独立 stub 承担，
  setup 运行时不再需要外部解压工具。
- builder 为 setup 与自包含 uninstaller 注入图标和 Unicode `VERSIONINFO`；原始 stub
  保持无产品资源，由项目 `output` 配置决定生成物的资源。
- 新增 Windows 10+ 可视化构建工具 `nano-installer-gui-x64.exe`，与 CLI 复用同一
  core build/inspection API，不启动 CLI 子进程。
- 发布流程只构建工具链，不再附带示例 setup。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`。
- 用真实 ZIP 与 7z 归档验证两个 release stub 的解压结果 SHA-256。
- builder、三个 stub 以及生成的 setup 的 Win7 PE 导入审计。

### 未完成

- runtime 启动时仍会整体读入内嵌 payload，需要改为 offset/mmap 访问。
- 无升级、覆盖安装、取消、完整进度页与故障回滚。
- 卸载只清理 manifest 内文件；无数据保留选项、快捷方式清理与进程终止。
- 尚未实现快捷方式、开机自启动、静默安装与项目 Rhai 脚本执行。
- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。

详见 [当前生产状态](docs/PRODUCTION_STATUS.md)。

## 如何贡献

见 [AGENTS.md](AGENTS.md)。
