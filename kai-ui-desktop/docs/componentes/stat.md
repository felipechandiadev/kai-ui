# Stat

Tarjeta compacta de estadística (`StatisticsCard`): etiqueta, cifra y una nota
opcional.

Código: `src/components.rs` (`stat`).

## Uso

```rust
use kai::components::stat;

stat("Ventas del mes", "$ 4.2 M", Some("+8 % vs. mes anterior"))
```

## API

`stat(label: &str, value: impl Into<String>, hint: Option<&str>) -> Element`.

## Partes

`label`, `value`, `hint`.

## Relacionados

[Card](card.md) · [Chart](chart.md) (sparkline al lado de la cifra)
