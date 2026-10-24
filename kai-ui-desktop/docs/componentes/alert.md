# Alert

Mensaje en caja (`.fs-alert`) con icono Lucide y borde y fondo semánticos.

Código: `src/components.rs` (`alert`).

## Uso

```rust
use kai::components::{alert, AlertVariant};

alert(AlertVariant::Warning, "Hay 3 facturas sin enviar")
alert(AlertVariant::Error, "No se pudo guardar: sin conexión")
```

## API

`alert(variant, message) -> Element`. `AlertVariant`: `Success`, `Info`,
`Warning`, `Error`.

## Relacionados

[Toast](toast.md) (el mismo aviso, temporal y flotante) · [Badge](badge.md)
