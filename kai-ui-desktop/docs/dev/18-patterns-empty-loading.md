# 18 — Patrones Empty y Loading

Patrones de datos alineados a Polaris `EmptyState` y Carbon empty/loading, en vocabulario AppKit (sin chrome web).

## EmptyState

```rust
use kai::components::{EmptyState, EmptyKind, ButtonVariant};

// Filter empty — la consulta no devolvió filas (icono Search).
EmptyState::filter("No hay resultados")
    .description("Prueba otro filtro o límpialo.")
    .action("Limpiar filtros", Msg::ClearFilters)
    .test_id("lista.empty.filter")
    .build()

// Resource empty — la colección nunca tuvo ítems (icono Inbox).
EmptyState::resource("Sin elementos")
    .description("Todavía no hay nada aquí.")
    .action("Crear elemento", Msg::Create)
    .test_id("lista.empty.resource")
    .build()
```

`EmptyKind::{Filter, Resource}` tipa la intención; `EmptyState::new(icon, title)` sigue disponible.

- Icono Lucide + título + descripción opcional + acción opcional.
- Rol `Status`; partes `icon`, `title`, `description`, `action`.
- Pensado para el cuerpo de una lista/tabla/card cuando no hay filas.

## Loading

El botón ya tiene `Button::loading` / `Element::busy` con `DotProgress` decorativo.

Para el cuerpo de una lista o tabla en carga:

```rust
use kai::components::loading_placeholder;

loading_placeholder("Cargando filas…")
```

- `DotProgress` + caption centrados; `test_id` `pattern.loading`.
- Sustituye el contenido virtualizado mientras llega la primera página.

## Demo

- **Tabla** → card «Empty y Loading» (`patrones.empty_loading`): filter + resource + loading.
- **Catálogo vivo** → ambas variantes bajo los filtros de tema/densidad/tamaño ([25](25-catalogo-vivo.md)).

## Qué no es

No es un Storybook web ni un skeleton CSS genérico: el placeholder reutiliza el mismo `DotProgress` que el botón ocupado.
