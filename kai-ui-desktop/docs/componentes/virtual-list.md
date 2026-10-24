# VirtualList

Lista con scroll de `count` filas de alto fijo, que construye solo las que
están a la vista. Base de [Table](table.md).

Código: `src/components/table.rs` (`virtual_list`).

## Uso

```rust
use std::rc::Rc;
use kai::components::virtual_list;

let nombres = Rc::clone(&self.nombres);
virtual_list(nombres.len(), 28.0, Rc::new(move |i| {
    text(&nombres[i]).class("px-3")
}))
.selecting_focus_group()
.class("h-96")
```

## API

`virtual_list(count, row_height, render_row) -> Element`.

- Dale alto con `.class(..)`.
- Filas sin rol se exponen como `ListItem`.
- `.selecting_focus_group()` la vuelve una parada de Tab donde las flechas
  eligen.

## Relacionados

[Table](table.md) · `docs/dev/15-tablas-con-muchos-registros.md`
