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
- **L6 — Completada para el alcance actual:** arquitectura Elm (`App` con `update`/`view`, runtime en `runtime.rs`), árbol tipo DOM `Element<Msg>`, estilos con clases tipo Tailwind resueltas contra el tema Kai, y componentes Button, TextField (alcance base de la web: variantes, densidades, adornos e iconos, password, required, helper, read-only), Card y Stat con las clases de Kai web ([03](03-eventos-y-estado.md), [05](05-widgets-y-tema.md)).
- **Accesibilidad:** AccessKit integrado, con el árbol derivado del layout y roles semánticos por elemento ([08](08-accesibilidad.md)).
- **Calidad:** 95 tests, Clippy `-D warnings`, job de CI en macOS (más un job de MSRV 1.89), benchmark y capturas offscreen como tests `#[ignore]` ([06](06-calidad-y-distribucion.md)). MCP/IA sigue siendo un adaptador opcional de aplicación con validación y autorización previas.

## Mapa de módulos

```text
kai-ui-desktop/src/
├── main.rs            demo (App: Model/Msg/update/view)
├── runtime.rs         runtime Elm: ventana, eventos → mensajes, frame
├── element.rs         árbol tipo DOM Element<Msg>, ids, FocusOrder
├── components.rs      Button, TextField, Card… con clases de Kai web
├── tw.rs              parser de clases tipo Tailwind → Style
├── style.rs           Style tipo CSS → taffy::Style, texto heredable
├── theme.rs           tokens de kai-ui-web/src/theme/tokens.css
├── layout.rs          Layout: Taffy, scroll, clip, z-index, hit-test
├── render/            DrawList, paint, texto, atlas de iconos, backend wgpu (quad.wgsl)
├── icons/             catálogo Lucide generado (scripts/gen-icons.mjs, assets/icons/)
├── text_field.rs      edición, IME y portapapeles de los inputs
├── ui_engine.rs       intents y dispatch de foco/activación/texto
├── platform_adapter.rs  winit → intents Kai
├── a11y.rs            árbol AccessKit desde el layout
└── geometry.rs, signal.rs, grid_layout.rs
```

Las librerías mencionadas son **candidatas a evaluar**, no decisiones irreversibles. Antes de adoptarlas, verificar compatibilidad con la versión de Rust elegida, macOS, licencias, mantenimiento e integración entre sí.

La guía de [craft-libs](07-integracion-craft-libs.md) distingue los crates de utilidad potencial de `craft-ui`, cuyo frontend está acoplado a egui y no reemplaza el renderer propio de Kai.

## Fuera del alcance inicial

- Widgets nativos de AppKit o SwiftUI para el dibujo de controles.
- Android, iOS, Windows y Linux.
- Un bus global de señales, componentes con estado local oculto, efectos asíncronos (`Cmd`/`Sub`), caché incremental sin mediciones y comandos serializables con undo/redo.
- Integraciones de aplicación como PDF, impresión, CMS, ajustes persistentes o agentes de IA dentro del renderer.
