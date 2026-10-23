# 20 — Contraste WCAG (AppKit)

Checklist medible de pares semánticos AppKit. Objetivo: WCAG 2.1 **AA** (4.5:1 texto normal, 3:1 large/UI).

## Cómo correrlo

```bash
cargo +1.95 test --lib theme::contrast
```

Los tests viven en `src/theme/contrast.rs`. Fallan si un par de `CRITICAL_PAIRS` queda bajo el umbral que el tema promete para ese par.

## Matriz (CRITICAL_PAIRS)

| Par | Apariencia | Umbral |
| --- | --- | --- |
| `label` / `window_background` | light, dark | AA normal 4.5 |
| `label` / `text_background` | light, dark | AA normal 4.5 |
| `on_accent` / `control_accent` | light, dark | AA large/UI 3.0 |
| `secondary_label` / `window_background` | light | AA large/UI 3.0 |
| `secondary_label` / `text_background` | light | AA large/UI 3.0 |
| `error` / `text_background` | light, dark | AA large/UI 3.0 |

Los colores con alpha (`label`, `secondary_label`) se compositan sobre el fondo antes de medir (`Color::contrast_ratio`).

## Excepciones documentadas

No se exige 4.5:1 en:

1. **`on_accent` / `control_accent`** — azul Apple `#007AFF` + blanco ≈ 4.0:1. Válido para títulos de botón (large/UI); no usar como cuerpo sobre relleno accent.
2. **`secondary_label`** — ≈ 3.8–4.0:1 en superficies claras. Para captions/helpers, no cuerpo.
3. **`error` (systemRed) / `text_background` (light)** — ≈ 3.5:1. Helpers de error OK; prosa de error a tamaño cuerpo: preferir `label` + superficie de alerta.
4. **`tertiary_label` / `quaternary_label`** — a propósito tenues; no están en `CRITICAL_PAIRS`.

Lista en código: `EXCEPTIONS` en `src/theme/contrast.rs`.

## API de medición

```rust
use kai::Color;

let ratio = fg.contrast_ratio(bg); // compositing incluido si fg.a < 255
let composed = fg.composite_over(bg);
```
