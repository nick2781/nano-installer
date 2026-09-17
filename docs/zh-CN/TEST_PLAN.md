# 测试计划

## 自动检查

```powershell
cargo fmt --all -- --check
cargo test --locked --workspace
.\scripts\build.ps1 -Project examples\TapTap
```

安装包级用例会真的构建并运行安装包，所以要先把运行时构建出来：

```powershell
cargo build -p nano-installer-stub-lzma -p nano-installer-stub-zlib -p nano-installer-uninstaller
$env:NANO_INSTALLER_E2E_REQUIRE_STUBS = "1"
cargo test -p nano-installer-core --test e2e_setup
```

单元测试覆盖 bundle 往返、payload 打包、按钮命中测试、临时目录部署、manifest 写入、拒绝覆盖已有
目录、升级与旧文件清理、失败回滚，以及卸载时的快捷方式与用户数据规则。布局测试覆盖嵌套流式容器
与间距、百分比尺寸、进度条裁剪、越界页面回退，以及进度页与状态文案的绑定。脚本测试覆盖脚本部署
文件与清单、脚本失败回滚、通过 `run_tracked_uninstall` 重放清单，以及脚本漏做清理时的库回退。

构建脚本审计构建器、三个运行时、安装包与内嵌卸载程序的 PE 导入，并把安装包与内嵌卸载程序里的清单
资源读回来，核对权限级别与 DPI 行为是否与项目配置推出来的结果一致。有一项测试会写 `HKCU`，它
在受限环境中默认忽略，要跑到它得在隔离虚拟机里显式执行。

`crates/nano-installer-core/tests/e2e_setup.rs` 覆盖单元测试看不到的交界处。布局打错、payload 丢失、
资源没注入，这类缺陷都发生在两个程序之间，单元测试全绿而安装包根本装不上。所以这套用例自己写出
一份项目，用真实构建器生成安装包，再运行这个安装包与它部署出来的卸载程序。

- 安装包内容：运行时读取的尾部标记、贴在真实 PE 之后的 payload、版本与清单资源，以及 payload
  格式对应的运行时。
- 安装：payload 落到磁盘且逐字节一致、manifest 记录实际写入的内容、卸载项指向部署出来的卸载程序、
  配置的 `%TEMP%` 路径会被展开、`--dir` 优先于配置路径，未声明支持或参数写错的运行不落任何文件。
- 升级：第二次安装覆盖第一次时删掉新版 payload 不再包含的文件，同时保留 payload 不属于它的文件。
- 卸载：产品、注册项与目录一并消失；目录里还有用户自己加的文件时保留目录。

每个用例都自造 fixture，不含任何产品 payload 与第三方素材，安装到临时目录下，并注册到只属于这个
用例的注册表键，所以并行执行互不干扰，重复运行也不会冲突；即使断言失败也会自己清理。运行时 stub
缺失时用例打印跳过并计为通过，所以真正验证安装包的 job 会设置 `NANO_INSTALLER_E2E_REQUIRE_STUBS=1`，
把跳过变成失败。

## 手动检查

启动 `examples/TapTap/dist/TapTap_Setup.exe`，确认：

- 客户区 720x450，无系统标题栏。
- 背景、Logo、标语与按钮图片可见，透明通道正确。
- 安装按钮与版本文案使用预期语言。
- 顶部空白区域可拖动窗口。
- 最小化与关闭可用。
- 中文、英文、俄文文字不乱码。
- 路径框可以拖选、双击选词、复制粘贴与撤销，文件夹图标能选目录并写回输入框。
- 在路径框里用中文输入法打字，组合窗停在光标处、候选框紧贴光标下方。
- 卸载结束后安装目录立即消失，用户自己放进去的文件仍保留。

## Windows 7 SP1 门禁

正式声明支持 Windows 7 之前，你要在干净的 Windows 7 SP1 x64 虚拟机上验证 WIC PNG 解码、GDI
文字、鼠标输入与窗口行为。再在隔离虚拟机里用一个新目录测试 ZIP 与 7z 解压、manifest、卸载项与
卸载按钮，然后检查装好的文件和用户自建的文件有没有保留、失败时能不能回滚，以及卸载完成后安装目录
是不是马上消失、临时目录里的清理副本有没有退出。

不要在日常工作站上测试 TapTap 的安装动作。示例打开了 `install.require_admin`，正确构建出的安装包
会在窗口出现前弹出 UAC 确认框；拒绝之后机器必须没有任何改动。要验证「不申请提权」的构建，请清空
`install.require_admin`，确认同一个安装包启动时不再弹框。要验证用户可写目录，请修改示例中只读的
`install.default_path` 并重新打包。

## 目前无法通过的生产用例

- Authenticode 签名链。

升级、失败回滚、快捷方式、自启动、数据保留选项、页面切换与进度显示都已经做好，只是还没在真实
Windows 7 虚拟机上验收。
