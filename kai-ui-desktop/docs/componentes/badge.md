# Badge

Indicador en píldora (`.fs-badge`) para un estado o una categoría.

Código: `src/components.rs` (`badge`, `badge_classes`).

## Uso

```rust
use kai::components::{badge, BadgeVariant};

badge("Pro", BadgeVariant::Primary)
badge("Vencido", BadgeVariant::ErrorOutlined)
```

## API

`badge(label, variant) -> Element`; `badge_classes(variant)` da solo las
clases.

`BadgeVariant`: `Primary`, `Secondary`, `Success`, `Error`, `Warning`,
`Info` y sus versiones con borde `PrimaryOutlined`, `SecondaryOutlined`,
`SuccessOutlined`, `ErrorOutlined`, `WarningOutlined`, `InfoOutlined`.

## Accesibilidad

El color no es la única señal: el texto del badge dice el estado. El
contraste de cada variante está medido en `docs/dev/20-contraste-wcag.md`.

## Relacionados

[Alert](alert.md) · [Sidebar](sidebar.md) (badges de conteo en filas)
