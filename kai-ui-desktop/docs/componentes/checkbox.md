# Checkbox

Casilla (`NSButton` `.checkbox`) con su título. Caja y título son un solo
control: un clic en el título también la alterna.

Código: `src/components.rs` (`checkbox`, `checkbox_with_options`).

## Uso

```rust
use kai::components::checkbox;
use kai::element::ControlState;

let estado = if self.acepta { ControlState::On } else { ControlState::Off };
checkbox("Acepto los términos", estado, Msg::AlternarAcepta)

// Estado mixto («algunos elegidos»).
checkbox("Todos", ControlState::Mixed, Msg::AlternarTodos)
```

## API

| Firma | Qué hace |
|---|---|
| `checkbox(label, state, on_toggle)` | Casilla habilitada. |
| `checkbox_with_options(label, state, on_toggle, disabled)` | Con estado deshabilitado. |

`ControlState`: `Off`, `On`, `Mixed` (muestra un guion). Qué hace un clic
desde cada estado lo decide la app en `on_toggle`. La caja sigue el tamaño de
control (`size-check`).

## Partes

`x.box`, `x.label`.

## Relacionados

[Switch](switch.md) · [Elementos comunes: `check_options`](elementos-comunes.md#listas-desplegables-y-menús-contextuales)
