# Tipografía

Textos con la escala del tema.

Código: `src/components.rs` (`heading`, `paragraph`, `caption`); el texto
base es `kai::element::text`.

## Uso

```rust
use kai::components::{heading, paragraph, caption};
use kai::element::text;

heading("Clientes")
paragraph("Lista de clientes activos con su plan y su último pago.")
caption("Actualizado hace 5 minutos")
text("Texto libre").class("text-sm font-medium text-muted-foreground")
```

## API

| Función | Uso |
|---|---|
| `heading(content)` | Título de sección (rol `Heading`). |
| `paragraph(content)` | Texto de cuerpo. |
| `caption(content)` | Texto secundario pequeño. |
| `text(content)` | Texto sin estilo propio; estilízalo con clases. |

Tamaños de clase: `text-xs`, `text-sm`, `text-base`, `text-lg`…, y
`text-[13px]` arbitrario. Colores: `text-foreground`, `text-muted-foreground`,
`text-accent`; en AppKit `text-label`, `text-secondary-label`.
