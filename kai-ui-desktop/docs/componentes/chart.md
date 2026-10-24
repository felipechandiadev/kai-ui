# Chart

Gráfico de barras o sparkline, dibujado por Kai.

Código: `src/charts/mod.rs`.

## Uso

```rust
use kai::components::{chart, ChartKind};

chart(ChartKind::Bar)
    .categories(["Ene", "Feb", "Mar"])
    .series("2025", &[4.0, 12.0, 9.0])
    .series("2026", &[6.0, 10.0, 14.0])
    .selected(self.mes)
    .on_select(Msg::Mes)
    .label("Ventas por mes")
    .class("h-48 w-full")

chart(ChartKind::Sparkline).series("Visitas", &self.visitas).class("h-8 flex-1")
```

## API

| Método | Qué hace |
|---|---|
| `chart(ChartKind)` | `Bar` (barras verticales, una ranura por categoría; varias series lado a lado) o `Sparkline` (línea compacta de la primera serie, sin ejes). |
| `categories(names)` | Nombres bajo las barras. |
| `series(name, &[f32])` | Agrega una serie; las cortas se completan con 0. Solo guarda números finitos. |
| `selected(Option<usize>)` | Categoría destacada. |
| `.on_select(f)` | Clic en una barra: índice de la categoría. |

Dale tamaño (`h-48`, `flex-1`) y nombre accesible.

## Accesibilidad

La descripción accesible lista la primera serie («Ene 4, Feb 12»).

## Relacionados

[Stat](stat.md)
