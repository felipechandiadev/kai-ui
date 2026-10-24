# DotProgress

Progreso indeterminado: puntos (3 por defecto) del color del texto, cada uno
salta 0,16 s después del anterior. Con «reducir movimiento» quedan quietos.

Código: `src/components/progress.rs`.

## Uso

```rust
use kai::components::{dot_progress, DotProgress, DotSize};

dot_progress() // tres puntos medianos, nombre «Cargando»

DotProgress::new().size(DotSize::Lg).label("Sincronizando").build()
```

## API

| Método | Qué hace |
|---|---|
| `dot_progress()` | Tres puntos medianos llamados «Cargando». |
| `DotProgress::new()` | Builder. |
| `count(n)` | Cantidad de puntos (mínimo 1). |
| `size(DotSize)` | `Sm` (6 pt, el de un botón ocupado), `Md`, `Lg` (16 pt). |
| `label(text)` | Nombre accesible (por defecto `DOT_PROGRESS_LABEL`, «Cargando»). |
| `decorative()` | Sin rol ni nombre: el estado ocupado es del elemento de alrededor. |
| `test_id`, `build()` | — |

El alto incluye el salto, así la ola nunca se recorta.

## Relacionados

[Button](button.md) (ocupado automático) · [EmptyState](empty-state.md)
