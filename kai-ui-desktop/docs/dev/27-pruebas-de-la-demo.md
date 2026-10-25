# 27 · Pruebas de la demo: qué es del framework y qué es de la demo

Paso previo a R5 ([26](26-router-y-kai-build.md)). La idea es rehacer la demo
desde cero, en su propio crate con `kai-build`, en vez de partir `src/demo.rs`.
Antes de borrar ese archivo hay que rescatar sus 104 pruebas: la mayoría
prueba el **framework** y solo usa la demo como banco de pruebas.

## 1. Criterio

- **Framework**: la prueba seguiría teniendo sentido con cualquier app que
  use el mismo componente o subsistema. Pasa a vivir junto a ese código, con
  una app mínima propia.
- **Demo**: la prueba comprueba la demo en sí (sus secciones, sus `test_id`,
  su contenido). Se reescribe en `tests/` del crate nuevo de la demo cuando
  la sección correspondiente exista.
- **Capturas y benchmarks**: `#[ignore]`; se mueven con la demo nueva.

Destino de las pruebas del framework: `src/session/<tema>_tests.rs`, como ya
hacen `busy_tests.rs`, `map_tests.rs` y `router_tests.rs`, o el módulo del
componente (`src/components/<x>.rs`).

### App de pruebas compartida

Muchas pruebas usan las mismas piezas de la demo: un formulario con nombre y
enviar, un contador con `Increment`/`Reset`, comandos con y sin entrada, una
acción destructiva. En vez de diez apps mínimas casi iguales, propongo **una**
`src/testing/sample_app.rs` (`#[cfg(test)]`) con eso y nada más. Las pruebas
de comandos, paleta, agentes y MCP la comparten.

## 2. Clasificación

Líneas de cada prueba entre paréntesis (total ≈ 5 000).

### Framework (≈ 80 pruebas)

| Destino | Pruebas |
|---|---|
| `session/commands_tests.rs` (13) | `availability_follows_update` (50), `invoke_runs_enabled_actions_and_reports_disabled_ones` (19), `menu_bar_follows_the_registry` (39), `command_shortcut_sends_the_same_msg_as_the_button` (10), `disabled_command_shortcut_sends_nothing` (10), `command_shortcuts_do_not_steal_copy_in_a_field` (14), `a_button_follows_its_command` (37), `disabled_command_stays_disabled_in_button_palette_and_menu` (16), `every_action_does_the_same_from_every_via` (17), `disabled_actions_do_nothing_from_any_via` (48), `actions_with_input_stay_out_of_menus_and_palette` (17), `command_z_undoes_typing_in_a_field_and_the_app_hears_it` (19), `messages_record_where_they_came_from` (46) |
| `session/palette_tests.rs` (8) | `palette_opens_with_focus_in_search_and_filters` (36), `enter_runs_the_selected_command_only_when_enabled` (20), `arrows_move_the_selection` (17), `assistive_tech_reads_the_selected_row_while_focus_stays_in_search` (55), `escape_gives_focus_back_and_app_ids_do_not_change` (22), `tab_stays_inside_the_palette` (18), `clicks_run_a_row_or_close_on_the_backdrop` (145), `the_palette_link_opens_the_palette` (16) |
| `session/agents_tests.rs` (8) | `json_calls_run_actions_and_queries` (75), `query_reads_the_model_without_msg` (91), `agents_confirm_destructive_actions_in_a_dialog` (51), `an_agent_wait_command_answers_only_after_the_task_finishes` (15), `agent_calls_wait_behind_the_dialog_and_can_be_cancelled` (37), `agent_calls_fail_before_asking_when_they_cannot_run` (28), `people_and_scripts_never_see_the_dialog` (19), `the_indicator_shows_the_agent_and_its_last_action` (22) |
| `session/mcp_tests.rs` (feature `mcp`, 3) | `mcp_lists_the_exposed_commands_as_tools` (45), `mcp_destructive_calls_wait_for_the_person` (56), `mcp_tool_calls_run_on_the_app` (76) |
| `session/busy_tests.rs` (ya existía y las cubría; solo faltaba `loading(true)`, que se añade como `loading_shows_the_dots_without_a_task`) | `waiting_buttons_show_the_dots_until_their_task_ends` (22), `a_chained_save_and_a_background_sync_in_the_demo` (20), `the_wait_command_is_in_course_everywhere_while_it_runs` (22) |
| `session/focus_tests.rs` (8; el segmentado y los diálogos con apps mínimas locales, el aviso tras confirmar ya lo prueba `components/toast.rs`) | `return_in_name_field_activates_the_default_button` (31), `return_in_the_name_field_submits_through_the_default_button` (9), `demo_focus_order_follows_document_order_and_skips_disabled` (70), `tab_moves_through_the_form_in_order_showing_the_ring` (19), `the_password_eye_toggles_and_leaves_focus_in_the_field` (11), `clicked_buttons_show_no_focus_ring_but_fields_keep_their_focus_look` (71), `background_effects_are_one_segmented_tab_stop` (51), `dialogs_trap_focus_and_confirming_shows_a_toast` (38) |
| `session/layout_tests.rs` (2) | `field_rows_stack_in_a_narrow_window` (66), `mix_c_buttons_line_up_with_fields_in_an_items_end_row` (65) |
| `session/document_tests.rs` (feature `document`, 16, sobre `testing::document_app::Writer`); `editor_inserts_unicode_and_selects_text` va a `text_field.rs` | `document_types_bold_and_undo`, `document_pointer_drag_selects_text`, `document_shift_arrows_extend_selection`, `document_clipboard_copy_cut_paste_plain`, `document_clipboard_copy_preserves_bold_via_html`, `document_multipage_scrolls_and_keeps_hit_target`, `document_inserts_table_and_picture_from_toolbar`, `document_ribbon_bold_and_insert_table`, `document_contextual_tabs_follow_table_and_picture`, `document_ribbon_collapses_with_the_window_width`, `document_new_comment_asks_for_its_text`, `document_ribbon_shows_bold_pressed_and_cut_disabled`, `document_ribbon_layout_review_view_tabs`, `document_save_docx_via_file_panel`, `document_open_docx_via_file_panel`, `document_export_pdf_via_file_panel`, `editor_inserts_unicode_and_selects_text` (≈ 500 en total) |
| `components/table` (6) | `the_toolbar_search_filters_and_the_headers_sort_the_table` (70), `exporting_saves_the_rows_the_table_shows_where_the_person_says` (31), `the_columns_button_hides_a_column_and_the_search_still_looks_in_it` (35), `a_client_row_opens_its_detail_and_the_export_leaves_it_out` (31), `the_filter_button_shows_a_field_per_column_and_typing_waits_for_a_pause` (63), `the_table_shows_an_avatar_by_each_name` (18) |
| `session/map_tests.rs` (3, ya existe) | `the_map_follows_gestures_and_agents` (64), `a_sector_is_edited_on_the_map_and_kept_only_on_finish` (81), `crossing_edges_block_finish_until_fixed` (44) |
| Componentes sueltos (6) | `toasts_close_on_their_own_or_with_their_button` → `components/toast.rs`; `the_all_checkbox_is_mixed_and_sets_every_permission` → checkbox; `clicking_a_bar_selects_its_category` → `charts`; `image_fields_take_picked_and_dropped_files_and_refuse_others`, `load_url_puts_a_downloaded_image_in_the_avatar` → `media`/image field; `the_inspector_describes_what_is_under_the_pointer` → `devtools` |
| `background` (2) | `background_panel_messages_drive_the_window_background` (37); `background_env_parses_kind_and_motion_and_warns_on_unknown_values` (24, junto con `background_from_env`, que sale de la demo) |

### Navegación: la cubren el router y `kai-build` (7)

Estas pruebas hablan de `Section`, que desaparece. Cada una se comprueba
contra `router_tests.rs`/`pages_tests.rs` y `fixtures/sections-app`. Si ya
está cubierta, se descarta; si no, se añade allí:

`the_sidebar_shows_one_section_at_a_time`, `the_sidebar_groups_every_section`,
`the_sidebar_collapses_to_a_rail_by_shortcut_button_or_command`,
`scripts_and_agents_change_the_section_with_ui_go_to`,
`each_section_opens_at_the_top`, `dev_restarts_keep_section_sidebar_and_sector`,
`preferences_survive_a_restart`.

### Demo (9): se reescriben en el crate nuevo

`update_follows_messages_and_view_reflects_the_model`,
`test_ids_are_unique_and_point_at_the_controls`,
`typing_a_name_enables_send_and_the_click_submits_it`,
`the_form_picks_a_role_and_accepts_the_terms_and_clear_resets_them`,
`the_demo_shows_every_component` (363),
`demo_uses_only_supported_classes_and_fills_the_window`,
`sectors_are_chosen_from_the_list_the_map_and_agents` (118),
`the_selected_sector_changes_color_with_the_color_field` y
`agents_replace_a_sector_outline_with_a_valid_ring` (valida los anillos de los
sectores de Parral, que solo existen en la demo) y `mcp_schema_matches_snapshot`
(`mcp-schema.json` es lo que la demo publica a los agentes; se va con ella).

### Capturas y benchmarks (8, `#[ignore]`)

`snapshot_demo`, `snapshots_appkit`, `snapshots_mix`, `snapshots_renders`,
`snapshots_backgrounds`, `bench_element_map`, `bench_layout_and_paint`,
`bench_map_pan`. Van con la demo nueva. `CLAUDE.md` y la documentación que
citan `snapshot_demo` se actualizan cuando se muevan.

## 3. Orden

1. `src/testing/sample_app.rs`.
2. Mover por bloques, uno por commit, empezando por los que no tocan la
   demo: commands → palette → agents → mcp → busy → focus → layout →
   document → componentes → map → background. Cada prueba movida se borra
   de `demo.rs` en el mismo commit; `cargo test` en los tres modos debe
   pasar en cada uno.
3. Revisar el bloque de navegación contra el router.
4. Lo que queda en `demo.rs` (pruebas de demo, capturas y benchmarks) se va
   con la demo nueva: R5 propiamente dicho.

La regla durante la mudanza: **una prueba no se debilita al moverla.** Si en
la app mínima pierde una aserción, se documenta en el commit.
