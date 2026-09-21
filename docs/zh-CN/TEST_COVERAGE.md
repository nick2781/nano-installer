# 测试覆盖

文档承诺的每一条行为，以及守住它的那条自动化用例。表格里的每一行都列出了该行为失效时会失败的
用例；没有出现在任何一行里的行为，就是没人看住的行为。

`cargo test --locked --workspace` 会跑 274 条用例：核心库 199 条，真构建并运行安装包的 39 条，
按构建器的方式读工程的 5 条，可视化构建器 29 条，解压运行时 2 条。安装包级用例需要真实的运行时
可执行文件，`.\scripts\run_e2e_setup.ps1` 会先把它们构建出来再跑，并把整次运行写进
`target/e2e-report.txt`。

## 每一层能证明什么

| 层 | 用例数 | 能证明 | 不能证明 |
| --- | --- | --- | --- |
| 核心库 | 199 | 一页会变成什么——图层、坐标、命中区域、文字；捆绑数据里装了什么；安装往磁盘和注册表写了什么；每个脚本原语做什么；脚本从页面上取回什么、脚本读到的组件选择；工程声明的依赖怎么被查出来、下载下来、校验并装上，以及哪些情形会被拒绝 | 打包出来的安装包能走到这些代码 |
| 安装包级 | 39 | 构建好的安装包在这台机器上装了一遍，它的窗口也是真的被驱动起来的：payload 字节、manifest、卸载项、快捷方式、自启动、项目脚本、脚本要跑的辅助程序、向导窗口、它自己页面上那些要等点击的行为、在列表上滚动滚轮、脚本的提示与提问所画的那张由点击作答的卡片、页面上的取值交到脚本手里、勾选决定这次装哪些组件，工程声明的依赖真的被问了一遍——缺的装上、已经有的不再装一遍、装不上的让整次安装停下、下载来的程序对不上哈希就一次都不跑——以及只有指针与键盘真的动起来才会发生的事——悬停与按下换上的状态位图、三种标准光标形状、语言菜单的上下键与 Enter/Escape、选目录对话框 | 输入法自己画出来的那两个窗口；在外壳的选目录对话框里选定一个目录之后会写进哪个输入框；以及不显示指针的会话上光标长什么样——那里这条用例打印自己的跳过理由 |
| 工程检查 | 5 | 构建之前窗口会显示的那份摘要与告警列表 | |
| 可视化构建器 | 29 | 窗口自己的状态、参数、日志与告警 | 真的去点界面上的控件 |
| 解压运行时 | 2 | 坏归档、以及归档里不安全的路径会被拒绝 | 解压一个完好的归档——安装包级用例会用真实运行时解真实 payload |
| 截图快照 | 6 页 | 示例工程每一页实际长什么样（量出来的，不是看出来的） | 字形本身读不读得通；流程走到那一页时会发生什么 |

## 配置项

| 配置 | 用例 |
| --- | --- |
| `project.name` | `uninstaller_metadata_uses_project_version_and_configured_name`、`a_silent_install_writes_the_shortcuts_and_the_autostart_entry`、`a_built_setup_installs_its_payload_and_registers_an_uninstall_entry` |
| `project.version`、`project.file_version` | `version::tests::parses_numeric_windows_versions`、`project_file_version_overrides_release_version_for_pe`、`inspects_taptap_project_without_dpi_warnings` |
| `project.description`、`project.publisher`、`project.copyright`、`project.output_name` | `version::tests::builds_unicode_version_resource` |
| `output.installer_name` | `the_summary_reports_what_the_project_declares`、`reset_output_restores_the_dist_path_named_by_the_project` |
| `output.installer_icon` | `window_icon_uses_the_brand_asset`、`icon::tests::parses_ico_and_creates_group_directory`、`icon::tests::rejects_ico_image_outside_file` |
| `output.uninstaller_name` | `uninstaller_name_rejects_directory_components`、`uninstaller_metadata_uses_project_version_and_configured_name` |
| `output.uninstaller_icon` | `inspects_taptap_project_without_dpi_warnings` |
| `install.default_path` | `configured_install_paths_are_expanded`、`an_explicit_install_path_wins_over_the_configured_one`、`a_run_without_any_install_path_is_refused`、`a_configured_percent_path_is_expanded_and_used`、`an_explicit_directory_wins_over_the_configured_one` |
| `install.exe_name` | `a_payload_without_the_declared_executable_is_refused`、`an_install_script_that_never_deploys_the_executable_is_refused` |
| `install.required_space_mb` | `value_sources_read_the_config_the_disk_and_the_running_step`、`formats_bound_disk_sizes`、`refuses_an_install_when_the_drive_holds_less_space_than_the_project_asks_for` |
| `install.require_admin` | `manifest::tests::manifest_asks_for_elevation_only_when_the_project_does`、`manifest::tests::the_manifest_reaches_a_real_executable`、`the_summary_reports_what_the_project_declares` |
| `install.kill_process_on_install`、`install.detect_running_process` | `an_install_closes_a_running_copy_of_the_product` |
| `registry.uninstall_key` | `registers_and_cleans_up_scoped_uninstall_key`、`registry_path_normalizes_legacy_escaped_separators`、`a_built_setup_carries_a_readable_bundle_and_real_resources` |
| `links.*` | `agreement_links_resolve_through_the_project_links_table`、`a_link_the_project_does_not_configure_stays_plain_text` |
| `shortcuts.desktop_shortcut`、`shortcuts.desktop_default` | `a_silent_install_writes_the_shortcuts_and_the_autostart_entry`、`removes_recorded_shortcuts_and_only_their_empty_folder` |
| `shortcuts.start_menu`、`shortcuts.start_menu_folder` | 同上两条，另加 `drops_the_shortcut_folder_once_it_is_empty` |
| `autostart.enabled`、`autostart.default`、`autostart.registry_key`、`autostart.registry_value_name` | `a_silent_install_writes_the_shortcuts_and_the_autostart_entry` |
| `components.items` | `the_page_and_the_project_decide_which_components_install`、`a_project_without_components_installs_none_of_them`、`a_script_sees_the_components_the_run_installs`、`a_silent_run_installs_the_components_the_project_defaults_to`、`the_boxes_the_page_carries_decide_which_components_install`、`two_payloads_that_carry_one_file_are_refused`、`refuses_a_component_without_an_id_or_a_payload`、`refuses_a_required_component_whose_default_is_false`、`refuses_an_unknown_component_key`、`refuses_two_components_that_share_an_id_or_a_payload` |
| `dependencies.items` | `a_silent_run_installs_the_dependency_the_machine_is_missing`、`a_dependency_the_machine_already_has_is_not_installed_again`、`a_dependency_that_cannot_be_installed_stops_the_install`、`a_downloaded_dependency_is_checked_before_it_runs`、`a_download_that_is_not_the_recorded_file_is_refused`、`accepts_a_dependency_the_machine_is_asked_about`、`refuses_a_dependency_that_is_not_checked_or_not_installed`、`refuses_a_download_that_does_not_say_what_should_arrive`、`refuses_a_detection_rule_that_asks_the_wrong_thing`、`refuses_a_dependency_that_cannot_be_run_or_told_apart` |
| `resources.*` | `project_bundle_roundtrips_layout_assets_and_locales`、`project_pack_progress_describes_assets_payload_and_uninstaller`、`the_payload_format_selects_the_runtime_that_gets_embedded`、`a_project_bundles_the_tools_directory_it_names`、`a_project_that_names_no_tools_bundles_none`、`a_setup_unpacks_the_tools_its_project_bundles` |
| `localization.default_locale` | `version::tests::maps_default_locale_to_version_language`、`the_summary_reports_what_the_project_declares` |
| `localization.supported_locales` | `a_supported_locale_without_a_file_is_reported`、`a_translation_missing_page_text_is_reported` |
| `wizard.pages`、`wizard.uninstall_pages` | `runtime_modes_select_distinct_layout_lists`、`out_of_range_pages_fall_back_to_the_first_layout` |
| `ui.dpi_aware`、`ui.dpi_threshold` | `a_display_scales_the_layout_by_its_own_dpi`、`a_layout_picks_the_image_density_the_display_asks_for`、`dpi_asset_resolution_prefers_requested_density_and_falls_back`、`dpi_scaling_rounds_layout_coordinates` |
| `ui.dialog_layout` | `a_project_without_a_dialog_layout_still_opens`、`a_dialog_is_drawn_over_the_page_and_centred`、`a_question_a_script_asks_is_drawn_with_both_of_its_answers`、`a_script_dialog_is_drawn_in_the_wizard` |
| `uninstall.data_paths` | `only_expands_data_paths_inside_a_user_profile`、`ignores_data_paths_when_the_project_declares_none`、`uninstalling_below_appdata_removes_the_data_only_when_the_box_is_cleared` |
| `advanced.silent_mode_support`、`advanced.uninstall_mode_support` | `a_project_without_silent_support_refuses_a_windowless_install`、`a_project_without_silent_support_refuses_a_windowless_uninstall`、`a_project_that_did_not_opt_in_refuses_a_windowless_run` |
| payload 格式判定 | `the_payload_format_selects_the_runtime_that_gets_embedded`、`zip_backend_rejects_invalid_archive`、`rejects_unsafe_7z_paths` |
| `installer_config.json` 里自己不读的键 | `accepts_a_configuration_of_read_settings`、`leaves_a_section_of_the_projects_own_alone`、`refuses_a_setting_that_does_nothing`、`refuses_a_misspelled_setting`、`refuses_a_section_that_does_nothing`、`refuses_an_unknown_page_key`、`refuses_a_page_role_the_runtime_does_not_run`、`refuses_two_pages_claiming_one_role`、`reports_every_problem_at_once`、`the_example_project_matches_the_schema` |
| 构建告警 | `an_asset_without_its_density_pair_is_reported`、`a_translation_missing_page_text_is_reported`、`a_supported_locale_without_a_file_is_reported`、`inspects_taptap_project_without_dpi_warnings`、`a_validation_message_the_page_asks_for_is_reported` |

## 页面与控件

| 行为 | 用例 |
| --- | --- |
| `Page` 底色、背景图、描边与圆角 | `a_page_paints_its_fill_under_its_image_and_its_outline_over_them`、`a_border_layer_paints_a_ring_and_leaves_the_center_empty` |
| 绝对定位的 `Image` 与 `Icon` | `an_absolute_image_and_icon_draw_at_the_rectangle_they_declare` |
| `file`/`dest`/`fade` 图片写法 | `a_styled_image_draws_into_a_sub_rectangle_at_the_opacity_it_declares`、`image_style_supports_plain_path_destination_and_fade` |
| 按显示 DPI 选密度，以及两个方向上的回退 | `a_layout_picks_the_image_density_the_display_asks_for`、`dpi_asset_resolution_prefers_requested_density_and_falls_back` |
| `Button` 各状态图及其回退 | `a_button_state_image_falls_back_to_the_normal_one`、`install_button_uses_xml_images_for_interaction_state`、`a_hover_and_a_press_show_the_pictures_the_button_declares` |
| `enabled-when` 支持的每种状态、它点名的输入框、记下的取值，以及一串条件 | `a_button_waits_for_each_state_its_condition_can_name`、`a_button_waits_for_the_field_its_condition_names`、`a_radio_group_holds_one_value_at_a_time` |
| 输入框自己的规矩决定哪些值算合格（`required`、`min-length`、`max-length`、`pattern`） | `a_field_checks_the_value_the_project_asks_it_to` |
| 值最先破坏的那条规矩，由页面上的提示说出来 | `a_hint_shows_the_rule_the_value_breaks` |
| 安装按钮等的是用户真的把那个字段填上 | `a_field_the_user_fills_in_is_what_lets_the_install_start` |
| 被禁用的按钮不收点击也不收悬停 | `a_disabled_button_registers_no_click_and_no_hover` |
| `HBox`、`VBox`、`Content` 布局：尺寸、间距、对齐、`flex-*`、最小值 | `bottom_hbox_distributes_fixed_and_flexible_items`、`a_container_measures_the_edge_its_children_are_asked_for`、`a_nested_container_reports_the_extent_its_children_need`、`a_shrinking_row_stops_at_the_minimum_its_items_declare`、`flow_attributes_become_the_item_a_container_shares_space_with`、`align_self_overrides_the_alignment_of_its_container`、`justify_content_places_the_run_inside_the_room_it_has`、`item_spacing_and_gap_leave_the_same_distance_between_items`、`each_container_tag_accepts_the_alignment_spelling_it_documents`、`vbox_stacks_children_vertically_with_padding_and_margins` |
| padding、margin 及单边写法 | `padding_and_margin_take_one_to_four_values_and_their_single_side_forms`、`control_padding_insets_what_the_control_draws` |
| 百分比与像素尺寸 | `percentage_and_pixel_extents_scale_with_the_layout` |
| 换行行 | `a_wrapping_row_starts_a_new_line_when_the_next_item_does_not_fit`、`a_wrapping_row_gives_each_line_the_height_of_its_tallest_item`、`flex_wrap_is_opt_in_per_container` |
| 脚本写入用户环境变量、登记文件类型，卸载再把这两件事都收回去 | `an_install_script_sets_a_variable_the_uninstall_takes_back`、`an_install_script_removes_a_variable_it_no_longer_wants`、`a_script_registers_a_file_type_where_windows_reads_it`、`a_file_type_that_would_write_outside_the_classes_tree_is_refused` |
| 可滚动容器：装得下多少、露出来的是哪一块、滚动条说的位置，以及推动它的滚轮 | `a_scrollable_container_shows_the_part_it_is_scrolled_to`、`a_scrollbar_says_where_the_list_stands`、`a_wheel_over_a_list_brings_the_rows_below_into_reach` |
| `Spacer` | `a_spacer_takes_what_the_fixed_items_leave` |
| 用 `right`、`bottom`、`inset` 及单边写法贴边 | `an_element_is_pinned_by_the_edge_attribute_it_carries` |
| `Label` 的文字、字体、对齐与颜色 | `a_label_takes_its_text_font_and_alignment_from_the_layout`、`text_colors_read_as_rgb_with_or_without_an_alpha_channel`、`a_bound_label_shows_its_own_text_beside_the_value_it_reads` |
| 组件由同名复选框选择，勾选结果就是这次装哪些组件 | `the_boxes_the_page_carries_decide_which_components_install`、`the_page_and_the_project_decide_which_components_install` |
| `Checkbox` 的状态图、文字、链接与切换 | `an_absolutely_placed_checkbox_draws_its_state_image_and_toggles` |
| `RadioButton`：一组一个取值、版面给的默认行，以及点掉它的那一次点击 | `a_radio_group_holds_one_value_at_a_time`、`a_click_on_a_radio_is_the_value_the_install_waits_for` |
| `Select`：语言列表，以及工程自己声明的选项 | `a_language_menu_lists_its_options_and_marks_the_one_in_use`、`a_closed_language_select_draws_its_arrow_over_its_fill_and_outline`、`a_select_offers_the_options_the_page_declares`、`the_language_menu_answers_to_the_keyboard` |
| `ProgressBar` 的轨道、裁剪与实时值 | `a_progress_bar_paints_a_rounded_track_and_follows_the_live_value`、`progress_bar_clips_its_sprite_to_the_completed_share` |
| 每一种 `value-source` 与 `value-format` | `value_sources_read_the_config_the_disk_and_the_running_step`、`formats_bound_disk_sizes`、`resolves_and_queries_windows_disk_root`、`status_source_replaces_placeholder_text_with_the_published_step` |
| 文本框编辑：光标、选区、撤销、按词按键 | `a_caret_sits_after_the_characters_before_it`、`a_double_click_selects_the_word_under_the_pointer`、`a_selection_band_covers_the_characters_it_selects`、`a_selection_is_ordered_from_whichever_end_the_caret_is_at`、`removing_a_selection_keeps_the_text_around_it`、`typing_coalesces_into_one_undo_step`、`undo_remembers_the_caret_that_belongs_to_the_value`、`word_keys_stop_at_the_boundaries_they_delete`、`byte_index_walks_characters_not_bytes`、`editable_text_fields_are_recorded_and_readonly_ones_are_not`、`a_typed_value_wins_over_the_bound_default`、`a_readonly_field_shows_its_value_without_taking_edits`、`an_empty_field_is_one_a_user_can_click_into` |
| 输入法组字窗与候选窗锚在页面画出的那个插入符上 | `an_input_method_anchors_at_the_caret_the_page_drew` |
| `visible="false"` 与面板显隐 | `a_hidden_element_takes_its_whole_subtree_with_it`、`a_panel_pair_shows_the_panel_and_only_the_control_that_fits` |
| 页面顺序与页面声明的职责（`next`、`back`、`role`） | `a_page_role_finds_the_page_that_holds_it`、`a_page_list_without_roles_keeps_its_positions`、`a_next_button_walks_to_the_page_the_project_declares` |
| 动作表里的每一个动作 | `every_action_in_the_table_answers_with_its_own_window_action`、`action_attributes_map_to_window_actions` |
| 对话框：位置、按钮、通知、随文字长高的卡片 | `a_dialog_is_drawn_over_the_page_and_centred`、`a_dialog_button_answers_with_its_own_action`、`a_dialog_button_is_drawn_from_the_question_rather_than_the_layout`、`a_notice_hides_the_secondary_button`、`a_page_without_a_dialog_draws_no_overlay`、`every_shipped_question_keeps_its_answers_inside_the_card`、`the_example_dialog_places_its_message_and_both_buttons` |
| 链接的解析顺序 | `agreement_links_resolve_through_the_project_links_table`、`a_link_the_project_does_not_configure_stays_plain_text`、`link_runs_carry_their_target_and_plain_runs_do_not`、`agreement_markdown_becomes_colored_visible_runs` |
| 只有声明了动作的元素才响应指针 | `an_element_answers_the_pointer_only_when_it_declares_an_action` |
| 指针形状：按钮上是手型、可输入的输入框上是工字光标、页面空白处是箭头 | `the_pointer_decides_which_cursor_the_wizard_shows` |
| 目录选择器写到哪个输入框 | `pick_directory_writes_to_the_field_the_page_offers_it`、`pick_directory_falls_back_to_the_layout_text_input` |
| 有 `pick_directory` 动作的按钮打开外壳自己的选目录对话框，关掉它不改变向导 | `a_browse_button_opens_the_folder_picker_and_leaving_it_changes_nothing` |
| 窗口摆放与缩放 | `a_window_is_centred_and_clamped_to_its_work_area`、`a_placed_window_is_pulled_back_inside_its_work_area`、`a_display_scales_the_layout_by_its_own_dpi` |
| 从窗口停掉正在跑的任务（`cancel` 按钮、确认后的 `close_confirm`） | `a_cancel_button_stops_the_project_script_and_leaves_nothing_installed` |
| 示例工程自己的页面 | `taptap_first_page_places_controls_at_192_dpi`、`taptap_uninstaller_buttons_have_distinct_hit_regions`、`progress_pages_render_every_control_they_declare` |

## 项目脚本

原语清单见 [脚本接口](SCRIPT_API.md)。进程内的用例直接驱动脚本驱动层；安装包级那两条证明 `scripts`
目录和工具目录在打包之后仍然活着，而且到得了脚本手上。

| 行为 | 用例 |
| --- | --- |
| 契约：必须部署指定的可执行文件，否则回滚 | `an_install_script_that_never_deploys_the_executable_is_refused`、`an_install_script_deploys_files_and_writes_the_manifest` |
| 脚本失败会撤销自己写过的东西，并报告日志 | `a_failing_install_script_removes_what_it_wrote`、`a_script_failure_reports_the_messages_it_logged`、`every_log_level_reaches_the_failure_the_wizard_shows` |
| `run_tracked_uninstall`，以及脚本跳过它时的库回退 | `an_uninstall_script_replays_the_manifest_it_asks_for`、`an_uninstall_script_that_skips_the_manifest_still_removes_the_product`、`replaying_the_manifest_is_refused_while_installing` |
| 文件与路径原语 | `file_primitives_create_copy_list_and_remove_files`、`path_primitives_join_split_and_name_paths` |
| 注册表原语，包括共享键那条规则 | `registry_primitives_round_trip_and_forget_a_key_they_created`、`uninstalling_a_shared_key_removes_only_the_value_the_script_wrote` |
| 快捷方式原语 | `an_install_script_creates_shortcuts_the_uninstall_takes_back`、`a_script_deletes_the_desktop_shortcut_and_the_start_menu_folder_it_created` |
| 进度、状态、模式、复选框与取消 | `out_of_range_progress_and_both_status_forms_do_not_disturb_the_install`、`an_uninstall_script_sees_the_uninstall_mode_and_the_keep_data_checkbox`、`a_script_step_text_wins_over_the_locale_key`、`a_built_in_step_clears_a_script_step_text`、`a_running_script_sees_the_cancel_request`、`a_cancelled_install_gives_up_after_the_script_and_undoes_what_it_wrote` |
| 脚本读到的组件选择：`is_component_selected` 与 `selected_components` 给出这次运行装的组件 | `a_script_sees_the_components_the_run_installs` |
| 脚本读到的页面取值：文本框与选项控件的当前值，页面没有声明的 id 读成空串 | `a_script_reads_the_values_the_page_holds`、`the_values_the_page_holds_reach_the_script` |
| 环境变量、配置、磁盘与当前映像 | `the_script_reads_the_environment_and_the_project_configuration`、`the_script_reports_the_image_it_runs_from`、`the_script_queries_fixed_disks_and_notifies_the_shell` |
| 工程打包进来的工具 | `a_script_reads_the_tools_the_project_bundled`、`a_script_that_asks_for_tools_a_project_did_not_bundle_gets_nothing`、`a_setup_unpacks_the_tools_its_project_bundles` |
| 依赖与下载：脚本问机器装没装工程声明的依赖、可以让它装上，也可以自己按地址取回一个文件、校验它并报出它的哈希 | `a_script_asks_the_machine_about_the_dependencies_the_project_declares`、`a_script_downloads_a_file_and_checks_what_arrived` |
| 进程 | `a_script_runs_a_command_and_sees_its_exit_code`、`a_script_recognises_a_running_process_by_its_image_name` |
| 保留还是删除用户数据 | `uninstalling_below_appdata_removes_the_data_only_when_the_box_is_cleared` |
| `scripts` 目录进入构建好的安装包 | `a_setup_runs_the_projects_own_install_and_uninstall_scripts` |
| 脚本发出的提示、报错与提问画在向导里、由点击作答 | `a_question_a_script_asks_is_drawn_with_both_of_its_answers`、`a_script_dialog_is_drawn_in_the_wizard` |

## 安装、升级与卸载

| 行为 | 用例 |
| --- | --- |
| 组件：装哪些由页面与工程一起决定，两个归档带同一个路径当场报错 | `a_silent_run_installs_the_components_the_project_defaults_to`、`the_boxes_the_page_carries_decide_which_components_install`、`two_payloads_that_carry_one_file_are_refused` |
| 依赖：装产品之前先问机器缺不缺，缺的装上，装不上就停下；下载来的程序要对得上工程记下的哈希 | `a_silent_run_installs_the_dependency_the_machine_is_missing`、`a_dependency_the_machine_already_has_is_not_installed_again`、`a_dependency_that_cannot_be_installed_stops_the_install`、`a_downloaded_dependency_is_checked_before_it_runs`、`a_download_that_is_not_the_recorded_file_is_refused` |
| 全新安装：文件、manifest、注册项 | `installs_a_fresh_directory_and_records_the_manifest`、`a_built_setup_installs_its_payload_and_registers_an_uninstall_entry` |
| 配置路径展开，显式目录优先 | `configured_install_paths_are_expanded`、`an_explicit_install_path_wins_over_the_configured_one`、`a_configured_percent_path_is_expanded_and_used`、`an_explicit_directory_wins_over_the_configured_one` |
| 拒绝装进不是自己创建的目录 | `refuses_to_install_over_a_directory_it_did_not_create`、`refuses_a_foreign_manifest_at_the_destination`、`refuses_relative_or_root_installation` |
| 升级：替换版本、删掉陈旧文件、保留别人的文件 | `an_upgrade_replaces_the_previous_version_and_drops_stale_files`、`installing_over_an_existing_installation_drops_stale_files`、`an_upgrade_keeps_a_file_the_payload_does_not_own` |
| 失败回滚 | `a_failed_upgrade_restores_the_previous_version`、`removes_a_fresh_install_whose_registration_fails` |
| 取消：任务在检查点停下并撤回已经写下的内容 | `a_cancel_request_stops_the_checkpoints_that_follow_it`、`a_cancelled_deployment_writes_nothing` |
| 卸载清掉产品、注册项与目录 | `uninstalling_removes_the_product_the_registration_and_the_directory` |
| 目录里还有用户文件时保留目录 | `uninstall_keeps_a_directory_that_still_holds_user_files` |
| 快捷方式与自启动的记账 | `a_silent_install_writes_the_shortcuts_and_the_autostart_entry`、`removes_recorded_shortcuts_and_only_their_empty_folder`、`drops_the_shortcut_folder_once_it_is_empty` |
| 无窗口运行，以及允许它的开关 | `silent_arguments_read_the_directory_and_reject_anything_else`、`an_unknown_silent_option_is_refused`、`a_project_that_did_not_opt_in_refuses_a_windowless_run` |
| 安装包带的捆绑数据，以及追加在它之后的签名 | `project_bundle_roundtrips_layout_assets_and_locales`、`bundle_index_streams_entries_without_loading_the_payload`、`bundle_index_ignores_images_without_a_footer`、`bundle_index_reads_a_bundle_that_a_signature_follows`、`a_setup_with_a_signature_appended_still_installs` |
| 向导窗口按工程声明的尺寸打开 | `the_setup_opens_its_wizard_window` |
| Windows 在进程启动前读的那份资源 | `a_built_setup_carries_a_readable_bundle_and_real_resources`、`manifest::tests::the_manifest_reaches_a_real_executable` |

## 可视化构建器

| 行为 | 用例 |
| --- | --- |
| 打开工程，没有配置文件的目录会被拒绝 | `open_project_reads_the_folder_that_holds_installer_config_json`、`open_project_names_the_missing_project_file` |
| 刷新，以及自定义输出路径 | `refresh_keeps_a_custom_output_path_for_the_same_project`、`refresh_replaces_a_custom_output_path_when_the_project_changes`、`validation_preserves_custom_output_for_same_project` |
| 重置输出、自动搜索运行时 | `reset_output_restores_the_dist_path_named_by_the_project`、`an_empty_runtime_directory_leaves_the_stub_search_automatic` |
| 参数进入构建请求 | `parameters_carry_the_project_output_and_runtime_directory_into_the_request`、`parameter_fields_use_the_available_work_area` |
| 构建重新读盘，而不是相信屏幕上那份摘要 | `build_setup_rechecks_a_project_marked_dirty_before_it_starts`、`a_build_reads_the_project_from_disk_in_its_worker_thread` |
| 重试沿用失败那次的参数 | `retry_repeats_a_failed_build_with_the_parameters_it_stored` |
| 日志不丢行、按顺序、构建文案固定英文 | `the_log_keeps_the_lines_the_view_scrolled_past`、`build_messages_reach_the_log_in_the_order_the_worker_sent_them`、`log_commands_are_english_when_interface_is_chinese`、`log_severity_colors_preserve_selectable_text`、`log_timestamp_has_fixed_width` |
| 菜单、面板、侧栏路径与告警 | `menu_labels_stay_on_one_line_in_both_languages`、`the_panels_draw_in_every_state_they_can_be_in`、`sidebar_paths_keep_drive_and_relevant_tail`、`inspection_findings_reach_the_sidebar_and_the_log` |
| 同一时间只跑一个构建，尺寸与进度如实 | `a_running_build_refuses_a_second_one`、`build_progress_is_monotonic`、`formats_gui_sizes`、`inspection_time_is_explicit_and_stable` |
| 图标、参数引号、归档拒绝 | `window_icon_uses_the_brand_asset`、`command_arguments_are_quoted`、`rejects_unsafe_7z_paths`、`zip_backend_rejects_invalid_archive` |
| 坏掉的工程文件 | `a_broken_project_file_is_reported_to_the_user` |

## 目前仍没有被自动化用例覆盖的部分

- **Windows 7 SP1。** 兼容性声明需要一台干净的 Windows 7 SP1 x64 机器，本项目没有这个环境。见
  [当前生产状态](PRODUCTION_STATUS.md)。
- **字形读不读得通。** 渲染检查能说明文字按布局要的颜色和位置画了出来，说明不了这句话好不好认。
  这套判断交给截图的模型复核脚本，或者交给人。
- **输入法自己画的那两个窗口，以及系统选目录对话框里的选择动作。** 用例能证明运行时交给输入法的
  组字点与候选点就落在插入符上，证明不了输入法是否照着它画了出来；能证明 `pick_directory` 的
  按钮开出了外壳的选目录对话框，证明不了在里面选定一个目录之后会写进页面上的哪个输入框。
- **测试无法收场的脚本原语：** `run_detached` 有意活得比这次运行长；`kill_process` 会结束一个
  不是测试启动的进程；`sleep_ms` 只能拿墙上时间做断言；`is_elevated` 的期望值只能照抄实现。
