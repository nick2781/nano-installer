# 更新日志

本文件记录 nano-installer 的版本变更历史。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。
版本号使用 [CalVer](https://calver.org/)（`完整年份.月.日`，月与日不补零），tag 形如 `v2026.9.17`。
日历日取本项目声明的 UTC+08:00（CalVer 允许项目自选日历日，写明即可），因此北京时间的深夜发布
仍算当天。同一个日历日第二次发布加修饰后缀（`v2026.9.17-r2`），不把日期往后写、也不加第四段
数字；`scripts/release_version.ps1` 会校验这些规则，构建与发布都会执行。

> **2026-09-17 版本号校正**：此前发布的 `v2026.9.17`、`v2026.9.18`、`v2026.9.19`、`v2026.9.20`
> 四个 tag 都把日期写在了实际发布日（9 月 16 日至 17 日）之后，后两个甚至是还没到的日期。
> 这些 tag 与对应 release 已删除，当天完成的内容统一作为 `2026.9.17` 重新发布。

## [2026.9.21]

本次发布把向导的交互补齐了：字段按工程写下的规矩判断取值，长列表可以滚动，正在跑的任务能中途停下。
项目脚本这边，提示与提问画在了产品自己的卡片上，脚本也可以把要用的辅助程序随安装包一起带走。

### 新增

- 脚本发出的提示、报错和提问画在向导窗口里的卡片上，和产品自己问的那张用同一份布局：卡片的标题
  就是脚本写的那句话，提问带上「是」「否」两个答案，用户点中的那个就是脚本接着往下走的答案。
  没有对话框布局、静默运行，或者窗口还没建立时，仍旧用系统对话框。
- 输入框可以带上工程对取值的规矩：必填，按字符数算的最短与最长长度，以及一段掩码——`*` 代表
  任意长的一段（可以为空），`?` 代表恰好一个字符，整段值都要对得上。没写规矩的字段一律合格，
  可留空的字段空着也算合格。页面可以拿「合格」「不合格」当按钮的启用条件，也可以用一条提示说出
  值最先破坏的那条规矩，文案按工程自己的语言查表。空的输入框照样能点进去。
- 下拉框和单选按钮成为页面能提供的选择，不只是语言控件。下拉框按工程声明的选项列出来，收起时
  显示当前那一行的文字；单选按钮按组归属，一组任何时刻只持有一个取值，版面可以标出起始选中的
  那一行；按钮可以按「某一行被选中」来决定自己能不能点。
- 长列表可以滚动。声明 `scrollable="true"` 的容器保住自己拿到的尺寸，子元素按各自声明的尺寸
  排开，放不下的部分整体裁掉。滚轮每格移动 48 像素，尾部一条滚动条说明列表露出了哪一块，点滑块
  两侧的轨道各翻一页；被滚过去的那一行既不画出来，也不再登记点击，列表下方的按钮因此能接到原本
  会被它吃掉的点击。
- 跑着的任务可以停掉。`cancel` 按钮，或者对关闭提问回答「是」，都会让任务在下一个检查点停下，
  撤回已经写下的内容，而不是在机器上留下一份装了一半的产品；工程脚本能看到同一个取消请求。
- 工程可以把脚本要用的辅助程序一起打进安装包：在配置里指明目录后，目录按原样进包，子目录也在内，
  脚本拿到解好的路径就能把它跑起来。工具放在临时目录，不会变成已安装产品的一部分，也不会进卸载
  清单。没指明目录的工程不会带上任何工具；脚本向一个没带工具的安装包要工具时，拿到空路径和一条
  日志告警，安装照常继续。
- 脚本可以写入当前用户的环境变量，也可以让某个扩展名默认用这个产品的程序打开。写过的值和建过的
  键都会记下来，卸载只按记录收回这些：环境变量记的是值而不是键，`PATH` 不会跟着被删掉。
- 页面之间可以前后走：按钮用 `next` 与 `back` 两个动作在工程声明的页面之间移动，走到头就停住，
  不打转。页面还可以声明自己是报告进度的页还是结束页，许可页、选项页因此可以排在它们前面；
  一个职责只能由一个页面承担。
- 工程可以把安装内容切成组件，让用户挑着装。`resources.payload_file` 之外，`components.items` 里的每个
  组件带自己的 ZIP 或 7z 归档，页面上同名的 `Checkbox` 决定这次装不装它；工程写在组件上的 `required`
  一律装，页面上没有这个复选框（含静默运行）时按 `default` 决定。脚本用 `is_component_selected` 与
  `selected_components` 读到同一个答案。所有归档由同一个运行时解压，格式不一致的组件在构建时就被
  拒绝；两个归档带同一个相对路径的组件在安装时报错，而不是按声明顺序互相覆盖，用户装到的东西不再
  取决于工程把组件排在前面还是后面。
- 脚本读得到用户留在页面上的取值。文本框里的字、下拉框和单选组当前的那一行，各按版面里的控件 id
  或组的名字取回，页面没有声明的 id 读成空串而不是报错；静默运行没有页面，读到的也是空串。
- 工程可以声明机器上必须先有的东西，安装时先补齐、再装产品。每条依赖写清怎么认（查一个文件，或查注册
  表的某个值是否存在、是否等于某个值、是否不低于某个版本）、怎么装（随安装包带一个 `.exe`，或按 URL
  下载一个），以及缺了算不算致命：已经有的不动，缺的装上，必需的装不上就停下并说出依赖名与安装程序的
  退出码，非必需的只记一条告警。下载回来的程序要先对得上工程记下的 SHA-256 才允许跑，对不上就地删掉。
  装进机器的是依赖本身，它不在卸载清单里——产品卸掉之后，机器上原本缺的东西仍然留着。脚本要自己挑时机
  时，`dependency_installed`、`install_dependency`、`download_file`、`download_file_with_hash` 和
  `sha256_of_file` 问的是同一份声明、走的是同一条取文件的路径。
- 脚本能读写注册表里的每一种类型：文本、可展开的文本、多行文本、DWORD、QWORD 与二进制，也能只问
  一个值在不在、机器把它存成了什么类型；用错类型的读法返回空值而不是硬把值转一次。键名可以在根键
  后面带上 `32` 或 `64`，指名 64 位 Windows 里的哪一份拷贝，视图跟着键名一起记进 manifest，卸载
  收回的正是脚本写过的那一份。删除键不再接受以根键本身为目标，脚本没法写一个 `HKCU\Software`
  就带走整棵软件树。
- 脚本跑过的程序，写下的内容能收回来：`run_command_output` 把退出码、标准输出与标准错误一起交回，
  字节先按 UTF-8 解、解不开就按这台机器的 ANSI 代码页解，中文 Windows 上的 `ipconfig` 因此读成中文
  而不是替换字符；用户停掉任务时，连这个程序已经写下的内容一起结束。
- 项目脚本可以装上自己的服务（`service_install`、`service_exists`、`service_running`、
  `service_start`、`service_stop`、`service_set_start_type`、`service_delete`）。服务不是安装目录
  里的文件，而是机器自己的一条记录，所以卸载按 manifest 先停后删，而且排在删文件之前——服务的程序
  就在安装目录里。同名却跑着别的程序的服务会被拒绝，卸载不会把别人的服务带走。这些都要求提权，
  没有权限时调用返回 `false` 并把 Windows 的原话写进日志；`service_install` 只装不启，
  什么时候让它跑由脚本自己决定。
- 每次安装或卸载都在磁盘上留下一份运行日志：默认落在临时目录的 `nano-installer` 目录里，
  文件名带着安装程序的名字、时刻和这次做的事；无窗口运行可以用 `--log` 指定写到哪个文件，
  无人值守部署因此能把日志收到一处。日志开头是这台机器——是否提权、Windows 版本与内部版本号、
  位数、界面语言——接着是产品名与版本、装到哪个目录，然后是运行自己的每一步和脚本写下的每一行，
  不再只有向导内存里那最后 32 行。运行失败时这份文件留在原地，向导把它的完整路径写在错误下面，
  无窗口运行写进标准错误，用户或者运维把它交出去就够；它不在安装目录里，所以失败的全新安装
  撤掉自己创建的目录之后，日志还在。

### 改进

- 构建器不再接受自己不读的配置键。以前能解析却什么都不做的设置会让构建失败，并指出该改用哪个
  设置；`links` 表和构建器不认识的段落照旧留给工程自己用。
- 每次测试运行都留下可以读的报告：一份文本和一份可以点开看的网页，列出每条用例、它的结果，以及
  它守住的文档行为，CI 把两份都留成产物。安装包级的用例由专门的脚本先构建运行时再跑，整批跳过
  不会被读成通过。
- 流水线的测试步骤不再挂住。它曾多次跑上几十分钟也不结束，连日志都留不下：步骤的每个子孙进程
  都继承着步骤自己的输出，而步骤要等输出关闭才算结束，于是一个活过命令的进程就能把它按在原地。
  这种步骤不理会任何期限——本机脚本的超时分支、workflow 给步骤写的 `timeout-minutes`、作业的
  `timeout-minutes` 都碰不到它（最近两次运行里都看到作业到了 62 分钟仍在跑，取消也要十几分钟
  才落到 `cancelled`）。要断的是继承：两个跑套件的脚本和构建步骤都把命令放进自带控制台的进程，
  命令的输出仍旧写进操作系统打开的普通文件，脚本读到的内容不再被构建的子孙进程握住；命令启动
  的进程另进一个随脚本结束的作业，作为第二道防线。命令都带着期限，超时的命令先把最后写下的
  几十行、以及当时还活着的进程（父进程与命令行也在内）打出来，再整棵结束掉，步骤随后以失败
  结束，日志因此留在产物里，也能看出它停在了哪里。
- 向导的指针与键盘行为第一次有自动化用例看守：悬停与按下换上的状态位图、按钮上的手型与输入框上的
  工字光标、语言菜单的上下键与 Enter/Escape、以及选目录按钮打开的那个系统对话框，都在真实窗口里
  跑一遍，读过窗口自己画出来的像素才算通过；其中光标那条要求会话真的在显示指针，托管 runner
  没有鼠标，它会在那里打印跳过理由。

### 修复

- 编译失败的套件仍然留下报告。一个目标都没跑出来时，`run_tests.ps1` 会在写报告之前停下，
  留下一条 PowerShell 报错而不是一份报告——而那正是最需要报告的场合。现在文本与页面照旧写出，
  退出码仍是命令自己的那个。
- 签过名的安装包能装了。签名把证书表追加在捆绑数据之后，而运行时原先直接读文件的最后十几个字节，
  于是对签过名的安装包报「找不到捆绑数据」；现在它从文件尾部向前搜索那个标记。
- 用 `inset-right` 或 `inset-bottom` 贴边的元素现在贴到了那一边，之前会退回普通的流式位置。
- 绝对定位的复选框不再失灵：它之前既不画勾选状态的图，也不接收点击，示例工程卸载页上的
  「保留用户数据」因此是死的。
- 空的输入框可以点进去。没有文字的字段之前不留光标，也不登记点击区域，页面要用户先填一个目录
  再开始安装时，用户点不进去。
- 两个脚本同时运行时，各自的日志不再串到对方身上。

### 已知限制

- 安装包尚未签名，Windows SmartScreen 仍会提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。
- 脚本发出的提示与提问在没有对话框布局、静默运行或窗口还没建立时，仍旧是系统对话框。

<!-- release-notes:end -->

### 技术细节

- 组件：配置新增 `components.items` 段（`id`、`payload`、`default`、`required` 四个键），
  `config::audit_components` 逐个条目校验，构建期报出缺 id 或 payload、重复的 id 或归档、指向
  `resources.payload_file` 的归档，以及 `required: true` 配 `default: false` 这种没有意义的组合。
  `install::selected_components` 按「写 required 的一律装、页面有同名复选框就听页面的、否则听
  default」算出这次装哪些；`extract_payload` 改为逐个归档先各自解到独立目录、比对之后才合并进暂存
  目录，重叠当场报错而不是互相覆盖；组件归档与基础载荷的格式一致性在 `inspect_project` 里校验。
  `ScriptEnvironment` 带上这次的选择，`api_ui::is_component_selected`/`selected_components` 读它，
  卸载侧一律为空数组。
- 对话框：`DialogKind` 新增 `Question`，`script_dialog()` 按 `ui.dialog_layout` 打开与
  `close_confirm` 同一份卡片，`ask_yes_no` 在工作线程上等点击，窗口照常重绘与接收点击，答案经
  `WindowAction::DialogOk`/`DialogCancel` 回到脚本；两个按钮的文字取 locale 的 `yes` 与 `no` 键，
  `visible-with="dismiss"` 现在也覆盖脚本的提问。没有布局、静默运行或窗口未建立时退回
  `MessageBoxW`。
- 字段校验：`TextInput` 的 `required`、`min-length`、`max-length` 与 `pattern` 各带一条消息键
  （`required-message` 等），值最先破坏的那条规矩决定 `value-source="field-error:<字段 id>"` 的
  `Label` 画出哪句文案；`enabled-when` 除复选框与面板外还认 `<字段 id>:valid` 与 `:invalid`。
  空字段原先在取文字的提前返回处被丢掉，`push_text_input` 现在在绝对与流式两条路径上都登记命中
  区域。
- 页面取值：`InstallSelection` 除复选框外还带上 `texts` 与 `choices` 两份快照，由 `start_install()` 从
  `InteractionState` 拷出，`api_ui::get_text_value`/`get_choice_value` 按同一份映射取值，缺失的 id 是
  空串；静默运行交出的是一份空选择，所以没有页面时读到的就是空串。
- 选择：当前取值存在 `InteractionState.choices` 里，键是下拉框自己的 `id` 或单选组的名字；
  `ToggleLanguageMenu` 改名 `ToggleSelectMenu`，展开的菜单按打开它的控件记账，键盘只走当前打开的
  那个菜单。
- 滚动：`ScrollView` 记 id、视口、偏移与上限，偏移存在 `InteractionState` 下同名的键上，重绘与
  翻回本页都留在原处；`scrollbar_track`/`scrollbar_thumb`/`scrollbar_pages` 画 8 像素轨道与不小于
  24 像素的滑块，`SCROLL_STEP = 48` 随 DPI 缩放；`WindowAction::ScrollPage` 承接轨道点击；图层、
  文字、命中区域、悬停区域与输入框按视口统一裁剪；声明 `flex-wrap` 或没写 `id` 的容器不滚动。
- 取消：`Cancellation` 句柄每个任务一个，`request_cancel()` 置位、`check_cancelled()` 在检查点读取；
  `Cancelled` 是独立的错误类型，向导因此报 `Operation cancelled` 而不是失败；等解压子进程的那一步
  改成轮询，收到请求就结束子进程，那一步本来没有东西要撤销；脚本侧的 `api_ui::is_cancelled` 读同
  一个句柄。
- 工具目录：`resources.tools_dir` 参与构建，捆绑索引新增按路径顺序整目录回读；`get_tools_dir()`
  解包到本次运行的临时目录并缓存路径，没带工具时返回空串并写一条「工程没有打包工具目录」告警。
- 环境与文件类型：`api_system::set_env`/`remove_env` 写 `HKCU\Environment` 并通知外壳；从资源管理器
  启动的进程读到的是它启动时缓存的那份环境块，所以新值要等下一个新进程才看得见，脚本自己的进程
  也一样。`api_association::register_file_association` 与
  `unregister_file_association` 按 Windows 读取文件类型的四处写注册表，写入前拒绝带路径分隔符的
  扩展名与程序 ID。清单把环境变量记成「值」这一片共享叶子，所以卸载删的是这个变量，不是 `PATH`。
- 页面导航：`WindowAction::NextPage`/`PreviousPage` 由 `action="next"`/`"back"` 解析，走到列表两端
  停下；`config.rs` 的 `PAGE_ROLES` 只接受 `progress` 与 `finish`，两个页面抢同一个角色在构建期
  报错。
- 配置审计：构建前按 `config.rs` 的键表核对 `installer_config.json`，一次报出所有不被读取的键并
  给出替代设置，`links` 表与构建器不认识的段落除外。顺带把「目标盘空间不足」从一个没有用例看守的
  隐性检查提成独立函数，并补上它一直没有的用例。
- 签名：运行时不再读文件最后 16 字节，而是在尾部向前搜索捆绑数据的尾标记；
  `a_setup_with_a_signature_appended_still_installs` 钉住这条行为。签名仍由发布方执行
  （`scripts/sign.ps1` 用 `NANO_INSTALLER_CERT_THUMBPRINT` 指定的证书签并校验），构建不调用它。
- 测试与报告：`run_tests.ps1` 与 `run_e2e_setup.ps1` 把每条命令的输出交给操作系统打开的文件再读回，
  不再用 PowerShell 管道：管道会在构建留下的孙进程上等待，本机实测同一子进程 8668 毫秒对 464
  毫秒。报告由 `report_data.ps1`、`report_html.ps1` 与 `report_text.json` 生成，语言用 `-Language`
  选（默认 `zh-CN`），用例说明取自 `docs/<语言>/TEST_CASES.md`，行为与用例的对应取自
  `docs/<语言>/TEST_COVERAGE.md`，`audit_case_descriptions.ps1` 在构建期检查每条用例都有说明。
- 指针：唯一需要真实鼠标指针的两条用例（悬停与按下换上的状态位图、窗口回哪种标准光标）现在互斥。
  它们各自把自己的窗口提到最前，再读回光标形状与像素；同时跑就会读到对方窗口的答案，
  谁先谁后因此由一把锁决定，而不是由套件的调度碰巧决定。
- 窗口级覆盖：`e2e_setup.rs` 新增四条真窗口用例，分别读回悬停与按下换上的状态位图、窗口在按钮、
  输入框和页面空白处回的标准光标、语言菜单的上下键与 Enter/Escape，以及 `pick_directory` 按钮开出
  的选目录对话框；它们把窗口提到最前、真的移动指针，读完一帧再比像素。运行时的组字点与候选点抽成
  `composition_points()`，由核心用例 `an_input_method_anchors_at_the_caret_the_page_drew` 钉住。
- 快照：`capture_setup_snapshots.ps1` 按 `wizard.pages` 与 `wizard.uninstall_pages` 逐页构建并拍照，
  对照版面检查客户区、切角、绝对坐标下的图片与标签、整页颜色数以及各页互不相同；
  `review_setup_snapshots.ps1` 把字形的判断交给本机运行的视觉模型。拍照需要桌面会话，因此不进 CI。
- 文档：新增 `docs/{en,zh-CN}/TEST_COVERAGE.md` 与 `docs/zh-CN/TEST_CASES.md`（报告在中文页面读这张
  表里的说明，英文页面读用例自己的文档注释），`SCRIPT_API.md`、
  `XML_LAYOUT_GUIDE.md`、`CONFIG_REFERENCE.md`、`ARCHITECTURE.md`、`TEST_PLAN.md`、
  `BUILD_AND_RELEASE.md` 与 `PRODUCTION_STATUS.md` 同步本次能力；中文页面按中文技术写作重写，去掉
  「由…」被动和「产物/自身/该/其」这类译文腔，落地页的 logo 与语言切换改成在站点根目录下也成立
  的路径。

- 依赖与获取：配置新增 `dependencies.items`，`config::audit_dependencies` 在构建期校验 id 非空且唯一、
  载荷与下载二选一、载荷必须是 `.exe`、`detect` 必填且文件与注册表二选一、`equals` 与 `at_least` 需
  要值名且互斥、下载必须是 http(s) 且带 64 位十六进制 `sha256`。`dependency::install_missing` 排在脚
  本分派之后、解压 payload 之前（占进度 6% 到 14%，状态键 `status.dependencies`），缺哪条装哪条，必
  需的失败就中止这次安装；`dependency::detect` 按文件或注册表值回答「有没有」；`dependency::install`
  用 `CREATE_NO_WINDOW` 起安装程序并轮询等待、可取消，退出码 0、1638（已装更新版本）、3010 与 1641（
  要重启）算成功，其余报出依赖名与退出码。`net::download` 走 WinHTTP（默认代理、跟随重定向、https 打
  开 TLS 1.1/1.2、逐块检查取消），哈希用 CryptoAPI 算（`advapi32`，不是 Windows 8 才有的
  `bcryptprimitives`）；长度与 `Content-Length` 不符或哈希不符都会把目标文件删掉。脚本侧新增
  `dependency_installed`、`install_dependency`、`download_file`、`download_file_with_hash` 与
  `sha256_of_file`：前两个读同一份声明，卸载模式下 `install_dependency` 一律返回 false。构建时依赖的
  载荷一并收进捆绑数据，`inspect_project` 校验这些文件确实存在。
- 注册表：核心库新增 `RegistryView`（`Native`、`Wow6432`、`Wow64`，分别对应不带标志、
  `KEY_WOW64_32KEY` 与 `KEY_WOW64_64KEY`）与 `RegistryKey { root, path, view }`，读、写、删都经过它；
  视图写在键名里，由 `parse_registry_key` 解析（`HKLM32`/`HKCU64`，`HKEY_LOCAL_MACHINE32` 这样的长名
  也认）。`registry_path` 因为要把根键与子键分开交给独立进程的卸载器，遇到带视图的键直接报错，
  卸载注册键与自启动项因此只能落在原生视图。删除键改成先按视图打开父键、再
  `RegDeleteTreeW(parent, leaf)`，单段路径（如 `HKCU\Software`）因此被拒绝，而不是删掉整棵软件树。
  `api_registry` 的十七个原语共用一条写入路径，写成功才把键与值记进 manifest。
- 命令输出：`shell::run_captured` 用 `CREATE_NO_WINDOW` 起子进程，两个输出管道各由一个线程读到底，
  主线程每 50 毫秒查一次取消并与 `try_wait` 一起等；收到取消就结束子进程、退出码记 `-1`。读回来的
  字节先按 UTF-8 解，解不开按这台机器的 ANSI 代码页解（`MultiByteToWideChar(CP_ACP)`，为此在核心库
  开了 `Win32_Globalization`）。`run_detached`、`run_command` 与 `run_command_output` 都走这条路径，
  `run_command_output` 返回 `#{code, stdout, stderr}`，起不来时 `code` 为 `-1`、`stderr` 里放着原因。
- 依赖检测：`dependency::detect` 改用 `parse_registry_key`，`dependencies.items[].detect.registry.key`
  因此可以带视图后缀，按位宽分开装的运行库可以用同一条规则来问。
- 服务：新增 `crates/nano-installer-core/src/service.rs`，`service::install` 用 `CreateServiceW` 装、
  `DeleteService` 删（先 `ControlService(SERVICE_CONTROL_STOP)` 再删，状态最多等 10 秒）、
  `QueryServiceStatus` 读状态、`ChangeServiceConfigW` 与 `ChangeServiceConfig2W` 改启动方式与延迟
  自启；命令行由 `command_line` 拼出，程序路径始终加引号。同名服务已经存在时先读回它的
  `lpBinaryPathName`，与这次要写的命令行逐字比较：一样就当自己（升级重放这一步），不一样则拒绝，
  并把两条命令行都写进错误。`script/api_service.rs` 把七个原语接到这套实现上：装成功后
  `ScriptState.services` 记名，`service_delete` 成功后划掉。manifest 新增 `services` 数组，
  `install::remove_recorded_artifacts` 从 `remove_recorded_services` 起手，排在删快捷方式与注册表
  之前。装、改、停、删都要求提权，`is_elevated` 的实现挪到 `shell.rs` 由两个模块共用。顺带让两份
  测试报告写明本次是否提权运行（`Test-Elevated`），因为从这一版起有几条用例的答案取决于它。

- 运行日志：新增 `crates/nano-installer-core/src/install_log.rs`，`begin` 打开文件并写下这台机器的
  事实（提权与否取 `shell::is_elevated`，Windows 版本从注册表的
  `SOFTWARE\Microsoft\Windows NT\CurrentVersion` 读——`GetVersionExW` 对我们这种没声明
  `supportedOS` 的清单会答 6.2，位数取 `GetNativeSystemInfo` 与 `size_of::<usize>()`，界面语言取
  `GetUserDefaultLocaleName`），`describe` 补上产品与目录，`note` 逐行落盘（每行 `flush`，写到 1 MiB
  停笔并注明截断），`end` 写下结局并把「日志在哪」那一行交回调用方。日志开关走
  `parse_silent_arguments`——它现在返回 `SilentOptions { directory, log }`，`--log` 与 `--dir`
  一样可以给一次，也可以不给；卸载侧只认 `--log`。`script::log` 除内存里的 32 行尾巴之外再写一份
  到文件，`install_setup` 与 `uninstall` 各写一条产品事实、并在每一步留一行。写日志失败一律不影响
  安装（`begin_log` 只把原因写进标准错误），因为写不了日志的机器照样得装上产品。向导的失败提示
  由 `result_notice` 拼出，错误下面跟着日志文件的完整路径。

构建代理上那个卡死的步骤查清了，原先的判断只对了一半：作业对象确在收拾脚本留下的进程，但步骤的
每个子孙进程同时继承着步骤自己的输出，runner 又要等输出关闭才算这一步结束，于是任何一个活过命令
的进程——链接器的遥测助手、用例没关掉的窗口——都能把步骤按在那里，而且不受任何期限约束。
为了分清卡住的是套件还是包着它的脚本，另起了一个一次性分支，把套件拆成三步、每步带自己的期限、
输出落到文件后上传：CI 上套件本身是绿的（服务用例 6 条 0.01 秒、核心库单线程 5.76 秒、整个工作区
210 通过、42 跳过、5 与 29 全过、0 失败）。真正的修法是断掉继承：`run_tests.ps1` 与
`run_e2e_setup.ps1` 用 `UseShellExecute` 给命令一个自带控制台的进程，`Build Win7+ toolchain`
与发布流水线的构建步骤同样处理。期限分支另加一段进程快照，超时时把还活着的进程连同父进程与
命令行写进日志——下次再卡，不必再从外面猜。

### 已验证

- `cargo test --locked --workspace`：共 294 条用例，293 通过、0 失败、1 忽略，退出码 0（核心库 214、
  安装包级 44、工程检查 5、可视化构建器 29，另加两个解压运行时用例；被忽略的
  `install::tests::registers_and_cleans_up_scoped_uninstall_key` 要在隔离环境里写 HKCU）。报告在
  `target/test-report.txt` 与 `target/test-report.html`。
- 安装包级的 44 条全部跑通，其中 13 条会打开真实的向导窗口；
  `run_e2e_setup.ps1 -RequireDesktop` 那次把跳过当成失败，报告在 `target/e2e-report.txt` 与
  `target/e2e-report.html`，同一个脚本加 `-Language en` 会另留一份英文版。
- 注册表类型、命令输出与服务这三条安装包级用例的答案是从机器上读回来的，不是从写下它的原语手里：
  六个值的类型由 `reg query` 报出（`REG_SZ`、`REG_EXPAND_SZ`、`REG_MULTI_SZ`、`REG_DWORD`、
  `REG_QWORD`、`REG_BINARY`），带视图后缀的键建得出、读得到、删得掉，命令留下的是退出码 5 与
  两个输出流的逐字节内容。本机装的那份 `EdgeUpdate` 只在 32 位视图里登记，因此 `HKLM32` 读得到、
  `HKLM64` 读不到，两个视图名在真机上分了岔；换成没有这种软件的机器时这条不做断言，
  用例会先向 `reg query` 问一次哪个视图有它。
- 服务这条路径在本机只跑到了没有权限的那一半：本机不是提权进程，装服务、改服务、删服务都返回
  `false`，机器上也没有留下任何服务（`sc query` 问过），两次报告因此都写着「提权运行 否」。
  装上再删掉的往返要有提权环境才跑得到，CI 的 runner 是管理员账户时就会跑到它。装上以后服务是否
  正常运行谁也跑不到：服务程序是产品自己的。
- 依赖用例跑的是真程序：随安装包带上的依赖真被装上，而且按它自己留下的文件判断机器上有没有；按 URL
  下载的依赖先过一遍独立算出的摘要（`certutil` 算的，不是运行时自己算给自己看的那份）才跑，摘要对不
  上时那个程序一次都没有被启动，目标文件也没留下。
- 需要真实鼠标指针的两条用例（悬停与按下换上的状态位图、窗口在按钮和输入框上回哪种标准光标）这一次
  跟着整批跑通了：早前几次它们失败或跳过，是因为工作站锁屏（`LogonUI` 在运行），与改动无关；
  没有指针的会话上它们打印自己的跳过理由。
- 示例工程 6 个页面的 10 张快照逐页对照版面检查通过，检查结果在 `target/setup-snapshots/manifest.json`；
  拍照要桌面会话，因此不进 CI。
- 签名实验：用 signtool 与本地签发的证书签过的安装包在 Windows 11 上装成。
- 这一版的两份报告在本地重新跑通：`run_tests.ps1` 退出码 0，报告里核心库 213 通过、1 忽略，
  安装包级 44 通过，工程检查 5，构建器 29，两个运行时各 1（合计 293 通过、0 失败、1 忽略）；
  `run_e2e_setup.ps1 -RequireDesktop` 44 通过、0 失败。两份报告都照旧写出，`提权运行` 一栏如实
  写着「否」，服务那一半的结论因此仍要等提权的机器（CI 的 runner 是管理员账户）。
- `scripts/verify_release_notes.ps1` 通过，生成器产出的发布正文与源码里的尾注逐字一致。

### 未完成

- 发布流水线尚未接入签名；`scripts/sign.ps1` 已就绪，但构建与发布都不调用它。
- 未通过真实 Windows 7 SP1 虚拟机端到端验收。

## [2026.9.17-r2]

今天第二个版本：安装包可以不开窗口地装完，仓库里那套「写了却从没跑过」的测试也换成了真的
端到端用例。

### 新增

- 支持无人值守安装与卸载。项目在配置里声明后，安装包与卸载程序都可以带 `--silent` 运行，
  全程不弹任何窗口；安装目录可以用 `--dir` 指定，写法与配置里的路径一样支持环境变量。
  写错的参数会让运行直接失败，而不是装到默认目录；没有声明的项目会被拒绝，不会被无人值守地
  装上或卸掉。无人值守卸载一律保留用户数据，因为没有复选框可以取消勾选。
- 提供一套真正的端到端测试：自造项目、构建安装包、执行安装、再执行卸载，覆盖产物结构、
  payload 逐字节还原、manifest 与卸载项的写入、升级时清理旧文件并保留用户文件，以及卸载后
  目录与注册项的清理。用例自己生成素材，不依赖示例项目，也不会在你机器上留下任何残留。

### 改进

- 「跳过」不再能伪装成通过：在专门验证安装包的流水线里，运行时缺失会让用例直接失败，
  而不是把每个用例都记为跳过却依然显示绿色。
- 新增一项检查，防止测试文件被放在永远不会被编译的位置。仓库根目录曾有一个这样的「集成测试」，
  它引用了早已不存在的类型，其中的用例一次也没有运行过。

### 已知限制

- 安装包尚未签名，Windows SmartScreen 仍会提示「未知发布者」。
- 尚未在真实 Windows 7 SP1 虚拟机上完成端到端验收。

<!-- release-notes:end -->

### 技术细节

- `nano-installer-core` 新增 `SILENT_FLAG`、`silent_mode()` 与 `mark_silent()`；三个 stub 的
  `main` 把 `--silent` 交给运行时入口，而不是当作未知参数报错（否则会弹出一个没人能点的模态框，
  把无人值守安装挂死）。
- `install::run_silent_install` / `run_silent_uninstall` 读取 bundle 索引与项目配置，用
  `require_silent_support` 校验 `advanced.silent_mode_support` 与 `advanced.uninstall_mode_support`；
  `parse_silent_arguments` 只接受 `--dir <path>`，其余一律拒绝。
- `resolve_install_destination` 统一了解析规则：显式路径优先于 `install.default_path`，两者都先经
  `shell::expand_environment`。未展开的 `%LOCALAPPDATA%\Product` 不是绝对路径，会被
  `validate_destination` 拒绝，这曾是自动安装无法落地的根因。
- `update_runtime` 在无窗口状态下退化为 no-op，进度与状态上报不再是错误路径；`show_notice` 与
  `script/api_system.rs` 的 `message_box`/`ask_yes_no` 在静默时改写日志，后者返回 `false`，
  绝不阻塞等待点击。
- 新增 `crates/nano-installer-core/tests/e2e_setup.rs`：12 个用例，分为产物、安装、升级、卸载四组。
  payload 由仓库内 `tools/7za.exe` 生成（同一工具产出 `-tzip` 与 `-t7z`），因此不引入归档依赖，
  `Cargo.lock` 保持不变。`Fixture` 用 `TempDir` 加唯一 id，注册到
  `HKCU\...\Uninstall\nano-installer-e2e-{id}`，`Drop` 里跑卸载并清注册表，断言失败也不留残留。
- `scripts/audit_test_targets.ps1` 读取 `cargo metadata` 得到工作区的包，若 `tests/*.rs` 不属于
  任何包则失败，同时拒绝 `autotests = false`；CI 与本地都可执行。
- `.github/workflows/ci.yml` 新增 `setup-end-to-end` job：先构建三个 stub，再以
  `NANO_INSTALLER_E2E_REQUIRE_STUBS=1` 运行 `e2e_setup`；`native-win7-build` job 增加测试目标审计。
- 删除 `tests/integration_test.rs`：它引用的 `nano_installer` crate 实际叫 `nano_installer_core`，
  且使用了 `i18n::LanguagePack`、`common::config::InstallerConfig` 等不存在的类型，5 个 `#[test]`
  从未被编译。
- `docs/{en,zh-CN}` 的 `CONFIG_REFERENCE.md` 增加「无人值守运行」章节并修正「已接受但尚未生效」
  段落（`advanced.*` 不再整体未生效）；`TEST_PLAN.md`、`BUILD_AND_RELEASE.md`、
  `PRODUCTION_STATUS.md` 同步。

## [2026.9.17]

本次发布把安装界面收成一个像样的 Windows 安装程序：退出确认不再是系统灰框，窗口不再乱跑，
高分屏和换屏幕也不会糊。以下内容在同一天内陆续完成，统一作为今天的版本发布。

### 新增

- 安装路径输入框支持完整的文本编辑：鼠标拖选、双击选词或 `Ctrl+A` 全选，选中部分会高亮显示；`Ctrl+C`/`Ctrl+X`/`Ctrl+V` 复制、剪切与粘贴，`Ctrl+Z` 撤销、`Ctrl+Y` 重做，`Ctrl+Backspace`/`Ctrl+Delete` 按词删除。
- 安装路径旁边的文件夹图标可以直接点击选择目录，选中的路径会写回输入框，旁边的可用空间读数同步更新。
- 卸载完成后，安装目录和卸载程序会被立即清理；用户自己放进目录里的文件仍会保留，目录也会随之保留。
- 界面布局支持 `flex-wrap`：一行放不下时元素自动折到下一行，窗口变窄也不会把内容压扁。
- 要装到 `Program Files` 的产品可以在配置里申请管理员权限：安装包启动时由 Windows 弹出提权确认，
  用户不必再右键「以管理员身份运行」。默认不申请，普通双击即可运行；内嵌的卸载程序申请同一级别，
  卸载项仍然能撤销一次提权安装。
- 高分屏下窗口由程序自己缩放，文字和图片保持清晰，而不是被系统整体拉伸。
- 安装路径输入框接入输入法组合窗：组合中的文字停在光标处，候选列表紧贴光标下方，中日韩输入与
  常见 Windows 输入框一致。
- 构建时检查译文：哪个语言少了页面文案（会列出具体键名）、哪个语言在配置里声明了却没有语言文件，
  都会出现在可视化构建工具的「构建警告」里。

### 改进

- 安装界面支持逐显示器 DPI：窗口在哪块屏幕上，就按哪块屏幕的缩放比例重新排版。高分辨率屏幕上
  文字与图标保持锐利，混用 100%、150%、200% 的多屏环境不再出现糊成一片的界面。
- 窗口被移到另一块屏幕后仍完整可见：新位置超出该屏幕可用范围时会被拉回屏幕内，比屏幕还大的
  布局则收缩到可用范围，不会露在屏幕外。
- 圆角在尺寸变化后依然是圆角：页面变高变宽时窗口形状会跟着重算，不再留下旧尺寸的直角缺口。

### 修复

- 确认退出安装时，弹出的是与安装界面同一套皮肤、同一套文案的确认框，不再出现系统默认的灰色
  弹窗；确认框画在窗口内部，因此不会被其它程序盖住或甩到后面。
- 提示信息（例如某个步骤出错）同样走这套皮肤，不再打断成系统弹窗。
- 安装窗口始终居中显示：按当前显示器的可用区域（已避开任务栏）计算位置，多显示器与高倍缩放下
  都停在正中间，布局比屏幕大时也会完整落在屏幕内，不会跑到左上角。
- 拖动无标题栏的窗口不再跳动：从空白处按下标题栏时按屏幕坐标发消息，窗口随鼠标平稳移动。
- 对话框里的问句文字不再丢失：一行文字按百分比宽度排版时，跨轴高度会被正确测量，文字不会再
  被压成零高度。
- 确认框不再把按钮挤到底边：卡片高度改为「至少」这个值，按钮下方留出固定间距，文案换行或
  译文更长时卡片会自己变高，按钮不会被压到边框上。

### 已知限制

- 安装包尚未签名，Windows SmartScreen 仍会提示「未知发布者」。
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
- 对话框高度：`render_dialog_overlay` 把 `Page` 的 `height` 当最小值，新增 `dialog_intrinsic_height`
  （跳过 `is_hidden` 与 `position="absolute"` 的子元素；子元素声明百分比高度时改测其内容，否则用
  `outer_extent_along`），内容更高时卡片自动变高并在页面上重新居中。示例 `msgBox.xml` 的按钮行
  下方因此拿到 24px 留白，两条长译文（es/pt）会把卡片从 180 撑到 199。
- 导入表：新增 `ClientToScreen`、`MonitorFromWindow`、`GetMonitorInfoW`、`MONITORINFO`、
  `MONITOR_DEFAULTTONEAREST`、`SetWindowPos`、`HWND_TOP`、`SWP_NOZORDER`/`SWP_NOACTIVATE`、
  `VK_RETURN`、`VK_ESCAPE`；移除 `GetSystemMetrics` 与 `IDYES`/`MB_YESNO`/`MB_ICONQUESTION`。
- 示例 `examples/TapTap/layouts/msgBox.xml` 重写为圆角卡片布局（信息文案 + 「继续安装」/「退出安装」），
  `examples/TapTap/installer_config.json` 的 `ui` 段显式声明 `"dialog_layout": "layouts/msgBox.xml"`。
- 文档：`XML_LAYOUT_GUIDE`（新增「对话框」一节与 `dialog_ok`/`dialog_cancel`）、
  `CONFIG_REFERENCE`（新增 `ui.dialog_layout`）、`ARCHITECTURE`（提问与提示的渲染）、
  `PRODUCTION_STATUS` 与 `QUICK_START` 同步为皮肤确认框，中英双语。

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

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`。
- `cargo test --locked --workspace`（71 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）。
- `scripts/build.ps1 -Project examples\TapTap` 全量构建、ZIP/7z backend smoke test 与 Win7 PE 导入审计。
- 手工验收：路径框拖选高亮、单击清除选区、输入与 Backspace 生效；清理副本端到端实验确认目标目录与副本都被删除，临时脚本无残留。

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`。
- `cargo test --locked --workspace`（73 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）。
- 提权机制实验：向测试镜像注入 `requireAdministrator` 后，未提权父进程调用 `CreateProcessW`
  返回 740（`ERROR_ELEVATION_REQUIRED`）；`asInvoker` 则正常启动。
- `scripts/build.ps1 -Project examples\TapTap` 全量构建、ZIP/7z backend smoke test、Win7 PE
  导入审计（含新增的 `imm32.dll`，Windows 7 自带），以及新增的清单审计。
- 复现并验证修复：同一份脚本按 cp1252 解码时，加 BOM 前尾注变成乱码、加 BOM 后完整可读；
  该版本的 release 正文尾注已重新生成并确认正常。
- 新增 `scripts/verify_release_notes.ps1`：用发布流程相同的方式（含 `GITHUB_REPOSITORY`）跑一遍
  正文生成器，再与生成器源码里解码后的尾注逐字比对。它只在 ANSI 代码页不是 UTF-8 的机器上才能
  失败，因此接在 CI 上；本机把 BOM 去掉做负向验证时，`audit_script_encoding.ps1` 如期以非零退出
  拦截，而该端到端校验在 UTF-8 环境下不报错。
- 用 Python 直读 PE 资源复核：`TapTap_Setup.exe` 与内嵌 `uninst.exe` 均声明
  `requireAdministrator` 与 `<dpiAware>true`，原始 stub 无清单；把期望级别故意写成 `asInvoker`
  时审计如期失败。
- 文档链接检查：38 个文件、138 条相对链接、0 条失效。

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

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`（83 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）。
- 清单审计：`examples/TapTap/dist/TapTap_Setup.exe` 与内嵌卸载程序均为
  `level=requireAdministrator, dpiAware=true, dpiAwareness=PerMonitorV2, PerMonitor`。
- 实测 DPI 切换：DPI 由 192 变为 384 时窗口从 `720,462,2160,1362`（1440x900）变为
  `0,12,2880,1812`（2880x1800，工作区 `0,0,2880,1824`），即整页按新比例重新排版；
  带建议矩形 `100,50` 时窗口落在 `0,24,2880,1824`，仍完整位于工作区内。
- `PrintWindow` 截图确认 2880x1800 下界面按 2 倍比例重排，文字与图标清晰。
- 构建审计恢复全套通过：Win7 PE 导入审计、ZIP/7z backend smoke test、脚本编码审计。

- `cargo fmt --all -- --check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、
  `cargo test --locked --workspace`（84 个 core 测试通过、1 个忽略，12 个 GUI 测试，2 个 stub 测试）。
- 新增回归测试 `every_shipped_question_keeps_its_answers_inside_the_card`：遍历示例全部 11 种语言的
  `locales/*.json`，断言答案距卡片底部 ≥16px、横向在卡片内、问句不与答案重叠、卡片 ≥180 且居中。
  把 `msgBox.xml` 还原成旧版本时该测试如期失败（打印 `the answers are flush with the card bottom
  (card ends at 315, answers end at 314)`），恢复后通过。
- 实测修复后的按钮距卡片底边 24 逻辑像素；es/pt 两行译文时卡片由 180 自动增高到 199 并保持居中。
- `scripts/build.ps1` 全量构建（ZIP/7z smoke test、Win7 PE 导入审计、清单审计）；
  `scripts/verify_release_version.ps1` 23 条用例；文档链接检查 38 个文件、140 条相对链接、0 条失效。

### 未完成

- 无代码签名；未通过真实 Windows 7 SP1 虚拟机端到端验收。

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
