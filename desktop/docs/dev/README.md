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

- **L0 — Completada:** tipos geométricos y conversión lógica/física extraídos a `desktop/src/geometry.rs`; 7 pruebas unitarias pasan y el usuario confirmó que la revisión visual está correcta.
- **L1 — Completada para el alcance actual:** `Signal<T>` local está implementada y usada por pressed/foco; 4 pruebas de señal pasan. El usuario confirmó pressed, hover y foco correctos.
- **L2–L3 — Gate básico aprobado:** intents/dispatcher puros y adaptador de plataforma implementados; tests automáticos y regresión manual básica reportados correctos. IME/portapapeles siguen siendo verificaciones específicas independientes.
- **L4 — En curso:** `DemoLayout` comparte cajas entre render e interacción; el texto se envuelve según el ancho medido y el overflow vertical usa scroll sin reducir controles. Mínimo 420×340 pt. 27 tests y Clippy pasan. Falta inspección visual de reflow/scroll y mover la ventana a una segunda pantalla. No hay caché ni dependencia `taffy`.
- **L5–L6 — Pendientes:** renderer desacoplado y árbol de widgets. MCP/IA seguirá siendo un adaptador opcional de aplicación con validación/autorización previa.

Las librerías mencionadas son **candidatas a evaluar**, no decisiones irreversibles. Antes de adoptarlas, verificar compatibilidad con la versión de Rust elegida, macOS, licencias, mantenimiento e integración entre sí.

La guía de [craft-libs](07-integracion-craft-libs.md) distingue los crates de utilidad potencial de `craft-ui`, cuyo frontend está acoplado a egui y no reemplaza el renderer propio de Kai.

## Fuera del alcance inicial

- Widgets nativos de AppKit o SwiftUI para el dibujo de controles.
- Android, iOS, Windows y Linux.
- Un bus global de señales, reactividad automática entre widgets, caché incremental sin mediciones y comandos serializables con undo/redo.
- Integraciones de aplicación como PDF, impresión, CMS, ajustes persistentes o agentes de IA dentro del renderer.
