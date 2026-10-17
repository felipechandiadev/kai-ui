# CollectionGrid

Grilla de cards con búsqueda, alta y paginación. El default export es el shell; la vista interna es `CollectionGridView`.

## Import

```ts
import { CollectionGrid, type CollectionGridProps } from "@felipechandiadev/ui";
```

## Props

- `totalRows`: total para el pie. Obligatorio.
- `paginationMode`: `url` (default) o `controlled`.
- `contentItems` o `children`: las cards.
- `contentGridColumns`: cantidad de columnas, o un mapa por breakpoint (`CollectionGridColumnConfig`).
- `showSearch`, `showFooter`, `fillViewport`.

## Servidor

Client component. Usa `next/navigation` para la query (`page`, `limit`, búsqueda). En una página de Next el primer render puede leer esa URL. No usa el wrapper `ssr: false` del DataGrid.

## Ejemplo mínimo

```tsx
<CollectionGrid title="Propiedades" totalRows={items.length} contentItems={items} />
```
