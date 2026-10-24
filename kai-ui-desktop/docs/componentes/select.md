# Select

Botón desplegable (`NSPopUpButton`): muestra la opción elegida, o un
placeholder sin elección, y abre la lista de opciones.

Código: `src/components.rs` (`select`, `select_with_options`, `select_menu`).

## Uso

```rust
use kai::components::{select, select_with_options};

let planes = ["Básico", "Pro", "Empresa"];
select(&planes, self.plan, "Elige un plan", Msg::Plan)
    .label("Plan")
    .test_id("form.plan")

// Deshabilitado.
select_with_options(&planes, self.plan, "Elige un plan", Msg::Plan, true)
```

`on_select` recibe el índice elegido.

## API

| Firma | Qué hace |
|---|---|
| `select(options, selected, placeholder, on_select)` | Desplegable habilitado. |
| `select_with_options(options, selected, placeholder, on_select, disabled)` | Con estado deshabilitado. |
| `select_menu(options, highlighted, frame)` | La lista abierta. La dibuja la sesión; no se llama desde la app. |

Ancho como un [TextField](text-field.md): `w-full`, mínimo 180 pt. Cámbialo
con `.class("w-[140px] min-w-[140px]")`.

## Teclado

Clic, Espacio, Return o una flecha abren la lista. En la lista: flechas,
Return elige, Escape o clic fuera cierra. **Búsqueda por tipeo**: escribir
salta a la opción que empieza así.

## Accesibilidad

Nómbralo con `.label(..)` cuando un título aparte lo describe. La lista
abierta es el panel `kai.menu` con filas `kai.menu.<i>`.

## Partes

`x.value` (el texto mostrado).

## Para menús propios

El mismo mecanismo está disponible en cualquier elemento con
`.options(..)` / `.on_select(..)`, y como menú de clic derecho con
`.context_menu()`. Ver [Elementos comunes](elementos-comunes.md#listas-desplegables-y-menús-contextuales).

## Relacionados

[Segmented](segmented.md) (pocas opciones a la vista) · [Checkbox](checkbox.md)
