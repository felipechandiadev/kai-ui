# Dialog

Diálogo modal: fondo atenuado sobre la ventana y un panel con título, mensaje
o contenido, y una fila de botones alineada a la derecha.

Código: `src/components/dialog.rs`.

## Uso

```rust
use kai::components::{Dialog, DialogWidth};

// En view: solo mientras debe verse.
self.confirmar_borrado.then(|| {
    Dialog::new("¿Eliminar borrador?")
        .message("Esta acción no se puede deshacer.")
        .cancel_button("Cancelar", Msg::CerrarDialogo)
        .default_button("Eliminar", Msg::Eliminar)
        .destructive(true)
        .width(DialogWidth::Sm)
        .test_id("dialogos.eliminar")
        .build()
})
```

Colócalo en la vista solo mientras debe verse; al dejar de mostrarlo, la
sesión devuelve el foco a donde estaba.

## API

| Método | Qué hace |
|---|---|
| `Dialog::new(title)` | Crea el diálogo. |
| `message(text)` | Texto bajo el título. |
| `content(el)` | Contenido libre (un formulario, una lista), después del mensaje. |
| `button(el)` | Botón extra, antes de cancelar y por defecto. |
| `cancel_button(label, msg)` | Escape y ⌘. lo presionan. |
| `default_button(label, msg)` | Return lo presiona; color de acento. |
| `destructive(bool)` | El botón por defecto en rojo. |
| `width(DialogWidth)` | `Sm` 360 pt (alerta), `Md` 480, `Lg` 640 (formulario). |
| `on_dismiss(msg)` | Escape y ⌘. cuando no hay botón de cancelar. |
| `test_id`, `build()` | — |
| `dialog(title, body, actions)` | Versión simple: no envía mensajes propios y **no** es modal. |

## Foco y teclado

Con `on_dismiss` o un botón de cancelar, el panel es `modal()`: el foco queda
dentro, los clics fuera se ignoran y el foco vuelve al cerrar. Escape envía
el mensaje de cancelar o, si no hay, `on_dismiss`.

## Accesibilidad

Rol `Dialog` con el título como nombre.

## Partes

`panel`, `title`, `body`, `actions`, `cancel`, `default`.

## Relacionados

[Toast](toast.md) · [Button](button.md)
