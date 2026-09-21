# Test coverage

Every behaviour the documentation promises, and the automated case that holds it. A row names the
cases that fail when that behaviour breaks; a behaviour with no row is one nobody is checking.

`cargo test --locked --workspace` runs 253 cases: 184 in the core library, 33 that build a real
setup and run it, 5 that read a project the way the builder does, 29 in the visual builder, and 2
in the extraction runtimes. The setup-level cases need real runtime executables built first, which
is what `.\scripts\run_e2e_setup.ps1` does before it runs them and writes `target/e2e-report.txt`.

## What each layer can prove

| Layer | Cases | Proves | Cannot prove |
| --- | --- | --- | --- |
| Core library | 184 | what a page becomes — layers, coordinates, hit regions, text — what the bundle carries, what an install writes to disk and the registry, what each script primitive does, and what a script reads back off the page and off the run | that a packaged setup reaches any of it |
| Setup end to end | 33 | a built setup installed on the machine, its own window driven: payload bytes, manifest, uninstall entry, shortcuts, autostart, project scripts, the helpers a script runs, the wizard window, the clicks its own pages wait for, a wheel over a list, the card a script's messages and questions are answered on, the page's values reaching the script, which components a page and a project put in, and what only a moving pointer and a real keyboard bring about -- the bitmaps hover and press swap in, the three standard cursor shapes, the language menu's arrow keys with Enter and Escape, and the folder picker | the two windows an input method draws itself; which field a directory chosen in the shell's folder dialog is written to; and the cursor shape on a session that is showing no pointer, where that case prints its own skip |
| Project inspection | 5 | the summary and the warning list the builder shows before a build | |
| Visual builder | 29 | the window's own state, parameters, log and warnings | clicking the real controls |
| Extraction runtimes | 2 | a broken archive, and an unsafe path inside one, are refused | extracting an archive that is sound -- the setup-level cases run a real runtime over a real payload |
| Snapshots | 6 pages | what every page of the example actually looks like, measured rather than judged | whether a glyph reads correctly, and what happens once the flow reaches a page |

## Configuration

| Setting | Cases |
| --- | --- |
| `project.name` | `uninstaller_metadata_uses_project_version_and_configured_name`, `a_silent_install_writes_the_shortcuts_and_the_autostart_entry`, `a_built_setup_installs_its_payload_and_registers_an_uninstall_entry` |
| `project.version`, `project.file_version` | `version::tests::parses_numeric_windows_versions`, `project_file_version_overrides_release_version_for_pe`, `inspects_taptap_project_without_dpi_warnings` |
| `project.description`, `project.publisher`, `project.copyright`, `project.output_name` | `version::tests::builds_unicode_version_resource` |
| `output.installer_name` | `the_summary_reports_what_the_project_declares`, `reset_output_restores_the_dist_path_named_by_the_project` |
| `output.installer_icon` | `window_icon_uses_the_brand_asset`, `icon::tests::parses_ico_and_creates_group_directory`, `icon::tests::rejects_ico_image_outside_file` |
| `output.uninstaller_name` | `uninstaller_name_rejects_directory_components`, `uninstaller_metadata_uses_project_version_and_configured_name` |
| `output.uninstaller_icon` | `inspects_taptap_project_without_dpi_warnings` |
| `install.default_path` | `configured_install_paths_are_expanded`, `an_explicit_install_path_wins_over_the_configured_one`, `a_run_without_any_install_path_is_refused`, `a_configured_percent_path_is_expanded_and_used`, `an_explicit_directory_wins_over_the_configured_one` |
| `install.exe_name` | `a_payload_without_the_declared_executable_is_refused`, `an_install_script_that_never_deploys_the_executable_is_refused` |
| `install.required_space_mb` | `value_sources_read_the_config_the_disk_and_the_running_step`, `formats_bound_disk_sizes`, `refuses_an_install_when_the_drive_holds_less_space_than_the_project_asks_for` |
| `install.require_admin` | `manifest::tests::manifest_asks_for_elevation_only_when_the_project_does`, `manifest::tests::the_manifest_reaches_a_real_executable`, `the_summary_reports_what_the_project_declares` |
| `install.kill_process_on_install`, `install.detect_running_process` | `an_install_closes_a_running_copy_of_the_product` |
| `registry.uninstall_key` | `registers_and_cleans_up_scoped_uninstall_key`, `registry_path_normalizes_legacy_escaped_separators`, `a_built_setup_carries_a_readable_bundle_and_real_resources` |
| `links.*` | `agreement_links_resolve_through_the_project_links_table`, `a_link_the_project_does_not_configure_stays_plain_text` |
| `shortcuts.desktop_shortcut`, `shortcuts.desktop_default` | `a_silent_install_writes_the_shortcuts_and_the_autostart_entry`, `removes_recorded_shortcuts_and_only_their_empty_folder` |
| `shortcuts.start_menu`, `shortcuts.start_menu_folder` | the same two, plus `drops_the_shortcut_folder_once_it_is_empty` |
| `autostart.enabled`, `autostart.default`, `autostart.registry_key`, `autostart.registry_value_name` | `a_silent_install_writes_the_shortcuts_and_the_autostart_entry` |
| `components.items` | `the_page_and_the_project_decide_which_components_install`, `a_project_without_components_installs_none_of_them`, `a_script_sees_the_components_the_run_installs`, `a_silent_run_installs_the_components_the_project_defaults_to`, `the_boxes_the_page_carries_decide_which_components_install`, `two_payloads_that_carry_one_file_are_refused`, `refuses_a_component_without_an_id_or_a_payload`, `refuses_a_required_component_whose_default_is_false`, `refuses_an_unknown_component_key`, `refuses_two_components_that_share_an_id_or_a_payload` |
| `resources.*` | `project_bundle_roundtrips_layout_assets_and_locales`, `project_pack_progress_describes_assets_payload_and_uninstaller`, `the_payload_format_selects_the_runtime_that_gets_embedded`, `a_project_bundles_the_tools_directory_it_names`, `a_project_that_names_no_tools_bundles_none`, `a_setup_unpacks_the_tools_its_project_bundles` |
| `localization.default_locale` | `version::tests::maps_default_locale_to_version_language`, `the_summary_reports_what_the_project_declares` |
| `localization.supported_locales` | `a_supported_locale_without_a_file_is_reported`, `a_translation_missing_page_text_is_reported` |
| `wizard.pages`, `wizard.uninstall_pages` | `runtime_modes_select_distinct_layout_lists`, `out_of_range_pages_fall_back_to_the_first_layout` |
| `ui.dpi_aware`, `ui.dpi_threshold` | `a_display_scales_the_layout_by_its_own_dpi`, `a_layout_picks_the_image_density_the_display_asks_for`, `dpi_asset_resolution_prefers_requested_density_and_falls_back`, `dpi_scaling_rounds_layout_coordinates` |
| `ui.dialog_layout` | `a_project_without_a_dialog_layout_still_opens`, `a_dialog_is_drawn_over_the_page_and_centred`, `a_question_a_script_asks_is_drawn_with_both_of_its_answers`, `a_script_dialog_is_drawn_in_the_wizard` |
| `uninstall.data_paths` | `only_expands_data_paths_inside_a_user_profile`, `ignores_data_paths_when_the_project_declares_none`, `uninstalling_below_appdata_removes_the_data_only_when_the_box_is_cleared` |
| `advanced.silent_mode_support`, `advanced.uninstall_mode_support` | `a_project_without_silent_support_refuses_a_windowless_install`, `a_project_without_silent_support_refuses_a_windowless_uninstall`, `a_project_that_did_not_opt_in_refuses_a_windowless_run` |
| payload format detection | `the_payload_format_selects_the_runtime_that_gets_embedded`, `zip_backend_rejects_invalid_archive`, `rejects_unsafe_7z_paths` |
| build warnings | `an_asset_without_its_density_pair_is_reported`, `a_translation_missing_page_text_is_reported`, `a_supported_locale_without_a_file_is_reported`, `inspects_taptap_project_without_dpi_warnings`, `a_validation_message_the_page_asks_for_is_reported` |
| `installer_config.json` keys this build does not read | `accepts_a_configuration_of_read_settings`, `leaves_a_section_of_the_projects_own_alone`, `refuses_a_setting_that_does_nothing`, `refuses_a_misspelled_setting`, `refuses_a_section_that_does_nothing`, `refuses_an_unknown_page_key`, `refuses_a_page_role_the_runtime_does_not_run`, `refuses_two_pages_claiming_one_role`, `reports_every_problem_at_once`, `the_example_project_matches_the_schema` |

## Pages and controls

| Behaviour | Cases |
| --- | --- |
| `Page` fill, image, outline and rounded corners | `a_page_paints_its_fill_under_its_image_and_its_outline_over_them`, `a_border_layer_paints_a_ring_and_leaves_the_center_empty` |
| absolute `Image` and `Icon` | `an_absolute_image_and_icon_draw_at_the_rectangle_they_declare` |
| the `file`/`dest`/`fade` image form | `a_styled_image_draws_into_a_sub_rectangle_at_the_opacity_it_declares`, `image_style_supports_plain_path_destination_and_fade` |
| density choice at the display's DPI, both fallbacks | `a_layout_picks_the_image_density_the_display_asks_for`, `dpi_asset_resolution_prefers_requested_density_and_falls_back` |
| `Button` state images and their fallback | `a_button_state_image_falls_back_to_the_normal_one`, `install_button_uses_xml_images_for_interaction_state`, `a_hover_and_a_press_show_the_pictures_the_button_declares` |
| `enabled-when` for every state it can name, a field, a recorded choice, and a list of conditions | `a_button_waits_for_each_state_its_condition_can_name`, `a_button_waits_for_the_field_its_condition_names`, `a_radio_group_holds_one_value_at_a_time` |
| a field's own rules decide which values it accepts (`required`, `min-length`, `max-length`, `pattern`) | `a_field_checks_the_value_the_project_asks_it_to` |
| the hint a field shows for the first rule its value breaks | `a_hint_shows_the_rule_the_value_breaks` |
| the install button waits for the field a person fills in | `a_field_the_user_fills_in_is_what_lets_the_install_start` |
| a disabled button takes no click and no hover | `a_disabled_button_registers_no_click_and_no_hover` |
| `HBox`, `VBox`, `Content` layout: sizes, spacing, alignment, `flex-*`, mins | `bottom_hbox_distributes_fixed_and_flexible_items`, `a_container_measures_the_edge_its_children_are_asked_for`, `a_nested_container_reports_the_extent_its_children_need`, `a_shrinking_row_stops_at_the_minimum_its_items_declare`, `flow_attributes_become_the_item_a_container_shares_space_with`, `align_self_overrides_the_alignment_of_its_container`, `justify_content_places_the_run_inside_the_room_it_has`, `item_spacing_and_gap_leave_the_same_distance_between_items`, `each_container_tag_accepts_the_alignment_spelling_it_documents`, `vbox_stacks_children_vertically_with_padding_and_margins` |
| padding, margin, and the single-side forms | `padding_and_margin_take_one_to_four_values_and_their_single_side_forms`, `control_padding_insets_what_the_control_draws` |
| percentages and pixel sizes | `percentage_and_pixel_extents_scale_with_the_layout` |
| wrapping rows | `a_wrapping_row_starts_a_new_line_when_the_next_item_does_not_fit`, `a_wrapping_row_gives_each_line_the_height_of_its_tallest_item`, `flex_wrap_is_opt_in_per_container` |
| a script writes a user's environment and claims a file type, and the uninstall takes both back | `an_install_script_sets_a_variable_the_uninstall_takes_back`, `an_install_script_removes_a_variable_it_no_longer_wants`, `a_script_registers_a_file_type_where_windows_reads_it`, `a_file_type_that_would_write_outside_the_classes_tree_is_refused` |
| a container that scrolls: what it holds, the view it shows, the bar, and the wheel that moves it | `a_scrollable_container_shows_the_part_it_is_scrolled_to`, `a_scrollbar_says_where_the_list_stands`, `a_wheel_over_a_list_brings_the_rows_below_into_reach` |
| `Spacer` | `a_spacer_takes_what_the_fixed_items_leave` |
| pinning by `right`, `bottom`, `inset` and the single-side forms | `an_element_is_pinned_by_the_edge_attribute_it_carries` |
| `Label` text, font, alignment and colour | `a_label_takes_its_text_font_and_alignment_from_the_layout`, `text_colors_read_as_rgb_with_or_without_an_alpha_channel`, `a_bound_label_shows_its_own_text_beside_the_value_it_reads` |
| a component chosen by a checkbox of its id, and what the page leaves ticked deciding which components install | `the_boxes_the_page_carries_decide_which_components_install`, `the_page_and_the_project_decide_which_components_install` |
| `Checkbox` state images, text, links, toggling | `an_absolutely_placed_checkbox_draws_its_state_image_and_toggles` |
| `RadioButton`: one value per group, the layout's default, and the click that replaces it | `a_radio_group_holds_one_value_at_a_time`, `a_click_on_a_radio_is_the_value_the_install_waits_for` |
| `Select`: the language list, and the options a project declares | `a_language_menu_lists_its_options_and_marks_the_one_in_use`, `a_closed_language_select_draws_its_arrow_over_its_fill_and_outline`, `a_select_offers_the_options_the_page_declares`, `the_language_menu_answers_to_the_keyboard` |
| `ProgressBar` track, clipping and live value | `a_progress_bar_paints_a_rounded_track_and_follows_the_live_value`, `progress_bar_clips_its_sprite_to_the_completed_share` |
| every `value-source` form and `value-format` | `value_sources_read_the_config_the_disk_and_the_running_step`, `formats_bound_disk_sizes`, `resolves_and_queries_windows_disk_root`, `status_source_replaces_placeholder_text_with_the_published_step` |
| text field editing: caret, selection, undo, word keys | `a_caret_sits_after_the_characters_before_it`, `a_double_click_selects_the_word_under_the_pointer`, `a_selection_band_covers_the_characters_it_selects`, `a_selection_is_ordered_from_whichever_end_the_caret_is_at`, `removing_a_selection_keeps_the_text_around_it`, `typing_coalesces_into_one_undo_step`, `undo_remembers_the_caret_that_belongs_to_the_value`, `word_keys_stop_at_the_boundaries_they_delete`, `byte_index_walks_characters_not_bytes`, `editable_text_fields_are_recorded_and_readonly_ones_are_not`, `a_typed_value_wins_over_the_bound_default`, `a_readonly_field_shows_its_value_without_taking_edits`, `an_empty_field_is_one_a_user_can_click_into` |
| the IME composition window and candidate list anchored at the caret the page drew | `an_input_method_anchors_at_the_caret_the_page_drew` |
| `visible="false"` and panel show/hide | `a_hidden_element_takes_its_whole_subtree_with_it`, `a_panel_pair_shows_the_panel_and_only_the_control_that_fits` |
| Page order and the job a page declares (`next`, `back`, `role`) | `a_page_role_finds_the_page_that_holds_it`, `a_page_list_without_roles_keeps_its_positions`, `a_next_button_walks_to_the_page_the_project_declares` |
| every action in the table | `every_action_in_the_table_answers_with_its_own_window_action`, `action_attributes_map_to_window_actions` |
| dialogs: placement, buttons, notices, growing cards | `a_dialog_is_drawn_over_the_page_and_centred`, `a_dialog_button_answers_with_its_own_action`, `a_dialog_button_is_drawn_from_the_question_rather_than_the_layout`, `a_notice_hides_the_secondary_button`, `a_page_without_a_dialog_draws_no_overlay`, `every_shipped_question_keeps_its_answers_inside_the_card`, `the_example_dialog_places_its_message_and_both_buttons` |
| link resolution order | `agreement_links_resolve_through_the_project_links_table`, `a_link_the_project_does_not_configure_stays_plain_text`, `link_runs_carry_their_target_and_plain_runs_do_not`, `agreement_markdown_becomes_colored_visible_runs` |
| the pointer answers only where an action is declared | `an_element_answers_the_pointer_only_when_it_declares_an_action` |
| the cursor shape over a control: a hand on a button, a beam on a field a person can type in, an arrow on the page | `the_pointer_decides_which_cursor_the_wizard_shows` |
| the folder picker's target | `pick_directory_writes_to_the_field_the_page_offers_it`, `pick_directory_falls_back_to_the_layout_text_input` |
| a `pick_directory` button opens the shell's own folder dialog, and closing it again leaves the wizard as it was | `a_browse_button_opens_the_folder_picker_and_leaving_it_changes_nothing` |
| window placement and scaling | `a_window_is_centred_and_clamped_to_its_work_area`, `a_placed_window_is_pulled_back_inside_its_work_area`, `a_display_scales_the_layout_by_its_own_dpi` |
| stopping the running task from the window (a `cancel` button, a confirmed `close_confirm`) | `a_cancel_button_stops_the_project_script_and_leaves_nothing_installed` |
| the example project's own pages | `taptap_first_page_places_controls_at_192_dpi`, `taptap_uninstaller_buttons_have_distinct_hit_regions`, `progress_pages_render_every_control_they_declare` |

## Project scripts

`docs/en/SCRIPT_API.md` lists the primitives. The in-process cases drive the driver directly; the
setup-level cases prove the `scripts` directory and the tools directory survive packaging.

| Behaviour | Cases |
| --- | --- |
| the contract: deploy the named executable, or roll back | `an_install_script_that_never_deploys_the_executable_is_refused`, `an_install_script_deploys_files_and_writes_the_manifest` |
| a failing script undoes what it wrote, and reports its log | `a_failing_install_script_removes_what_it_wrote`, `a_script_failure_reports_the_messages_it_logged`, `every_log_level_reaches_the_failure_the_wizard_shows` |
| `run_tracked_uninstall`, and the library fallback when it is skipped | `an_uninstall_script_replays_the_manifest_it_asks_for`, `an_uninstall_script_that_skips_the_manifest_still_removes_the_product`, `replaying_the_manifest_is_refused_while_installing` |
| file and path primitives | `file_primitives_create_copy_list_and_remove_files`, `path_primitives_join_split_and_name_paths` |
| registry primitives, including the shared-key rule | `registry_primitives_round_trip_and_forget_a_key_they_created`, `uninstalling_a_shared_key_removes_only_the_value_the_script_wrote` |
| shortcut primitives | `an_install_script_creates_shortcuts_the_uninstall_takes_back`, `a_script_deletes_the_desktop_shortcut_and_the_start_menu_folder_it_created` |
| progress, status, mode, checkboxes and cancellation | `out_of_range_progress_and_both_status_forms_do_not_disturb_the_install`, `an_uninstall_script_sees_the_uninstall_mode_and_the_keep_data_checkbox`, `a_script_step_text_wins_over_the_locale_key`, `a_built_in_step_clears_a_script_step_text`, `a_running_script_sees_the_cancel_request`, `a_cancelled_install_gives_up_after_the_script_and_undoes_what_it_wrote` |
| the components a script reads off the run: `is_component_selected` and `selected_components` | `a_script_sees_the_components_the_run_installs` |
| the values the script reads off the page: the text a field holds and the value a choice control stands on, and an empty string for an id the page never declares | `a_script_reads_the_values_the_page_holds`, `the_values_the_page_holds_reach_the_script` |
| environment, configuration, drives and the running image | `the_script_reads_the_environment_and_the_project_configuration`, `the_script_reports_the_image_it_runs_from`, `the_script_queries_fixed_disks_and_notifies_the_shell` |
| the tools a project bundles | `a_script_reads_the_tools_the_project_bundled`, `a_script_that_asks_for_tools_a_project_did_not_bundle_gets_nothing`, `a_setup_unpacks_the_tools_its_project_bundles` |
| processes | `a_script_runs_a_command_and_sees_its_exit_code`, `a_script_recognises_a_running_process_by_its_image_name` |
| keeping or deleting user data | `uninstalling_below_appdata_removes_the_data_only_when_the_box_is_cleared` |
| the `scripts` directory reaching a built setup | `a_setup_runs_the_projects_own_install_and_uninstall_scripts` |
| a message, an error and a question a script raises, drawn in the wizard and answered by a click | `a_question_a_script_asks_is_drawn_with_both_of_its_answers`, `a_script_dialog_is_drawn_in_the_wizard` |

## Install, upgrade and uninstall

| Behaviour | Cases |
| --- | --- |
| components: the page and the project together decide which ones install, and two archives carrying one path are refused | `a_silent_run_installs_the_components_the_project_defaults_to`, `the_boxes_the_page_carries_decide_which_components_install`, `two_payloads_that_carry_one_file_are_refused` |
| a fresh install: files, manifest, registration | `installs_a_fresh_directory_and_records_the_manifest`, `a_built_setup_installs_its_payload_and_registers_an_uninstall_entry` |
| config paths expanded, explicit directory wins | `configured_install_paths_are_expanded`, `an_explicit_install_path_wins_over_the_configured_one`, `a_configured_percent_path_is_expanded_and_used`, `an_explicit_directory_wins_over_the_configured_one` |
| refusing to install over a directory it did not create | `refuses_to_install_over_a_directory_it_did_not_create`, `refuses_a_foreign_manifest_at_the_destination`, `refuses_relative_or_root_installation` |
| upgrade: replacing the version, dropping stale files, keeping foreign ones | `an_upgrade_replaces_the_previous_version_and_drops_stale_files`, `installing_over_an_existing_installation_drops_stale_files`, `an_upgrade_keeps_a_file_the_payload_does_not_own` |
| rollback on failure | `a_failed_upgrade_restores_the_previous_version`, `removes_a_fresh_install_whose_registration_fails` |
| cancellation: the task gives up at a checkpoint and undoes what it wrote | `a_cancel_request_stops_the_checkpoints_that_follow_it`, `a_cancelled_deployment_writes_nothing` |
| uninstall removes the product, the registration and the directory | `uninstalling_removes_the_product_the_registration_and_the_directory` |
| uninstall keeps a directory holding user files | `uninstall_keeps_a_directory_that_still_holds_user_files` |
| shortcut and autostart bookkeeping | `a_silent_install_writes_the_shortcuts_and_the_autostart_entry`, `removes_recorded_shortcuts_and_only_their_empty_folder`, `drops_the_shortcut_folder_once_it_is_empty` |
| windowless runs and the switches that allow them | `silent_arguments_read_the_directory_and_reject_anything_else`, `an_unknown_silent_option_is_refused`, `a_project_that_did_not_opt_in_refuses_a_windowless_run` |
| the bundle the setup carries, and a signature appended behind it | `project_bundle_roundtrips_layout_assets_and_locales`, `bundle_index_streams_entries_without_loading_the_payload`, `bundle_index_ignores_images_without_a_footer`, `bundle_index_reads_a_bundle_that_a_signature_follows`, `a_setup_with_a_signature_appended_still_installs` |
| the wizard window opens at the size the project declares | `the_setup_opens_its_wizard_window` |
| the resources Windows reads before the process starts | `a_built_setup_carries_a_readable_bundle_and_real_resources`, `manifest::tests::the_manifest_reaches_a_real_executable` |

## The visual builder

| Behaviour | Cases |
| --- | --- |
| open a project, and refuse a folder without one | `open_project_reads_the_folder_that_holds_installer_config_json`, `open_project_names_the_missing_project_file` |
| refresh, and a custom output path | `refresh_keeps_a_custom_output_path_for_the_same_project`, `refresh_replaces_a_custom_output_path_when_the_project_changes`, `validation_preserves_custom_output_for_same_project` |
| reset output, automatic stub search | `reset_output_restores_the_dist_path_named_by_the_project`, `an_empty_runtime_directory_leaves_the_stub_search_automatic` |
| parameters reach the build request | `parameters_carry_the_project_output_and_runtime_directory_into_the_request`, `parameter_fields_use_the_available_work_area` |
| the build re-reads the project instead of trusting the screen | `build_setup_rechecks_a_project_marked_dirty_before_it_starts`, `a_build_reads_the_project_from_disk_in_its_worker_thread` |
| retry repeats the failed build | `retry_repeats_a_failed_build_with_the_parameters_it_stored` |
| the log keeps every line, in order, in English | `the_log_keeps_the_lines_the_view_scrolled_past`, `build_messages_reach_the_log_in_the_order_the_worker_sent_them`, `log_commands_are_english_when_interface_is_chinese`, `log_severity_colors_preserve_selectable_text`, `log_timestamp_has_fixed_width` |
| menus, panels, sidebar paths, warnings | `menu_labels_stay_on_one_line_in_both_languages`, `the_panels_draw_in_every_state_they_can_be_in`, `sidebar_paths_keep_drive_and_relevant_tail`, `inspection_findings_reach_the_sidebar_and_the_log` |
| one build at a time, and honest sizes and progress | `a_running_build_refuses_a_second_one`, `build_progress_is_monotonic`, `formats_gui_sizes`, `inspection_time_is_explicit_and_stable` |
| icons, argument quoting, archive rejection | `window_icon_uses_the_brand_asset`, `command_arguments_are_quoted`, `rejects_unsafe_7z_paths`, `zip_backend_rejects_invalid_archive` |
| a broken project file | `a_broken_project_file_is_reported_to_the_user` |

## What is still not covered by an automated case

- **Windows 7 SP1.** The compatibility claims need a clean Windows 7 SP1 x64 machine, which this
  project has no environment for. See [production status](PRODUCTION_STATUS.md).
- **Whether a glyph reads correctly.** A rendering check can tell that text was drawn in the colour
  and position the layout asked for; it cannot tell that the sentence is legible. The snapshot
  review script hands that question to a local vision model, or to a person.
- **The two windows an input method draws itself, and choosing inside the shell's folder dialog.** A
  case proves that the composition point and the candidate point the runtime hands the input method
  sit on the caret; it cannot prove the input method drew them there. It proves that a
  `pick_directory` button opened the shell's folder dialog; it cannot prove which field a directory
  chosen there is written to.
- **Script primitives that cannot be undone by a test:** `run_detached` deliberately outlives the
  run; `kill_process` ends a process the test did not start; `sleep_ms` has only elapsed time to
  assert on; `is_elevated` would only mirror the implementation.
