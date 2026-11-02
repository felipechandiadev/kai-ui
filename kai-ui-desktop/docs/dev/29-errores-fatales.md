# 29 · Errores fatales: informe, estado y aviso al volver a abrir

Tarea #3 de la hoja de ruta. Cuando la app hace `panic`, Kai deja un
informe en disco, guarda dónde estaba la persona y, al volver a abrir, la
app puede avisar con un toast. Todo está en `src/crash.rs` (`kai::crash`) y
es **opcional**: sin `kai::crash::install` nada cambia.

## 1. Uso

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Lo primero de main, en el hilo de la interfaz.
    kai::crash::install("dev.kai.ui-desktop");
    kai::runtime::run(MiApp::default(), Theme::default())
}

impl App for MiApp {
    fn init(&mut self) -> Effects<Msg> {
        if let Some(crash) = kai::crash::take_last() {
            let toast = crash.toast(self.next_toast_id());
            self.toasts.push(toast); // se dibuja con toast_stack
        }
        Effects::none()
    }
    // …
}
```

`Crash::toast` arma un toast de error (no se cierra solo, WCAG 2.2.1) con
el título «La app se cerró por un error», la ruta del informe y, si se
recuperó el estado, «Se recuperó la ventana y lo que tenías abierto».

Con la raíz que genera `kai-build` falta un paso: que la raíz llame a
`take_last` en su `init` y lo empuje a su pila de toasts (`show_toast`).
Queda para un cambio de `kai-build/src/generate.rs`.

## 2. Qué pasa en un panic

| Dónde | Informe (`panic-<unix>-<pid>-<n>.log`) | Marca (`last-crash.json`) | Estado (`crash-state.json`) |
|---|---|---|---|
| Hilo de la interfaz (`update`, `view`, eventos) | Sí | Sí | Sí, si el panic sale del bucle de eventos |
| Tarea del pool (`Effects::perform`) | Sí | No: la tarea no envía nada y la app sigue | No |
| Otro hilo (MCP, stdin) | Sí | No | No |

- **Informe**: mensaje, `archivo:línea:columna`, hilo, versión de Kai y
  backtrace (`Backtrace::force_capture`, sin depender de `RUST_BACKTRACE`).
  Se guardan los últimos `KEPT_LOGS` (10).
- **Marca**: un `Crash` guardado con `Store` (JSON con versión, escritura
  atómica). `take_last` la lee y la borra: el aviso sale una vez.
- **Estado**: el mismo `DevState` que usa `kai dev` (marco de la ventana,
  foco y `App::dev_state`), guardado con `Store`. El siguiente `run` lo
  restaura una vez, igual que tras una recompilación de `kai dev`. Si la
  app ya define `dev_state` para `kai dev`, sirve tal cual.
- El hook anterior sigue corriendo después del de Kai: el mensaje llega a
  stderr como siempre.

### Bucles de crash

Una ejecución que arrancó desde un estado guardado **no** guarda otro si
vuelve a fallar: si el estado es lo que provoca el panic, la tercera
apertura arranca limpia. Si leer `dev_state` del modelo a medias también
hace panic, el informe queda sin estado.

### Panics atrapados por la app

Si la app atrapa un panic en el hilo de la interfaz (`catch_unwind` propio)
y sigue, la marca de esa ejecución se borra al cerrar bien la ventana: no
hay aviso falso. El informe sí queda.

## 3. Carpeta de informes

`kai::crash::dir(id)`; se crea con el primer informe y nunca es un error:

| Sistema | Carpeta |
|---|---|
| macOS | `~/Library/Logs/<id>` (se ve en Consola.app) |
| Windows | `%LOCALAPPDATA%\<id>\Logs` (o `%APPDATA%`) |
| Linux y otros | `$XDG_STATE_HOME/<id>` o `~/.local/state/<id>` |
| Sin esas variables (sin `HOME`) | `<temp>/<id>` |

## 4. Qué no hace (decisiones)

- **No hay `catch_unwind` alrededor de `update` ni de `view`.** Seguir
  después de un panic en `update` deja el modelo a medias, y winit en macOS
  ya detiene su bucle y devuelve el panic a `run_app`. El único
  `catch_unwind` nuevo está alrededor de `run_app`, para guardar el estado
  antes de seguir con el panic (`resume_unwind`): el proceso termina igual
  que antes.
- Con `panic = "abort"` el informe y la marca se escriben (el hook corre
  antes de abortar), pero el estado no.
- No sube informes a ningún servidor.

## 5. Pruebas

`cargo test --lib crash::` (sin ventana): carpeta por sistema y sin `HOME`,
mensaje del payload, informe y marca en el hilo de la interfaz, solo
informe en otros hilos, poda de informes, estado una sola vez, sin bucles
de crash, panic al leer el estado, salida limpia y el texto del toast.

En macOS conviene probar a mano: un `panic!` en `update` de la demo con
`install` → informe en `~/Library/Logs/<id>`, y al reabrir la ventana en el
mismo lugar.
