# 快速开始

一个项目目录进去，一个安装包出来。不用先装什么，也不用编译。构建器就是个下载下来就能跑的
exe。它生成的安装包在用户机器上运行时不依赖你自己带的运行时，因为真正干活的那个程序已经打
进包里了。

## 1. 下载构建器

每次发布都会把构建需要的可执行文件发出来，每个文件带一行摘要：

| 文件 | 是什么 |
| --- | --- |
| `nano-installer-native-x64.exe` | 命令行构建器 |
| `nano-installer-gui-x64.exe` | 可视化构建器，Windows 10 及以上 |
| `lzma-stub-native.exe` | 应用文件是 7z 时安装包里带的运行时 |
| `zlib-stub-native.exe` | 应用文件是 zip 时安装包里带的运行时 |
| `uninst-stub-native.exe` | 安装包里带的卸载程序 |
| `SHA256SUMS.txt` | 上面每个文件一行的摘要 |

到[最新发布](https://github.com/nick2781/nano-installer/releases/latest)把它们取下来，放在同一个
目录里；构建器会先在自己旁边找这三个运行时，也会看旁边的 `stubs` 子目录，想指到别处就用
`--stubs <目录>`，或者设 `NANO_INSTALLER_NATIVE_STUB_DIR` 环境变量。

校验下载只要一条命令，它比对的是 `SHA256SUMS.txt` 里那一行：

```powershell
certutil -hashfile nano-installer-native-x64.exe SHA256
```

发布出来的东西目前都没有代码签名，所以第一次运行时 Windows 可能提示「未知发布者」；
[当前生产状态](PRODUCTION_STATUS.md)写着它影响什么、不影响什么。

## 2. 让它对着项目目录跑

项目目录里放着 `installer_config.json`、页面布局 XML、每种语言一个 JSON 文件、你的素材，以及
应用文件——也就是打包进安装包的那份归档，zip 或 7z 都行。项目目录里能放哪些文件，见
[配置参考](CONFIG_REFERENCE.md)和[页面布局](XML_LAYOUT_GUIDE.md)，而仓库里的 `examples/TapTap`
就是一个完整的项目目录。

```powershell
.\nano-installer-native-x64.exe build --project C:\path\to\my-project
```

生成的安装包放在项目的 `dist/<output.installer_name>` 里；加上 `--msi dist\MyProduct.msi`，还能
同时拿到企业按 MSI 分发的那个包，它能做什么，见
[构建与发布](BUILD_AND_RELEASE.md#安装包外的-msi)。

想从示例开始，就把仓库克隆下来，把 `examples\TapTap` 当成项目目录交给构建器。示例的图片和
`tools/` 下的压缩工具都存在 Git LFS 里，直接下载源码 ZIP 只会拿到指针文件；至于应用文件
`examples\TapTap\payload\app.7z`，仓库里根本没放，构建前你得自己补一个 7z 归档进去。

喜欢点界面的话，从同一次发布里取 `nano-installer-gui-x64.exe`，打开项目目录后点
**Build setup**：GUI 走的是同一套引擎，生成的安装包也完全一样，详见[可视化构建](GUI.md)。

## 3. 在虚拟机中试用

把安装包复制到一台干净的虚拟机里。示例打开了 `install.require_admin`，默认安装路径也落在
`Program Files` 下，所以双击之后 Windows 会先弹一个管理员权限提示（UAC）；你确认了，安装包才
会带着需要的权限继续运行。

可以逐条验证：

- 背景、Logo、标语和按钮图片都能正常显示，透明通道也对。
- 窗口没有系统标题栏，但按住空白处就能拖动，最小化和关闭也都能用。
- 点安装会解压应用文件，写入文件和快捷方式，还会在 Windows「程序和功能」里留下一条卸载
  记录，过程中有进度显示。完成页可以直接启动刚装好的程序。
- 再装一次走一遍升级路径，然后用卸载条目验证移除和数据保留选项。
- 切换语言，看看译文是否正常显示；菜单展开后，也能用上下键、回车和 Esc 操作。
- 把鼠标移到文件夹图标、安装按钮和协议链接上，光标会变成手型，而协议链接会在浏览器里打开
  配置好的页面。
- 点关闭按钮，窗口内会弹出确认框，皮肤和安装界面是同一套，确认之后才会退出。
- 在路径框里按住左键拖动，或者双击，都能选中文字；`Ctrl+C`/`Ctrl+V` 复制粘贴，`Ctrl+Z`
  撤销。点旁边的文件夹图标能选目录，选中的路径会写回输入框。输入法也能正常组合中文，
  候选窗照常弹出。
- 把安装路径改到 `Program Files` 下，也能写进去，因为启动时你已经确认过管理员权限提示。
- 卸载完成后，安装目录会立刻消失；但你要是往目录里放过自己的文件，这个目录就会留下。

想换别的安装目录，可以直接点路径输入框旁的文件夹图标选一个；也可以改掉
`examples/TapTap/installer_config.json` 里的 `install.default_path`，再重新生成安装包。你在
界面上选的目录只在本次运行中覆盖配置的默认值，旁边显示的可用空间数值也会跟着变。

不要在普通工作站上运行示例的安装：它会写入文件和注册表。

## 想改工具本身

上面这些都不需要编译器；从源码构建工具是贡献者做的事：[构建与发布](BUILD_AND_RELEASE.md)讲了
`scripts\build.ps1`，它会在 `target/release/` 下产出同样的五个可执行文件，
[贡献指南](https://github.com/nick2781/nano-installer/blob/main/CONTRIBUTING.md)则讲了怎么准备
一份工作副本，以及改动要过的检查。
