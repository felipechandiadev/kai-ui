# 25 — Catálogo vivo (Fase E)

Sección demo **Catálogo → Catálogo vivo**: matriz filtrable de controles clave bajo tema × densidad × tamaño — sin Storybook web.

## Filtros

| Filtro | Valores | Efecto |
|---|---|---|
| Tema | AppKit / AppKit Dark / Glass / Glass Dark / Kai | `App::theme()` mientras la sección está abierta |
| Densidad | Comfortable / Compact | `Theme.density` + `FieldDensity` / `SwitchDensity` |
| Tamaño | Mini … XL | `Theme.control_size` + `.control_size` donde aplica |

Al salir de la sección, el tema vuelve al de `KAI_THEME` / `demo_theme()`.

## Qué muestra

1. **Button** — Primary / Secondary / Outlined / Danger / disabled / loading.
2. **TextField** — default (editable), required, helper, error.
3. **Checkbox · Switch · Select** — checkbox On/Mixed/disabled; switch on/disabled (densidad); select elegido/disabled.
4. **Badge · Alert** — variantes sólidas + outlined; alertas Info/Warning (no pills WIP).
5. **DotProgress · Tabs** — progreso indeterminado (Sm/Md/Lg ≈ ControlSize) y 2 pestañas.
6. **Empty filter vs resource** — `EmptyState::filter` vs `EmptyState::resource` ([18](18-patterns-empty-loading.md)).
7. **Capas** — swatches de layering ([24](24-layering.md)).

Las matrices estáticas de Campos / Botones / Selección (P0) siguen; el catálogo las **filtra** en vivo.

## Cómo verlo

`cargo run` → sidebar **Catálogo** → **Catálogo vivo** (`test_id` `catalogo`).

## Qué no es / fuera a propósito

- No cablea `TRANSITION_*` stubs (sin infra de animación de layout).
- No incluye **pills** (`wip-pills`), Slider (si no hay API estable), Table/Map/Chart/Document (secciones propias), IconButton, Avatar, Toast/Dialog (overlays).
- No es una matriz filtrable de *todos* los componentes.
