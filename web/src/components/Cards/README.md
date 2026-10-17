# Cards

`Card` para un bloque con título y acciones, y `StatisticsCard` para un valor destacado.

## Import

```ts
import { Card, StatisticsCard } from "@felipechandiadev/ui";
```

## Props

- `Card`: contenido y acciones de texto o icono (`CardAction`).
- `StatisticsCard`: `label`, `value` y `tone` (`primary`, `success`, `info`, `warning`).

## Servidor

Client components. Next los incluye en el HTML y los hidrata.

## Ejemplo mínimo

```tsx
<StatisticsCard label="Ventas" value="12" tone="primary" />
```
