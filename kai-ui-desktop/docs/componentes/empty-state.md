# EmptyState

Superficie vacía reutilizable: icono, título, descripción opcional y acción
opcional. También `loading_placeholder` para la carga.

Código: `src/components/empty_state.rs`. Criterios de texto:
`docs/dev/18-patterns-empty-loading.md`.

## Uso

```rust
use kai::components::{EmptyState, loading_placeholder};

// Búsqueda o filtro sin resultados.
EmptyState::filter("Sin clientes que coincidan")
    .description("Prueba con otro nombre o limpia los filtros.")
    .action("Limpiar filtros", Msg::LimpiarFiltros)
    .build()

// Colección que nunca tuvo datos.
EmptyState::resource("Aún no hay clientes")
    .action("Crear cliente", Msg::Nuevo)
    .build()

// Mientras carga.
loading_placeholder("Cargando clientes")
```

## API

| Método | Qué hace |
|---|---|
| `EmptyState::new(icon, title)` | Icono a elección. |
| `EmptyState::filter(title)` | Lupa; tipo `EmptyKind::Filter`. |
| `EmptyState::resource(title)` | Bandeja; tipo `EmptyKind::Resource`. |
| `kind(EmptyKind)` / `empty_kind()` | Tipo, que guía el texto recomendado. |
| `description(text)` | Texto bajo el título. |
| `action(label, msg)` | Botón (Primary por defecto). |
| `action_variant(ButtonVariant)` | Variante del botón. |
| `test_id`, `build()` | — |
| `loading_placeholder(label)` | Carga indeterminada centrada para una lista o tabla. |

## Relacionados

[DotProgress](dot-progress.md) · [Table](table.md)
