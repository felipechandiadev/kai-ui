# DotProgress

Indicador de puntos. Sirve como pasos de un stepper o como loading.

## Import

```ts
import { DotProgress } from "@felipechandiadev/ui";
```

## Props

- `variant`: `step` ilumina un punto. `wave` los anima en secuencia.
- `count` o `totalSteps`: cantidad de puntos.
- `activeStep`: índice activo. Si no se pasa, `step` avanza solo con `interval`.

## Servidor

Client component. Next lo incluye en el HTML. El avance automático corre después de hidratar.

## Ejemplo mínimo

```tsx
<DotProgress variant="wave" count={5} />
```
