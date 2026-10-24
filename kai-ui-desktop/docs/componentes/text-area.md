# TextArea

Campo de varias líneas con el aspecto de [TextField](text-field.md). Return
corta la línea y ⌘Return envía `on_submit`.

Código: `src/components/text_area.rs`.

## Uso

```rust
use kai::components::text_area;

text_area("Observaciones", &self.notas)
    .auto_grow(3, 8)
    .max_length(500)
    .helper_text("Visible solo para el equipo")
    .on_input(Msg::Notas)
    .on_submit(Msg::Guardar)
    .test_id("form.notas")
    .build()
```

## API

| Método | Qué hace |
|---|---|
| `text_area(label, value)` | Crea el campo. |
| `rows(n)` | Siempre `n` líneas de alto (3 por defecto). |
| `auto_grow(min, max)` | Crece con el texto entre `min` y `max` líneas; después, scroll. |
| `max_length(n)` | Corta lo que se escribe y muestra el contador `n/max`. |
| `counter(bool)` | Mostrar u ocultar el contador (activo con `max_length`). |
| `placeholder`, `helper_text`, `required`, `read_only`, `disabled` | Como en TextField. |
| `on_input(f)`, `on_submit(msg)` (⌘Return) | — |
| `test_id`, `build()` | — |

## Partes

Las de [TextField](text-field.md#partes-con-test_idx) más `x.counter`.

## Relacionados

[TextField](text-field.md) · [Document](document.md) (texto con formato)
