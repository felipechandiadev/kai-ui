# Guía de desarrollo de Kai UI Desktop

Esta documentación describe la implementación gradual de un framework de interfaz propio en Rust. El alcance inicial es **macOS de escritorio**: Kai dibuja sus controles; no se delega el dibujo de botones, campos, diálogos ni otros widgets al sistema operativo. Android y otros sistemas quedan fuera del primer hito.

## Ruta de lectura

1. [Visión y decisiones arquitectónicas](00-vision-y-arquitectura.md)
2. [Prototipo vertical para macOS](01-prototipo-macos.md)
3. [Geometría y layout](02-geometria-y-layout.md)
4. [Eventos y modelo de estado](03-eventos-y-estado.md)
5. [Renderer y texto](04-renderer-y-texto.md)
6. [Widgets y tema Kai](05-widgets-y-tema.md)
7. [Calidad y distribución](06-calidad-y-distribucion.md)
8. [Evaluación de craft-libs](07-integracion-craft-libs.md)
9. [Accesibilidad (AccessKit)](08-accesibilidad.md)
10. [Framework, registro de comandos y MCP](09-framework-y-plataforma.md)
11. [Mapas interactivos](10-mapas.md)
12. [Tema AppKit para macOS](11-tema-appkit.md)
13. [Guía: modelo AppKit como estándar de Kai](12-modelo-appkit.md)
14. [Hoja de ruta: de Kai UI a plataforma de desarrollo de apps](13-hoja-de-ruta-plataforma.md)
15. [MCP experimental: cómo probarlo](14-mcp.md)

## Dónde estamos

Las capas L0–L6 y los Pasos 0 a 15 del modelo AppKit ([12](12-modelo-appkit.md)) están hechos. La hoja de ruta vigente es [13](13-hoja-de-ruta-plataforma.md): sus Etapas 0 a 3 (librería, pruebas, comandos y agentes) están cerradas y la Etapa 4 (la app guía, DSP desktop) está en curso. La tabla de una página con el estado y el costo de lo que falta está en [13 §1.1](13-hoja-de-ruta-plataforma.md#11-estado-al-20-de-octubre-de-2026).

## Orden recomendado (capas L0–L6, completado)

La fase 1 (prototipo vertical) validó el riesgo de plataforma. Después, la implementación siguió el roadmap L0–L6, pasando el gate de cada fase antes de iniciar la siguiente:

1. **L0 — Capa base geométrica y unidades:** tipos y conversiones compartidos por layout, hit-testing y renderer.
2. **L1 — Modelo de estado reactivo y señales:** estado tipado con señales locales explícitas; no se introduce un bus global. Completada para el alcance actual.
3. **L2–L3 — Catálogo unificado de comandos:** traducir entrada de plataforma a intenciones tipadas de Kai.
4. **L4 — Motor de maquetación incremental y caché:** primero layout determinista completo; incrementalidad/caché solo con paridad y mediciones.
5. **L5 — Capa de renderizado desacoplada:** lista de operaciones de dibujo independiente del backend `wgpu`.
6. **L6 — Frontend y árbol de widgets reactivos:** migrar primero Button y TextField sobre las capas anteriores.

El orden detallado, los contratos y los criterios de aceptación están en los documentos 02–06. Mantener inicialmente un solo crate y extraer módulos internos; crear crates o estabilizar una API pública únicamente cuando las fronteras estén probadas. Cada fase debe conservar el comportamiento ya validado y pasar `cargo fmt`, `cargo check`, `cargo test` y `cargo clippy` antes de avanzar.

## Estado de implementación por capas

- **L0 — Completada:** tipos geométricos y conversión lógica/física extraídos a `kai-ui-desktop/src/geometry.rs`; 7 pruebas unitarias pasan y el usuario confirmó que la revisión visual está correcta.
- **L1 — Completada para el alcance actual:** `Signal<T>` local está implementada y usada por pressed/foco; 4 pruebas de señal pasan. El usuario confirmó pressed, hover y foco correctos.
- **L2–L3 — Gate básico aprobado:** intents/dispatcher puros y adaptador de plataforma implementados; tests automáticos y regresión manual básica reportados correctos. IME/portapapeles siguen siendo verificaciones específicas independientes.
- **L4 — Completada para el alcance actual:** modelo de cajas tipo CSS sobre Taffy (flex, grid, block, `position: absolute`). La raíz es siempre la ventana; scroll y clip por nodo con `overflow` (clip al padding box); z-index en pintura y hit-test. Sin caché: vista + layout + paint cuestan ≈ 130 µs (p95) en Apple M4 ([02](02-geometria-y-layout.md)).
- **L5 — Completada:** `DrawList` independiente del backend, `paint` puro en orden de pintor, quads SDF instanciados con radio, borde, blur y clip por instancia, lotes alternos de cajas y texto, render offscreen para capturas, e iconos Lucide en un atlas de máscaras ([04](04-renderer-y-texto.md)).
- **L6 — Completada para el alcance actual:** arquitectura Elm (`App` con `update`/`view`, runtime en `runtime.rs` y `session.rs`), árbol tipo DOM `Element<Msg>`, estilos con clases tipo Tailwind resueltas contra el tema Kai, y componentes Button, TextField (alcance base de la web: variantes, densidades, adornos e iconos, password, required, helper, read-only), Card y Stat con las clases de Kai web ([03](03-eventos-y-estado.md), [05](05-widgets-y-tema.md)).
- **Modelo y tema AppKit (Pasos 0–15 y 17 de [12](12-modelo-appkit.md); el tema se elige por versión de macOS, todavía sin leer colores de AppKit en runtime):** `ControlProps` (tamaño heredable, estado, botón por defecto, equivalentes de teclado), estado de ventana *key*/*main*, cadena de responders (`responder.rs`), variantes `appkit:`/`dark:`/`window-inactive:`/`control-*`, `Theme::appkit_static` y `Theme::appkit_current`, mezcla en gamma. La demo elige tema con `KAI_THEME`; la galería (`cargo run --example gallery`) muestra todos los controles en los 5 tamaños, con los dos temas, claro/oscuro y ventana activa/inactiva ([11](11-tema-appkit.md)).
- **Librería y ejemplos (Paso 0 de [13](13-hoja-de-ruta-plataforma.md)):** crate de librería `kai` con la demo en `src/demo.rs`; `cargo run` y `cargo run --example demo` abren la demo; `fixtures/external-app` usa solo la API pública y el CI la compila junto con `cargo doc -D warnings`.
- **`App` v2 (Paso 1 de 13):** `App` con `init` y `update -> Effects` (los efectos llegaron con el Paso 15); `SimpleApp` mantiene el contrato anterior y la demo y la galería lo usan.
- **Identificación y consultas (Pasos 2 y 3 de 13):** `.test_id(..)` en `Element` y en los componentes, expuesto como `author_id` de AccessKit (`accessibilityIdentifier` en macOS) sin cambiar el `WidgetId`; en debug se reportan `key` y `test_id` duplicados con las rutas de ambos. `kai::testing` encuentra elementos sin ventana ni GPU con `Screen` y filtros `By` (rol, nombre accesible, label, texto, `test_id`, `within`), con el mismo rol y nombre que lee VoiceOver.
- **`Session` y `TestApp` (Paso 4 de 13):** la parte sin ventana del runtime vive en `Session` (`session.rs`); la ventana aplica lo que la sesión le pide mediante un `Outbox`. `kai::testing::TestApp` maneja una app completa sobre esa misma sesión (clic, escribir, teclas, log de `Msg`, capturas por elemento), sin ventana ni GPU salvo para capturas ([03](03-eventos-y-estado.md)).
- **Sub-ids y tests sin búsquedas frágiles (Pasos 5 y 6 de 13):** con `.test_id("x")`, los componentes nombran solos sus partes (`x.input`, `x.title`, `x.track`…), y `.part(..)`/`.parts_root()` hacen lo mismo en componentes propios ([05](05-widgets-y-tema.md)). El switch se publica como `Role::Switch` con el nombre de su label. Los tests localizan por `test_id` o por consultas, no por posición ni por clases. Con esto se cierra la Etapa 1 de [13](13-hoja-de-ruta-plataforma.md).
- **Comandos (Pasos 7 y 8 de 13):** registro tipado de acciones y consultas (`kai::commands`) con disponibilidad tras cada `update` y validación en debug. Lo consumen los atajos, `on_command` en los botones, el menú nativo de macOS (`kai::menu`, con muda) y la paleta ⇧⌘P; una acción deshabilitada se ve atenuada en los tres ([03](03-eventos-y-estado.md), [05](05-widgets-y-tema.md)).
- **Comandos en pruebas y demo migrada (Pasos 9 y 10 de 13):** `TestApp::invoke`, `query`, `available` y `perform`; cada acción de la demo hace lo mismo desde botón, atajo, menú, paleta y código. Con esto se cierra la Etapa 2.
- **Esquemas (Paso 11 de 13):** cada comando es su propia entrada, deserializable con serde y descrita en JSON Schema 2020-12 con schemars. `Session::call` y `TestApp::call` ejecutan un comando desde JSON, y el registro se publica en `mcp-schema.json`, comparado en un test, y con `cargo run -- --describe-commands`.
- **MCP experimental (Paso 12 de 13):** con `--features mcp`, la demo lanzada con `--mcp` es un servidor MCP por stdio (`rmcp`). Las herramientas son los comandos marcados con `.expose()`, y cada llamada corre en el hilo de UI como `Session::call`. Una acción puede responder con una consulta (`.returns::<Q>()`). `TestApp::mcp_client()` lo prueba en proceso. Falta la prueba a mano con Cursor o Claude Desktop ([14](14-mcp.md)).
- **Confirmación y auditoría (Paso 13 de 13):** un agente no corre una acción `Destructive` sin una persona. La ventana muestra un diálogo modal (`components::dialog`, `Role::Dialog`) con «Permitir una vez», «Permitir en esta sesión» y «Rechazar». Sin respuesta en 30 s, o si el cliente cancela, la llamada no corre. Cada llamada por JSON queda en una `AuditLog` en memoria, con su origen y la entrada redactada (los campos `writeOnly` no se guardan). Un indicador muestra el agente conectado y su última acción. Con esto se cierra la Etapa 3.
- **Devtools (Paso 1b de 13, solo en debug, por stderr):**
  - `KAI_DEVTOOLS=msgs` imprime cada `Msg` con su origen (puntero, teclado, accesibilidad, menú, script o agente) y el tiempo de `update`.
  - `KAI_DEVTOOLS=frames` imprime p50 y p95 de view, layout, paint y render cada 120 frames; `all` enciende los dos.
  - ⌘⌥I abre el inspector: contorno y datos (rol, id, nombre, `test_id`, bounds, clases) del elemento bajo el puntero, y el árbol completo en la consola.
  - En tests: `TestApp::message_sources()`, `hover()` y `dump()`.
- **Widgets de selección (antes del Paso 14 de 13):** checkbox con estado mixto, segmented control (una sola parada de Tab, con flechas entre segmentos) y `Select`, cuya lista abre y maneja la `Session`. Están en la galería (sección Selection) y en la demo, con roles de AccessKit propios ([12](12-modelo-appkit.md), Paso 20).
- **Sidebar de secciones:** `components::sidebar`, una sola parada de Tab donde ↑/↓ eligen la fila, publicada como `AXOutline` ([12](12-modelo-appkit.md), Paso 20.7). La fila elegida es vidrio de elección (cyan `secondary`, ver «Lenguaje de color de la selección» en [13](13-hoja-de-ruta-plataforma.md)) y las demás responden al hover. Se pliega a un riel de iconos de 48 pt con tooltips (`Sidebar::collapsed`, `on_toggle`).
- **Backlog de widgets del Paso 14 de 13:** search field, textarea, toolbar dibujada por Kai, `Dialog` modal, avisos con temporizadores (`Effects::after`), tabla y lista virtualizadas, y edición por palabra y línea con doble y triple clic ([12](12-modelo-appkit.md), 20.5 y 20.8). La demo tiene una toolbar en lugar del header y diez secciones (Inicio, Campos, Botones, Selección, Contenedores, Badges y alertas, Multimedia, Tabla, Diálogos y avisos, y Mapa) que muestran todos los widgets, con un test de cobertura. Los comandos `ui.ir_a`, `ui.alternar_sidebar`, `avisos.mostrar`, `multimedia.cargar_url`, `mapa.*` y `areas.*` la manejan desde scripts y agentes.
- **Efectos async (Paso 15 de 13, parcial):** `Effects::perform` corre trabajo bloqueante en un pool de 4 hilos y devuelve un `Msg`; `.keyed(key)` y `Effects::cancel` reemplazan o descartan; `Effects::pick_file` abre el panel de archivos de macOS y `on_drop_files` recibe archivos arrastrados desde Finder. En tests: `run_until_idle`, `advance`, `on_file_pick` y `drop_files`. Faltan futures y suscripciones. **Fallo conocido:** la app aborta si se termina con el panel de archivos abierto ([13](13-hoja-de-ruta-plataforma.md), Paso 15).
- **Imágenes (Paso 14b de 13):** `image(src)` con `ImageSource` (archivo, bytes o URL con la feature `net`), decodificadas en el pool y dibujadas desde un atlas RGBA con radio propio; `object-*` y `aspect-*`. Encima, `Avatar` y `MediaSlot`: la imagen es el control (clic o arrastrar un archivo), y bajo el puntero toma el vidrio de elección ([04](04-renderer-y-texto.md)).
- **Mapas (Pasos 15b–15d y M3 de 13, parciales):** `map` con mapa base vectorial propio (lector PMTiles, MVT y etiquetas, extracto de Parral en `assets/maps/`), cámara con inercia, `marker`, polígonos desde GeoJSON con selección y `fit_bounds`, y `polygon_editor` para editar un polígono existente. Faltan los modos Punto y Línea y dibujar desde cero ([10](10-mapas.md)).
- **Campo de color:** `components::color_field` (builder `ColorField`): hex con muestra y paleta de sugeridos como `RadioGroup` ([13](13-hoja-de-ruta-plataforma.md), Paso 14).
- **Lenguaje de color de la selección:** todo lo que es una elección activa (fila de sidebar, opción del select, segmento, ítem de toolbar presionado, fila de tabla, muestra de color) usa el `secondary` de Kai (cyan) como vidrio teñido: token `choice` y las clases `CHOICE_ACTIVE` y `CHOICE_HOVER`. El azul (acento) queda para acción, encendido y foco ([13](13-hoja-de-ruta-plataforma.md), Paso 14).
- **`kai dev` (Paso 18 de 13, empezado):** el crate `kai-cli` (al lado de este) instala el binario `kai`. `kai dev` recompila al guardar y reinicia la app conservando la ventana y lo que devuelva `App::dev_state`; si la compilación falla, la versión anterior sigue abierta. Es lo único de la CLI que existe por ahora.
- **Accesibilidad:** AccessKit integrado, con el árbol derivado del layout y roles semánticos por elemento ([08](08-accesibilidad.md)).
- **Calidad:** 500 tests normales y 14 `#[ignore]` de benchmarks y capturas (con `--features mcp` son 504 normales), más 4 doctests (3 ignorados) y 8 tests de `kai-cli`. Clippy `-D warnings` (sin features, con `snapshots` y con `mcp`), `cargo doc -D warnings`, jobs de CI en macOS para la librería y `kai-cli` (más un job de MSRV 1.89). Las capturas de la demo se comparan con `scripts/snapshot-baseline.sh --check` ([06](06-calidad-y-distribucion.md)). MCP/IA sigue siendo un adaptador opcional de aplicación con validación y autorización previas.

## Mapa de módulos

```text
kai-ui-desktop/
├── src/
│   ├── lib.rs             crate `kai`: API pública e internos #[doc(hidden)]
│   ├── main.rs            lanzador de `cargo run` (llama a demo::run_from_env)
│   ├── demo.rs            demo (SimpleApp: Model/Msg/update/view), sus tests y capturas
│   ├── devtools.rs        KAI_DEVTOOLS (log de Msg con origen, tiempos por frame), inspector
│   │                      ⌘⌥I y volcado del árbol; solo en debug, por stderr
│   ├── gallery.rs         galería de controles por tamaño, tema, apariencia y ventana
│   ├── runtime.rs         App v2 y SimpleApp; ventana: winit → Session, Outbox → ventana, render
│   ├── session.rs         runtime sin ventana: árbol, layout, foco, inputs, eventos → mensajes
│   ├── session/calls.rs   llamadas por JSON: scripts y agentes, confirmación, cola, indicador
│   ├── element.rs         árbol tipo DOM Element<Msg>, ids, test_id, validate_ids, FocusOrder
│   ├── components.rs      Button, TextField, Card, Select, Switch, paleta… con clases Kai web y
│   │                      AppKit; constantes del vidrio de elección (CHOICE_ACTIVE, CHOICE_HOVER)
│   ├── components/        color (ColorField), dialog, media (Avatar, MediaSlot), progress
│   │                      (DotProgress), search, sidebar, table (Table, virtual_list),
│   │                      text_area, toast, toolbar
│   ├── effects.rs         pool de tareas de Effects::perform y bandeja de resultados
│   ├── media.rs           imágenes: ImageSource, carga (archivo, bytes, URL con `net`), ImageStore
│   ├── maps/              map, cámara, capas de polígonos, editor de polígonos, PMTiles, MVT,
│   │                      estilo del mapa base y etiquetas
│   ├── geo.rs             coordenadas geográficas y proyección Web Mercator
│   ├── platform/          piezas nativas fuera de winit: panel de archivos (NSOpenPanel)
│   ├── dev_state.rs       lo que `kai dev` conserva entre reinicios (solo en debug)
│   ├── virtual_rows.rs    filas visibles de las listas virtualizadas
│   ├── responder.rs       acciones, cadena de responders, equivalentes y atajos de comandos
│   ├── commands/          registro de comandos: Action, Query, disponibilidad, validación,
│   │                      entrada JSON, esquemas (schema.rs: describe, mcp-schema.json) y
│   │                      auditoría (audit.rs: AuditLog, redact)
│   ├── mcp/               servidor MCP por stdio con rmcp (feature `mcp`): Tools, Invocation,
│   │                      Event, serve; plazo de 30 s y cancelación
│   ├── menu.rs            barra de menús desde el registro (menu_model; muda en macOS)
│   ├── tw.rs              parser de clases tipo Tailwind → Style
│   ├── style.rs           Style tipo CSS → taffy::Style, texto heredable
│   ├── theme.rs, theme/   tema Kai (tokens.css) y AppKit (appkit.rs, generation.rs)
│   ├── layout.rs          Layout: Taffy, scroll, clip, z-index, hit-test
│   ├── render/            DrawList, paint, texto, atlas de iconos y de imágenes, geometría
│   │                      (lyon), backend wgpu (quad.wgsl, image.wgsl, geometry.wgsl)
│   ├── snapshot.rs        render offscreen, diff y PNG para capturas
│   ├── background.rs      fondo animado (background.wgsl)
│   ├── icons/             catálogo Lucide generado (scripts/gen-icons.mjs, assets/icons/)
│   ├── text_field.rs      edición, IME y portapapeles de los inputs
│   ├── ui_engine.rs       intents y dispatch de foco/activación/texto
│   ├── platform_adapter.rs  winit → intents Kai
│   ├── a11y.rs            árbol AccessKit desde el layout; rol y nombre accesible compartidos
│   ├── testing/           layout_of, consultas Screen/By (query.rs), TestApp (app.rs) y
│   │                      McpClient (mcp.rs, feature `mcp`)
│   └── geometry.rs, signal.rs, grid_layout.rs, fonts.rs, version.rs
├── assets/                SVG de iconos Lucide y mapas (parral.pmtiles, dsp-areas.geojson)
├── examples/              demo.rs y gallery.rs, binarios finos sobre `kai`
├── fixtures/external-app/ app mínima que usa solo la API pública (compilada en CI)
├── mcp-schema.json        comandos de la demo con sus esquemas (snapshot de `describe`)
├── snapshots/baseline/    capturas de referencia y SHA256SUMS, locales (en .gitignore: dependen de la GPU)
└── scripts/               snapshot-baseline.sh y generadores

kai-cli/                   crate aparte con el binario `kai` (por ahora solo `kai dev`):
└── src/                   main.rs (argumentos), dev.rs (ciclo compilar y reiniciar),
                           cargo.rs (metadata y build), watch.rs (observador por fechas)
```

Las librerías mencionadas son **candidatas a evaluar**, no decisiones irreversibles. Antes de adoptarlas, verificar compatibilidad con la versión de Rust elegida, macOS, licencias, mantenimiento e integración entre sí.

La guía de [craft-libs](07-integracion-craft-libs.md) distingue los crates de utilidad potencial de `craft-ui`, cuyo frontend está acoplado a egui y no reemplaza el renderer propio de Kai.

## Fuera del alcance inicial

- Widgets nativos de AppKit o SwiftUI para el dibujo de controles.
- Android, iOS, Windows y Linux.
- Un bus global de señales, componentes con estado local oculto, caché incremental sin mediciones y undo/redo de la app (deshacer comandos llega en el Paso 25 de [13](13-hoja-de-ruta-plataforma.md); los campos de texto ya deshacen su edición con ⌘Z). Los efectos async ya no están fuera: existen desde el Paso 15.
- Integraciones de aplicación como PDF, impresión, CMS, ajustes persistentes o agentes de IA dentro del renderer.
