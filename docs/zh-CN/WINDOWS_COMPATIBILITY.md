# Windows 兼容性

你发布出去的每个文件都针对 `x86_64-win7-windows-msvc` 构建，最低支持 Windows 7 SP1 x64。只有这一套
构建，没有 32 位或 ANSI 变体要跟着维护。

可视化构建器（`nano-installer-gui-x64.exe`）用的是 eframe/egui，所以需要 Windows 10 x64 及以上。
它只负责生成安装包，生成的安装包仍然能在 Windows 7 SP1 上运行。

## 运行时依赖什么

- Win32 Unicode API：窗口、控件与文字
- WIC 解码 PNG，GDI 做透明混合绘制
- Windows 7 SP1 就已存在的注册表、shell 与进程 API：注册表键的 32/64 位视图用
  `KEY_WOW64_32KEY` 与 `KEY_WOW64_64KEY` 读，命令输出按 ANSI 代码页解开用 `MultiByteToWideChar`，
  两者都自 Windows XP 起就有
- IMM32：管文本框的输入法组合窗与候选窗，Windows 7 自带
- WinHTTP 取回工程声明要下载的依赖，https 时打开 TLS 1.1/1.2（Windows 7 SP1 上还需要机器自己的
  schannel 支持 TLS 1.2，即 KB3140245 及后续更新；缺了就报错，不会退回明文）

正式构建使用固定的 `nightly-2025-11-08` 工具链、`rust-src`、
`-Z build-std=std,panic_abort`、静态链接 CRT 与 `panic=abort`。

## 兼容性如何检查

每次构建都会过一遍 PE 导入表，构建器、三个运行时和每个生成出来的安装包都在内：一旦出现被禁用的
Windows 8 或 Windows 10 API，构建就失败。这样能证明文件在静态导入层面不依赖更新的系统 API。

静态检查证明不了安装包真的能用。要正式声明支持 Windows 7 SP1 x64，你还得在干净机器上把 PNG
解码、文字渲染、鼠标输入、窗口行为、解压、安装与卸载整套验证跑一遍。目标机器建议安装 KB3033929
更新（SHA-2 代码签名支持）。
