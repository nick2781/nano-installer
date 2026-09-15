# 更新日志

本文件记录 nano-installer 的版本变更历史。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。
版本号使用 CalVer（`YYYY.M.D`），tag 形如 `v2026.9.15`。

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
