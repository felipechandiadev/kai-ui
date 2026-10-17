# Button

Botón y grupos de botones. También exporta `ButtonPill`, `ButtonGroup`, `ButtonGroupItem` y `ButtonGroupToggle`.

## Import

```ts
import { Button, ButtonPill, ButtonGroup } from "@felipechandiadev/ui";
```

## Props

- `variant`: `primary`, `secondary`, `outlined`, `outlinedSecondary`, `text`, `danger`.
- `size`: `sm`, `md`, `lg`.
- `loading`: muestra estado de carga y deshabilita el click.
- `fullWidth` y `color` se ignoran; el ancho se controla con `className`.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<Button variant="primary" onClick={() => {}}>
  Guardar
</Button>
```
