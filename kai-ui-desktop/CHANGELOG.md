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
- **Efectos async:** `Effects::perform` en un pool de 4 hilos, `after`, `keyed`/`cancel`, `pick_file` (panel de archivos de macOS) y archivos arrastrados (`on_drop_files`, variante `drag:`).
- **Imágenes:** `image(src)` desde archivo, URL (feature `net`), bytes o RGBA; atlas RGBA en GPU con radio propio, `object-*` y `aspect-*`.
- **Geometría y mapas:** mallas con `lyon` y MSAA 4×; `kai::maps` con mapa base vectorial propio (PMTiles, MVT, estilo y etiquetas; extracto de Parral), cámara con inercia, marcadores, polígonos GeoJSON, `fit_bounds`, editor de polígonos y comandos `mapa.*` y `areas.*` en la demo.
- **Lenguaje de color de la selección:** token `choice` (el `secondary` de Kai) y las clases `CHOICE_ACTIVE` y `CHOICE_HOVER` para toda elección activa; el select termina en `ChevronDown`.
- **Iconos:** 24 iconos Lucide nuevos para DSP desktop.
- **`kai dev`** (crate aparte `../kai-cli`): recompila al guardar y reinicia conservando la ventana y `App::dev_state`.

### Changed

- La opción elegida del select ya no lleva check; se marca con el vidrio de elección.
- `MediaSlot` ya no tiene botón de agregar ni de cámara: la imagen es el control (`ActionPlacement` eliminado).
- La demo pasó a ser un shell con toolbar, sidebar y diez secciones; la baseline de capturas tiene 21 imágenes.

### Known issues

- La app aborta si se termina con el panel de archivos abierto (`runModal` dentro del manejador de eventos de winit).

## [0.1.0] - 2026-10-18

### Added

- Prototipo macOS: layout, tema, TextField base, iconos Lucide, accesibilidad inicial y CI.
