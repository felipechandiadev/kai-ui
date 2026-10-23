# 25 — Catálogo vivo (Fase E)

Sección demo **Catálogo → Catálogo vivo**: matriz filtrable de controles clave (Button, TextField, EmptyState) bajo tema × densidad × tamaño — sin Storybook web.

## Filtros

| Filtro | Valores | Efecto |
|---|---|---|
| Tema | AppKit / AppKit Dark / Glass / Glass Dark / Kai | `App::theme()` mientras la sección está abierta |
| Densidad | Comfortable / Compact | `Theme.density` + `FieldDensity` en TextField |
| Tamaño | Mini … XL | `Theme.control_size` + `.control_size` en muestras |

Al salir de la sección, el tema vuelve al de `KAI_THEME` / `demo_theme()`.

## Qué muestra

1. **Button** — Primary / Secondary / Outlined / Danger / disabled / loading.
2. **TextField** — default (editable), required, helper, error.
3. **Empty filter vs resource** — `EmptyState::filter` vs `EmptyState::resource` ([18](18-patterns-empty-loading.md)).
4. **Capas** — swatches de layering ([24](24-layering.md)).

Las matrices estáticas de Campos / Botones (P0) siguen; el catálogo las **filtra** en vivo.

## Cómo verlo

`cargo run` → sidebar **Catálogo** → **Catálogo vivo** (`test_id` `catalogo`).

## Qué no es

- No cablea `TRANSITION_*` stubs (sin infra de animación de layout).
- No es una matriz filtrable de *todos* los componentes (deuda opcional).
