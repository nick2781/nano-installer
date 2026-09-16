# 更新日志

本文件记录 nano-installer 的版本变更历史。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。
版本号使用 CalVer（`YYYY.M.D`），tag 形如 `v2026.9.16`。

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
