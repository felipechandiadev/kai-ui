# NumberStepper

Campo numérico con botones para sumar y restar.

## Import

```ts
import { NumberStepper } from "@felipechandiadev/ui";
```

## Props

- `value` y `onChange`: número controlado. Obligatorios.
- `step`, `min`, `max`: rango.
- `allowNegative` y `allowFloat`: qué valores acepta el texto.
- `selectOnFocus`: al enfocar, selecciona todo el valor (default `true`).
- `label`: opcional.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<NumberStepper label="Cantidad" value={1} onChange={setQty} min={0} />
```
