# 快速开始

下面用一个示例项目生成安装包，并在虚拟机中运行它。

## 1. 准备环境

- Windows x64，已装 MSVC 与 Windows SDK（Visual Studio 2022 Build Tools）
- Rust，使用本仓库固定的工具链，并包含 `rust-src` 组件

```powershell
rustup toolchain install nightly-2025-11-08 --component rust-src,rustfmt,clippy
```

## 2. 构建工具

在仓库根目录执行：

```powershell
.\scripts\build.ps1
```

脚本会构建构建器、两个运行时和卸载运行时，用真实 ZIP 与 7z 归档验证解包是否正确，并对每个
产物做 Windows 7 基线审计。结果位于 `target/release/`。

## 3. 生成示例安装包

```powershell
.\scripts\build.ps1 -Project examples\TapTap
```

示例项目输出 `examples/TapTap/dist/TapTap_Setup.exe`。它的 payload
`examples/TapTap/payload/app.7z` 未存入仓库，构建前需要自行放入一个 7z 归档。

更习惯可视化操作？运行 `target/release/nano-installer-gui-x64.exe`，打开项目目录后点
**Build setup**。GUI 驱动的是同一套构建引擎，产物完全一致。详见[可视化构建](GUI.md)。

## 4. 在虚拟机中试用

把安装包复制到一台干净的虚拟机。示例打开了 `install.require_admin`，默认又装到 `Program Files`
下，因此双击后 Windows 会先弹出提权确认框。确认后安装包即以所需权限运行。

现在可以验证：

- 背景、Logo、标语和按钮图片正常显示，透明通道正确。
- 窗口无系统标题栏，可从空白处拖动，最小化和关闭可用。
- 点击安装会解压 payload、写入文件与快捷方式并注册卸载项；过程中显示进度，完成页可直接启动
  刚装好的程序。
- 再装一次可以体验升级路径；用卸载入口可以验证移除与数据保留选项。
- 切换语言，确认译文正常显示；展开的菜单也可以用上下键、回车和 Esc 操作。
- 把鼠标移到文件夹图标、安装按钮和协议链接上：光标会变成手型，协议链接会在浏览器里打开
  配置好的页面。
- 点击关闭按钮：窗口关闭前会弹出 `close_confirm_message` 中的本地化确认问题。
- 在路径框里按住左键拖动或双击可以选中文字，`Ctrl+C`/`Ctrl+V` 复制粘贴，`Ctrl+Z` 撤销；点旁边
  的文件夹图标可以选目录，选中的路径会写回输入框；输入法可以正常组合中文并显示候选。
- 把安装路径改到 `Program Files` 下：能写进去，靠的就是启动时确认的那次提权。
- 卸载完成后安装目录会立即消失；如果你往目录里放过自己的文件，目录会保留。

想测试其他安装目录，可以直接点路径输入框旁的文件夹图标选择目录，也可以修改
`examples/TapTap/installer_config.json` 中的 `install.default_path` 后重新生成安装包。选中的
目录会在本次运行中替换配置的默认值，旁边的可用空间读数也会随之更新。

不要在普通工作站上执行示例的安装动作：它会写入文件和注册表。
