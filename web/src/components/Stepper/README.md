# Stepper

Indicador de pasos con el contenido del paso activo debajo.

## Import

```ts
import { Stepper, type StepperStepItem } from "@felipechandiadev/ui";
```

## Props

- `steps`: `{ id, title, description? }`.
- `activeIndex`: paso activo, desde cero.
- `children`: formulario del paso actual.

Si el stepper vive dentro de un `Dialog`, los botones Atrás/Siguiente van en las acciones del diálogo, no en el pie del stepper.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<Stepper
  activeIndex={0}
  steps={[{ id: "datos", title: "Datos" }]}
>
  Campos del paso
</Stepper>
```
