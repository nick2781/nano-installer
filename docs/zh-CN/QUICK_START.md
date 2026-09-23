# 快速开始

你用一个示例项目生成安装包，再在虚拟机里跑一遍。

## 1. 准备环境

- Windows x64，装好 MSVC 与 Windows SDK（Visual Studio 2022 Build Tools）
- Rust，用仓库固定的工具链，并带上 `rust-src` 组件

```powershell
rustup toolchain install nightly-2025-11-08 --component rust-src,rustfmt,clippy
```

## 2. 构建工具

在仓库根目录执行：

```powershell
.\scripts\build.ps1
```

脚本会把构建器、两个运行时和卸载运行时都构建出来，再用真实的 ZIP 与 7z 归档验证解包是否正确。
最后对每个生成的文件做一遍 Windows 7 基线审计。结果都在 `target/release/`。

## 3. 生成示例安装包

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

示例项目会生成 `examples/TapTap/dist/TapTap_Setup.exe`。它的应用文件（payload）放在
`examples/TapTap/payload/app.7z`，仓库里没有这个文件，构建之前你要自己放一个 7z 归档进去。

在构建器上再加一个 `--msi dist\\TapTap.msi`，就能同时拿到企业按 MSI 分发的那个包；
它做什么、怎么部署见[构建与发布](BUILD_AND_RELEASE.md#安装包外的-msi)。

你也可以运行 `target/release/nano-installer-gui-x64.exe`，打开项目目录后点 **Build setup**。
GUI 走的是同一套构建引擎，生成的安装包完全一致。详见[可视化构建](GUI.md)。

## 4. 在虚拟机中试用

把安装包复制到一台干净的虚拟机里。示例打开了 `install.require_admin`，
默认安装路径也落在 `Program Files` 下，所以双击之后 Windows 会先弹一个提权确认框。
你确认之后，安装包才会带着需要的权限继续运行。

可以逐条验证：

- 背景、Logo、标语和按钮图片都能正常显示，透明通道也对。
- 窗口没有系统标题栏，按住空白处可以拖动，最小化和关闭都能用。
- 点安装会解压 payload，写入文件和快捷方式，并注册卸载项；过程中有进度显示，
  完成页可以直接启动刚装好的程序。
- 再装一次就能走一遍升级路径；用卸载入口可以验证移除和数据保留选项。
- 切换语言，看看译文是否正常显示；菜单展开后也能用上下键、回车和 Esc 操作。
- 把鼠标移到文件夹图标、安装按钮和协议链接上，光标会变成手型，
  协议链接会在浏览器里打开配置好的页面。
- 点关闭按钮，窗口内会弹出和安装界面同一套皮肤的确认框，确认之后才会退出。
- 在路径框里按住左键拖动或双击可以选中文字，`Ctrl+C`/`Ctrl+V` 复制粘贴，`Ctrl+Z` 撤销；
  点旁边的文件夹图标能选目录，选中的路径会写回输入框；输入法可以正常组合中文并显示候选。
- 把安装路径改到 `Program Files` 下也能写进去，因为启动时已经确认过提权。
- 卸载完成后安装目录会立刻消失；如果你往目录里放过自己的文件，这个目录就会留下。

想换别的安装目录，可以直接点路径输入框旁的文件夹图标选一个，
也可以改掉 `examples/TapTap/installer_config.json` 里的 `install.default_path`，再重新生成安装包。
你在界面上选的目录只在本次运行中覆盖配置的默认值，旁边的可用空间读数也会跟着变。

不要在普通工作站上执行示例的安装动作：它会写入文件和注册表。
