# Changelog — kai-ui-desktop-prototype

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).

## [Unreleased]

Avance hacia la plataforma descrita en [`docs/dev/13-hoja-de-ruta-plataforma.md`](docs/dev/13-hoja-de-ruta-plataforma.md) (Etapas 0 a 3 cerradas, Etapa 4 en curso).

### Added

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

### Known issues

- La app aborta si se termina con un panel de archivos (abrir o guardar) abierto (`runModal` dentro del manejador de eventos de winit).
- La barra vertical del cuerpo de una tabla más ancha que su caja queda en el borde derecho de la grilla y se desplaza con el scroll horizontal.

## [0.1.0] - 2026-10-18

### Added

- Prototipo macOS: layout, tema, TextField base, iconos Lucide, accesibilidad inicial y CI.
