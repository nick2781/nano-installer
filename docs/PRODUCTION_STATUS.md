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
- 首屏 XML 位图与 Unicode locale 文字原生渲染。
- 无边框圆角、拖动、最小化和关闭。
- 单一 Win7 SP1+ x64 release 和 PE import audit。

生产阻塞项：

1. 当前启动解析 bundle 时仍会短暂读入/复制完整 payload，随后才释放；需要改为 offset/mmap 访问。
2. 基础安装只接受不存在的新目录；升级、已有安装覆盖、取消、完整进度页和故障回滚尚未完成。
3. 卸载仅清理 manifest 内文件；运行中的 uninstaller 安排在重启时删除，未实现数据保留选项、
   快捷方式清理和进程终止。
4. 首屏 HBox/Content、Checkbox、展开面板、语言 Select、enabled-when 和路径/空间绑定已实现；
   完整嵌套布局、页面状态、任务进度、项目 Rhai 脚本和快捷方式/自启动仍未完成。
5. setup 与 uninstaller icon、Unicode version resource 已注入；双重签名尚未接入。
6. 尚未通过真实 Win7 SP1 VM 端到端验收。

完成这些门禁后，生产项目才能按照 `examples/TapTap` 的结构接入。
