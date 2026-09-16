# 更新日志

本文件记录 nano-installer 的版本变更历史。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。
版本号使用 CalVer（`YYYY.M.D`），tag 形如 `v2026.9.16`。

## [2026.9.20]

安装窗口现在跟着显示器走。把窗口拖到另一块缩放比例不同的屏幕上，界面的字号、间距和图片会按那块
屏幕重新排一遍，而不是被 Windows 拉伸成一张发虚的位图。以前只有 Windows 7/8.1 能读的那种写法，
现在换成了 Windows 10 起的逐显示器感知。

### 改进

- 安装界面支持逐显示器 DPI：窗口在哪块屏幕上，就按哪块屏幕的缩放比例重新排版。高分辨率屏幕上
  文字与图标保持锐利，混用 100%、150%、200% 的多屏环境不再出现糊成一片的界面。
- 窗口被移到另一块屏幕后仍完整可见：新位置超出该屏幕可用范围时会被拉回屏幕内，比屏幕还大的
  布局则收缩到可用范围，不会露在屏幕外。
- 圆角在尺寸变化后依然是圆角：页面变高变宽时窗口形状会跟着重算，不再留下旧尺寸的直角缺口。

### 已知限制

- 安装包尚未签名，Windows SmartScreen 仍会提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。

<!-- release-notes:end -->

### 技术细节

- 清单同时声明两个元素：`dpiAware`（Windows 7/8/8.1 读）与 `dpiAwareness`（Windows 10 1607 起
  优先读）。后者取值 `PerMonitorV2, PerMonitor`：1607-1703 只认 `PerMonitor`，1703 及以后取第一个
  认识的值，因此每个受支持的版本都拿到它能提供的最清晰行为。项目关闭缩放时写 `unaware`，避免
  新版 Windows 又替它缩放。
- `DpiContext::for_dpi` 由 DPI 与 `DpiSettings { aware, threshold }` 推出布局比例与 `@2x` 选择；
  `DpiSettings` 与解析结果一起存进 `RuntimeState`，因此收到 DPI 变化时可以重新推导。
- 新增 `WM_DPICHANGED` 处理：`wparam` 低字是 X 轴 DPI，据此重建 `DpiContext`，调用
  `rebuild_runtime_ui` 重新排版整页，再按 Windows 建议的矩形（`lparam` 指向的 `RECT`）或居中结果
  重新放置窗口。缩放比例未变时直接返回，不做无谓重排。
- 该消息由 Windows 同步投递，运行时的测试钩子也必须用 `SendMessageW`：`PostMessageW` 会拒绝携带
  指针的消息（`0x80070487`）。
- 新增 `clamped_bounds`（把 `left`/`top` 夹进工作区，窗口大于工作区时收缩）与 `place_window`
  （`SetWindowPos` + 重算圆角区域）。`center_window` 改为复用两者，窗口形状不再只在创建时设置一次。
- `CreateRectRgn` 加入导入表，用于项目声明直角时清掉上一块区域。
- 验证钩子 `NANO_INSTALLER_TEST_DPI_CHANGE`（配合可选的 `NANO_INSTALLER_TEST_DPI_RECT`）让单显示器
  机器也能驱动真实处理函数，与既有的 `NANO_INSTALLER_TEST_DPI` 同一风格，未设置时完全不生效。
- `scripts/audit_application_manifest.ps1` 现在校验 `dpiAwareness`：缺失、模式名非法、关闭缩放却
  仍声明、或第一个模式不是 `PerMonitorV2` 都会导致构建失败。
- 新增测试：`a_display_scales_the_layout_by_its_own_dpi`（96/120/144/192/384 的换算与 `@2x` 选择、
  关闭缩放时保持基准）、`a_placed_window_is_pulled_back_inside_its_work_area`（建议位置在范围内时
  保留、越界时拉回、负坐标副屏、大于工作区时收缩）。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`（83 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）。
- 清单审计：`examples/TapTap/dist/TapTap_Setup.exe` 与内嵌卸载程序均为
  `level=requireAdministrator, dpiAware=true, dpiAwareness=PerMonitorV2, PerMonitor`。
- 实测 DPI 切换：DPI 由 192 变为 384 时窗口从 `720,462,2160,1362`（1440x900）变为
  `0,12,2880,1812`（2880x1800，工作区 `0,0,2880,1824`），即整页按新比例重新排版；
  带建议矩形 `100,50` 时窗口落在 `0,24,2880,1824`，仍完整位于工作区内。
- `PrintWindow` 截图确认 2880x1800 下界面按 2 倍比例重排，文字与图标清晰。
- 构建审计恢复全套通过：Win7 PE 导入审计、ZIP/7z backend smoke test、脚本编码审计。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。

## [2026.9.19]

退出安装的确认框终于和安装界面是一套皮肤了，不再突然跳出一个系统灰框。安装窗口也老老实实待在
屏幕正中间：不管你的显示器缩放调到多少、副屏摆在哪一侧，它都不会再溜到左上角去。

### 修复

- 确认退出安装时，弹出的是与安装界面同一套皮肤、同一套文案的确认框，不再出现系统默认的灰色
  弹窗；确认框画在窗口内部，因此不会被其它程序盖住或甩到后面。
- 提示信息（例如某个步骤出错）同样走这套皮肤，不再打断成系统弹窗。
- 安装窗口始终居中显示：按当前显示器的可用区域（已避开任务栏）计算位置，多显示器与高倍缩放下
  都停在正中间，布局比屏幕大时也会完整落在屏幕内，不会跑到左上角。
- 拖动无标题栏的窗口不再跳动：从空白处按下标题栏时按屏幕坐标发消息，窗口随鼠标平稳移动。
- 对话框里的问句文字不再丢失：一行文字按百分比宽度排版时，跨轴高度会被正确测量，文字不会再
  被压成零高度。

### 已知限制

- 安装包尚未签名，Windows SmartScreen 仍会提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。

<!-- release-notes:end -->

### 技术细节

- 结束系统 MessageBox：`confirm_close()` 原先调用 `MessageBoxW(None, ...)`，弹出的无主 `#32770`
  与安装窗口同为顶层窗口，z 序会错乱，这正是「安装界面看起来永远在最上面」的来源。改为自绘
  对话框后，运行期不再创建任何 `#32770`。
- 对话框即布局：新增 `DialogState { kind, message, accept_label, dismiss_label }`、
  `DialogKind::{CloseConfirm, Notice}`（`offers_dismiss()` 让同一份布局既能提问也能提示）、
  `DialogUi { actions, text_hits, hover_regions }`，以及 `WindowAction::{DialogOk, DialogCancel}`
  与 `dialog_ok`/`dialog_cancel` 两个 action。
- 布局新能力：`value-source="dialog:message|accept|dismiss"` 读取当次提问，`visible-with="dismiss"`
  只在对话框提供第二种答案时绘制控件。
- 渲染：从 `load_layout` 抽出 `render_layout_content`（页面与对话框共用），新增
  `render_dialog_overlay`（按 `config["ui"]["dialog_layout"]` 加载，缺省
  `DEFAULT_DIALOG_LAYOUT = "layouts/msgBox.xml"`）、`translate_layout` 与遮罩
  `DIALOG_SCRIM = "66000000"`。项目没有该布局时优雅退化：不弹确认框，点关闭直接退出。
- 模态：`window_action_at`/`hover_control_at`/`text_input_at` 在对话框打开时只认对话框区域；
  `dialog_is_open()` 让 `WM_KEYDOWN` 优先把 `VK_RETURN` 当确认、`VK_ESCAPE` 当取消，排在
  「Esc 关闭窗口」分支之前。
- 窗口居中：新增 `center_window`/`monitor_work_area`/`centered_bounds`。位置取自
  `MonitorFromWindow` + `GetMonitorInfoW(rcWork)`，尺寸夹取到工作区内，取代原先的
  `GetSystemMetrics(SM_CXSCREEN/CYSCREEN)`（主屏像素、忽略多显示器/DPI/任务栏，缩放后窗口
  达到或超过屏幕时会算出 `(1440-1440)/2=0`、`(960-900)/2=30`，也就是左上角）。
  `RuntimeState.window_size` 做守卫，只在页面尺寸变化时重新居中，用户手动移动过的窗口不会被拽回。
- 拖动标题栏：`WM_LBUTTONDOWN` 转发的 `WM_NCLBUTTONDOWN` 需要屏幕坐标，原先直接传客户区坐标，
  窗口会按指针在窗口内的位置跳走；现改用 `ClientToScreen` 转换。
- 布局引擎：`container_intrinsic_size` 在测量容器跨轴尺寸时，对子元素调用了
  `cross_size_for_node(..., axis.cross_measure(), ...)` 并沿错误的轴累加 margin，等于用宽度回答
  高度的问题，`width="100%"` 的行因此被算成零高度、整行被丢弃。现拆出
  `container_intrinsic_content`（不含自身内边距的子树尺寸）与 `outer_extent_along`（子元素沿被测量
  轴的显式尺寸 → 自身内容的固有尺寸 → 0，再加 padding/margin），跨轴测量按正确方向进行。
- 导入表：新增 `ClientToScreen`、`MonitorFromWindow`、`GetMonitorInfoW`、`MONITORINFO`、
  `MONITOR_DEFAULTTONEAREST`、`SetWindowPos`、`HWND_TOP`、`SWP_NOZORDER`/`SWP_NOACTIVATE`、
  `VK_RETURN`、`VK_ESCAPE`；移除 `GetSystemMetrics` 与 `IDYES`/`MB_YESNO`/`MB_ICONQUESTION`。
- 示例 `examples/TapTap/layouts/msgBox.xml` 重写为圆角卡片布局（信息文案 + 「继续安装」/「退出安装」），
  `examples/TapTap/installer_config.json` 的 `ui` 段显式声明 `"dialog_layout": "layouts/msgBox.xml"`。
- 文档：`XML_LAYOUT_GUIDE`（新增「对话框」一节与 `dialog_ok`/`dialog_cancel`）、
  `CONFIG_REFERENCE`（新增 `ui.dialog_layout`）、`ARCHITECTURE`（提问与提示的渲染）、
  `PRODUCTION_STATUS` 与 `QUICK_START` 同步为皮肤确认框，中英双语。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`。
- `cargo test --locked --workspace`（81 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）；
  新增回归测试覆盖对话框居中与模态、按钮动作、提示框隐藏次按钮、无对话框布局的退化、
  `centered_bounds` 的夹取，以及跨轴容器尺寸，并以示例自带的 `msgBox.xml` 端到端断言问句与两个按钮
  都被真实排版。
- 实测窗口不再是置顶：`ex=0x40000`（仅 `WS_EX_APPWINDOW`，无 `WS_EX_TOPMOST`），z 序低于
  `Shell_TrayWnd`，普通窗口可以排到它上面。
- 实测高 DPI：`NANO_INSTALLER_TEST_DPI=384` 时窗口为 `0,12,2880,1812`（2880x1800，工作区
  `0,0,2880,1824`），旧算术在同场景下得到 `0,30` 的贴左上角结果。
- 实测皮肤对话框：点击关闭按钮后枚举窗口只剩 `NanoInstallerNativeRuntime`，没有任何 `#32770`；
  `PrintWindow` 截图确认问句与「继续安装」/「退出安装」按皮肤渲染且居中。
- 实测键盘：对话框打开时 `VK_ESCAPE` 只收起对话框（窗口仍在），`VK_RETURN` 确认后销毁窗口。
- `scripts/build.ps1 -Project examples\TapTap` 全量构建、ZIP/7z backend smoke test、Win7 PE 导入
  审计与清单审计；文档链接检查 38 个文件、140 条相对链接、0 条失效。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。
- 清单申请的是旧版 `dpiAware`，尚未申请逐显示器感知。

## [2026.9.18]

安装包现在会自己申请该有的权限，不用再让用户右键「以管理员身份运行」了。要装到 `Program Files`
的产品，在配置里打开一个开关，双击时 Windows 就会正常弹出确认框；只写到用户目录的产品什么都不用
改，仍然是安静启动、不打扰用户。

输入中文也不再别扭：在路径框里打字时，输入法组合窗停在光标处，候选框就贴在光标下方，跟在记事本里
打字的手感一致。

构建时顺手把译文查一遍：哪个语言少了页面文案、哪个语言声明了却没有语言文件，都会出现在可视化
构建工具的「构建警告」里，不用等用户来反馈「这里显示的是英文」。

### 新增

- 要装到 `Program Files` 的产品可以在配置里申请管理员权限：安装包启动时由 Windows 弹出提权确认，
  用户不必再右键「以管理员身份运行」。默认不申请，普通双击即可运行；内嵌的卸载程序申请同一级别，
  卸载项仍然能撤销一次提权安装。
- 高分屏下窗口由程序自己缩放，文字和图片保持清晰，而不是被系统整体拉伸。
- 安装路径输入框接入输入法组合窗：组合中的文字停在光标处，候选列表紧贴光标下方，中日韩输入与
  常见 Windows 输入框一致。
- 构建时检查译文：哪个语言少了页面文案（会列出具体键名）、哪个语言在配置里声明了却没有语言文件，
  都会出现在可视化构建工具的「构建警告」里。

### 已知限制

- 安装包尚未签名，Windows SmartScreen 仍会提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。

<!-- release-notes:end -->

### 技术细节

- 修复 `scripts/changelog_notes.ps1` 的发布正文尾注乱码：该文件含一行中文，而 Windows
  PowerShell 会用 ANSI 代码页解码没有 BOM 的脚本，因此开发机上看不出问题、英文 runner 上却发出
  乱码。脚本改为保存为带 UTF-8 BOM，并新增 `scripts/audit_script_encoding.ps1` 在每次构建开始时
  拦截「含非 ASCII 却无 BOM」的脚本：`scripts/build.ps1` 开头与 CI 都会执行它。
- 新增 `scripts/audit_application_manifest.ps1`：从生成的安装包与内嵌卸载程序中读回 `RT_MANIFEST`
  资源，校验 `requestedExecutionLevel` 与 `dpiAware` 是否和项目配置推导出的结果一致，由
  `scripts/build.ps1` 与 `scripts/audit_embedded_uninstaller.ps1` 调用；`install.require_admin`
  悄悄失效时会直接让构建失败。原始 stub 按设计不含清单，由构建器按项目注入。
- 配置项：`install.require_admin`（默认 `false`）决定 `requestedExecutionLevel` 是
  `requireAdministrator` 还是 `asInvoker`；`ui.dpi_aware`（默认 `true`）决定清单里的
  `<dpiAware>`；`localization.supported_locales` 参与构建期语言校验。
- 新增 `crates/nano-installer-core/src/manifest.rs`：`ManifestSettings::from_config` 读取
  `install.require_admin`（默认 `false`）与 `ui.dpi_aware`（默认 `true`），`xml()` 逐行拼接
  清单文本（不用 Rust 字符串续行，否则会吃掉缩进把两个属性粘在一起）。清单包含
  `requestedExecutionLevel`、Common-Controls 6.0 依赖与 `<dpiAware>`，并以
  `BeginUpdateResourceW`/`UpdateResourceW`/`EndUpdateResourceW` 写入 `RT_MANIFEST`(24)/ID 1，
  语言写 0（中性），这是 Windows 查找可执行文件清单的位置。
- `build` 流程在写完 `VERSIONINFO` 后写入安装包清单，内嵌卸载程序同样注入；两者都由同一份配置
  派生，进度输出会带上本次的提权与 DPI 设置。
- 输入法支持：`RuntimeUi` 增加 `caret_rect`，`focus_text_input` 与 `WM_APP_REFRESH` 调用
  `place_ime_windows`（`ImmGetContext`/`ImmSetCompositionWindow`(CFS_POINT)/
  `ImmSetCandidateWindow`(CFS_CANDIDATEPOS)/`ImmReleaseContext`）。没有输入法或取不到上下文时
  静默跳过，退回系统默认位置；worker 线程移动光标后由主线程重新定位。
- `Cargo.toml` 增加 `Win32_UI_Input_Ime` feature。
- 多语言校验：新增 `locale_scale_warnings`、`required_locale_keys`、`collect_xml_paths`，只读取
  以 `@` 开头的属性，避免把 `logo@2x.png` 当成键；只报告默认语言里存在的键，`inspect_project`
  把素材告警与语言告警合并排序去重。
- `ProjectSummary` 增加 `require_admin` 与 `dpi_aware`，可视化构建接口的侧栏据此显示管理员权限
  一行；`asset_warnings` 文案改名为 `build_warnings`（中英各一条）。
- 示例 `examples/TapTap/locales/*.json`：es/id/ja/ko/pt/th/vi/zh-TW 补齐 `taskbar_checkbox` 与
  `start_install_button`，键数统一为 82。
- 文档：README、文档首页与各语言指南改为产品口吻，把提权、输入法与语言校验从「未做」移到
  「已做」，技术页（架构、配置参考、构建发布、Windows 兼容性）保留实现细节。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`。
- `cargo test --locked --workspace`（73 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）。
- 提权机制实验：向测试镜像注入 `requireAdministrator` 后，未提权父进程调用 `CreateProcessW`
  返回 740（`ERROR_ELEVATION_REQUIRED`）；`asInvoker` 则正常启动。
- `scripts/build.ps1 -Project examples\TapTap` 全量构建、ZIP/7z backend smoke test、Win7 PE
  导入审计（含新增的 `imm32.dll`，Windows 7 自带），以及新增的清单审计。
- 复现并验证修复：同一份脚本按 cp1252 解码时，加 BOM 前尾注变成乱码、加 BOM 后完整可读；
  `v2026.9.18` 的 release 正文尾注已重新生成并确认正常。
- 新增 `scripts/verify_release_notes.ps1`：用发布流程相同的方式（含 `GITHUB_REPOSITORY`）跑一遍
  正文生成器，再与生成器源码里解码后的尾注逐字比对。它只在 ANSI 代码页不是 UTF-8 的机器上才能
  失败，因此接在 CI 上；本机把 BOM 去掉做负向验证时，`audit_script_encoding.ps1` 如期以非零退出
  拦截，而该端到端校验在 UTF-8 环境下不报错。
- 用 Python 直读 PE 资源复核：`TapTap_Setup.exe` 与内嵌 `uninst.exe` 均声明
  `requireAdministrator` 与 `<dpiAware>true`，原始 stub 无清单；把期望级别故意写成 `asInvoker`
  时审计如期失败。
- 文档链接检查：38 个文件、138 条相对链接、0 条失效。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。
- 清单申请的是旧版 `dpiAware`，尚未申请逐显示器感知。

## [2026.9.17]

安装界面更像一个正经的安装程序了：路径框可以像记事本一样选中、复制、粘贴和撤销，装完之后
安装目录会立刻消失，不会留下一个「只有卸载程序」的空文件夹。

### 新增

- 安装路径输入框支持完整的文本编辑：鼠标拖选、双击选词或 `Ctrl+A` 全选，选中部分会高亮显示；`Ctrl+C`/`Ctrl+X`/`Ctrl+V` 复制、剪切与粘贴，`Ctrl+Z` 撤销、`Ctrl+Y` 重做，`Ctrl+Backspace`/`Ctrl+Delete` 按词删除。
- 安装路径旁边的文件夹图标可以直接点击选择目录，选中的路径会写回输入框，旁边的可用空间读数同步更新。
- 卸载完成后，安装目录和卸载程序会被立即清理；用户自己放进目录里的文件仍会保留，目录也会随之保留。
- 界面布局支持 `flex-wrap`：一行放不下时元素自动折到下一行，窗口变窄也不会把内容压扁。

### 已知限制

- 安装包尚未签名，Windows 可能提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。

<!-- release-notes:end -->

### 技术细节

- 双击选词：`begin_text_selection_drag` 用 `last_press` 与 `GetDoubleClickTime()` 判断双击，命中后由 `word_range`/`same_selection_run`/`is_word_character` 选出一个词（字母数字下划线成词、空白成段、分隔符各自独立）。
- 按词操作：`word_start_before`/`word_end_after` 供 Ctrl+Backspace/Delete 与 Ctrl+Left/Right 使用，`delete_before_caret`/`delete_after_caret`/`move_caret` 因此各多一个 `whole_word` 参数。
- 文本编辑：`InteractionState` 新增 `selection_anchor` 与 `selection_range()`/`clear_selection()`/`remove_selection()`，并抽出 `text_offset_for_index` 供光标与选区共用；选区以 alpha 96 的浅蓝高亮带绘制在图片层之上、文字之下。
- 鼠标拖选：`begin_text_selection_drag`/`extend_text_selection_drag`/`end_text_selection_drag` 配合 `WM_LBUTTONDOWN` 的 `SetCapture` 与 `WM_MOUSEMOVE` 完成，原地单击不会留下选区。
- 剪贴板：`write_clipboard_text` 走 `OpenClipboard`/`EmptyClipboard`/`GlobalAlloc(GMEM_MOVEABLE)`/`SetClipboardData(CF_UNICODETEXT)`，分配失败时 `GlobalFree` 回收；`read_clipboard_text` 复用原有实现。
- 撤销栈：`TextSnapshot { id, text, caret }` 与 `UNDO_DEPTH = 64`；连续输入经 `typing_run` 合并为一步，其余编辑各占一步，`restore_snapshot` 会对光标做 clamp。
- `Delete`/`Backspace` 修正为存在选区时只删除选区，不再多吃一个字符；`key_down(VIRTUAL_KEY)` 取代原先直读 `VK_CONTROL` 的写法。
- `flex-wrap`：新增 `wraps(node)` 与 `wrap_lines(&[FlowItem], available_main, gap)`，按 `FlowItem::basis_size()` 折行，超宽元素独占一行；`render_flow` 改为逐行布局，多行时每行高度取该行最高元素，`justify-content` 与 `align-self` 仍然生效；`container_intrinsic_size` 在开启折行且声明了主轴尺寸时按「最宽行」测量。
- 卸载即时清理：`finish_uninstall` 调整为先删注册表、再删 manifest、最后启动清理副本，任一步失败都不会留下空的卸载项；`try_spawn_cleanup_helper` 把自身 exe 复制到 `%TEMP%` 后以 `CLEANUP_FLAG` 与 `CREATE_NO_WINDOW` 启动，失败时回退到 `MOVEFILE_DELAY_UNTIL_REBOOT`。
- 清理副本自身：实验确认 Windows 拒绝删除进程正在运行的镜像，也不接受对其的 delete-on-close（`ACCESS_DENIED`），因此 `self_delete_current_image` 改为把删除交给短生命周期 `cmd` 脚本，脚本等待进程结束后删除副本并自删；`cleanup_after_uninstall` 以 2400×250ms（上限 10 分钟）轮询等待卸载程序可删，随后删除卸载程序与空目录。
- `docs/en` 与 `docs/zh-CN` 的 `XML_LAYOUT_GUIDE.md`、`PRODUCTION_STATUS.md`、`ARCHITECTURE.md`、`TEST_PLAN.md`、`QUICK_START.md` 同步本次能力变更。
- 示例 `examples/TapTap/layouts/configpage.xml`：`editDir` 去掉 `readonly="true"` 并加 `cursor="text"`，`editDirIcon` 补 `action="pick_directory"`、`target` 与 `cursor="hand"`；`chkAgree` 不再声明 `flex-basis="0"`，否则协议文案会被压窄折行。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`。
- `cargo test --locked --workspace`（71 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）。
- `scripts/build.ps1 -Project examples\TapTap` 全量构建、ZIP/7z backend smoke test 与 Win7 PE 导入审计。
- 手工验收：路径框拖选高亮、单击清除选区、输入与 Backspace 生效；清理副本端到端实验确认目标目录与副本都被删除，临时脚本无残留。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。
- 文本框仍无输入法组合窗，中日韩文字的候选依赖系统输入法自己显示。

## [2026.9.16]

安装与卸载流程已完整可用。

### 新增

- 安装与卸载全过程显示实时进度、当前步骤和完成页；完成后可以直接启动刚装好的应用。
- 重复运行安装包会按升级处理：只替换有变化的文件并清理旧版本遗留的文件，中途出错会回到安装前的状态。
- 安装时可以勾选创建桌面快捷方式、开始菜单项和开机自启动，卸载时一并清理。
- 卸载前会先关闭正在运行的产品；用户数据默认保留，取消勾选「保留用户数据」才会删除。
- 界面文案支持 11 种语言，安装与卸载的每一步提示都会跟随所选语言。
- 项目方可以用脚本自定义安装与卸载步骤；脚本出错会回滚本次改动，清理不完整时安装器会自动兜底，不留下残留。
- 界面布局支持容器嵌套、内外边距、百分比尺寸与对齐方式，进度条可按百分比显示。

### 已知限制

- 安装包尚未签名，Windows 可能提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。

<!-- release-notes:end -->

### 技术细节

- 安装改为按偏移索引读取内嵌 bundle，启动不再把整个 payload 载入内存。
- 目标目录存在同一项目的先前安装时按升级处理：替换文件、清理旧版遗留文件，注册失败时
  回滚到先前版本。
- 安装按 `shortcuts.*` 与 `chkShotcut`/`chkAutoRun` 创建桌面、开始菜单快捷方式和开机
  自启动项；`autostart.*` 会先记录原值，失败时还原。
- 卸载会终止运行中的产品进程，按 manifest 清理快捷方式，并按 `uninstall.data_paths`
  删除用户数据；`chkReserveData` 未勾选保留数据时才会删除，默认保留。
- 示例 `examples/TapTap` 重新声明 `uninstall.data_paths`，卸载页补回 `chkReserveData`。
- 安装与卸载会切换到各自的进度页，实时更新进度条和步骤文案，任务结束后切到完成页；
  完成页的 `launch_app` 启动刚部署的 EXE。
- 布局引擎支持嵌套 `VBox`/`HBox`/`Content`，新增 `padding`、`margin`（含单边写法）、
  百分比尺寸、`justify-content` 与 `align-items`；`ProgressBar` 按百分比裁剪 `bar-image`。
- `value-source="status"` 让进度页显示运行时发布的 locale 键，`action="finish"` 与
  `action="launch_app"` 分别对应关闭窗口和启动已安装应用。
- 11 个 locale 补齐 `status.extracting`、`status.deploying`、`status.finishing`。
- 接入项目 Rhai 脚本：`scripts/install.rhai` 与 `scripts/uninstall.rhai` 存在时，安装与卸载
  步骤由脚本决定，原语复用内置流程的部署、回滚与 manifest 代码。脚本失败回滚本次改动；
  卸载脚本漏调 `run_tracked_uninstall` 时由库回退清理，产品不会残留。
- `set_status_key` 让脚本步骤文案走 locale，`set_status` 仍显示字面文本；11 个 locale 补齐
  `status.checking_processes`、`status.installing_uninstaller`、`status.creating_shortcuts`、
  `status.writing_registry` 与 `uninstall.status.cleaning_game_data`。
- 仓库公开后恢复 GitHub Pages 部署：重建 `.github/workflows/docs.yml`（触发分支 `main`），
  并在 `docs/` 下补齐 docsify 站点入口 `index.html`、`_sidebar.md`、`_navbar.md` 与
  `.nojekyll`；logo 走 Git LFS，checkout 因此显式开启 `lfs: true`。
- README 页头改为 logo 与产品名水平居中、垂直居中对齐（`align="texttop"`），
  并同步 `README.zh-CN.md`、`docs/README.md`、`docs/GUI.md`。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`（53 个 core 测试、12 个 GUI 测试、2 个 stub 测试）。
- `scripts/build.ps1` 全量构建、ZIP/7z backend smoke test 与 Win7 PE 导入审计。
- CLI 与 GUI 产物内嵌 16/24/32/48/64/128/256 七种尺寸图标，逐尺寸与各自 branding PNG
  缩放结果比对；三个原始 stub 无图标和版本资源。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。

## [2026.9.15]

本次发布的是构建工具链本身，不包含示例安装包。

### 新增

- 新增图形化构建工具（Windows 10 及以上运行），可以可视化地配置并生成安装包，不必再走命令行。
- 安装包自带解压能力，用户机器上不再需要额外安装解压工具。

### 变更

- 项目结构由 `installer/` 迁移到 `crates/`，各组件统一命名。
- 运行时产物收敛为一套：64 位 Windows 7 SP1 及以上，不再单独维护 Windows 10 版本。
- 安装包与卸载程序的图标、版本信息在构建时写入。
- 发布流程只构建工具链，不再附带示例安装包。

### 已知限制

- 安装包尚未签名；未在真实 Windows 7 SP1 虚拟机上验收。

<!-- release-notes:end -->

### 技术细节

- 目录结构由 `installer/**` 迁移到 `crates/**`，crate 名统一为 `nano-installer-*`：
  `nano-installer-core`、`nano-installer-cli`、`nano-installer-gui`、
  `nano-installer-stub-lzma`、`nano-installer-stub-zlib`、`nano-installer-uninstaller`。
- 运行时基线收敛为单一 `x86_64-win7-windows-msvc`，最低 Windows 7 SP1 x64，不再提供
  单独的 Windows 10 产物。GUI 只作为 Windows 10+ 构建工具，不进入 setup。
- 解压由外部 `tools/7za.exe` 改为内嵌：7z/LZMA 与 ZIP/Deflate 各由独立 stub 承担，
  setup 运行时不再需要外部解压工具。
- builder 为 setup 与自包含 uninstaller 注入图标和 Unicode `VERSIONINFO`；原始 stub
  保持无产品资源，由项目 `output` 配置决定生成物的资源。
- 新增 Windows 10+ 可视化构建工具 `nano-installer-gui-x64.exe`，与 CLI 复用同一
  core build/inspection API，不启动 CLI 子进程。
- 发布流程只构建工具链，不再附带示例 setup。

### 已验证

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`。
- 用真实 ZIP 与 7z 归档验证两个 release stub 的解压结果 SHA-256。
- builder、三个 stub 以及生成的 setup 的 Win7 PE 导入审计。

### 未完成

- runtime 启动时仍会整体读入内嵌 payload，需要改为 offset/mmap 访问。
- 无升级、覆盖安装、取消、完整进度页与故障回滚。
- 卸载只清理 manifest 内文件；无数据保留选项、快捷方式清理与进程终止。
- 尚未实现快捷方式、开机自启动、静默安装与项目 Rhai 脚本执行。
- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。

详见 [当前生产状态](docs/zh-CN/PRODUCTION_STATUS.md)。
## 如何贡献

见 [AGENTS.md](AGENTS.md)。
