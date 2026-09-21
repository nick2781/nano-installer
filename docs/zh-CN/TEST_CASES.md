# 用例说明

这张表是测试报告里“检查什么”一列的中文文本，一条用例一行。报告里取不到这里的中文说明时，退回该用例的
Rust doc comment，再退回用例名。

| 用例 | 检查什么 |
| --- | --- |
| `a_border_layer_paints_a_ring_and_leaves_the_center_empty` | 带 `border-color` 的层画成一个环：边缘像素上色，内部留空；没有 `border-color` 的控件一个层都不加。 |
| `a_bound_label_shows_its_own_text_beside_the_value_it_reads` | 绑定了 `value-source` 的 `Label` 在自己那句翻译后面接上读到的值，同一份版面在任何语言下都读得通；没有值可显示时留下占位符而不是空一行。 |
| `a_broken_project_file_is_reported_to_the_user` | 读不出来的工程文件会被报告出来，而不是让程序崩溃。这些文件是人手改的，所以窗口必须扛得住改坏的那一份：用例先把 JSON 改坏、再把一份版面改坏，要求在侧栏和日志里看到原因，而不是 panic。 |
| `a_browse_button_opens_the_folder_picker_and_leaving_it_changes_nothing` | 有 `pick_directory` 动作的按钮点下去会开出 Windows 自己的选目录对话框：进程名下多出一个类名是外壳 `#32770` 的窗口，向导还在它底下。把这个对话框关掉，向导没消失、客户区尺寸没变、安装进程也还活着，而且这一路点下来一个文件都没落盘——离开对话框不等于做了选择。 |
| `a_build_reads_the_project_from_disk_in_its_worker_thread` | 构建在工作线程里从磁盘重读工程，而侧栏那份摘要只是上次检查的快照。在快照之后把工程文件删掉，构建必须失败并指出是哪个文件，因为工作线程会重新检查目录，而不是相信窗口上还显示着的东西。 |
| `a_built_in_step_clears_a_script_step_text` | 内置步骤用自己的 locale 键起名，脚本发布过的文字不会留到这些步骤里；进度同时被夹在 0 到 100 之间，脚本推不过头。 |
| `a_built_setup_carries_a_readable_bundle_and_real_resources` | 构建产物是 stub、捆绑数据和 Windows 资源三部分，捆绑数据要在文件最末尾描述自己，运行时才找得到。 |
| `a_built_setup_installs_its_payload_and_registers_an_uninstall_entry` | 安装包存在的意义：payload 落到磁盘上，manifest 记下写了哪些东西，卸载项在注册表里登记，payload 自己的字节也没在途中被改动。 |
| `a_button_state_image_falls_back_to_the_normal_one` | 只画了部分状态的版面照样画得出按钮：缺 `hover-image`、`pressed-image` 或 `disabled-image` 时退回 `normal-image`。被条件挡住的按钮优先用 `disabled-image`，没有 id 的控件则完全收不到悬停和按下。 |
| `a_button_waits_for_each_state_its_condition_can_name` | `enabled-when` 让一个控件取决于另一个控件，指南列的 `checked`、`unchecked`、`visible`、`hidden` 四种状态按它们点名的复选框或面板判断；条件写成一串用逗号隔开的条件时全部成立才算成立，多打的逗号留下的空条件算不成立；运行时看不懂的状态、页面上找不到的控件都会把按钮挡住而不是放过点击，没写条件的按钮可用，说明这个属性是可选的。 |
| `a_button_waits_for_the_field_its_condition_names` | `enabled-when` 点名一个输入框时，按钮等的是那个值合不合格：字段还空着（`required` 说的就是这种）时安装键既不登记点击区域也不登记悬停，填进一个工程接受的路径就把点击还回来，清空后又收回去；同一个页面上写 `dir:invalid` 的那个按钮正好相反。 |
| `a_caret_sits_after_the_characters_before_it` | 文本光标画在前面的字符之后，位置随下标右移；下标超出文本长度时仍然留在输入框内。 |
| `a_cancel_button_stops_the_project_script_and_leaves_nothing_installed` | 页面上放一个 `action="cancel"` 的按钮，在真窗口里点它：正在跑的任务在脚本那一步停下，向导回到任务起始的那一页，脚本已经建出来的目录被撤掉，一个字节都没留下。 |
| `a_cancel_request_stops_the_checkpoints_that_follow_it` | 收到取消请求的检查点报出 `cancelled by the user`，而且带的是它自己的错误类型，向导据此说"已取消"而不是"失败"；取消只作用于这一个任务，下一个任务拿到的是全新的句柄。 |
| `a_cancelled_deployment_writes_nothing` | 部署一开始就已经被要求取消时，一个文件都不会复制；回滚随后把这次新建的安装目录整个删掉。 |
| `a_cancelled_install_gives_up_after_the_script_and_undoes_what_it_wrote` | 脚本自己注意到取消、结束长步骤并正常返回后，运行时在脚本之后的检查点停下，报 `cancelled by the user`，脚本写下的目录一并撤回。 |
| `a_click_on_a_radio_is_the_value_the_install_waits_for` | 在真实窗口里点单选按钮：安装键等的是被点中那一行的取值。先点安装键没反应；点亮版面默认选中的那一行再点安装，仍然没反应；改点另一行再点安装，窗口切到任务页，产品装进配置指定的目录。版面级用例只证明点击会变成一个动作，这条证明窗口真的记下了被点的是哪一行。 |
| `a_closed_language_select_draws_its_arrow_over_its_fill_and_outline` | 收起的 `Select` 由底色、描边和箭头三层组成，箭头从远端边向内缩一段并垂直居中，跟着显示缩放一起变大；收起时画向下的那张，展开时画向上的那张。 |
| `a_configured_percent_path_is_expanded_and_used` | 配置里带 `%LOCALAPPDATA%` 的路径必须先展开再用，没展开的路径不是绝对路径，安装会拒绝相对目录。这条用例完全不带 `--dir` 运行，等同于静默运行里没有指定目录的情况。 |
| `a_container_measures_the_edge_its_children_are_asked_for` | 问 `HBox` 要竖直方向的尺寸时，报的是最高的那个子项加上自己的上下内边距；问水平方向时报子项沿宽度要的总和。内嵌的百分比宽度容器透过它自己的子项来量，不会把外层的行撑大或压塌。 |
| `a_dialog_button_answers_with_its_own_action` | 对话框里的按钮按自己声明的动作登记点击区域，确认键给出 `DialogOk`，取消键给出 `DialogCancel`。 |
| `a_dialog_button_is_drawn_from_the_question_rather_than_the_layout` | 同一份对话框版面服务所有问题，按钮上的字来自对话框状态本身，也就是问题、确认和取消三个角色；对话框没提供文字的角色不显示内容，版面里写死的占位文字也不会顶上来。 |
| `a_dialog_is_drawn_over_the_page_and_centred` | 对话框画在页面之上，比页面小时居中放置，它自己的版面因此可以用普通坐标系；画出来的问题文字来自对话框，而不是版面里的占位文字。 |
| `a_disabled_button_registers_no_click_and_no_hover` | 条件不满足时按钮是惰性的，既不登记点击区域也不登记悬停区域；条件满足后恢复成它声明的动作，区域就是布局给它的那个矩形。 |
| `a_display_scales_the_layout_by_its_own_dpi` | 开着 DPI 感知时，96 DPI 是 1.0 倍，144 和 192 DPI 分别是 1.5 和 2.0 倍，120 DPI 是 1.25 倍但仍取 1x 素材。关掉 DPI 感知的工程不管落在什么显示器上都留在 96 DPI 基准，缩放交给外壳去做。 |
| `a_double_click_selects_the_word_under_the_pointer` | 双击选中指针下的那个词：字母和数字连成一段，路径分隔符单独一个，空白按一段算，下划线算词的一部分，点在文本之外没有东西可选。 |
| `a_failed_upgrade_restores_the_previous_version` | 升级中途失败（这里让注册表登记报错）会回滚到上个版本：旧文件和旧 manifest 都还原，新 payload 的文件不在，manifest 里也只有旧文件。 |
| `a_failing_install_script_removes_what_it_wrote` | 安装脚本抛错时，它写过的东西被清掉，错误里带着脚本抛出的那句消息，安装目录也不留残余。 |
| `a_field_checks_the_value_the_project_asks_it_to` | 输入框自己的规矩就写在版面上：`required` 管值有没有，`min-length` 与 `max-length` 按字符数算（中文按字算，不按字节），`pattern` 是掩码而不是正则——`*` 是任意长的一段（可以为空），`?` 恰好一个字符，整段值都要对得上，所以安装目录写 `?:*` 就是要一个盘符开头的路径。没写规矩的字段一律合格，可留空的字段空着也合格；值最先破坏的那条规矩留下自己那句文案，没写文案的规矩只让字段不合格、什么也不说。 |
| `a_field_the_user_fills_in_is_what_lets_the_install_start` | 在真窗口里从头走一遍要填字段的流程：字段空着时点安装键没有反应，只勾同意也不行，往字段里逐字符敲进一个路径之后安装才开始，产品落在敲进去的那个目录里。字段、按钮条件和安装动作三件事里任何一环没接上，它都会停在原地。 |
| `a_file_type_that_would_write_outside_the_classes_tree_is_refused` | 文件类型的两个名字先查再用：扩展名或程序 id 里带路径分隔符、或其中任何一个为空，调用直接返回 `false`；命令行空着的调用同样被拒绝，因为那样的文件类型打不开任何东西。五种被拒的调用一个字节都没写进注册表，manifest 里也没有记录。 |
| `a_hidden_element_takes_its_whole_subtree_with_it` | 祖先上的 `visible="false"` 把底下整个子树都藏起来：子控件的动作、文字和面板自己的底色都不画，页面上别的地方不受影响。把属性改回 `true` 后这个分支又完整画出来。 |
| `a_hint_shows_the_rule_the_value_breaks` | 绑 `value-source="field-error:<字段 id>"` 的标签画出那条被破坏的规矩写下的文案：字段空着时报 `required` 那句，值不合格时报 `pattern` 那句，文案按工程自己的语言查表；值合格时这个标签什么都不画，点名页面上没有的字段也一样。 |
| `a_hover_and_a_press_show_the_pictures_the_button_declares` | 在真窗口里驱动指针：指针落到按钮上，窗口画的是 `hover-image`；按住不放，画的是 `pressed-image`；指针离开窗口，按钮回到 `normal-image`，而且整帧和指针来之前一个像素都不差。三种状态图和页面底色各是一种纯色，所以读按钮中心那一个点就知道此刻挂着哪一张；再看改动有没有落到按钮矩形之外，就知道变的是按钮，而不是整页重画了一遍。 |
| `a_label_takes_its_text_font_and_alignment_from_the_layout` | `Label` 的内容全是文字，字号、加粗、颜色和对齐都按版面写的那样生效，`value` 和 `text` 一样被接受；没写对齐的标签从自己的左边缘开始，字号也跟着显示缩放走。 |
| `a_language_menu_lists_its_options_and_marks_the_one_in_use` | 展开的语言菜单按选项一行一个地画在页面之上，当前语言那一行填上选中底色，`visible="false"` 的选项不出现在列表里；点某一行会给出切换到那个语言的区域。 |
| `a_layout_picks_the_image_density_the_display_asks_for` | 版面只点一个文件名，由运行时选版本：低密度显示器用 1x，高密度用 `@2x`。版面直接写 `@2x` 的文件名也会被归一化，只发布其中一个版本时退回另一个。 |
| `a_link_the_project_does_not_configure_stays_plain_text` | 指南的解析顺序最后一条是解析不到目标的链接保持普通文字：字照常显示，点了什么也不发生，标签不会因此画不出来。 |
| `a_nested_container_reports_the_extent_its_children_need` | 没声明尺寸的面板有多大由内容决定：沿自己的轴把子项和间距相加，垂直于轴取最大的子项，自己的内边距只算一次。外层容器用同样的量法放置它，版面不必给包装层声明尺寸。 |
| `a_next_button_walks_to_the_page_the_project_declares` | 在真实窗口里点一次 `next`，向导要切到工程声明的第二页，再点 `back` 要回到第一页：两页声明的客户区不同，所以窗口尺寸就是证据。 |
| `a_notice_hides_the_secondary_button` | 通知只有一个答复，版面里的取消键不画也不能点；换成提问时取消键又画回来，一份对话框版面因此可以两用。 |
| `a_page_list_without_roles_keeps_its_positions` | 没有写 `role` 的页面列表保持老规矩：第二页汇报进度、最后一页收尾；只有一页的列表既没有可汇报的页也没有可收尾的页。 |
| `a_page_paints_its_fill_under_its_image_and_its_outline_over_them` | 页面的装饰按指南的顺序落层：底色、铺在它上面的背景图、从边缘向内画的描边，`border-radius` 转成窗口区域上报而不是画进图层，倍率放大时圆角跟着放大。完全没写尺寸的页面退回文档里的默认客户区大小。 |
| `a_page_role_finds_the_page_that_holds_it` | 页面用 `role` 声明自己的职责时，任务报到写 `progress` 的那一页，而不是第二页；收尾同样按 `finish` 走，工程因此可以自由排页。 |
| `a_page_without_a_dialog_draws_no_overlay` | 没有对话框时，版面不报对话框，也不产生任何覆盖层或覆盖文字。 |
| `a_panel_pair_shows_the_panel_and_only_the_control_that_fits` | `toggle_panel:<id>:show` 和 `:hide` 是面板两侧的两个控件：面板收起时只画展开那个，展开后两个换过来，面板自己的底色也跟着出现或消失。 |
| `a_payload_without_the_declared_executable_is_refused` | `install.exe_name` 指明 payload 里必须有的那个可执行文件。部署一份不含它的 payload，装出来的产品启动不了，所以这次运行在写任何东西之前就停下来。 |
| `a_placed_window_is_pulled_back_inside_its_work_area` | 建议位置已经放得下就不动它；探出工作区右边或下边的窗口被拉回来；来自左侧副显示器的负坐标不会把它顶出去；比工作区还大的窗口缩到工作区大小。 |
| `a_progress_bar_paints_a_rounded_track_and_follows_the_live_value` | 进度条先画轨道，`border-radius` 把它修成胶囊形；版面里写的 `progress` 是空闲时的样子，任务上报的进度会盖住它，任务还没开始时只画轨道，没有任务时又回到版面写的值。 |
| `a_project_bundles_the_tools_directory_it_names` | 工程把 `resources.tools_dir` 指到的目录整个打进安装包，子目录和它们的相对路径都在内：脚本拿到的是工程自己那份目录的布局，而不是一堆压平的文件名。 |
| `a_project_that_did_not_opt_in_refuses_a_windowless_run` | 无窗口运行是工程自己的决定，从没声明过它的工程既不能被无人值守地安装，也不能被无人值守地卸载。 |
| `a_project_that_names_no_tools_bundles_none` | 没写 `resources.tools_dir` 的工程，目录就算摆在自己的树里也不进包：安装包只带工程点名要的东西，不带碰巧放在旁边的东西。 |
| `a_project_without_a_dialog_layout_still_opens` | 没带对话框版面的工程照样能用：页面自己画出来，调用方退回成不问直接关闭。 |
| `a_project_without_silent_support_refuses_a_windowless_install` | 从没声明支持静默安装的工程必须拒绝无窗口运行，而不是照样无人值守地装下去。 |
| `a_project_without_silent_support_refuses_a_windowless_uninstall` | 卸载一个从没声明支持静默的工程会被拒绝，产品不能靠作者没同意过的开关被无人值守地删掉。 |
| `a_question_a_script_asks_is_drawn_with_both_of_its_answers` | 脚本提出的问题带着两个答案一起画出来：卡片上同时有确认和取消两个按钮各自的位置，问题正文和 `yes`/`no` 两个标签都用传给脚本调用的那几个词。`ask_yes_no` 要等一个答案，只有一个按钮的卡片会让脚本永远拿不到另一种回答。 |
| `a_radio_group_holds_one_value_at_a_time` | 单选按钮按组记值：版面用 `checked="true"` 标出起始选中的一行，点任意一行就为整组记下那一行的值，选中图和旁边按钮的可用状态跟着换，一组任何时刻只有一行是选中的。 |
| `a_readonly_field_shows_its_value_without_taking_edits` | `readonly="true"` 的输入框仍然把值画出来，但不接受键入，也不会被记成可编辑字段；`readonly="false"` 则照常可编辑。 |
| `a_run_without_any_install_path_is_refused` | 命令行和配置都没有给出目录可供退而求其次，这次运行会停下，并在提示里点明提供安装路径的两条途径。 |
| `a_running_build_refuses_a_second_one` | 正在跑的构建会拒绝第二个打包任务。有任务在跑时按钮是禁用的，这条用例就是按钮背后那道检查：同时开两个会把同一个输出文件写坏。 |
| `a_running_script_sees_the_cancel_request` | 任务运行中脚本里的 `is_cancelled()` 会变成 `true`：用例像窗口那样从另一个线程在 50 毫秒后提出取消，脚本的等待循环随即退出，并报出自己等了多久。 |
| `a_script_deletes_the_desktop_shortcut_and_the_start_menu_folder_it_created` | 卸载脚本用 `delete_desktop_shortcut` 和 `delete_start_menu_folder` 删掉安装时建的桌面快捷方式和开始菜单文件夹，两个原语各自报告自己删掉了东西，事后链接和文件夹都不在了。 |
| `a_script_dialog_is_drawn_in_the_wizard` | 脚本的提示与提问画在向导窗口里，用的是产品自己的皮肤，点一下卡片就把答案交回正在等待的脚本。用例自己写了一份 400x180 的卡片版面，两个按钮摆到它点得到的位置：先证明卡片亮出来时脚本还停在原地（回答文件此刻不存在），再逐个点中卡片上的确认键，让 `ask_yes_no` 拿到答案、`show_message` 和 `show_error` 被收起，每答一次就检查脚本接下来写下的那个文件，最后窗口走到完成页。卡片要是像原来那样另开系统对话框，这些点就会落空。 |
| `a_script_failure_reports_the_messages_it_logged` | 脚本失败时，它之前用 `log_warn` 之类写下的日志跟着错误一起报出来，作者能看到最后那几行。 |
| `a_script_reads_the_tools_the_project_bundled` | `get_tools_dir()` 把打包进来的工具摊到磁盘上并返回目录路径，嵌套文件按原相对路径读得到、内容一致；再问一次返回同一个目录，不会重复摊一遍。 |
| `a_script_recognises_a_running_process_by_its_image_name` | `is_process_running` 按映像名判断进程：正在跑的那个测试可执行文件返回 true，编出来的不存在名字返回 false。这条检查就是安装时不肯覆盖正在运行的产品的原因。 |
| `a_script_registers_a_file_type_where_windows_reads_it` | 脚本登记的文件类型落在 Windows 真正读取的四个位置：扩展名指向程序 id，程序 id 上挂着资源管理器显示的类型名、图标，以及带 `"%1"` 的文件命令行。安装把它们全部记入 manifest，卸载时这个文件类型连同程序 id 一起从注册表里消失。 |
| `a_script_runs_a_command_and_sees_its_exit_code` | `run_command` 返回命令的退出码，用 `ComSpec` 跑 `exit 3` 和 `exit 0` 分别拿到 3 和 0；起不来的命令返回 -1，脚本因此分得清跑了但失败和根本没跑。 |
| `a_script_step_text_wins_over_the_locale_key` | 脚本发布的字面状态文字会留在屏幕上，即使更早步骤记下的 locale 键还在。 |
| `a_script_that_asks_for_tools_a_project_did_not_bundle_gets_nothing` | 没写 `resources.tools_dir` 的工程，以及写了这项设置但包里没有对应条目的安装包，`get_tools_dir()` 都返回空字符串并在日志里留一条告警，安装照常完成——没有那个程序时怎么办，由脚本自己决定。 |
| `a_scrollable_container_shows_the_part_it_is_scrolled_to` | 装不下容器的那部分内容只从窗口里露出一块：偏移量把各行整体推上去，推出容器边缘的那一行既不画出来也不再登记点击，偏移量超出列表末尾时停在末尾，不会露出底下的空白。列表有多长由各行自己声明的高度决定，跟容器拿到多少地方无关，这正是「能滚」与「被压扁」的分界。 |
| `a_scrollbar_says_where_the_list_stands` | 滚动条画在视口尾部那条 8 像素宽的轨道上，滑块的长度是列表露出来的那部分所占的比例，位置跟着偏移量走；点轨道上滑块之外的两段各把视图挪动一页（一页就是视口本身的大小），所以不拖滑块也能翻。列表装得下时不画轨道、也不登记翻页区域；声明了 `scrollable` 却没有 `id` 的容器不开滚动，免得页面上的无名列表共用一个位置。 |
| `a_select_offers_the_options_the_page_declares` | 下拉框是页面提供的一种选择，不只是语言控件：关闭时显示当前选项的文字（没人点过时是第一个），点击它要的是自己的菜单而不是语言列表，展开的选项按版面顺序排列、各用各的文字，隐藏的选项不出现；被选中的值决定旁边按钮是否可点。 |
| `a_selection_band_covers_the_characters_it_selects` | 选中区域画出一条色带盖住选中的字符：没选中的范围什么都不画，色带始终停在输入框内，纵向也留出与文字高度匹配的位置。 |
| `a_selection_is_ordered_from_whichever_end_the_caret_is_at` | 选区按两端排好序，从哪头拖都得到同一段区间；光标和锚点重合不算选区，清空后也没有选区。 |
| `a_setup_runs_the_projects_own_install_and_uninstall_scripts` | 自带步骤的工程会把这些步骤带进安装包，由真实运行时执行；进程内的脚本用例直接驱动脚本驱动层，而它和工程目录之间还隔着把 `scripts/` 打进捆绑数据、再由 stub 找回来这两件事。少了其中任何一件的安装包，仍然能让那批进程内用例全部通过。 |
| `a_setup_with_a_signature_appended_still_installs` | 被集成方签过名的安装包还是安装包：签名属于发布流水线而不是构建器，Authenticode 会把证书表追加在构建写下的所有内容之后，页脚也在内。只看自己文件最后几个字节的运行时会把这种包认成没有捆绑数据而拒绝安装，所以签过名的安装包必须扛得住。 |
| `a_setup_unpacks_the_tools_its_project_bundles` | 工程用 `resources.tools_dir` 打包的辅助程序确实进了安装包：安装时脚本从 `get_tools_dir()` 拿到的目录里，那个批处理文件逐字节和工程里的一致（安装包跑起来的时候，工程目录已经不在旁边了），而且能被 `run_command` 真的跑起来，返回它自己声明的退出码 7。 |
| `a_shrinking_row_stops_at_the_minimum_its_items_declare` | `flex-shrink` 让一行容得下文字旁边的固定按钮，`min-width` 保住控件还能读：行宁可溢出，也不会把某个项压到版面声明的下限以下。`flex-shrink="0"` 的按钮保持设计宽度，溢出全由可以收缩的那个项承担。 |
| `a_silent_install_writes_the_shortcuts_and_the_autostart_entry` | 无窗口安装没有复选框可读，只能照工程里的默认值处理快捷方式和自启动，它写下的 manifest 则列出安装目录之外创建的每个文件和注册表值。这些条目是用户还没启动产品就先碰到的东西，也是安装唯一写到自身目录之外的内容；跳过它们的安装包照样装得成功，只是会留下一个再也没人回收的开始菜单项。 |
| `a_spacer_takes_what_the_fixed_items_leave` | `Spacer` 自己不画东西，它把后面的项推到另一端：两个定宽按钮之间剩下的 200 像素全被它吸收。 |
| `a_styled_image_draws_into_a_sub_rectangle_at_the_opacity_it_declares` | `file='...' dest='...' fade='...'` 这种写法把图片画进控件内的一个子矩形，并按 `fade` 给透明度；目标矩形相对控件而不是页面，跟着控件一起被缩放。 |
| `a_supported_locale_without_a_file_is_reported` | 工程声明支持、却没有对应语言文件的语言会被报告出来，因为别的环节不会报：运行时会退回默认语言，产品只是显示成另一种语言而已。 |
| `a_translation_missing_page_text_is_reported` | 默认语言能回答的页面文案键，凡是某个语言漏掉的都要报出来，没漏的不报。 |
| `a_typed_value_wins_over_the_bound_default` | 绑定到工程文件的输入框先显示配置里的路径，但用户输入或选择过的值要一直留在屏幕上，`disk-free:` 绑定读的也是同一个输入框。 |
| `a_validation_message_the_page_asks_for_is_reported` | 字段不合格时显示的文案也是页面文案，构建期和别的键一样逐语言比对：默认语言里有、某个语言文件里漏掉的那条会被报出来，而不是等用户看到一句没翻译好的提示。 |
| `a_wheel_over_a_list_brings_the_rows_below_into_reach` | 在真实窗口里滚滚轮：先点列表下方那片本该被后面一行盖住的位置，接住点击的是页面自己放在那儿的按钮，说明被裁掉的行确实收不到点击；在列表上滚一格滚轮之后，点同一个位置落在刚滚进来的那一行上，改由它接管。滚轮消息带的是屏幕坐标，这条用例走的就是系统把消息交给窗口时的那条路。 |
| `a_window_is_centred_and_clamped_to_its_work_area` | 窗口在工作区里居中，工作区不从原点开始时保留它自己的偏移；比桌面还大的版面被夹到桌面范围内而不是挂在边缘外，只有一个方向超出时另一个方向照常居中。 |
| `a_wrapping_row_gives_each_line_the_height_of_its_tallest_item` | 窗口变窄时换行行重新排布，下一行从上一行最高那个项的下方开始，再加上间距，高度不同的卡片因此不会互相压住。 |
| `a_wrapping_row_starts_a_new_line_when_the_next_item_does_not_fit` | 放不下下一个项时换行行另起一行；整行放得下就不换；比行还宽的项自己占一行而不是被丢掉；空容器没有行。 |
| `accepts_a_configuration_of_read_settings` | 一份只写了本项目真会读的设置的配置能通过检查：每个区块的合法键都试一遍，`links` 这种由工程自己命名的表不在管辖之内。 |
| `action_attributes_map_to_window_actions` | `action` 属性映射到窗口动作：`close_confirm`、`open_url:`、`pick_directory` 各自落到对应的动作上，链接表的键先解析成 URL，直接写 URL 的照原样用，没配置的键什么也不做。 |
| `agreement_links_resolve_through_the_project_links_table` | 链接名通过工程的 `links` 表解析：示例语言里用的 `agreement`、`policy` 别名和完整的 `terms_of_service` 都要落到配置的 URL 上，绝对 URL 绕过这张表，表里没有的名字解析不出目标。 |
| `agreement_markdown_becomes_colored_visible_runs` | 协议句里的 Markdown 链接变成按链接色着色的文字段：方括号和圆括号被去掉，链接名留在原处，普通文字仍旧用原来的颜色。 |
| `align_self_overrides_the_alignment_of_its_container` | `align-items` 决定十字轴上的对齐，单个项可以用 `align-self` 脱离它：横向的行里一项居中、一项贴底、一项贴顶；纵向容器量的是页面宽度，`align-self="end"` 于是把项推到右边。 |
| `an_absolute_image_and_icon_draw_at_the_rectangle_they_declare` | `Image` 和 `Icon` 是自带图片的标签，绝对定位加上声明的宽高才会出现在指定的矩形里，图片按自己的尺寸解码；没有尺寸的控件没有东西可画。 |
| `an_absolutely_placed_checkbox_draws_its_state_image_and_toggles` | 给了坐标的复选框和在流式容器里的行为一样：按状态画出对应的图，并登记翻转该状态的点击。随包发布的卸载页把它的保留数据框绝对定位，那个页级复选框既拿不到状态图也拿不到点击区域时，框就画成了一段没人能点的说明文字。 |
| `an_asset_without_its_density_pair_is_reported` | 图片是画页面时按密度选的，缺了另一半的文件只会在另一种缩放比例的显示器上才露馅。构建两个方向都报：只有 1x 没有 2x 的，和只有 2x 没有 1x 的。 |
| `an_element_answers_the_pointer_only_when_it_declares_an_action` | 元素只有声明了 `action` 才响应指针：没有动作的标签、图片和按钮即使盖住同一块地方也不登记，装饰因此吞不掉点击。声明了动作的图片给出 `pick_directory`，`Select` 给出切换语言菜单的动作。 |
| `an_element_is_pinned_by_the_edge_attribute_it_carries` | `right` 和 `bottom` 从远端边量起，`inset` 是一次写四边的简写；声明了近端边时以它为准，单边写法能覆盖简写，页面本身就是绝对定位控件的参照父级。 |
| `an_empty_field_is_one_a_user_can_click_into` | 空着的输入框仍然是输入框：它一个字的文字都不画，却照样被记成可编辑字段，光标也按它声明的字号和颜色落进去——用户就是靠这一步才能把页面要的路径敲进来。`readonly="true"` 的字段和别人一样不在这份名单里；待在流式容器里的字段同样被记下，位置跟着容器给它的槽位。 |
| `an_empty_runtime_directory_leaves_the_stub_search_automatic` | 运行时目录为空时 stub 的查找保持自动：勾上自动搜索 stub 会清空这个输入框，而把空路径当目录传下去会先在那里搜并且搜不到。所以请求里根本不能带这个覆盖项，预览里也不能显示它。 |
| `an_explicit_directory_wins_over_the_configured_one` | 命令行给出的目录优先于配置里的目录，静默运行才能自己决定产品装到哪里。 |
| `an_explicit_install_path_wins_over_the_configured_one` | 显式给出的目录优先于配置里的目录，这样不改工程也能让静默运行决定产品装到哪里。 |
| `an_input_method_anchors_at_the_caret_the_page_drew` | 输入法组字窗与候选窗锚在页面画出的那个插入符上：组字点就是插入符的左上角，候选点在同一个 x 上、比插入符低一个插入符的高度，用户打字时眼睛就落在那里；光标沿着一行往右走，两个点跟着一起走，候选列表不会留在第一个字底下。输入法自己的窗口不归运行时管，运行时只负责回答插入符画在哪儿。 |
| `an_install_closes_a_running_copy_of_the_product` | 工程可以要求先关掉正在运行的自身副本，应用开着也能就地升级。 |
| `an_install_script_creates_shortcuts_the_uninstall_takes_back` | 安装脚本用快捷方式原语建的桌面、开始菜单和卸载链接都落在 Windows 会去找的位置，并被 manifest 逐条记下。卸载时三个链接和安装器自己建的开始菜单文件夹一起删掉，菜单里不留空的产品目录。 |
| `an_install_script_deploys_files_and_writes_the_manifest` | 安装脚本部署文件后，manifest 列出它管理的文件和注册表值，说明注册表根是 `HKCU`；卸载器和 manifest 自己管自己，不在卸载时删除的文件清单里，产品名也从注册表读得到。 |
| `an_install_script_removes_a_variable_it_no_longer_wants` | `remove_env` 删掉机器上本来就有的那个变量，返回 `true`；它同时把这条记录从待撤销清单里去掉，所以卸载不会再去删一个已经不在的值，也不会把 `Environment` 这个键当成自己的删掉。 |
| `an_install_script_sets_a_variable_the_uninstall_takes_back` | `set_env` 把变量写进 Windows 读环境变量的那个键，装完之后新进程就能读到；脚本自己的进程仍保留启动时的环境，这与 Windows 对写入者的行为一致。manifest 只记这个值（不记键），卸载把值撤回去，`Environment` 键连同里面的 PATH 都留着。 |
| `an_install_script_that_never_deploys_the_executable_is_refused` | 脚本没部署 `install.exe_name` 指定的可执行文件时安装被拒绝，错误里点名缺的是哪个文件；失败的一次不会留下写了一半的目录。 |
| `an_uninstall_script_replays_the_manifest_it_asks_for` | 卸载脚本调用 `run_tracked_uninstall` 后，manifest 记下的文件和注册表值都被清掉，manifest 自己也不在了。 |
| `an_uninstall_script_sees_the_uninstall_mode_and_the_keep_data_checkbox` | 卸载脚本在两种勾选状态下都看得到 `get_mode()` 返回 uninstall，`get_checkbox_value("keep_data")` 如实反映复选框，没登记过的复选框返回 false，`is_cancelled()` 返回 false。 |
| `an_uninstall_script_that_skips_the_manifest_still_removes_the_product` | 卸载脚本没调用 `run_tracked_uninstall` 时，库自己回退着把 manifest 记下的东西删掉，产品仍然被卸干净。 |
| `an_unknown_silent_option_is_refused` | 写错的选项必须让这次运行停下。改成装进配置的默认目录，会把文件放到没人要求的位置。 |
| `an_upgrade_keeps_a_file_the_payload_does_not_own` | 产品自己写的本地设置文件不属于 payload，升级时必须原样留着。 |
| `an_upgrade_replaces_the_previous_version_and_drops_stale_files` | 覆盖安装把上个版本的文件换成本次的，旧版本里不再有的文件被删掉，manifest 里只剩这次部署的文件。 |
| `bottom_hbox_distributes_fixed_and_flexible_items` | 底部一行的宽度分配：两个可伸缩项平分剩余空间，定宽按钮保持 184 像素，算出 196、196、184。 |
| `build_messages_reach_the_log_in_the_order_the_worker_sent_them` | 构建消息按工作线程发出的顺序进入日志，行文由构建器决定，包括 `Uninstaller bundle:` 标题下的第一遍和第二遍里重复出现的目录名。窗口只是转发，用例喂进那串消息后检查途中没有合并也没有丢行、payload 只被点名一次、两遍都还看得见。 |
| `build_progress_is_monotonic` | 构建各阶段的进度值依次递增，从校验、选 stub、打包、写资源一路到完成，不会往回走。 |
| `build_setup_rechecks_a_project_marked_dirty_before_it_starts` | 构建开始前会重读屏幕上改动过的工程：编辑工程目录会把摘要标成过期，构建必须先重新检查目录，文件读不出来时拒绝，而不是打包侧栏还在描述的那份。 |
| `builds_unicode_version_resource` | 版本信息资源按 UTF-16 写入，公司名和版权里的非 ASCII 文字能原样读回来，产出的资源结构也完整。 |
| `bundle_index_ignores_images_without_a_footer` | 没有页脚的镜像不算捆绑数据：随便一个文件读出来没有索引，页脚长度字段比文件本身还大的那份被当成错误，而不是有效的捆绑数据。 |
| `bundle_index_reads_a_bundle_that_a_signature_follows` | 签过名的安装包还是安装包，而 Authenticode 把证书表追加在构建写下的所有内容之后，从文件最后几个字节读页脚的做法正好会把它报成根本没有捆绑数据。 |
| `bundle_index_streams_entries_without_loading_the_payload` | 捆绑索引按条目读取：3MB 多的 payload 读回来和打包的字节完全一致，流式复制到磁盘的文件也逐字节相同，界面用的文件集里没有 payload，读不存在的条目报错且不留下文件。 |
| `byte_index_walks_characters_not_bytes` | 字符下标转成字节下标时按字符边界走，多字节字符的字段里插入文字不会破坏前后字符；下标超出长度就停在末尾。 |
| `command_arguments_are_quoted` | 命令行参数带空格时加上引号，参数里的引号被转义。 |
| `configured_install_paths_are_expanded` | 配置里的默认路径写着环境变量，没展开的 `%LOCALAPPDATA%\Product` 不是绝对路径，跳过这一步的安装会在写下任何东西之前被拒绝。 |
| `control_padding_insets_what_the_control_draws` | 控件上的 `padding` 把它的内容往里缩：暂停按钮写 `padding="25 0 0 0"`，标签就从自己流式槽位的顶部下移 25 像素，文字可用的高度也跟着变小。 |
| `dpi_asset_resolution_prefers_requested_density_and_falls_back` | 素材路径按密度解析：要 2x 时选 `@2x` 文件，只要 1x 时选基础文件；版面直接写 `@2x` 时在低密度下退回基础文件，只有基础文件时高密度下也用它。 |
| `dpi_scaling_rounds_layout_coordinates` | 布局坐标按缩放倍率换算并取整，负坐标同样处理。 |
| `drops_the_shortcut_folder_once_it_is_empty` | 快捷方式删掉后，开始菜单里那个产品文件夹空了就被删掉，而共享的开始菜单根目录永远不动。 |
| `each_container_tag_accepts_the_alignment_spelling_it_documents` | 每种容器标签接受指南里写的对齐写法：`HBox` 横排、`VBox` 纵排，`Content` 只有写了 `layout` 才决定方向；`horizontal-align`、`vertical-align` 指的是该标签的主轴和十字轴，`align-self` 让单项脱离容器的对齐。 |
| `editable_text_fields_are_recorded_and_readonly_ones_are_not` | 只有能输入的 `TextInput` 被记成可编辑字段，带 `readonly="true"` 的那个不记，但两个都把值画出来。 |
| `every_action_in_the_table_answers_with_its_own_window_action` | 动作表里的每一行都映射到它点名的窗口动作：最小化、关闭、确认关闭、选目录、开链接、安装、卸载、启动应用、完成、切换语言、显示或隐藏面板、对话框确认与取消。表里没有的名字让控件保持惰性，不会误关向导。 |
| `every_log_level_reaches_the_failure_the_wizard_shows` | 脚本失败时，info、warn、error 三种级别的日志都跟着错误一起报出来，作者能从向导的错误框里看出是哪一步失败、为什么。 |
| `every_shipped_question_keeps_its_answers_inside_the_card` | 对话框的按钮必须留在包住它们的那张卡片里：问题文字来自产品自己的翻译，句子一长就折到第二行，而卡片过去是固定高度、按钮贴着底边，两行的问题会把它们顶出边框。这里把随包发布的每种语言都查一遍，因为句子最长的那些语言恰恰只在用户机器上才露馅。 |
| `file_primitives_create_copy_list_and_remove_files` | 文件原语逐个核对结果：建目录、写文件、判断存在与是否为目录、取文件大小（缺文件是 -1）、读文本（缺文件读成空）、列目录、复制、删除（重复删也成功）、递归删目录，以及相对路径的递归删除被拒绝。 |
| `flex_wrap_is_opt_in_per_container` | `flex-wrap` 是每个容器各自选的：写 `true` 或 `wrap` 才换行，没写或写 `false` 都不换。 |
| `flow_attributes_become_the_item_a_container_shares_space_with` | 容器通过这些字段读子项，所以 `flex-basis`、`flex-grow`、`flex-shrink`、`min-width` 和内边距外边距都要被读进去：有 `flex-basis` 的项先占位再分剩余空间，没写的按宽度加内边距和外边距算，`Spacer` 只声明可增长，纵向容器从另一条边读同样的属性。 |
| `flow_items_honour_align_self_basis_and_anchored_edges` | 流式项既遵守容器的 `align-items` 和 `align-self`，也遵守 `right`、`bottom` 这类贴边属性：居中的按钮落在行的中间，`align-self="start"` 的贴顶，写死右边和下边的按钮落在页面那个角上。 |
| `formats_bound_disk_sizes` | 绑定磁盘容量时按 MB、GB 这样的单位格式化字节数。 |
| `formats_gui_sizes` | 构建器里显示的字节数按 KiB、MiB 格式化，并保留相应的小数位。 |
| `hbox_shrinks_text_before_fixed_button` | 一行放不下时，可收缩的文本项先把空间让出去，定宽按钮保持 184 像素，结果是 392、0、184。 |
| `ignores_data_paths_when_the_project_declares_none` | 工程没声明 `uninstall.data_paths` 时，保留用户数据的路径列表为空。 |
| `image_style_supports_plain_path_destination_and_fade` | 图片写法两种都认：只写路径时整块控件都用这张图、完全不透明；写 `file`、`dest`、`fade` 时目标矩形的宽高由坐标算出来，透明度按 `fade` 取。 |
| `inspection_findings_reach_the_sidebar_and_the_log` | 检查发现的问题要同时进侧栏和日志：用例造出指南列的那三种——只有 1x 没有 2x 的 PNG、缺了默认语言定义过的页面文案的语言、列了却没有对应文件的语言——要求同一次检查填上侧栏显示的条数和日志里的行。 |
| `inspection_time_is_explicit_and_stable` | 检查时间格式化成固定宽度的 `2026-09-15 13:04`，月和日补零。 |
| `inspects_taptap_project_without_dpi_warnings` | 检查示例工程 TapTap：名字、版本、文件版本、payload 格式、卸载器名字、默认安装路径、卸载器图标和 payload 大小都对得上，而且一条告警都没有。示例 payload 不在仓库里时这条用例跳过。 |
| `install_button_uses_xml_images_for_interaction_state` | 安装按钮的状态图来自版面：条件没满足时用 `disabled-image`，满足后用 `normal-image`，悬停和按下各自换成对应的图；没有条件的按钮只用普通和禁用两张。 |
| `installing_over_an_existing_installation_drops_stale_files` | 用这个版本覆盖上一个版本会替换产品，并删掉新 payload 里不再有的文件。 |
| `installs_a_fresh_directory_and_records_the_manifest` | 全新安装把 payload 的文件放到目标目录，manifest 列出这些文件，卸载器也一起放进去。 |
| `item_spacing_and_gap_leave_the_same_distance_between_items` | `gap` 和 `item-spacing` 在项之间留出同样的距离，两个同时写时用更具体的那个。 |
| `justify_content_places_the_run_inside_the_room_it_has` | `justify-content` 决定整行在剩余空间里的位置：默认靠左，`center` 居中，`end` 靠右，示例用的 `horizontal-align="right"` 是同一个意思。 |
| `leaves_a_section_of_the_projects_own_alone` | 工程自己增加的区块不会被拒绝：脚本用 `get_config_value` 把值读回去，所以检查只管本项目自己拥有的那几个区块。 |
| `link_runs_carry_their_target_and_plain_runs_do_not` | 解析文字段时只有链接段带上目标名，前后两段普通文字不带。 |
| `log_commands_are_english_when_interface_is_chinese` | 界面语言是中文时界面文字翻成中文，而日志里的构建文案仍旧是固定的英文。 |
| `log_severity_colors_preserve_selectable_text` | 日志按严重级别着色时控件里的文本一字不改，只把不同级别的行分成不同颜色的段落，选中的文字仍然能复制。 |
| `log_timestamp_has_fixed_width` | 日志时间戳格式固定为 `[2026-09-15 07:05:09.042]`，各位补零，宽度一致。 |
| `manifest_asks_for_elevation_only_when_the_project_does` | 清单文件只在工程要求时才请求提权：默认是 `asInvoker`，`install.require_admin` 为真时写 `requireAdministrator`；DPI 相关的两个元素跟着 `ui.dpi_aware` 一起切换，整份清单仍然是 Windows 加载器读得通的完整 XML。 |
| `maps_default_locale_to_version_language` | 默认 locale 映射成 PE 版本资源里的语言 ID：`zh-CN` 是 0x0804、`ru` 是 0x0419，认不出的用英文 0x0409。 |
| `menu_labels_stay_on_one_line_in_both_languages` | 菜单的宽度不变，标签都待在一行里。菜单栏只排一次，切换界面语言后重新读一遍，标签一折行，用户每次换语言旁边的菜单都会跟着移动。 |
| `only_expands_data_paths_inside_a_user_profile` | `uninstall.data_paths` 里只有用户配置目录下的路径会被展开并保留，`%APPDATA%` 本身、`%SystemRoot%` 和相对路径都被过滤掉。 |
| `open_project_names_the_missing_project_file` | 选中一个没有工程文件的文件夹会被拒绝，并指出缺的是哪个文件：选错文件夹是最常见的失误，什么都不报的窗口只会让用户猜它想要哪个文件。 |
| `open_project_reads_the_folder_that_holds_installer_config_json` | 打开工程读的是用户选中的那个文件夹，之后窗口显示的每个名字、路径和默认值都出自这里。用例把应用指向一个文件夹，要求摘要从磁盘上的文件填出来。 |
| `out_of_range_pages_fall_back_to_the_first_layout` | 安装模式取到 `installingpage.xml` 和 `finishpage.xml`，卸载模式取到 `uninstallingpage.xml` 和 `uninstallpage.xml`；页码越界时退回第一份版面，两个模式的页面总数都是 3。 |
| `out_of_range_progress_and_both_status_forms_do_not_disturb_the_install` | 进度设成负数、超过 100 或非数字，以及两种形式的状态文字，都不会打断安装：部署照常完成，manifest 写好，模式和取消状态也照常读得到。 |
| `padding_and_margin_take_one_to_four_values_and_their_single_side_forms` | `padding` 和 `margin` 接受一到四个值，按 CSS 的写法分到四边；单边写法独立生效并覆盖简写里那一边，两个属性互不干扰，所有值都按 96 DPI 计算并随显示缩放。 |
| `parameter_fields_use_the_available_work_area` | 参数面板里的输入框宽度按可用工作区算出来，扣掉标签、浏览按钮和两处间距，宽度不够时保底 120。 |
| `parameters_carry_the_project_output_and_runtime_directory_into_the_request` | 参数把工程、输出和运行时这三个路径带进构建请求，工作线程只能看到这份请求。留在屏幕上的路径会让构建走到构建器自己的默认值上，所以每个字段都必须写进去。 |
| `parses_ico_and_creates_group_directory` | 解析 ico 文件并生成图标组目录数据，只有一张图的 ico 生成的组头字节符合格式。 |
| `parses_numeric_windows_versions` | 版本号解析成四个数字：`1.2.3` 补成 `[1,2,3,0]`，带字母的、多于四段的、只有后缀的都算错误；带 `-r2` 或 `+notes` 的 CalVer 仍旧接受，只取数字部分。 |
| `path_primitives_join_split_and_name_paths` | 路径原语按预期工作：拼接、取父目录、取文件名，只有文件名时父目录为空，把目录名当文件名也能取到，临时目录来自系统。 |
| `percentage_and_pixel_extents_scale_with_the_layout` | 百分比尺寸相对父级算，像素尺寸不受影响；2 倍缩放下像素值翻倍，百分比仍旧相对。 |
| `pick_directory_falls_back_to_the_layout_text_input` | 目录选择按钮没写 `target` 时，退回用版面上第一个可写的 `TextInput`，被 `readonly="true"` 标记的那个跳过。 |
| `pick_directory_writes_to_the_field_the_page_offers_it` | 选中的目录写进哪个输入框：`target` 指定的优先，没指定就用第一个可写的，只剩只读字段时才用它，页面一个字段都没有时退回运行时记下的那个。 |
| `progress_bar_clips_its_sprite_to_the_completed_share` | 进度条先把贴图按完成比例裁剪再画：轨道铺满控件，填充占控件四分之一时画的是贴图开头的四分之一。 |
| `progress_pages_render_every_control_they_declare` | 示例工程的两张进度页把声明的控件都画出来了：安装页的进度轨道和文字都在位，卸载页的 `VBox` 把子项堆起来，进度条和文字也都在。 |
| `project_bundle_roundtrips_layout_assets_and_locales` | 工程打包再解开后，配置文件、版面、素材、语言文件和 payload 五项内容和原文件一致。 |
| `project_file_version_overrides_release_version_for_pe` | PE 资源的文件版本取自 `project.file_version`，发布版本号里带的后缀不进版本资源。 |
| `project_pack_progress_describes_assets_payload_and_uninstaller` | 打包过程按进度报出它做了什么：收集素材的数量、没有中间皮肤包、加入 payload、嵌入卸载运行时，卸载运行时也确实进了捆绑数据。 |
| `refresh_keeps_a_custom_output_path_for_the_same_project` | 刷新会保留为这个工程选好的输出路径：这个输入框可以指向任何地方，刷新时把它重置，下一次安装包就会在没人打招呼的情况下被放回工程目录树里。 |
| `refresh_replaces_a_custom_output_path_when_the_project_changes` | 自定义输出路径属于当初为它选定的那个工程。换了工程，这条路径就没意义了，留着它会把新工程的安装包写进新工程从没配置过的文件夹。 |
| `refuses_a_foreign_manifest_at_the_destination` | 目标目录里的 manifest 属于别的产品时，读取上一次安装会被拒绝。 |
| `refuses_a_misspelled_setting` | `install.exe_nmae` 这种拼错的键会让构建失败并指出是哪一条，而不是安静地什么也不做。 |
| `refuses_a_page_role_the_runtime_does_not_run` | 页面写了运行时不会跑的职责（比如 `license`）会让构建失败，指出该用 `progress` 或 `finish`。 |
| `refuses_a_section_that_does_nothing` | `validation` 这类整个没有被读取的区块会被拒绝，消息里指出真正会跑的是 `install.required_space_mb`。 |
| `refuses_a_setting_that_does_nothing` | 写了却没人读的设置会让构建失败：`install.append_to_path` 报错时会说清现在没有任何设置能往 PATH 里加目录。 |
| `refuses_an_install_when_the_drive_holds_less_space_than_the_project_asks_for` | 目标盘剩余空间少于 `install.required_space_mb` 时，安装在任何文件写下去之前就停下，并报出要多少、报的是哪个盘；要 0 MiB 的工程不受影响。 |
| `refuses_an_unknown_page_key` | 向导页条目里多写的键会被拒绝，并指出是第几页，防止一个多打的键悄悄沉在配置里。 |
| `refuses_relative_or_root_installation` | 安装目标必须是绝对路径，相对路径和盘符根目录都被拒绝。 |
| `refuses_to_install_over_a_directory_it_did_not_create` | 目标目录已经存在、里面还有用户的文件时，读取上次安装和部署都失败，用户的文件原样留着，也不写 manifest。 |
| `refuses_two_pages_claiming_one_role` | 两个页面都声明同一个职责会被拒绝，并点出是第几页，免得任务报到哪一页变得看运气。 |
| `registers_and_cleans_up_scoped_uninstall_key` | 注册卸载项会把键写进去，重复注册同一个键失败，带清理标志再注册一次后键被删掉。 |
| `registry_path_normalizes_legacy_escaped_separators` | 旧的、带双反斜杠的注册表路径被归一化成单反斜杠，不支持的根键名报错。 |
| `registry_primitives_round_trip_and_forget_a_key_they_created` | 注册表原语逐个核对：写字符串和 dword 后读得回来，读不存在的值得到空，删单个值不影响同键的其它值，删子键后它不在而父键还在；manifest 里也只留下脚本仍然拥有的那个键和它的两个值。 |
| `rejects_ico_image_outside_file` | ico 里记录的图像数据超出文件范围时报错，错误说明它落在文件之外。 |
| `rejects_unsafe_7z_paths` | 解压时拒绝带 `..` 的相对路径和带盘符的绝对路径，只允许归档内的相对路径。 |
| `removes_a_fresh_install_whose_registration_fails` | 全新安装的注册表登记失败时，整个安装被回滚，目标目录也不留下。 |
| `removes_recorded_shortcuts_and_only_their_empty_folder` | 卸载删掉 manifest 记下的快捷方式，但那个开始菜单文件夹里还有用户放的文件时保留它，共享的根目录也不动。 |
| `removing_a_selection_keeps_the_text_around_it` | 删除选区只删选中的部分，前后的文字留着，光标停在删除处，选区清空；没有选区时什么也不删。 |
| `replaying_the_manifest_is_refused_while_installing` | 安装过程中调用 `run_tracked_uninstall` 会失败并返回 false，安装照常完成。 |
| `reports_every_problem_at_once` | 一次报出配置里所有的问题，而不是只报第一条，省得改一处跑一趟。 |
| `reset_output_restores_the_dist_path_named_by_the_project` | 重置输出把工程自己的输出路径放回输入框，恢复的就是摘要里那条 `dist/<output.installer_name>`。这条路径一旦漂掉，就会掩盖一次把下次构建指错文件的重置。 |
| `resolves_and_queries_windows_disk_root` | 从安装路径取出盘符根目录，再问 Windows 该卷还剩多少空间，得到的字节数是正数。 |
| `retry_repeats_a_failed_build_with_the_parameters_it_stored` | 重试用失败那次存下的参数重跑构建：重试的意义就在于变的只是磁盘上的一个文件，所以用例在失败后改掉屏幕上的字段，要求工作线程收到的是存下的那份请求，而不是表单上的。 |
| `runtime_modes_select_distinct_layout_lists` | 安装和卸载两种模式各自取自己那份版面列表的第一页，互不混用。 |
| `sidebar_paths_keep_drive_and_relevant_tail` | 侧栏里的长路径压缩成盘符加末尾几段，路径本身很短时原样显示。 |
| `silent_arguments_read_the_directory_and_reject_anything_else` | 静默参数只认 `--dir`：没有参数时没有目录覆盖，`--dir` 后面带上路径就用它；写错的选项和后面缺路径的 `--dir` 都让运行停下。 |
| `startup_waits_for_project_selection` | 构建器启动后还没选工程：工程目录和输出路径都是空的，摘要、日志和结果都没有。 |
| `status_source_replaces_placeholder_text_with_the_published_step` | `value-source="status"` 的标签在步骤发布时显示该步骤的翻译文字，没有发布的步骤时保留版面里写死的占位文字。 |
| `taptap_first_page_places_controls_at_192_dpi` | 在 2 倍缩放下加载示例工程首页：窗口是 1440×900，图层数和顺序符合版面，页面底色、语言选择框、最小化和关闭按钮的位置与透明度都按倍率算对。 |
| `taptap_uninstaller_buttons_have_distinct_hit_regions` | 示例卸载页上卸载和取消两个按钮的点击区域不重叠，卸载键完全在取消键右侧。 |
| `text_colors_read_as_rgb_with_or_without_an_alpha_channel` | 颜色写六位和八位都读成同一个 RGB，画文字时忽略 alpha，字节顺序是 RGB 而不是 BGR；读不懂的颜色画成白色。 |
| `the_example_dialog_places_its_message_and_both_buttons` | 示例工程自己的对话框版面能把问题文字和两个按钮都摆好：问题是一行有真实高度、宽度和位置的折行文字，两个按钮并排不重叠，问题也不被按钮盖住。 |
| `the_example_project_matches_the_schema` | 示例工程自己的配置也要过这张表：它是别人照抄的模板，不能带着没人读的键。 |
| `the_language_menu_answers_to_the_keyboard` | 在真窗口里用键盘走一遍语言菜单：点一下控件把菜单展开，当前语言那一行标着记号；按一次下箭头，高亮落到下一行，而当前语言那行上的记号还在；按 Escape，菜单收起，页面回到展开之前的样子，一个像素都没变；再展开、再按下箭头、按 Enter，页面上的那句话换成另一种语言写的，而且除那句话和这个控件，别处都没有被重画。 |
| `the_log_keeps_the_lines_the_view_scrolled_past` | 日志留着视图滚过去的那些行：保存日志写下的和全部复制复制的是同一份文本，导出的应该是整份日志，而不是面板一次能显示的那几行。用例把它填到远超一屏，检查每一行都还在、措辞没变、时间戳还是自己那个。 |
| `the_manifest_reaches_a_real_executable` | 把清单写进一个真实的 PE 映像再读回来，内容与写进去的一致：提权级别和 DPI 相关的两个元素都在，整份 XML 仍然能被解析，Windows 不会因为清单坏了而拒绝加载。 |
| `the_panels_draw_in_every_state_they_can_be_in` | 面板在指南说到的每种状态下都画得出来：这里没有一条用例能开窗口，状态是画在无窗口上下文里的——什么都没打开、检查过工程、构建进行中、构建失败、构建完成，两种界面语言都算。一个根本画不出来的状态会让用户看到空白窗口，而画的过程改掉要展示的状态则更糟。 |
| `the_payload_format_selects_the_runtime_that_gets_embedded` | payload 格式决定嵌入哪个运行时。一直声称错误格式的安装包什么都装不上，因为那个 stub 读不懂归档。 |
| `the_pointer_decides_which_cursor_the_wizard_shows` | 指针在向导上是什么形状，由它底下那个控件说了算：按钮上是手型，可以输入的文本框上是工字光标，落在页面空白处则是普通箭头；指针再挪回按钮，形状跟着回去。三种标准光标必须先能彼此区分（句柄互不相同），否则这条用例不管窗口做什么都会通过。 |
| `the_script_queries_fixed_disks_and_notifies_the_shell` | 脚本列出的固定磁盘都是 `C:\` 这样的根目录，测试所在的盘也在里面；取到的可用空间是正数且不超过总容量；`shell_notify` 之后脚本继续往下走。 |
| `the_script_reads_the_environment_and_the_project_configuration` | 脚本读环境变量拿到真实值，机器上没有的变量读成空字符串而不是报错；读工程配置拿到产品名和可执行文件名，不存在的配置路径类型是 unit，对象是 map，脚本因此分得清写错和空值。 |
| `the_script_reports_the_image_it_runs_from` | 脚本报告自己运行的映像路径和它所在目录，值就是跑测试的那个可执行文件，而不是它读的捆绑数据。 |
| `the_setup_opens_its_wizard_window` | 不带 `--silent` 打开真实的安装包，等它的向导窗口画出来再量客户区，这是无窗口用例够不到的那道缝：版面加载失败、捆绑数据丢了资源、窗口类没注册，都可能让静默安装照样成功、让所有只读文件的检查通过。开窗口需要交互式的桌面会话，以服务方式启动的构建代理没有桌面，那种环境下用例跳过并说明原因；在本该有桌面的机器上，`NANO_INSTALLER_E2E_REQUIRE_DESKTOP` 会把这次跳过变成失败，因为从没跑过的检查不能算作跑过并通过。 |
| `the_summary_reports_what_the_project_declares` | 窗口显示的摘要是工程声明的内容，包括工程可以省略的那些默认值。 |
| `typing_coalesces_into_one_undo_step` | 连续键入只记一步撤销：一串按键共用同一份快照，Ctrl+Z 回到这串输入开始前的值，换一种编辑动作才另起一步。 |
| `undo_remembers_the_caret_that_belongs_to_the_value` | 撤销把光标恢复到那个值对应的位置，而不是恢复前的位置；快照里的光标超出缩短后的文本时被拉回文本末尾。 |
| `uninstall_keeps_a_directory_that_still_holds_user_files` | 用户放进安装目录里的文件会留住这个目录，这是 manifest 清理所做的承诺，同时部署上去的文件照样删掉。 |
| `uninstaller_metadata_uses_project_version_and_configured_name` | 卸载器的版本信息取自工程版本和配置的卸载器文件名，文件描述和公司名也跟着填好。 |
| `uninstaller_name_rejects_directory_components` | 卸载器文件名不能带目录部分，带斜杠或 `..` 的都算配置错误。 |
| `uninstalling_a_shared_key_removes_only_the_value_the_script_wrote` | 脚本写在共享键（例如 `Run`）下的值被 manifest 单独记下，卸载只删这个值，整个键和别的产品的值都留着。 |
| `uninstalling_below_appdata_removes_the_data_only_when_the_box_is_cleared` | `uninstall.data_paths` 下的数据目录只有在保留数据没有被勾上时才删；勾上时用户的文件原样留在配置目录里。 |
| `uninstalling_removes_the_product_the_registration_and_the_directory` | 卸载删掉自己部署的东西、删掉注册项，也删掉安装目录本身。 |
| `validation_preserves_custom_output_for_same_project` | 校验时同一个工程保留自定义输出路径，输入框为空则重新给出工程的默认路径，换了工程才替换掉自定义路径。 |
| `value_sources_read_the_config_the_disk_and_the_running_step` | 三种取值来源都读得对：`config:` 走工程文件并把数字按格式变成文字，`disk-free:` 问 Windows 那个输入框指向的卷还剩多少空间，认不出的来源和不存在的路径都解析不出值。 |
| `vbox_stacks_children_vertically_with_padding_and_margins` | 纵向容器按顺序堆叠子项：内边距把内容框往里缩，标签紧跟 60 像素的 `Spacer`，`align-items="center"` 让它在内容框里水平居中，按钮被 `margin-top` 推到标签下面而不是压在上面，没有图的按钮也登记点击区域。 |
| `window_icon_uses_the_brand_asset` | 构建器窗口的图标来自仓库里的品牌 PNG，解码成 512×512 的 RGBA 位图。 |
| `word_keys_stop_at_the_boundaries_they_delete` | Ctrl+Backspace 和 Ctrl+Delete 在词的边界停下：一次删一个名字或一段分隔符，分隔符一次只吃掉自己，末尾的空白先跳过，光标到了头或尾就不再删。 |
| `zip_backend_rejects_invalid_archive` | zip 后端遇到不是压缩包的文件时解压失败，而不是写出半份东西。 |
