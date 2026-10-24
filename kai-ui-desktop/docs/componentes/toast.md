# Toast

Avisos flotantes en la esquina inferior derecha que se cierran solos a los 4
segundos (los de error se quedan).

Código: `src/components/toast.rs`.

## Uso

```rust
use kai::components::{Toast, toast_stack, toast_timer, AlertVariant};

// Al agregar uno, en update:
let id = self.siguiente_id;
self.siguiente_id += 1;
let aviso = Toast::new(id, AlertVariant::Success, "Cliente guardado")
    .message("Ana Pérez quedó en el plan Pro")
    .action("Deshacer", Msg::Deshacer);
let efecto = toast_timer(&aviso, Msg::CerrarAviso(id));
self.avisos.push(aviso);
return efecto;

// En la vista, dentro de un contenedor `relative`:
toast_stack(&self.avisos, Msg::CerrarAviso)
```

## API

| Elemento | Qué hace |
|---|---|
| `Toast::new(id, variant, title)` | Un aviso; `id` lo distingue (claves, test ids, temporizadores). |
| `message(text)`, `action(label, msg)` | Texto y botón opcionales. |
| `toast_timer(&toast, close) -> Effects` | Envía `close` tras `TOAST_DURATION` (4 s). Espera mientras el puntero o el foco están sobre el aviso. Un error no tiene temporizador (WCAG 2.2.1). |
| `toast_key(id)` | Clave del temporizador, para cancelarlo con `Effects::cancel`. |
| `toast_stack(toasts, on_close)` | Pila de avisos, el más nuevo abajo, en la esquina inferior derecha del ancestro `relative` más cercano. |

## Accesibilidad

Errores: rol `Alert` (se leen al momento). El resto: rol `Status`. Cada uno
tiene botón de cerrar.

## Tests

Pila: `kai.avisos`; cada aviso `kai.aviso.<id>` con partes `title`,
`message`, `action`, `close`.

## Relacionados

[Alert](alert.md) · [Dialog](dialog.md)
