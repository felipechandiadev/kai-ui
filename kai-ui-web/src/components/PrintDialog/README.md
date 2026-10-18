# PrintDialog

Diálogo para previsualizar e imprimir un contenido. También exporta `usePrint`.

## Import

```ts
import { PrintDialog, usePrint } from "@felipechandiadev/ui";
```

## Props

- `open` y `onClose`.
- `children`: lo que se imprime.
- `pageSize`: `A4`, `Letter`, `A3`, `A5`.
- `pageOrientation`: `portrait` o `landscape`.
- `fileName`: nombre sugerido del archivo.
- `printIconButton`: el botón de imprimir es un `IconButton` en vez de texto.

## Servidor

Client component. Con `open={false}` el diálogo no está en el HTML. La impresión corre en el cliente.

## Ejemplo mínimo

```tsx
<PrintDialog open={open} onClose={() => setOpen(false)} title="Recibo">
  <p>Contenido</p>
</PrintDialog>
```
