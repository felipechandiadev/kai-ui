# 03 — Eventos y estado

**Fases:** L1 (modelo de estado reactivo y señales) y L2–L3 (catálogo de comandos unificado).

## Objetivo

Acordar cómo la entrada del usuario recorre el árbol propio de Kai y cómo actualiza el estado sin que cada widget controle directamente la ventana o el renderer.

## Modelo inicial recomendado

Usar un ciclo unidireccional simple: **evento de plataforma → evento Kai → hit-test/foco → mensaje → `update` del modelo → `view` → layout/dibujo**. Es la arquitectura Elm, implementada en `runtime.rs` sin incorporar Iced como dependencia. No hay bus global ni suscripciones implícitas.

## Runtime Elm (`runtime.rs`)

Una aplicación implementa `App`:

```rust
pub trait App: 'static {
    type Msg: Clone + 'static;
    fn title(&self) -> String;
    fn update(&mut self, msg: Self::Msg);
    fn view(&self) -> Element<Self::Msg>;
}

runtime::run(Demo::default(), Theme::kai(Density::Comfortable))
```

- **El estado de la app vive en el modelo.** `view` se ejecuta en cada frame y produce un árbol `Element<Msg>` nuevo; los ids se derivan de la ruta (o de `key`), así que foco, hover, scroll y el caret de cada input sobreviven a la reconstrucción.
- **Eventos como mensajes.** `on_click(msg)`, `on_input(|valor| msg)` y `on_submit(msg)` registran el mensaje que el runtime envía a `update`. El clic burbujea desde el nodo más interno de `hit_path` hasta el control más cercano, igual que un evento DOM.
- **Inputs controlados**, como en React: el valor del árbol se copia al editor en cada frame si difiere. El editor conserva caret, selección e IME; cada edición que cambia el texto emite `on_input` con el valor nuevo, y Enter emite `on_submit`. Un input sin `on_input` es de solo lectura.
- **Estado de interacción fuera del modelo.** El runtime es dueño de `UiModel` (foco y presión como `Signal<Option<WidgetId>>`), del hover, de los offsets de scroll y de los `TextFieldState`. Las variantes `hover:`, `focus:`, `focus-within:`, `active:`, `disabled:` y `group-*` se resuelven en el layout a partir de ese estado, sin que la app lo gestione.
- **Ciclo de un frame:** `view` → `FocusOrder` + handlers → sincronizar inputs controlados → layout → hover → paint → render → árbol AccessKit. El foco por teclado desplaza el contenedor con scroll para revelar el elemento (`Layout::reveal`); la rueda desplaza el contenedor con overflow más interno bajo el puntero que aún puede moverse.

## Estado de L1

- [x] `kai-ui-desktop/src/signal.rs` implementa `Signal<T>` local: `get`, `set` (indica si cambió) y `revision`.
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

- [x] `kai-ui-desktop/src/ui_engine.rs` define intents y comandos tipados de Kai para foco, puntero, activación, teclado, texto comprometido, portapapeles e IME; no importa `winit` ni `wgpu`.
- [x] El dispatcher puro reduce intents al modelo local y retorna acciones de texto/activación para que la aplicación ejecute efectos externos.
- [x] `kai-ui-desktop/src/platform_adapter.rs` convierte `winit` a tipos Kai; tecla física/semántica, texto comprometido y ciclo IME viajan por intents separados.
- [x] El event loop actual envía mouse, teclado, texto e IME por el punto explícito de despacho; el adaptador de aplicación conserva operaciones de ventana, portapapeles, edición del buffer y redraw.
- [x] Tests deterministas cubren navegación Tab/Shift+Tab, activación por Enter/Espacio/mouse, gating por foco, texto/IME y separación de tecla/texto.
- [x] Validación manual en macOS tras el nuevo flujo: el usuario reportó correcto el comportamiento probado de teclado, foco, botón e interacción básica.
- [x] Validación manual reportada tras el nuevo flujo: el usuario confirmó que acentos y portapapeles funcionan.
- [x] Validación manual reportada como correcta de IME japonés tras el nuevo flujo: `mihongo` + Espacio muestra candidatos y permite escribir en japonés.

El gate interactivo básico de L2–L3 queda aprobado según la confirmación del usuario; acentos, portapapeles, composición IME japonesa con candidatos y escritura en japonés están confirmados manualmente. No se registraron por separado commit con Enter ni cancelación con Escape. Este es un primer corte del catálogo para el prototipo, no una API pública ni un crate separado. No incluye serialización de comandos, undo/redo, ejecución MCP/IA ni llamadas desde widgets a ventana/GPU.

### Evolución tras L5–L6 y el runtime Elm

- [x] `FocusTarget` fue reemplazado por `WidgetId`. `UiModel` guarda `focused` y `pressed` como `Signal<Option<WidgetId>>`.
- [x] `dispatch(model, &FocusOrder, intent)` recibe el orden de foco derivado del árbol (`FocusOrder::from_tree`): solo los botones e inputs presentes y habilitados reciben foco, presión o activación. Tab/Shift+Tab recorren ese orden con wrap; un clic en espacio vacío quita el foco.
- [x] `sync_focus` descarta foco y presión que apuntan a elementos eliminados o deshabilitados tras reconstruir la vista.
- [x] `DispatchResult` informa `Activation { widget, source, count }`, que el runtime convierte en el mensaje `on_click`, y acciones de texto `(WidgetId, TextIntent)` dirigidas al input enfocado.
- [x] La edición vive en `text_field.rs`: `TextFieldState::apply(TextIntent, &mut FontSystem, &mut dyn ClipboardPort) -> TextFieldEffect { changed, status }`; `set_text` aplica el valor controlado solo cuando difiere. El portapapeles del sistema (`SystemClipboard`, `arboard`) queda detrás de `ClipboardPort`; los tests usan un mock.
- [x] Tests: `update`/`view` de la demo reaccionan a los mensajes, orden de foco de la demo, el clic burbujea hasta el control, cursor según clases, Tab desde el árbol saltando disabled, `sync_focus`, gating de texto por foco y, en `text_field.rs`, selección, portapapeles con mock, IME y valor controlado.

### Integración futura de MCP/agentes

MCP/agentes pertenecen a un adaptador opcional de aplicación, nunca a `ui-core`, `ui-render` o al árbol de widgets. El adaptador autentica/valida la solicitud, aplica autorización y allowlist, y solo entonces la traduce a comandos tipados admitidos por `ui-engine`; el resultado se devuelve como estado/evento de aplicación. No ejecutar herramientas ni conceder permisos por el mero hecho de recibir un comando. `craft-ai-core` es un candidato opcional de contratos para esa capa; su evaluación e inclusión requieren un caso de uso, revisión de dependencias/MSRV/seguridad y pruebas de integración. Ver [07-integracion-craft-libs.md](07-integracion-craft-libs.md).

## Consideraciones de macOS

Un renderer propio no elimina la integración de plataforma. Validar portapapeles, accesos de teclado, composición/IME, accesibilidad y eventos de ciclo de vida. No acoplar esos detalles al tipo público de cada widget; aislarlos en adaptadores de plataforma.

## Mantener fuera del MVP

- Bus global de señales, suscripciones cruzadas, componentes con estado local oculto y comandos/efectos asíncronos (`Cmd`/`Sub` de Elm) hasta que una app los necesite.
- Tokio como propietario del event loop gráfico.
- Undo/redo, persistencia de acciones y serialización de cada mensaje.
- Ejecución de comandos de IA/MCP dentro del núcleo visual. Una app puede añadir un adaptador opcional y controlado fuera del MVP.

Estos elementos se pueden añadir como módulos de aplicación después de definir casos concretos y límites de seguridad.
