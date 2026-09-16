# 更新日志

本文件记录 nano-installer 的版本变更历史。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。
版本号使用 CalVer（`YYYY.M.D`），tag 形如 `v2026.9.16`。

## [2026.9.16]

安装与卸载流程已完整可用。

### 新增

- 安装与卸载全过程显示实时进度、当前步骤和完成页；完成后可以直接启动刚装好的应用。
- 重复运行安装包会按升级处理：只替换有变化的文件并清理旧版本遗留的文件，中途出错会回到安装前的状态。
- 安装时可以勾选创建桌面快捷方式、开始菜单项和开机自启动，卸载时一并清理。
- 卸载前会先关闭正在运行的产品；用户数据默认保留，取消勾选「保留用户数据」才会删除。
- 界面文案支持 11 种语言，安装与卸载的每一步提示都会跟随所选语言。
- 项目方可以用脚本自定义安装与卸载步骤；脚本出错会回滚本次改动，清理不完整时安装器会自动兜底，不留下残留。
- 界面布局支持容器嵌套、内外边距、百分比尺寸与对齐方式，进度条可按百分比显示。

### 已知限制

- 安装包尚未签名，Windows 可能提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。

<!-- release-notes:end -->

### 技术细节

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
- 仓库公开后恢复 GitHub Pages 部署：重建 `.github/workflows/docs.yml`（触发分支 `main`），
  并在 `docs/` 下补齐 docsify 站点入口 `index.html`、`_sidebar.md`、`_navbar.md` 与
  `.nojekyll`；logo 走 Git LFS，checkout 因此显式开启 `lfs: true`。
- README 页头改为 logo 与产品名水平居中、垂直居中对齐（`align="texttop"`），
  并同步 `README.zh-CN.md`、`docs/README.md`、`docs/GUI.md`。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`（53 个 core 测试、12 个 GUI 测试、2 个 stub 测试）。
- `scripts/build.ps1` 全量构建、ZIP/7z backend smoke test 与 Win7 PE 导入审计。
- CLI 与 GUI 产物内嵌 16/24/32/48/64/128/256 七种尺寸图标，逐尺寸与各自 branding PNG
  缩放结果比对；三个原始 stub 无图标和版本资源。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。

## [2026.9.15]

本次发布的是构建工具链本身，不包含示例安装包。

### 新增

- 新增图形化构建工具（Windows 10 及以上运行），可以可视化地配置并生成安装包，不必再走命令行。
- 安装包自带解压能力，用户机器上不再需要额外安装解压工具。

### 变更

- 项目结构由 `installer/` 迁移到 `crates/`，各组件统一命名。
- 运行时产物收敛为一套：64 位 Windows 7 SP1 及以上，不再单独维护 Windows 10 版本。
- 安装包与卸载程序的图标、版本信息在构建时写入。
- 发布流程只构建工具链，不再附带示例安装包。

### 已知限制

- 安装包尚未签名；未在真实 Windows 7 SP1 虚拟机上验收。

<!-- release-notes:end -->

### 技术细节

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

详见 [当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md)。
## 如何贡献

见 [AGENTS.md](AGENTS.md)。
