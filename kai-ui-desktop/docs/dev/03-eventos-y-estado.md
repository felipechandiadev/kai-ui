# 03 — Eventos y estado

**Fases:** L1 (modelo de estado reactivo y señales) y L2–L3 (catálogo de comandos unificado).

## Objetivo

Acordar cómo la entrada del usuario recorre el árbol propio de Kai y cómo actualiza el estado sin que cada widget controle directamente la ventana o el renderer.

## Modelo inicial recomendado

Usar un ciclo unidireccional simple: **evento de plataforma → evento Kai → hit-test/foco → mensaje → `update` del modelo → `view` → layout/dibujo**. Es la arquitectura Elm, implementada en `runtime.rs` sin incorporar Iced como dependencia. No hay bus global ni suscripciones implícitas.

## Runtime Elm (`runtime.rs`)

Una aplicación implementa `App` o, si no necesita efectos, `SimpleApp`:

```rust
pub trait App: 'static {
    type Msg: Clone + Debug + 'static;
    fn init(&mut self) -> Effects<Self::Msg> { Effects::none() } // una vez, antes del primer view
    fn title(&self) -> String;
    fn update(&mut self, msg: Self::Msg) -> Effects<Self::Msg>;
    fn view(&self) -> Element<Self::Msg>;
    // background, theme y window_override con valor por defecto
}

pub trait SimpleApp: 'static { /* igual, con update(&mut self, msg) sin efectos */ }
impl<T: SimpleApp> App for T { /* devuelve Effects::none() */ }

runtime::run(Demo::default(), Theme::kai(Density::Comfortable))
```

`Effects` es opaco y describe el trabajo que `update` le pide al runtime (Paso 15 de [13](13-hoja-de-ruta-plataforma.md)):

- `Effects::perform(tarea, a_msg)` corre una función bloqueante en un pool de 4 hilos (`effects.rs`); su resultado vuelve por el `EventLoopProxy` y entra a `update` como un `Msg`.
- `Effects::after(duración, msg)` es un temporizador (los toasts lo usan para cerrarse).
- `.keyed(key)` reemplaza la tarea o el temporizador pendiente con la misma key, y `Effects::cancel(key)` descarta su resultado.
- **Temporizadores retenidos.** Si la key de un temporizador es el `test_id` de un elemento, el temporizador espera mientras el puntero (`hover_path`) o el foco estén dentro de ese elemento. Cuando los dos salen, todavía espera al menos `HELD_TIMER_RESUME` (2 s). Así un toast (`toast_key(id)` es también su test id) no se cierra mientras alguien lo lee o llega a sus botones (WCAG 2.2.1). `toast_timer(&toast, close)` no crea temporizador para un error: queda hasta que se cierra.
- `Effects::pick_file(PickFile::images(título), a_msg)` abre el panel de archivos de macOS y entrega la ruta o `None`. `Effects::save_file(SaveFile::new(título, nombre).types(&["xlsx"]), a_msg)` abre el panel de guardar (`NSSavePanel`) con un nombre propuesto y entrega la ruta elegida o `None`; la demo lo usa para exportar la tabla a Excel. Hoy los dos paneles son modales dentro del manejador de eventos, y la app aborta si se termina con uno abierto (fallo conocido, ver el Paso 15 de 13).
- `Effects::none()` y `Effects::batch(..)` combinan.

En `TestApp`, `run_until_idle()` espera las tareas, `advance(duración)` mueve el reloj de los temporizadores, y `on_file_pick(ruta)` y `on_file_save(ruta)` responden los paneles de archivos. Faltan futures (`perform` recibe una función, no un `async`) y suscripciones.

**Botón ocupado.** Cada tarea de `Effects::perform` guarda qué la lanzó (`TaskOrigin`): el control clicado (`Widget(id)`) o el comando (`Command(id)`), ya venga de un atajo, del menú, de la paleta, de un botón con `on_command`, de un script o de un agente. Mientras la tarea no entrega su `Msg`, ese origen está ocupado:

- La sesión marca `busy` el control en cada `rebuild` (y los botones del comando, vía `bind_commands`). El control muestra sus clases `busy:` y no recibe clics; el resto de la ventana sigue respondiendo.
- Un comando ocupado pasa a `Disabled { reason: "En curso" }` (`session::COMMAND_BUSY`) en el menú y la paleta. `perform_command` lo rechaza, igual que `send_command`/`invoke` y las llamadas de agentes, que devuelven ese motivo.
- El `Msg` de una tarea se envía con el origen de la tarea, así una tarea que esa respuesta lanza (guardar y luego recargar) lo hereda y el botón sigue ocupado hasta la última. `Effects::after` y `pick_file` no ocupan nada.
- Una tarea que entra en pánico libera su origen sin enviar `Msg`.
- `Element::busy_feedback(false)` deja usable un control mientras corre su tarea, y `Element::busy(true)` lo ocupa por una razón que la sesión no ve.
- En `TestApp`, `is_busy(by)` lo consulta. El cursor cambia sin mover el puntero, porque la sesión vuelve a calcularlo en cada frame.

- **El estado de la app vive en el modelo.** `view` se ejecuta en cada frame y produce un árbol `Element<Msg>` nuevo; los ids se derivan de la ruta (o de `key`), así que foco, hover, scroll y el caret de cada input sobreviven a la reconstrucción.
- **Eventos como mensajes.** `on_click(msg)`, `on_input(|valor| msg)` y `on_submit(msg)` registran el mensaje que el runtime envía a `update`. El clic burbujea desde el nodo más interno de `hit_path` hasta el control más cercano, igual que un evento DOM.
- **Inputs controlados**, como en React: el valor del árbol se copia al editor en cada frame si difiere. El editor conserva caret, selección e IME; cada edición que cambia el texto emite `on_input` con el valor nuevo, y Enter emite `on_submit`. Un input sin `on_input` es de solo lectura.
- **Estado de interacción fuera del modelo.** El runtime es dueño de `UiModel` (foco y presión como `Signal<Option<WidgetId>>`), del hover, de los offsets de scroll y de los `TextFieldState`. Las variantes `hover:`, `focus:`, `focus-within:`, `active:`, `disabled:` y `group-*` se resuelven en el layout a partir de ese estado, sin que la app lo gestione.
- **Ciclo de un frame:** `view` → `FocusOrder` + handlers → sincronizar inputs controlados → layout → hover → paint → render → árbol AccessKit. El foco por teclado desplaza el contenedor con scroll para revelar el elemento (`Layout::reveal`, en los dos ejes y moviéndolo lo mínimo: una fila más ancha que la vista conserva el scroll horizontal); la rueda desplaza, en cada eje, el contenedor con overflow más interno bajo el puntero que aún puede moverse, y Mayús+rueda desplaza a lo ancho.
- **Menú contextual.** Un clic derecho, o Ctrl+clic, abre el `Element::context_menu` del elemento más interno bajo el puntero que tenga uno, en el puntero y sin moverle el foco; la acción `ShowContextMenu` de AccessKit lo abre también. Es la misma lista de la `Session` que el `Select`, con opciones que pueden ser checks (`check_options`) y no cerrarse al elegir. En `TestApp`, `right_click(by)`.

### `Session`, `WindowState` y `Outbox`

El runtime está partido en dos para que los tests recorran el mismo camino que la ventana (Paso 4 de [13](13-hoja-de-ruta-plataforma.md)):

- **`Session<A>`** (`session.rs`, sin winit) tiene la app, el árbol, los handlers, el `FocusOrder`, el layout, `UiModel`, el hover, el scroll, los `TextFieldState`, el portapapeles (`ClipboardPort`) y, en tests, el log de `Msg`.
  - Recibe entradas en unidades lógicas: `pointer_moved(Option<Point>)`, `pointer_button(pressed)`, `secondary_click()`, `wheel(delta, líneas)`, `scroll_by(delta_y)`, `scroll_by_x(delta_x)`, `apply_intent(Intent)`, `set_window_active(bool)`, `accessibility_action(&ActionRequest)` y `set_viewport(Size)`.
  - Para un frame ofrece `prepare_frame()` (view, layout, hover, scroll y buffers de los inputs), `paint_list()` y `a11y_tree(scale)`.
  - Mide el texto con las fuentes (`Measure::Real`, en la ventana) o con `FixedMeasure` (`Measure::Fixed`, en tests).
- **`Outbox`**: lo que la sesión pide a la ventana. Guarda la última petición de cada tipo: `redraw`, `ime_allowed`, `ime_cursor_area` (en puntos lógicos), `cursor`, `title`, `theme` y `commands` (estados de los comandos, para el menú).
- **`WindowState`** (`runtime.rs`) se queda con lo que depende de la plataforma: la ventana, el renderer wgpu, el adaptador de AccessKit, el factor de escala, los modificadores, la oclusión y el reloj del fondo animado.
  - Traduce cada `WindowEvent` a una entrada de la sesión.
  - Después de cada evento vacía el `Outbox` y lo aplica: escala el área del IME, elige el `CursorIcon`, pone el título y el tema, y pide el redibujado.
  - En `RedrawRequested` llama a `prepare_frame`, pinta y renderiza `paint_list`, y actualiza AccessKit con `a11y_tree`.
- **`kai::testing::TestApp`** usa la misma `Session` con `Measure::Fixed` y un `MemoryClipboard`. Después de cada acción prepara un frame y descarta el `Outbox`.

### Comandos

Lo que la app sabe hacer se declara una vez en `App::commands` (Pasos 7 y 8 de [13](13-hoja-de-ruta-plataforma.md)):

```rust
fn commands(registry: &mut CommandRegistry<Self>) {
    registry.action::<Incrementar>().shortcut(Shortcut::cmd(Key::Char('i'))).menu("Contador");
    registry.action::<Reiniciar>().shortcut(Shortcut::cmd(Key::Char('0'))).menu("Contador");
    registry.query::<Saludo>();
}
```

- **`Action`** produce un `Msg` (`into_msg`) y dice si está disponible (`available(&App) -> Availability`). **`Query`** lee el modelo y no pasa por `update`.
- **`Session`** recalcula la disponibilidad tras cada `update`, igual que `view`. Si cambió, la deja en `Outbox.commands` y la ventana actualiza el menú nativo (`kai::menu`, muda).
- **`Session::perform_command(id)`** es la entrada única de atajos, menú y paleta: envía el `Msg` si la acción está habilitada, y `kai.paleta` abre o cierra la paleta.
- **Ruta de teclas**: `key_equivalent_route` ofrece la tecla primero a los equivalentes de los controles (botón por defecto, cancelar, atajos propios) y después a los atajos de los comandos habilitados, como AppKit. Con un campo enfocado no se toman teclas sin ⌘ ni ⌃, y el registro reserva ⌘A/C/X/V/Z y ⇧⌘Z para la edición. `KeyRoute::Command(id)` sale del dispatch como `Action::Command(id)`, que la sesión atiende antes de la cadena de responders.
- **En la ventana**, el menú consume primero los atajos de sus ítems habilitados y llega al bucle como `UserEvent::Menu(id)`; si el ítem está deshabilitado, la ruta de teclas tampoco lo dispara.
- **Paleta**: mientras está abierta, la sesión arma el `FocusOrder` solo con ella y el único atajo activo es ⇧⌘P. Atiende las flechas y Return de la búsqueda antes del dispatch, cierra con `CancelOperation` (Esc, ⌘.) o con un clic fuera del panel, y devuelve el foco que había.
- **En `TestApp`** (Paso 9) los comandos se prueban sin pasar por la UI:

```rust
app.invoke(Incrementar)?;                          // disponibilidad → into_msg → update → frame
assert!(app.invoke(Reiniciar).is_err());            // Disabled: devuelve el motivo y no envía Msg
let saludo = app.query(Saludo)?;                    // lee el modelo, sin Msg
assert_eq!(app.available::<Reiniciar>(), Availability::Enabled);
app.perform("contador.reiniciar");                  // la vía del menú y los atajos
```

  `invoke` usa `Session::send_command`, que respeta la misma disponibilidad que el menú, la paleta y `on_command`; `commands()` devuelve los estados que ellos ven. La demo prueba cada acción por todas sus vías (invocación, JSON, botón, atajo, menú y paleta) y compara el resultado con `invoke` (`every_action_does_the_same_from_every_via`).
- **Desde JSON** (Paso 11), como lo hará un agente: `Session::call(id, input)` y `TestApp::call` decodifican la entrada con serde, comprueban la disponibilidad y envían el `Msg` (devuelven `null`) o, si es una consulta, devuelven su salida serializada.

```rust
app.call("contador.sumar", json!({"cantidad": 3}))?;     // Msg::Add(3)
app.call("contador.incrementar", json!({}))?;            // acción sin parámetros: {}
assert_eq!(app.call("form.ultimo_envio", json!({}))?, json!(null));
app.call("contador.sumar", json!({})).unwrap_err();      // entrada inválida …: missing field `cantidad`
```

  Una acción cuya entrada `{}` no basta (`needs_input`) no aparece en el menú ni en la paleta, y el registro reporta en debug si declara atajo o menú. `kai::commands::describe::<App>()` publica todos los comandos con sus esquemas ([13](13-hoja-de-ruta-plataforma.md), Paso 11).

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
- [x] La edición vive en `text_field.rs`: `TextFieldState::apply(TextIntent, &mut FontSystem, &mut dyn ClipboardPort) -> TextFieldEffect { changed, status }`; `set_text` aplica el valor controlado solo cuando difiere. **Deshacer en campos**: ⌘Z y ⇧⌘Z llegan al campo enfocado como `TextIntent::Undo`/`Redo`. Cada `TextFieldState` guarda hasta 100 pasos (texto, caret y selección). Escribir seguido es un paso por palabra (el espacio queda con la palabra anterior); borrar seguido es un paso; pegar, cortar y Return son un paso cada uno; mover el caret o hacer clic cierra el paso abierto. Un valor que impone la app (`set_text` con un texto distinto) borra el historial, y un campo enmascarado o de solo lectura no guarda nada. Deshacer cambia el texto como cualquier edición: la app lo recibe por `on_input`. El portapapeles del sistema (`SystemClipboard`, `arboard`) queda detrás de `ClipboardPort`; los tests usan un mock.
- [x] Tests: `update`/`view` de la demo reaccionan a los mensajes, orden de foco de la demo, el clic burbujea hasta el control, cursor según clases, Tab desde el árbol saltando disabled, `sync_focus`, gating de texto por foco y, en `text_field.rs`, selección, portapapeles con mock, IME y valor controlado.

### Integración futura de MCP/agentes

MCP/agentes pertenecen a un adaptador opcional de aplicación, nunca a `ui-core`, `ui-render` o al árbol de widgets. El adaptador autentica/valida la solicitud, aplica autorización y allowlist, y solo entonces la traduce a comandos tipados admitidos por `ui-engine`; el resultado se devuelve como estado/evento de aplicación. No ejecutar herramientas ni conceder permisos por el mero hecho de recibir un comando. `craft-ai-core` es un candidato opcional de contratos para esa capa; su evaluación e inclusión requieren un caso de uso, revisión de dependencias/MSRV/seguridad y pruebas de integración. Ver [07-integracion-craft-libs.md](07-integracion-craft-libs.md).

## Consideraciones de macOS

Un renderer propio no elimina la integración de plataforma. Validar portapapeles, accesos de teclado, composición/IME, accesibilidad y eventos de ciclo de vida. No acoplar esos detalles al tipo público de cada widget; aislarlos en adaptadores de plataforma.

## Mantener fuera del MVP

- Bus global de señales, suscripciones cruzadas y componentes con estado local oculto hasta que una app los necesite. Los efectos (`Cmd` de Elm) ya existen como `Effects`; las suscripciones (`Sub`) llegarán con los websockets de DSP (Paso 15 de 13).
- Tokio como propietario del event loop gráfico.
- Undo/redo de la app (los campos de texto ya deshacen su edición), persistencia de acciones y serialización de cada mensaje.
- Ejecución de comandos de IA/MCP dentro del núcleo visual. Una app puede añadir un adaptador opcional y controlado fuera del MVP.

Estos elementos se pueden añadir como módulos de aplicación después de definir casos concretos y límites de seguridad.
