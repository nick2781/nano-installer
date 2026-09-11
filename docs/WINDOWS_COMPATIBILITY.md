# Windows 兼容性

## 支持状态

| 平台 | 状态 |
| --- | --- |
| Windows 10/11 x64 | 正式构建和测试平台 |
| Windows 7 SP1 x64 | 兼容构建与 PE 导入审计已建立，等待 VM 验收 |
| 所有 Windows x86 / Windows 7 RTM | 不支持 |
| VxKex | 不作为支持方案 |

Win7 产物使用 Rust 的 `x86_64-win7-windows-msvc` Tier 3 target。Tier 3 意味着项目需要
自行维护工具链、构建和运行测试，不能沿用 Tier 1 的官方支持承诺。

## 构建

安装固定的 nightly 与标准库源码：

```powershell
rustup toolchain install nightly-2025-11-08 --component rust-src
```

构建并审计示例：

```powershell
.\scripts\build-win7.ps1 `
  -Project examples\TapTap `
  -Toolchain nightly-2025-11-08
```

需要签名时增加：

```powershell
-SignScript scripts\sign.ps1
```

脚本执行以下流程：

1. 使用 `build-std=std,panic_abort` 构建 Win7 installer/uninstaller stubs。
2. 使用标准 host target 构建 CLI。
3. 让 CLI 使用 Win7 stubs 生成 setup。
4. 审计三个预编译 stub、资源化后的卸载器和最终 setup 的 PE import table。

`.cargo/config.toml` 为 Win7 target 启用静态 CRT，release profile 使用
`panic=abort`。因此 setup 启动前不依赖 VC++ Redistributable 或 Universal CRT。

## 已处理的兼容性阻塞

- DPI 查询使用 Win7 已存在的 GDI API，不再静态导入 Win10 DPI API。
- 文件夹选择器直接使用 Vista+ 的 `IFileDialog`，不再依赖 `rfd` 和旧
  `windows-targets 0.48`。
- Win7 标准库 target 不导入 `WaitOnAddress` 或
  `GetSystemTimePreciseAsFileTime`。
- 静态 CRT 避免 `VCRUNTIME140.dll` 和 `api-ms-win-crt-*` 启动依赖。
- `scripts/audit_win7_imports.ps1` 会阻止这些依赖以及 `combase.dll` 回归。

`eframe 0.33` 使用的 `windows-sys 0.61` 会把 `CoTaskMemFree` 链接到 Win8+ 的
`combase.dll`，而该函数在 Win7 位于 `ole32.dll`。Win7 构建脚本会在签名和资源注入前
运行 `retarget_win7_com_import.ps1`；脚本只有在 `combase.dll` 唯一导入恰好是
`CoTaskMemFree` 时才修改 PE import descriptor，否则立即失败。最终导入审计仍禁止
`combase.dll`。

`DwmSetWindowAttribute` 本身从 Vista 存在；Win11 圆角属性在旧系统上返回失败，调用方
忽略该非关键错误。

当前三个 Win7 x64 stub、资源化卸载器和 TapTap setup 已在本机构建并通过 PE import
审计。该结论证明装载器不会静态请求已知的 Win8/10 API 或 DLL，但不能代替 Win7 VM
上的图形驱动、UAC 和完整安装流程测试。

## 为什么不降级 eframe

`eframe 0.33` 需要新 Rust，但 `winit 0.30` 的 Windows 后端仍保留 Win7 路径，并对
较新的 DPI API 做动态加载。实际 POC 已经编译完整的 `eframe`、`winit`、`glutin` 和
项目 runtime，因此当前方案选择自建 Win7 标准库，而不是冻结在旧 Rust 和旧 GUI 栈。

## VM 验收门槛

正式声明 Win7 支持前，必须在不安装 VxKex 的 Win7 SP1 x64 VM 完成：

- 干净系统启动，确认无缺失 DLL 或入口点。
- OpenGL 渲染、目录选择器、DPI、中文字体和多语言。
- UAC 提权、首装、覆盖安装、上一生产版本升级。
- 静默安装、静默卸载、失败返回码和回滚。
- 已签名 setup 与解压后 uninstaller 的 Authenticode 验证。

面向外部用户的 Win7 发布建议要求 SHA-2 支持更新 KB3033929。GitHub Hosted Runner
只能负责编译和 PE 审计，真实运行门禁需要自托管 Win7 VM。
