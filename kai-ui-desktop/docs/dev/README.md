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

## Orden recomendado

La fase 1 (prototipo vertical) ya validó el riesgo de plataforma. La implementación continúa ahora en incrementos, siguiendo el roadmap L0–L6 y pasando el gate de cada fase antes de iniciar la siguiente:

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
- **`App` v2 (Paso 1 de 13):** `App` con `init` y `update -> Effects` (vacío hasta los efectos async); `SimpleApp` mantiene el contrato anterior y la demo y la galería lo usan.
- **Identificación y consultas (Pasos 2 y 3 de 13):** `.test_id(..)` en `Element` y en los componentes, expuesto como `author_id` de AccessKit (`accessibilityIdentifier` en macOS) sin cambiar el `WidgetId`; en debug se reportan `key` y `test_id` duplicados con las rutas de ambos. `kai::testing` encuentra elementos sin ventana ni GPU con `Screen` y filtros `By` (rol, nombre accesible, label, texto, `test_id`, `within`), con el mismo rol y nombre que lee VoiceOver.
- **`Session` y `TestApp` (Paso 4 de 13):** la parte sin ventana del runtime vive en `Session` (`session.rs`); la ventana aplica lo que la sesión le pide mediante un `Outbox`. `kai::testing::TestApp` maneja una app completa sobre esa misma sesión (clic, escribir, teclas, log de `Msg`, capturas por elemento), sin ventana ni GPU salvo para capturas ([03](03-eventos-y-estado.md)).
- **Sub-ids y tests sin búsquedas frágiles (Pasos 5 y 6 de 13):** con `.test_id("x")`, los componentes nombran solos sus partes (`x.input`, `x.title`, `x.track`…), y `.part(..)`/`.parts_root()` hacen lo mismo en componentes propios ([05](05-widgets-y-tema.md)). El switch se publica como `Role::Switch` con el nombre de su label. Los tests localizan por `test_id` o por consultas, no por posición ni por clases. Con esto se cierra la Etapa 1 de [13](13-hoja-de-ruta-plataforma.md).
- **Accesibilidad:** AccessKit integrado, con el árbol derivado del layout y roles semánticos por elemento ([08](08-accesibilidad.md)).
- **Calidad:** 243 tests (236 normales y 7 `#[ignore]` de benchmark y capturas) más 2 doctests, Clippy `-D warnings`, `cargo doc -D warnings`, job de CI en macOS (más un job de MSRV 1.89). Las capturas de la demo se comparan con `scripts/snapshot-baseline.sh --check` ([06](06-calidad-y-distribucion.md)). MCP/IA sigue siendo un adaptador opcional de aplicación con validación y autorización previas.

## Mapa de módulos

```text
kai-ui-desktop/
├── src/
│   ├── lib.rs             crate `kai`: API pública e internos #[doc(hidden)]
│   ├── main.rs            lanzador de `cargo run` (llama a demo::run_from_env)
│   ├── demo.rs            demo (SimpleApp: Model/Msg/update/view), sus tests y capturas
│   ├── gallery.rs         galería de controles por tamaño, tema, apariencia y ventana
│   ├── runtime.rs         App v2 y SimpleApp; ventana: winit → Session, Outbox → ventana, render
│   ├── session.rs         runtime sin ventana: árbol, layout, foco, inputs, eventos → mensajes
│   ├── element.rs         árbol tipo DOM Element<Msg>, ids, test_id, validate_ids, FocusOrder
│   ├── components.rs      Button, TextField, Card… con clases Kai web y AppKit
│   ├── responder.rs       acciones, cadena de responders, equivalentes de teclado
│   ├── tw.rs              parser de clases tipo Tailwind → Style
│   ├── style.rs           Style tipo CSS → taffy::Style, texto heredable
│   ├── theme.rs, theme/   tema Kai (tokens.css) y AppKit (appkit.rs, generation.rs)
│   ├── layout.rs          Layout: Taffy, scroll, clip, z-index, hit-test
│   ├── render/            DrawList, paint, texto, atlas de iconos, backend wgpu (quad.wgsl)
│   ├── snapshot.rs        render offscreen, diff y PNG para capturas
│   ├── background.rs      fondo animado (background.wgsl)
│   ├── icons/             catálogo Lucide generado (scripts/gen-icons.mjs, assets/icons/)
│   ├── text_field.rs      edición, IME y portapapeles de los inputs
│   ├── ui_engine.rs       intents y dispatch de foco/activación/texto
│   ├── platform_adapter.rs  winit → intents Kai
│   ├── a11y.rs            árbol AccessKit desde el layout; rol y nombre accesible compartidos
│   ├── testing/           layout_of, consultas Screen/By (query.rs) y TestApp (app.rs)
│   └── geometry.rs, signal.rs, grid_layout.rs, fonts.rs, version.rs
├── examples/              demo.rs y gallery.rs, binarios finos sobre `kai`
├── fixtures/external-app/ app mínima que usa solo la API pública (compilada en CI)
└── scripts/               snapshot-baseline.sh y generadores
```

Las librerías mencionadas son **candidatas a evaluar**, no decisiones irreversibles. Antes de adoptarlas, verificar compatibilidad con la versión de Rust elegida, macOS, licencias, mantenimiento e integración entre sí.

La guía de [craft-libs](07-integracion-craft-libs.md) distingue los crates de utilidad potencial de `craft-ui`, cuyo frontend está acoplado a egui y no reemplaza el renderer propio de Kai.

## Fuera del alcance inicial

- Widgets nativos de AppKit o SwiftUI para el dibujo de controles.
- Android, iOS, Windows y Linux.
- Un bus global de señales, componentes con estado local oculto, efectos asíncronos (`Cmd`/`Sub`), caché incremental sin mediciones y comandos serializables con undo/redo.
- Integraciones de aplicación como PDF, impresión, CMS, ajustes persistentes o agentes de IA dentro del renderer.
