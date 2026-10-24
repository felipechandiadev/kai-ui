# Segmented

Control segmentado (`NSSegmentedControl`, elegir uno): un segmento por
opción, el elegido en estado activo.

Código: `src/components.rs` (`segmented`, `segmented_with_options`).

## Uso

```rust
use kai::components::segmented;

segmented(&["Día", "Semana", "Mes"], self.rango, Msg::Rango)
    .label("Rango")
    .test_id("graficos.rango")
```

## API

| Firma | Qué hace |
|---|---|
| `segmented(options, selected, on_select)` | Control habilitado. |
| `segmented_with_options(options, selected, on_select, disabled)` | Con estado deshabilitado. |

## Teclado y accesibilidad

Es una sola parada de Tab; las flechas recorren los segmentos y Espacio o un
clic eligen (`on_select(índice)`). Nombra el grupo con `.label(..)`.

## Partes

Una por segmento, por índice: `x.0`, `x.1`…

## Cuándo usar otra cosa

Más de 4–5 opciones o textos largos → [Select](select.md). Cambiar de
panel → [Tabs](tabs.md).
