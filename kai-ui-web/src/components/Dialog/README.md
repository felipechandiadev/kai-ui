# Dialog

Diálogo modal. También exporta `DeleteDialog` y `DialogToPrint`.

## Import

```ts
import { Dialog, DeleteDialog } from "@felipechandiadev/ui";
```

## Props

- `open` y `onClose`: visibilidad.
- `size`: `xs` a `xxl`, o `custom` con `maxWidth`.
- `scroll`: `body` (la página scrollea) o `paper` (scroll interno).
- `persistent`: no cierra con Escape ni con click en el fondo.
- `disableBackdropClick`: bloquea solo el click en el fondo.

## Servidor

Client component. El panel se monta en un portal cuando `open` es true. Con `open={false}` no hay diálogo en el HTML.

## Ejemplo mínimo

```tsx
<Dialog open={open} onClose={() => setOpen(false)} title="Editar">
  Contenido
</Dialog>
```
