# FAQ

## 当前支持哪些系统？

正式支持 Windows 10/11 x64。Win7 SP1 x64 已有专用构建、PE 兼容改写和导入审计，
但在真实 Win7 SP1 VM 完成安装/升级/卸载验收前仍属于兼容构建，详见
[Windows 兼容性](WINDOWS_COMPATIBILITY.md)。不提供任何 Windows x86 版本。

## 为什么不是 Win7+？

主要约束不是业务代码里的版本判断，而是标准 `x86_64-pc-windows-msvc` 目标的
Windows 10 基线，以及 `eframe/windows-sys` 的导入表。项目现在使用 Rust 的
`x86_64-win7-windows-msvc` target 自建标准库，并对 `CoTaskMemFree` 做受校验的 Win7
导入兼容处理。

## 为什么 stub 仍是几 MB，而 NSIS 只有几百 KB？

NSIS stub 是高度专用的原生 C/C++ runtime 和字节码解释器。nano-installer 的 stub 则包含
`eframe/egui`、`winit/glutin`、OpenGL 后端、图片解码、XML 布局、Rhai 脚本、日志与安装
事务逻辑；`lzma-x64.exe` 还内嵌 `7za.exe`。即使启用 LTO、strip、`panic=abort`、移除默认
字体和 stub 图标，这些通用能力仍是主体。

按压缩后端拆分后，当前标准 release 约为：LZMA 6.95 MiB、zlib 4.83 MiB、uninstaller
4.80 MiB。要进入 NSIS 的几百 KB 区间，不能只继续调编译参数，必须把现有 GUI/XML/Rhai
运行时换成原生 Win32 小型 runtime，或改成几百 KB bootstrap 加外置/按需下载的共享运行时。

## 修改什么需要重编 stub？

修改 `installer/**` 后，先重编 `nano-installer-lzma`、`nano-installer-zlib`、`uninst` 和
`nano-installer-cli`，再重打 setup。只改产品资源时可以使用
`scripts/build.ps1 -SkipStubs`。

## 产品项目必须放在本仓库吗？

不必。CLI 接受任意 `--project` 路径。把 CLI 和所选 release stubs 放在同一工具目录，或设置
`NANO_INSTALLER_STUB_DIR` 即可从独立产品仓库构建。

## `1x/@2x` 如何工作？

XML 引用普通资源路径。运行时根据 DPI 阈值选择同目录的 `@2x` 文件，缺失时回退到
普通资源。资源 lint 会检查尺寸和配对问题。

## 产品逻辑放在哪里？

通用能力放 JSON，界面和默认状态放 XML，产品专用副作用放 Rhai。详见
[配置与脚本边界](CONFIG_VS_SCRIPT.md)。

## 可以直接用于外部生产发布吗？

构建器已经支持通过 `--sign-script` 先签卸载器、再签最终 setup。正式发布仍需配置企业
证书或签名服务，并完成升级、卸载、静默返回码和目标 OS 矩阵验收。
