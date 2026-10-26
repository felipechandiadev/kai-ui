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

### Framework (65, ya fuera de la demo)

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
| Componentes sueltos (3) | `clicking_a_bar_selects_its_category` → `charts` (app mínima); `the_inspector_describes_what_is_under_the_pointer` → `session/devtools_tests.rs` (sobre `Sample`); `toasts_close_on_their_own_or_with_their_button` se descarta: `components/toast.rs` ya cubre roles, cierre automático, errores que se quedan y el botón de cerrar |

### Navegación (7): se quedan con la demo hasta R5

Prueban la navegación hecha a mano en la demo (`Section`, `ui.go_to`,
`DevState`, `DemoPreferences`), que R5 reemplaza por el router; mientras
la demo la use, la siguen protegiendo:
`the_sidebar_shows_one_section_at_a_time`, `the_sidebar_groups_every_section`,
`the_sidebar_collapses_to_a_rail_by_shortcut_button_or_command`,
`scripts_and_agents_change_the_section_with_ui_go_to`,
`each_section_opens_at_the_top`, `dev_restarts_keep_section_sidebar_and_sector`,
`preferences_survive_a_restart`.

Comparadas con lo que ya prueban `router_tests.rs`, `pages_tests.rs` y
`fixtures/sections-app`:

| Lo que comprueba la demo | En la raíz con router |
|---|---|
| un clic en la barra lateral muestra una sección | `a_section_keeps_its_state_and_the_sidebar_switches_sections` (sections-app) |
| ↑/↓ recorren las secciones | **nueva:** `the_arrows_walk_the_sidebar_sections` |
| cada sección abre arriba | `coming_back_restores_the_scroll_and_the_focus` («a new route starts at the top») |
| `ui.go_to` desde scripts y agentes | **nueva:** `kai_navigate_shows_a_route_and_the_sidebar_follows` |
| grupos de la barra lateral | `sidebar_rows_are_one_selecting_stop_counted_across_groups` (componente) |

Corregido de paso: al volver a una sección con ↑ en la barra lateral, el
router restauraba el foco que había guardado para esa ruta, que ya estaba
en la fila de abajo. Ahora el foco se queda si sigue en el mismo control
en las dos vistas (mismo id, rol, nombre y `test_id`; los ids son
posicionales). Lo prueban `the_focus_stays_on_the_row_the_arrows_chose` y
`the_kept_focus_wins_over_a_control_that_only_shares_its_id`.

Lo que la raíz generada **todavía no tiene** y R5 debe resolver antes de
pasar la demo a ella:

- ~~**Barra lateral plegable a riel**~~ ✅ Comando del framework
  `kai.toggle_sidebar` (⌃⌘S, menú «Ver»; `CommandRegistry::sidebar` y el
  trait `CollapsibleSidebar`), botón al pie de la barra, ventana de menos
  de 640 pt y `dev_state`. Lo prueba `the_sidebar_collapses_to_a_rail`
  (`fixtures/sections-app`). De paso, los comandos de navegación se
  registran como del framework y ya no sale «kai.* son del framework».
- ~~**Mantener la ruta en los reinicios de `kai dev`**~~ ✅ La raíz
  generada guarda `{"path": …}` en `App::dev_state` y la reabre en
  `restore_dev_state`; una ruta que ya no existe se ignora. Lo prueba
  `kai_dev_reopens_the_route_on_screen_and_ignores_an_unknown_one`
  (`fixtures/sections-app`).
- ~~**Recordar la última sección** entre ejecuciones~~ ✅ `run` abre un
  `Store` con el nombre del paquete y `AppRoot::with_store` arranca en la
  sección guardada (`kai::router::LastSection`), si aún existe; cada
  cambio de sección se guarda fuera del hilo de UI. Sin `with_store` (las
  pruebas) no se guarda nada. Lo prueba
  `the_app_reopens_at_the_last_section_it_showed` (`fixtures/sections-app`).
- ~~**`kai.navigate` con una ruta desconocida contesta `Ok`**~~ ✅ Ahora
  contesta «No hay ninguna vista en …»: `Navigable::has_route` (la raíz de
  `kai-build` la genera con `Route::parse`) y `Action::validate`, que el
  registro llama tras decodificar la entrada.

### Demo (24): se reescriben en el crate nuevo

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

También se quedan con la demo las pruebas de componentes que en realidad
prueban cómo la demo los conecta; las piezas ya tienen sus pruebas en
`components/table.rs`, `components/table/view.rs`, `components/media.rs` y
`components.rs`:

- Tabla (6): `the_toolbar_search_filters_and_the_headers_sort_the_table`,
  `exporting_saves_the_rows_the_table_shows_where_the_person_says`,
  `the_columns_button_hides_a_column_and_the_search_still_looks_in_it`,
  `a_client_row_opens_its_detail_and_the_export_leaves_it_out`,
  `the_filter_button_shows_a_field_per_column_and_typing_waits_for_a_pause`
  (la pausa, `FILTER_PAUSE`, es de la demo) y
  `the_table_shows_an_avatar_by_each_name`.
- `the_all_checkbox_is_mixed_and_sets_every_permission` (la lógica de
  «Todos» es de la demo; el checkbox mixto ya se prueba en `components.rs`).
- `image_fields_take_picked_and_dropped_files_and_refuse_others` y
  `load_url_puts_a_downloaded_image_in_the_avatar` (el comando
  `media.load_url` y los títulos de los paneles son de la demo).
- Mapa (3): `the_map_follows_gestures_and_agents`,
  `a_sector_is_edited_on_the_map_and_kept_only_on_finish` y
  `crossing_edges_block_finish_until_fixed`. Los gestos, el deslizamiento
  sin eco, el salto sin animación y el editor de polígonos (arrastre, ⌘Z,
  punto medio, bordes cruzados, Return y Escape) ya se prueban en
  `maps/tests.rs` y `maps/camera.rs`; lo que queda es de la demo: los
  comandos `map.*`, los sectores de Parral, la superficie y la regla de no
  guardar un borrador cruzado. (`session/map_tests.rs` prueba
  `Element::map`, no el mapa.)
- Fondo (2): `background_panel_messages_drive_the_window_background` y
  `background_env_parses_kind_and_motion_and_warns_on_unknown_values`.
  Prueban el panel «Fondo de ventana» y `background_from_env` (`KAI_BG`,
  `KAI_BG_ANIM`), que son de la demo; `BackgroundKind::parse` y «sin efecto,
  nada que animar» ya se prueban en `background.rs`.

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

## 4. La demo nueva (`kai-ui-desktop/demo`)

Crate aparte, `kai-demo`, con la raíz generada por `kai-build`: una carpeta
por sección en `demo/src/sections/`. La raíz no puede vivir dentro del
crate `kai` (el código generado nombra `kai::…`), por eso no va en
`src/sections/`. `scripts/check.sh` le pasa fmt, clippy y tests. `demo.rs`
sigue funcionando hasta que todas las secciones estén aquí.

| Sección | Estado | Pruebas |
|---|---|---|
| Botones (`/buttons`) | ✅ | `the_buttons_count_clicks_and_danger_resets_them`, `a_waiting_button_says_when_its_task_ends`, `the_section_command_runs_from_its_button_and_from_agents` |
| Inicio (`/home`) | ✅ (sin el fondo) | `the_counter_follows_its_button_shortcut_and_commands`, `typing_a_name_enables_send_and_the_click_submits_it`, `the_form_picks_a_role_and_accepts_the_terms_and_clear_resets_them` |
| Campos (`/fields`) | ✅ | `the_fields_take_what_is_typed_and_their_ids_reach_the_controls`, `the_eye_shows_and_hides_each_secret_on_its_own` |
| Selección (`/selection`) | ✅ | `the_all_checkbox_is_mixed_and_sets_every_permission` |
| Contenedores (`/containers`) | ✅ | `the_card_counts_its_actions_and_the_tabs_show_one_panel_at_a_time` |
| Badges y alertas, Multimedia, Tabla, Mapa, Gráficos, Documento, Diálogos y avisos, Lista + detalle, Formulario, Vista de datos, Catálogo vivo | pendiente | |

Diferencias con `demo.rs`:

- Cada sección guarda su estado; el contador de clics de «Botones» es suyo
  (antes era el de Inicio).
- ~~**Hueco:** una sección no puede registrar comandos~~ ✅ Comandos por
  página: `PageAction` (como `Action`, pero lee la página y produce su
  mensaje), `Page::commands` para declararlos con
  `CommandRegistry::page_action`, `HasPage<P>` (la raíz generada lo
  implementa por cada página y llama a `commands` de todas) y
  `Element::on_page_command` para el botón. Un comando de página está
  oculto mientras su página no está abierta; una sección sigue abierta al
  cambiar a otra. Lo prueban `a_page_command_runs_while_its_page_is_open`
  (`session/pages_tests.rs`) y
  `the_section_command_runs_from_its_button_and_from_agents` (la demo,
  `buttons.simulate_wait`).
- Consultas por página: `PageQuery` (lee la página y el estado
  compartido), `CommandRegistry::page_query` y
  `CommandBuilder::returns_page` (`form.submit` contesta con
  `form.last_submission`). `kai::commands::Key` se reexporta para que las
  apps declaren atajos.
- Inicio queda **sin** lo que la raíz generada todavía no permite:
  - la tarjeta «Fondo de ventana» (`App::background` es de la raíz; una
    página no puede cambiarlo);
  - la cuenta de envíos en la fila de la barra lateral (`SectionMeta` es
    constante, sin badge);
  - la barra de herramientas (`bar.add`, `bar.find`) y los clics en el
    título de la ventana.
- El ojo de «Contraseña» usa un mensaje de la página (`password`), no el
  comando `form.toggle_password`, que tampoco se exponía a agentes.
- Las estadísticas de Contenedores cuentan lo de la propia sección
  (aceptadas, descartadas, pestaña abierta); en `demo.rs` leían la tabla y
  los avisos de otras secciones.
- `TestApp::unknown_classes()` es pública para que las apps externas
  comprueben sus clases.
