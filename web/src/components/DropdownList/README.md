# DropdownList

Panel de opciones anclado a un disparador. Lo usan `Select` y `AutoComplete`.

## Import

```ts
import { DropdownList } from "@felipechandiadev/ui";
```

## Props

- `open`: muestra u oculta el panel.
- `anchorRef`: elemento disparador. Con `usePortal`, el panel se renderiza en `document.body`.
- `dropUp`: abre hacia arriba.
- `children`: filas. La clase de cada opción es `dropdownOptionClass`.

## Servidor

Client component. Con `open={false}` el panel no está en el HTML.

## Ejemplo mínimo

```tsx
<DropdownList open={open} anchorRef={anchorRef} usePortal>
  <button type="button" className="fs-dropdown__option">Opción</button>
</DropdownList>
```
