# IconButton

Botón cuadrado o redondo con solo un icono Lucide. El texto que recibe es su
nombre accesible y su tooltip.

Código: `src/components.rs` (`icon_button`, `icon_button_sized`).

## Cuándo usarlo

- Acciones repetidas y obvias por su icono: cerrar, borrar, editar fila.
- Si la acción no es obvia por el icono, usa un [Button](button.md) con
  texto. En barras, [`toolbar_item`](toolbar.md).

## Uso

```rust
use kai::components::{icon_button, icon_button_sized, IconButtonVariant, IconButtonSize};
use kai::icons::Icon;

icon_button(Icon::Trash2, "Eliminar cliente", IconButtonVariant::Neutral)
    .on_click(Msg::Eliminar(id))

icon_button_sized(Icon::Plus, "Nuevo", IconButtonVariant::Primary, IconButtonSize::Lg)
    .on_command(NuevoCliente)
```

## API

| Firma | Qué hace |
|---|---|
| `icon_button(icon, label, variant) -> Element` | Tamaño medio (alto 40). |
| `icon_button_sized(icon, label, variant, size) -> Element` | Con tamaño. |

`IconButtonVariant`: `Action`, `Primary`, `Secondary`, `Neutral`, `Text`,
`Outlined`. `IconButtonSize`: `Sm`, `Md`, `Lg`.

## Accesibilidad

El `label` es obligatorio: sin texto visible, es lo único que lee VoiceOver.
Rol `Button`; muestra el `label` como tooltip.

## Relacionados

[Button](button.md) · [Toolbar](toolbar.md)
