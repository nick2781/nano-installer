<h1 align="center"><img src="../assets/nano-technology.png" width="48" height="48" alt="Nano Installer icon"> GUI 构建工具</h1>

GUI 是 CLI 的 Windows 10+ 可视化前端。它不启动 CLI 子进程，
而是与 CLI 一样直接调用 `nano-installer-core` 的项目检查和构建 API。界面按 MakeNSISW
这类传统构建工作台组织：顶部只保留标准菜单，左侧是项目检查结果，主区域是构建日志或
参数页，底部是唯一的构建操作区和持续可见的状态栏。

左侧项目侧栏按用途列出配置校验、版本与安装包默认语言、打包输入（payload、匹配 stub、
安装/卸载图标与卸载文件名）以及默认安装目录。长路径显示盘符和末端目录，悬停查看完整
路径；需要编辑或复制完整路径时使用参数页。侧栏的“上次校验通过”同时显示检查时间，
只反映当时的项目检查；磁盘文件变化后需刷新，构建任务的成功/失败另由构建工作区和底部
状态栏表示。DPI 素材警告计数只针对 PNG 的 1x/2x 成对检查，不能当作完整生产验收结果。
字段和值沿各自左边界排列，窄侧栏会截断过长的值，悬停仍可看完整内容。

## 启动

```powershell
.\target\release\nano-installer-gui-x64.exe
```

GUI 启动后显示空项目的构建工作区，不会自动打开任何示例：

1. 使用 `Open project` 选择包含 `installer_config.json` 的项目目录；TapTap 验证项目也必须
   由用户自行选择。
2. 使用 `Refresh` 重新读取 JSON、XML、资源和 payload。刷新不会覆盖同一项目已经自定义的
   输出路径。
3. 在 `Parameters` 页设置项目目录、输出 EXE 和可选的 native stub 目录。`Reset output`
   恢复 `dist/<output.installer_name>`，`Use automatic stub search` 恢复 core 的自动查找。
   路径输入与只读命令预览随工作区宽度伸缩，切换视图使用顶部固定标签或视图菜单；参数页
   不再有重复的“返回构建工作区”按钮。
4. 使用 `Build setup` 异步生成安装包。构建会在 worker 中再次执行 core 校验，避免只依赖 GUI
   当前显示的摘要。
5. 构建失败后，`Retry`/`Retry last build` 会用上一次失败请求的完全相同参数重试，便于修复文件或
   stub 后继续操作。
6. 使用 `Save log...` 导出完整日志，`Open output` 在 Explorer 中选中生成的 setup。参数页的
   `After build` 复选框控制成功后是否自动定位产物。

`视图 > 界面语言` 提供 English 和简体中文。它只翻译 GUI 的操作文案；项目安装包
的多语言内容仍由项目自己的 `locales/` 和 XML 配置控制。
构建日志的工具文案始终使用英文，不随 GUI 语言切换。底层 Windows/文件系统返回的原始错误
可能仍由操作系统本地化，保留原始内容便于定位错误码和路径。

构建日志记录实际执行步骤，包括 installer/uninstaller stub 的解析路径和大小、uninstaller
在 bundle 内的名称、各资源目录的文件数量和大小、payload 格式、bundle 索引、icon、
VERSIONINFO 和 footer。当前资源直接写入自定义 bundle，不会生成中间 `skins.zip`；payload
自身保持项目提供的 ZIP 或 7z 压缩格式。
卸载器有独立的不含 payload 的 UI bundle，因此 `layouts/assets/locales/scripts` 会以
`Uninstaller bundle:` 前缀收集一次，之后再进入主 setup 一次；payload 只打包进主 setup。
`Project inspection passed` 是 GUI 打开/刷新项目时的预检查，`Validating project` 是
每次真正构建时 core 的再次检查。`Bundled runtime install handler` 只说明编入安装能力，
构建本身不会解压、部署文件或修改注册表。

构建器会从 `uninst-stub-native.exe` 生成自包含的卸载程序，写入
`output.uninstaller_icon`、项目 VERSIONINFO 和不含 payload 的卸载 UI bundle，再按
`runtime/<output.uninstaller_name>` 嵌入 setup。安装动作在临时目录使用匹配的 archive
backend 解压并检查配置的 `install.exe_name`，随后部署文件、自包含卸载器、manifest 和卸载
注册表；卸载动作只删除 manifest 记录的文件，运行中的卸载器文件安排在重启后删除。
构建日志会列出这些行为，不再报“等待安装动作部署卸载器”的警告。

日志窗口使用不可修改但可交互的多行文本控件，支持鼠标拖选、滚轮、横向滚动、`Ctrl+A`、
`Ctrl+C` 和滚动条；`复制全部` 可以不选择文本直接复制完整日志。所有日志统一使用本地日期时间
`[YYYY-MM-DD HH:mm:ss.SSS]` 前缀，保存和复制时保留时间戳。

菜单使用固定最小宽度和单行文字；中英文切换不会改变菜单几何结构，也不会把命令拆成多行。
可选中的日志窗口对 Warning 使用琥珀色、Error/失败使用红色；保存和复制仍保留原始文字。

菜单还提供 `Open config`（用系统默认程序打开 `installer_config.json`）、清空日志和退出；
所有按钮在构建期间会按状态禁用，避免同时启动两个打包任务。

GUI 与 CLI 使用相同的 stub 搜索、payload 文件头识别、bundle、icon 和 version resource
写入逻辑。GUI 的 eframe/egui/winit 依赖仅存在于 `nano-installer-gui` crate，不影响 CLI、
stubs 或 setup 的大小及 Windows 7 SP1+ 兼容性。

GUI 只编辑本次构建的参数，不直接改写 JSON/XML；项目配置仍然是唯一事实来源。压缩方式由
payload 文件签名决定（`.7z` 选择 LZMA，`.zip` 选择 Deflate），因此 GUI 不提供一个不会
生效的“压缩方式”假下拉框。参数页的命令预览等价于 CLI：

```text
nano-installer-native-x64.exe build --project <dir> [--output <exe>] [--stubs <dir>]
```
