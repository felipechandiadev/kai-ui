# GroupBox

Caja con título (`NSBox`). En AppKit el título va encima de la caja, fuera de
ella; con el tema Kai es una [Card](card.md) con `card_header`.

Código: `src/components.rs` (`group_box`).

## Uso

```rust
use kai::components::group_box;

group_box(Some("Dirección"), [
    TextField::new("Calle", &self.calle).on_input(Msg::Calle).build(),
    TextField::new("Ciudad", &self.ciudad).on_input(Msg::Ciudad).build(),
])
```

## API

`group_box(title: Option<&str>, content) -> Element`. El contenido va dentro
de la caja, en un `card_content`.

## Partes

`label` (el título AppKit de arriba), `box`, y las de la tarjeta: `header`,
`title`, `content`.
