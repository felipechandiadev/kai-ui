# Button

Botón con texto para ejecutar una acción. En Kai es un `.fs-button`; con el
tema AppKit es un `NSButton` (push, bordered o borderless).

Código: `src/components.rs` (`button`, `button_sized`, `Button`).

## Cuándo usarlo

- Acciones con nombre: «Guardar», «Exportar», «Eliminar».
- Para una acción con solo icono usa [IconButton](icon-button.md); dentro de
  una barra, [`toolbar_item`](toolbar.md).

## Uso

```rust
use kai::components::{button, button_sized, Button, ButtonVariant, ButtonSize};

// Rápido: variante y mensaje.
button("Guardar", ButtonVariant::Primary).on_click(Msg::Guardar)

// Con tamaño.
button_sized("Cancelar", ButtonVariant::Text, ButtonSize::Sm).on_click(Msg::Cancelar)

// Modelo AppKit completo.
Button::new("Eliminar")
    .destructive(true)
    .default_button(true) // Return lo activa
    .on_click(Msg::Eliminar)
    .build()

// Ejecutar un comando del registro (se deshabilita solo si no está disponible).
button("Exportar", ButtonVariant::Outlined).on_command(ExportarTabla)
```

## API

### Funciones

| Firma | Qué hace |
|---|---|
| `button(label, variant) -> Element` | Botón mediano. |
| `button_sized(label, variant, size) -> Element` | `Sm`/`Lg` también fijan el tamaño de control (Small/Large) que leen las clases AppKit. |
| `button_classes(variant, size) -> String` | Solo las clases, para estilizar otro elemento como botón. |
| `appkit_button_classes(bezel, accent, destructive) -> String` | Clases AppKit de un bezel. |

### `ButtonVariant`

| Variante | Kai | AppKit |
|---|---|---|
| `Primary` | Relleno de color primario | Acento |
| `Outlined` | Borde | Push normal |
| `Text` | Solo texto | Borderless (título en color de acento) |
| `Danger` | Relleno rojo | Destructivo |

`ButtonSize`: `Sm`, `Md`, `Lg`.

### Builder `Button`

| Método | Qué hace |
|---|---|
| `new(label)` | Crea el botón. |
| `bezel(BezelStyle)` | `Push` (bezel con sombra), `Bordered` (relleno sin sombra, estilo toolbar), `Borderless`. |
| `accent(bool)` | Relleno de acento sin ser el botón por defecto. |
| `default_button(bool)` | Acento y Return lo activa. |
| `cancel_button()` | Escape y ⌘. lo activan. |
| `destructive(bool)` | Rojo. |
| `control_size(ControlSize)` | Tamaño de control. |
| `loading(bool)` | Ocupado por algo que la sesión no ve: puntos sobre el texto y sin clics. |
| `busy_feedback(bool)` | `false`: sigue usable mientras corre una tarea que lanzó. |
| `disabled(bool)`, `on_click(msg)`, `build()` | — |

Con el tema Kai, `Button` vuelve a las variantes Kai: por defecto o acento →
`Primary`, borderless → `Text`, destructivo → `Danger`, el resto →
secundario.

## Comportamiento

- **Ocupado automático**: si el clic lanza una tarea con
  `Effects::perform`, el botón queda ocupado hasta que llega su `Msg` (y los
  que esa respuesta encadene). Conserva color y ancho, muestra tres puntos en
  ola ([DotProgress](dot-progress.md)) y no recibe clics. Si lo lanzó un
  comando, el comando aparece «En curso» en el menú, la paleta y para agentes.
- Espacio y Return lo activan con foco; Return también si es
  `default_button`, sin foco.

## Accesibilidad

Rol `Button`, nombre = el texto. Deshabilitado por `on_command` expone el
motivo como descripción. El foco de teclado se dibuja con anillo
(`keyboard-focus:`).

## Temas

- **Kai**: alto `h-control` del tema (Md), relleno o borde según la variante.
- **AppKit**: alto por tamaño de control; push con sombra.
- **Glass** (macOS 26+): bezel plano `fill-2`, sin borde ni sombra.

## Tests

```rust
app.click(By::test_id("form.enviar"));
assert!(app.is_busy(By::test_id("botones.simular")));
```

## Relacionados

[IconButton](icon-button.md) · [Toolbar](toolbar.md) · [Dialog](dialog.md) ·
[DotProgress](dot-progress.md)
