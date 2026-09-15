# 测试计划

## 自动检查

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
.\scripts\build.ps1 -Project examples\TapTap
```

Native 单元测试覆盖 bundle roundtrip、payload 文件进入 bundle、按钮命中、临时目录部署、
manifest 写入、已有目录拒绝覆盖、升级与旧文件清理、失败回滚，以及卸载时的快捷方式
与用户数据清理规则。
构建脚本审计 builder、三个 stubs、setup 和内嵌 uninstaller 的 PE imports。受限本机环境
不允许写 HKCU，`registers_and_cleans_up_scoped_uninstall_key` 默认忽略；在隔离 VM 内显式执行。

## 当前手动检查

启动 `examples/TapTap/dist/TapTap_Setup.exe`，检查：

- 客户区 720×450，无系统标题栏。
- 背景、Logo、tagline、按钮图片可见且透明通道正确。
- `Install Now`/对应 locale 文案和版本号可见。
- 顶部空白区域可拖动。
- 最小化与关闭按钮响应。
- 中文、英文、俄文不乱码。

## Win7 SP1 VM 门禁

在正式声明支持前，必须在干净 Win7 SP1 x64 VM 验证 WIC PNG、GDI 字体、鼠标输入和窗口
操作。仅在隔离 VM 用新目录测试 ZIP/7z 解压、manifest、卸载注册表和卸载按钮；随后检查
安装文件、用户自建文件的保留、失败回滚和卸载器重启后清理。不要在日常工作站测试 TapTap
安装动作。默认路径在 `Program Files` 时手动以管理员身份启动 setup；当前不会自动申请 UAC。
要验证用户可写的新目录，先在示例配置中修改只读的 `install.default_path` 并重新打包 setup。
UAC、升级、静默模式和完整回滚仍需补充测试。

## 当前不能通过的生产用例

- 执行项目 Rhai 脚本。
- 页面切换与进度显示。
- 卸载器即时自删除（当前安排在重启时删除）。
- Authenticode 签名链。

升级、失败回滚、快捷方式、自启动和数据保留选项已实现，但尚未在真实 Win7 VM 中验收。
