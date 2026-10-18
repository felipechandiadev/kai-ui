# 03 — Eventos y estado

**Fases:** L1 (modelo de estado reactivo y señales) y L2–L3 (catálogo de comandos unificado).

## Objetivo

Acordar cómo la entrada del usuario recorre el árbol propio de Kai y cómo actualiza el estado sin que cada widget controle directamente la ventana o el renderer.

## Modelo inicial recomendado

Usar un ciclo unidireccional simple: **evento de plataforma → evento Kai → hit-test/foco → comando/mensaje → actualización explícita de estado → layout/dibujo**. L1 permite señales locales y tipadas para observar estado; no crea un bus global ni suscripciones implícitas. El modelo puede inspirarse en Elm/Iced, pero no se incorpora Iced como runtime visual.

## Estado de L1

- [x] `desktop/src/signal.rs` implementa `Signal<T>` local: `get`, `set` (indica si cambió) y `revision`.
- [x] Un cambio real incrementa revisión; un `set` idempotente no provoca transición. La señal no depende de la plataforma ni agenda render.
- [x] El prototipo usa señales para `button_pressed`, `button_focused` e `input_focused`; el event loop sigue siendo dueño explícito de las decisiones de `request_redraw`.
- [x] Tests cubren estado inicial, cambios, idempotencia y estado genérico no `Copy`.
- [x] Validación manual en macOS: el usuario confirmó que el botón cambia visualmente al presionarlo y que el hover funciona.
- [x] Validación manual en macOS: el usuario confirmó que el foco funciona perfectamente tras migrar los estados a `Signal<T>`.

L1 queda aceptada para el alcance actual: estado local, transición observable, pressed, hover y foco validados. No incluye un sistema de suscripciones ni es el modelo final de todos los widgets. Si los consumidores necesitan notificaciones más finas, se diseñarán luego de un caso real y con control explícito de ciclo de vida.

## Pasos

1. **L1:** implementar `Signal<T>` local y tipada con lectura, actualización explícita, revisión monotónica y resultado que indique si el valor cambió.
2. **L1:** adaptar estado de interacción ya existente de forma incremental; la señal no agenda trabajo ni conoce ventanas/renderers. El consumidor decide invalidación/redraw ante cambio.
3. **L1:** probar valor inicial, cambios/no-cambios, revisión y uso sin dependencias de plataforma.
4. **L2–L3:** definir eventos internos independientes de `winit` y un adaptador de plataforma que preserve datos de IME, texto y tecla física.
5. **L2–L3:** definir en la frontera lógica `ui-engine` comandos tipados de foco, activación, edición y selección; diferenciar preedit/commit IME de texto insertado.
6. **L2–L3:** despachar comandos por un punto explícito; ni widgets ni `ui-engine` dependen de MCP, `craft-ai-core` o `wgpu`.
7. Implementar foco, tabulación y hit-testing a partir de la geometría y los bounds compartidos.
8. Añadir comandos asíncronos solo cuando una app lo necesite; nunca bloquear el event loop.
9. Probar traducción, orden y despacho de eventos, captura del puntero, tabulación, foco y cambios de estado.

## Estado de L2–L3

- [x] `desktop/src/ui_engine.rs` define intents y comandos tipados de Kai para foco, puntero, activación, teclado, texto comprometido, portapapeles e IME; no importa `winit` ni `wgpu`.
- [x] El dispatcher puro reduce intents al modelo local y retorna acciones de texto/activación para que la aplicación ejecute efectos externos.
- [x] `desktop/src/platform_adapter.rs` convierte `winit` a tipos Kai; tecla física/semántica, texto comprometido y ciclo IME viajan por intents separados.
- [x] El event loop actual envía mouse, teclado, texto e IME por el punto explícito de despacho; el adaptador de aplicación conserva operaciones de ventana, portapapeles, edición del buffer y redraw.
- [x] Tests deterministas cubren navegación Tab/Shift+Tab, activación por Enter/Espacio/mouse, gating por foco, texto/IME y separación de tecla/texto.
- [x] Validación manual en macOS tras el nuevo flujo: el usuario reportó correcto el comportamiento probado de teclado, foco, botón e interacción básica.
- [x] Validación manual reportada tras el nuevo flujo: el usuario confirmó que acentos y portapapeles funcionan.
- [x] Validación manual reportada como correcta de IME japonés tras el nuevo flujo: `mihongo` + Espacio muestra candidatos y permite escribir en japonés.

El gate interactivo básico de L2–L3 queda aprobado según la confirmación del usuario; acentos, portapapeles, composición IME japonesa con candidatos y escritura en japonés están confirmados manualmente. No se registraron por separado commit con Enter ni cancelación con Escape. Este es un primer corte del catálogo para el prototipo, no una API pública ni un crate separado. No incluye serialización de comandos, undo/redo, ejecución MCP/IA ni llamadas desde widgets a ventana/GPU.

### Integración futura de MCP/agentes

MCP/agentes pertenecen a un adaptador opcional de aplicación, nunca a `ui-core`, `ui-render` o al árbol de widgets. El adaptador autentica/valida la solicitud, aplica autorización y allowlist, y solo entonces la traduce a comandos tipados admitidos por `ui-engine`; el resultado se devuelve como estado/evento de aplicación. No ejecutar herramientas ni conceder permisos por el mero hecho de recibir un comando. `craft-ai-core` es un candidato opcional de contratos para esa capa; su evaluación e inclusión requieren un caso de uso, revisión de dependencias/MSRV/seguridad y pruebas de integración. Ver [07-integracion-craft-libs.md](07-integracion-craft-libs.md).

## Consideraciones de macOS

Un renderer propio no elimina la integración de plataforma. Validar portapapeles, accesos de teclado, composición/IME, accesibilidad y eventos de ciclo de vida. No acoplar esos detalles al tipo público de cada widget; aislarlos en adaptadores de plataforma.

## Mantener fuera del MVP

- Bus global de señales, suscripciones cruzadas y reactividad automática compleja.
- Tokio como propietario del event loop gráfico.
- Undo/redo, persistencia de acciones y serialización de cada mensaje.
- Ejecución de comandos de IA/MCP dentro del núcleo visual. Una app puede añadir un adaptador opcional y controlado fuera del MVP.

Estos elementos se pueden añadir como módulos de aplicación después de definir casos concretos y límites de seguridad.
