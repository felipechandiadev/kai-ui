# 23 — Escala de spacing, densidad y do/don't (Fase D)

Gobierno continuo del layout sobre [21 — PageGrid](21-page-layout.md) y [22 — Plantillas](22-page-templates.md). AppKit canónico; **no** se copia Carbon 16 ni IDs `spacing-01…13`.

## Escala extendida

El motor `tw` ya resuelve **cualquier paso ≥ 0**: `gap-N` / `p-N` / `m-N` / `px-N`… = `N × Theme::spacing_unit()`.

- Comfortable: `spacing_unit` = **4 pt** → `gap-4` = 16 pt (= `--space-4`), `gap-16` = **64 pt**.
- Compact: el unit se encoge (Kai típ. 3 pt); los mismos pasos producen menos pt.

Constantes tipadas: `kai::theme::spacing` (`steps::S0…S16`, `comfortable_pt`, `gap_class` / `pt_for`). Alias semánticos:

| Alias | Steps | Clase | Uso |
|---|---|---|---|
| `FORM_STACK_STEPS` | 5 | `gap-5` | `FormPage` / `PageDensity::Comfortable` |
| `DATA_STACK_STEPS` | 3 | `gap-3` | `DataPage` / `PageDensity::Compact` |
| `PAGE_GUTTER_STEPS` | 4 | `gap-4` | PageGrid gutter |
| `CONTROL_CLUSTER_STEPS` | 2 | `gap-2` | botones / actions en fila |
| `CARD_PADDING_STEPS` | 4 | `p-4` | padding de card habitual |

`SpaceTokens::{space_1…space_4, page_gap, …}` siguen siendo los rem del tema web; la escala extendida **no** los reemplaza — los nombra encima del unit Tailwind.

```rust
use kai::theme::{gap_class, pt_for, spacing_steps, FORM_STACK_STEPS};

let g = gap_class(FORM_STACK_STEPS);           // "gap-5"
let pt = pt_for(&theme, spacing_steps::S16);   // 64.0 comfortable
```

## Guía de densidad

| Contexto | `PageDensity` | `Theme::Density` | Ritmo |
|---|---|---|---|
| Formularios, settings, alta/edición | `Comfortable` | `Comfortable` | `gap-5` (step 5) |
| Tablas, métricas, gráficos densos | `Compact` | `Compact` | `gap-3` (step 3) |

Reglas:

1. La **plantilla** declara intención (`FormPage::density` / `DataPage::density`).
2. La **ventana** puede alinear el tema con `PageDensity::theme_density()` (opcional).
3. No mezclar compact en campos de formulario “largos” ni comfortable en tablas de 10k filas sin motivo.
4. Demo: sidebar **Plantillas** → Formulario / Vista de datos muestran ambos ritmos.

## Forms pattern (ligero)

Preferir `FormPage` + `form_field_cell` (span 6) / `form_full_cell` (span 12):

- Mitad de fila para nombre/correo; fila completa para notas / textarea.
- Actions al pie alineadas al final (`justify-end`, `gap-2`).
- No apilar `p-8` + `gap-8` + `space-4` del tema a la vez: un ritmo vertical basta (`FORM_STACK_STEPS`).
- Labels vía `TextField` / `FieldLabels` del tema; no inventar márgenes ad-hoc entre label y control.

Detalle de API: [22](22-page-templates.md).

## Do / don't

### Button

| Do | Don't |
|---|---|
| Agrupar actions con `gap-2` (`CONTROL_CLUSTER_STEPS`) | Separar cada botón con `mt-8` suelto |
| Primary a la derecha en pies de formulario (AppKit) | Primary + Danger juntos sin confirmación |
| `gap-control` entre controles vecinos del mismo toolbar | Mezclar `gap-1` y `gap-6` en la misma fila de actions |

### TextField

| Do | Don't |
|---|---|
| Campos en PageGrid span 6/12 dentro de `FormPage` | Anchos fijos `w-[347px]` por campo |
| Un helper/error bajo el campo (`TextField::error`) | Márgenes negativos para “pegar” el error al borde |
| Densidad comfortable en formularios largos | Compact + labels arriba en formularios de alta |

### Document / página

| Do | Don't |
|---|---|
| Shell `page` → `page_body` → grid/plantilla | Scroll en el body **y** en cada card a la vez sin necesidad |
| Documento: canvas a altura completa (sin card/`max-w` de formulario) | Meter el editor Document dentro de `FormPage` |
| PageGrid 12; apilar con span ≥ 6 bajo `sm` | Clonar grilla Carbon 16 |
| Márgenes de página (`px-4 sm:px-6 md:px-8`) del shell | Añadir otro `px-16` dentro del body “por si acaso” |

## Motion

No hay aún un sistema general de transitions. Duraciones **productivas** documentadas en `kai::theme::motion` (deben coincidir con las constantes de runtime):

| Token doc | Valor | Runtime |
|---|---|---|
| `CARET_BLINK` | 530 ms | `text_field::CARET_BLINK_PERIOD` |
| `TOAST` | 4 s | `components::TOAST_DURATION` |
| `TYPEAHEAD_RESET` | 1 s | `session` typeahead |

Stubs no cableados: `TRANSITION_SHORT_STUB_MS` (120), `TRANSITION_MEDIUM_STUB_MS` (200). Respetar `SystemPrefs::reduce_motion` (caret fijo, fondos quietos).

## Qué no es

- No es Carbon spacing-01…13 ni UI Shell web.
- No amplía PageGrid más allá de 12.
- No es la matriz filtrable de componentes (Fase E).

## Fase E

Hecha: catálogo vivo filtrable — [25-catalogo-vivo.md](25-catalogo-vivo.md). Stubs `TRANSITION_*` siguen sin cablear (sin infra de animación).
