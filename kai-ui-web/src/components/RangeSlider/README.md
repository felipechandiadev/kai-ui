# RangeSlider

Selector de un rango numérico con dos extremos.

## Import

```ts
import { RangeSlider } from "@felipechandiadev/ui";
```

## Props

- `min` y `max`: límites. Default `0` y `100`.
- `value`: tupla `[mínimo, máximo]`.
- `onChange`: recibe la tupla nueva.

## Servidor

Client component. Next lo incluye en el HTML. El arrastre se resuelve en el cliente.

## Ejemplo mínimo

```tsx
<RangeSlider min={0} max={100} value={[20, 80]} onChange={setRange} />
```
