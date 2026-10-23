# 18 — Patrones Empty y Loading

Patrones de datos alineados a Polaris `EmptyState` y Carbon empty/loading, en vocabulario AppKit (sin chrome web).

## EmptyState

```rust
use kai::components::{EmptyState, ButtonVariant};
use kai::icons::Icon;

EmptyState::new(Icon::Inbox, "No hay resultados")
    .description("Prueba otro filtro o crea un elemento.")
    .action("Crear elemento", Msg::Create)
    .action_variant(ButtonVariant::Primary) // default
    .test_id("lista.empty")
    .build()
```

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

Sección **Tabla** → card «Empty y Loading» (`patrones.empty_loading`).

## Qué no es

No es un Storybook web ni un skeleton CSS genérico: el placeholder reutiliza el mismo `DotProgress` que el botón ocupado.
