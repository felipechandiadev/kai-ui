# Changelog — kai-ui-desktop-prototype

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).

## [Unreleased]

Avance hacia la plataforma descrita en [`docs/dev/13-hoja-de-ruta-plataforma.md`](docs/dev/13-hoja-de-ruta-plataforma.md) (Etapas 0 a 3 cerradas, Etapa 4 en curso).

### Added

- **Spacing + densidad + do/don't (Fase D):** `theme::spacing` (pasos tipados hasta step 16 ≈ 64 pt; alias `FORM_STACK_STEPS` / `DATA_STACK_STEPS`); `theme::motion` (CARET_BLINK / TOAST / TYPEAHEAD_RESET documentados). Guía do/don't Button / TextField / Document. Docs [`23-spacing-density.md`](docs/dev/23-spacing-density.md).
- **Plantillas de página (Fase C):** `PageHeader` / breadcrumbs, `MasterDetailPage`, `FormPage` (comfortable), `DataPage` (compact) sobre PageGrid 12. Demo: grupo sidebar **Plantillas** (Lista + detalle, Formulario, Vista de datos). Docs [`22-page-templates.md`](docs/dev/22-page-templates.md).
- **PageGrid 12 + shell de página (Fase B):** `page` / `PageBody` / `page_grid` / `PageGridItem` (span 1–12; 4/8/12 cols; gutter 16 pt; márgenes responsivos). Utilidad `col-span-N` en el motor. Demo: shell de sección + formularios Inicio/Campos. Docs [`21-page-layout.md`](docs/dev/21-page-layout.md). No es Carbon 16 ni UI Shell web.

- **Design system P0 (Polaris/Carbon/M3 gap):**
  - Matriz de estados en la demo: Campos → «Estados de TextField» (default/focus/disabled/required/helper/error) y Botones → «Estados de Button» (variantes + disabled + loading). Placeholders neutros. `TextField::error` (borde + helper danger AppKit).
  - Patrones `EmptyState` + `loading_placeholder` (lista/tabla); demo en Tabla. Docs [`18-patterns-empty-loading.md`](docs/dev/18-patterns-empty-loading.md).
  - Mapa de iconografía de campos (`FieldKind`, `field_icon_for`, `TextField::field_icon`); iconos Lucide `lock`/`phone`. Docs [`19-iconografia-campos.md`](docs/dev/19-iconografia-campos.md). Correo de Inicio ya no usa lupa.
  - Contraste WCAG E2E: `Color::contrast_ratio` / `composite_over` + tests `theme::contrast` (AA con excepciones documentadas). Docs [`20-contraste-wcag.md`](docs/dev/20-contraste-wcag.md).

- **Ribbon Documento AppKit (iconos + ES):** comandos como `toolbar_item` (icono + tooltip ES + atajo); labels de grupo 11px muted y separadores verticales; tabs Inicio/Insertar/Diseño/…; QAT Abrir/Guardar/Exportar PDF a la izquierda de las tabs (`DocumentRibbon::leading`). Catálogo Lucide ampliado (bold/italic/align/undo/…). Docs [`17-document-ribbon.md`](docs/dev/17-document-ribbon.md). Ids de comando siguen en EN.

- **Caret blink (P0):** fase booleana ~530 ms (AppKit) en TextField y Document; `Session::animating` pide frames con foco de texto; reset al tipar/mover caret; `reduce_motion` → caret fijo visible; Document usa `semantic.caret` y oculta si ventana inactiva.
- **Demo Documento Pages (P0):** shell sin card/`h-96`/`max-w-3xl`; ribbon + Archivo + canvas `flex-1 min-h-0` / `bg-under-page`; scale viewport `min(w/pw, h/ph)` para caber una hoja Letter.
- **Default AppKit (P0):** `demo_theme` / `run_from_env` sin `KAI_THEME` (o vacío/`Err`) → AppKit current light; `KAI_THEME=kai` explícito.


- **Widget `document` — Ribbon Kai (ampliado):** `RibbonMode::Full` añade Layout / Review / References / View desde el catálogo WordCraft (allowlist; `default_run_params` para márgenes/columnas/saltos). Demo + TestApp cambian de tab y disparan cmds. Docs [`17-document-ribbon.md`](docs/dev/17-document-ribbon.md). Design / keytips / Backstage = deuda. Excluye cmds con dialogs egui/rfd o params interactivos.
- **Widget `document` — Ribbon Kai (MVP):** chrome `document_ribbon` / `RibbonMode` alimentado por `wordcraft_engine::cmd::registry()` (`CommandSpec` id/label/location/shortcut). Tabs Home+Insert (allowlist); clic → `DocumentCmd` / `Session::run`. Demo Datos→Documento reemplaza la toolbar ad-hoc; test_ids `documento.ribbon*`. Docs [`17-document-ribbon.md`](docs/dev/17-document-ribbon.md). Keytips / Backstage / resto de pestañas Word = deuda. Atlas glifos + clipboard body siguen en stash WIP (no bloquean Ribbon).
- **Widget `document` — fase 3 (archivo):** abrir/guardar `.docx` (`wordcraft-docx`) y exportar PDF (`wordcraft-pdf`) vía `Effects::pick_file` / `save_file` + `perform` (encode off-thread; `DocumentCmd::TakeDocument` clona el modelo). Demo Archivo + Actions `documento.abrir` / `documento.guardar` / `documento.exportar_pdf`. Plan Document 0–3 cerrado a nivel widget demo; atlas glifos / clipboard / markdown rich = deuda post-plan.
- **Widget `document` — fase 2 (tablas e imágenes):** `insert.table` (2×2) e `insert.picture` (PNG demo); celdas/bordes como quads; bits vía `ImageDraw` / atlas Kai (WordCraft layout de la caja). Toolbar + Actions `insert.table` / `insert.picture`. Bridge TextStore de glifos sigue como deuda.
- **Widget `document` — fase 1 (páginas):** viewport fijo + stack Letter + `scroll_y` interno; hit-test y selección arrastre multi-página vía `wordcraft-layout`; demo seed multi-página; TestApp scroll + tipar. Bridge `TextStore`→paint sigue como deuda de atlas (no shaping del cuerpo).
- **Widget `document` — fase 0 (nota rica):** con `feature = "document"`, una `wordcraft_engine::Session` por `WidgetId` tipa (IME/teclas), selecciona, aplica `format.bold` / `format.italic` / `para.bullets` / `edit.undo` / `edit.redo`, y pinta la display list como `DrawOp::{Text, Quad}`. Demo Datos → Documento con toolbar Kai y persistencia Markdown en el modelo. Comandos públicos `format.bold`, `format.italic`, `para.bullets`, `edit.undo`, `edit.redo`. Bridge temporal de glifos vía `TextStore`. Sin egui / vello_cpu / dsp-27.

- **Corte A1 (Etapa 5 / P17–P18):** `kai.toml` parseado por `kai-cli`; comandos `kai doctor`, `kai test`, `kai new`; `kai dev` lee `[dev]` y observa el manifiesto. Ejemplo en `kai-ui-desktop/kai.toml`. Fuera de este corte: `commands`/`mcp` CLI, bundle/firma (P20), runtime leyendo el manifiesto.

- **Frontera pública — `Color`:** `kai::Color` / `kai::color` con `parse_hex` / `hex` / `to_hex`. Las apps no deben usar `kai::render::Color` (módulo `#[doc(hidden)]`). `fixtures/external-app` compila contra la ruta estable. DSP desktop aún importa `kai::render::Color` (sigue compilando mientras `render` sea `pub` oculto); migrar a `kai::Color`.

- **Clip redondeado de contenedores (P14b):** `rounded-* overflow-hidden` recorta hijos e imágenes a la curva (máscara SDF heredada). `ClipRect`/`RoundClip` en layout, hit-test y `resolve`; quads, imágenes y geometría descartan fuera de la máscara en GPU. El texto sigue el AABB del clip.

- **Gráficos nativos:** `chart(ChartKind::Bar)` y `Sparkline`, pintados con `Quad` y `Mesh` como el mapa. Las barras usan tokens del tema (`primary`, `success`, `warning`, `error`; la seleccionada, `choice`); un clic envía `on_select(índice)`. Sparkline: polilínea con relleno al 20 %. Tamaño por clases (`h-48`, `flex-1`). Sección **Gráficos** en la demo.

- **Librería `kai`:** la demo y la galería como ejemplos sobre la API pública; `fixtures/external-app` compilada en CI.
- **Modelo y tema AppKit** (Pasos 0–15 y 17 de la guía 12): `ThemeKind` Kai/AppKit, claro y oscuro, generaciones Aqua11, Glass26 y Glass27, `ControlProps`, cadena de responders, variantes `appkit:`, `dark:`, `window-inactive:`, `selected:`, mezcla en gamma y galería de controles.
- **Arquitectura y pruebas:** `App` v2 con `init` y `update -> Effects`; `Session` sin ventana; `kai::testing` con consultas por rol, nombre y `test_id`, y `TestApp` (clic, texto, teclas, log de `Msg`, capturas por elemento); sub-ids por convención en los componentes.
- **Devtools** (solo debug): `KAI_DEVTOOLS=msgs,frames` e inspector con ⌘⌥I.
- **Comandos:** registro tipado de acciones y consultas con disponibilidad; atajos, menú nativo (muda), paleta ⇧⌘P y `on_command`; entrada JSON y esquemas (`mcp-schema.json`, `--describe-commands`).
- **MCP experimental** (feature `mcp`): servidor stdio con `rmcp`, confirmación humana de acciones destructivas, auditoría e indicador de agente.
- **Widgets:** checkbox con estado mixto, segmented, `Select`, sidebar plegable a un riel con tooltips, toolbar, search field, textarea, `Dialog` modal, toasts, tabla y lista virtualizadas (10 000 filas), edición por palabra y línea, `Avatar`, `MediaSlot` y `ColorField`.
- **Efectos async:** `Effects::perform` en un pool de 4 hilos, `after`, `keyed`/`cancel`, `pick_file` y `save_file` (paneles de abrir y guardar de macOS) y archivos arrastrados (`on_drop_files`, variante `drag:`).
- **Tabla completa:** columnas con valor tipado (`Column::text/number/date/boolean`, `format`, `render` con componente, `actions`) y `TableView` pura con búsqueda global (también en columnas ocultas), filtros por columna, orden estable, columnas ocultas y filas expandidas. Modo filtro con un campo por columna bajo el encabezado. Menú de columnas con checks que queda abierto, por clic derecho, Ctrl+clic o `ShowContextMenu` en el encabezado, y `column_menu` para un botón. Exportar a Excel lo que se ve (`TableView::export`, `kai::xlsx` con `rust_xlsxwriter`), sin columnas ocultas ni acciones; comando `tabla.exportar` en la demo. Filas expandibles con panel de alto declarado (`expandable`) y `virtual_rows` de alto variable. Scroll horizontal con ancho mínimo por columna (`min_width`) y columnas fijas al inicio o al final (`Column::pin`, `Pin`); las acciones quedan fijas al final por defecto.
- **Motor:** overflow por eje (`overflow-x-*`, `overflow-y-*`) con offsets, recorte, hit-test, `reveal`, barra y AccessKit (`scroll_x`) a lo ancho; Mayús+rueda; `sticky` horizontal (`left-*`/`right-*`) que se pinta y recibe clics encima y que `reveal` respeta; menús contextuales (`Element::context_menu`, `Session::secondary_click`, `TestApp::right_click`) con opciones marcables (`check_options`, `MenuItemCheckBox`); `Role::Region`, `expanded` en filas y `Element::own_tab_stops`; icono `ChevronRight`.
- **`DotProgress` y botón ocupado:** puntos en ola (`DotProgress`, `dot_progress`, rol `ProgressIndicator`) con la primera animación calculada al pintar (`Motion`, respeta `reduce_motion`). El botón o comando que lanzó un `Effects::perform` queda ocupado hasta que la tarea, y las que encadene, terminan: puntos sobre el texto, mismo color y ancho, cursor de espera y sin clics. Un comando ocupado queda «En curso». Incluye `Button::loading`, `busy_feedback`, las variantes `busy:` y `group-busy:`, `bg-current`, `cursor-wait` y `TestApp::is_busy`.
- **Accesibilidad y usabilidad:** las preferencias de macOS (reducir movimiento, aumentar contraste, reducir transparencia, acceso completo por teclado) se leen al arrancar y al activar la ventana (`Session::set_system_prefs`), con las variantes `motion-reduce:`, `contrast-more:` y `transparency-reduce:` y bordes sólidos en los controles con contraste aumentado. La fila elegida de la paleta es el `active_descendant` de la búsqueda. Los campos de texto deshacen y rehacen por palabra (⌘Z, ⇧⌘Z). La lista del `Select` busca por tipeo. Componente `Tabs` (`NSTabView`): roles `TabList`, `Tab` y `TabPanel`, y grupo de foco `FocusGroup::Tabs` con ← y →.
- **Imágenes:** `image(src)` desde archivo, URL (feature `net`), bytes o RGBA; atlas RGBA en GPU con radio propio, `object-*` y `aspect-*`.
- **Geometría y mapas:** mallas con `lyon` y MSAA 4×; `kai::maps` con mapa base vectorial propio (PMTiles, MVT, estilo y etiquetas; extracto de Parral), cámara con inercia, marcadores, polígonos GeoJSON, `fit_bounds`, editor de polígonos y comandos `mapa.*` y `areas.*` en la demo.
- **Lenguaje de color de la selección:** token `choice` (el `secondary` de Kai) y las clases `CHOICE_ACTIVE` y `CHOICE_HOVER` para toda elección activa; el select termina en `ChevronDown`.
- **Iconos:** 24 iconos Lucide nuevos para DSP desktop.
- **`kai dev`** (crate aparte `../kai-cli`): recompila al guardar y reinicia conservando la ventana y `App::dev_state`.

### Changed

- **Select del mismo ancho que un TextField:** `w-full` y mínimo 180 pt (antes `inline-flex` con 160 pt en Kai y 96 en AppKit). En la demo ocupa una celda de la misma grilla que los campos.
- **Search field con variantes y tamaños:** `SearchVariant::{Toolbar, Field, Inline}` (cápsula de toolbar, campo de formulario y celda densa sin ancho mínimo; las tres son cápsula `rounded-full`), y `control_size` achica también el campo en Kai (caja, texto, lupa y botón de borrar; Small mide 28 pt). Los filtros de la tabla usan `Inline` y `Small`: la fila baja de 40 a 36 pt en Kai, y en AppKit crece con el campo cuando el estilo mezclado le da la altura de Kai.
- **`Table` cambia de API:** `Table::new(datos, columnas, &vista)` con `on_view(ViewChange)` reemplaza a `Table::new(columnas, total, cell)`, `sort`/`on_sort` y `leading`; `selected` y `on_select` usan el índice del registro. `Column::new(..).numeric()` pasa a `Column::number(..)`.
- `Layout::reveal` mueve lo mínimo en los dos ejes: una fila más ancha que la vista conserva el scroll horizontal.
- Los avisos de error ya no se cierran solos, y un aviso no se cierra mientras tiene el puntero o el foco encima (`toast_timer(&toast, close)`).
- Las filas de la paleta de comandos son `Row` dentro de una `List` «Comandos».
- La opción elegida del select ya no lleva check; se marca con el vidrio de elección.
- `MediaSlot` ya no tiene botón de agregar ni de cámara: la imagen es el control (`ActionPlacement` eliminado).
- La demo pasó a ser un shell con toolbar, sidebar y diez secciones; la baseline de capturas tiene 22 imágenes.
- `TestApp` arranca su reloj en el inicio de la sesión, así las animaciones dan los mismos frames en cada corrida.

### Fixed

- **Paneles de archivos al cerrar:** `Effects::pick_file` / `save_file` abren `NSOpenPanel` / `NSSavePanel` como hoja (`beginSheetModalForWindow` + `block2`), no con `runModal` dentro del manejador de winit. La ruta vuelve como `UserEvent::FilePicked`. Cerrar la ventana o reiniciar con `kai dev` con el panel abierto ya no aborta.

### Known issues

- La barra vertical del cuerpo de una tabla más ancha que su caja queda en el borde derecho de la grilla y se desplaza con el scroll horizontal.

## [0.1.0] - 2026-10-18

### Added

- Prototipo macOS: layout, tema, TextField base, iconos Lucide, accesibilidad inicial y CI.
