# Card

Tarjeta (`.fs-card`) con encabezado, contenido y acciones. Con el tema AppKit
se dibuja como un group box.

Código: `src/components.rs` (`card`, `card_header`, `card_content`, `card_actions`).

## Uso

```rust
use kai::components::*;

card().class("w-full max-w-xl").children([
    card_header("Clientes", Some("Últimos 30 días")),
    card_content().children([stat("Nuevos", "128", Some("+12 %"))]),
    card_actions().children([button("Ver todos", ButtonVariant::Text).on_click(Msg::Ver)]),
])
```

## API

| Función | Qué es |
|---|---|
| `card()` | El contenedor. |
| `card_header(title, subtitle)` | Título y subtítulo opcional. |
| `card_content()` | Cuerpo. |
| `card_actions()` | Fila de botones al pie. |

## Partes

`header`, `title`, `subtitle`, `content`, `actions`.

## Relacionados

[GroupBox](group-box.md) · [Stat](stat.md) · [Page](page.md)
