# Layouts

Estructura de página: encabezado, contenido y, en las colecciones, búsqueda y grilla.

## Import

```ts
import {
  BasicPageLayout,
  TabPageLayout,
  EntityDetailLayout,
  EntityDetailSectionNav,
  CollectionPageLayout,
} from "@felipechandiadev/ui";
```

## Props

- `BasicPageLayout`: `title`, `subtitle`, `headerActions`, `children`.
- `TabPageLayout`: igual, más la fila de pestañas en el encabezado.
- `EntityDetailLayout`: detalle de entidad mobile-first — header de identidad (`title`, `meta`, `backAction`, `headerActions`), slot `sectionNav` debajo, y un panel de contenido. Usar con `EntityDetailSectionNav` (client) para tabs underline con wrap.
- `CollectionPageLayout`: título, búsqueda y grilla de contenido. `contentGridColumns` acepta un número o un mapa por breakpoint.
- También exporta clases de tipografía (`typographyPageTitleClassName` y el resto) para no inventar tamaños en cada pantalla.

## Servidor

`BasicPageLayout`, `TabPageLayout` y `EntityDetailLayout` no tienen `"use client"`. Se pueden usar en un Server Component. El slot `tabs` / `sectionNav` puede ser un client component.

`EntityDetailSectionNav` es client (estado controlado de la sección activa).

`CollectionPageLayout` es client component: lee la ruta para la búsqueda. Next incluye ese HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<BasicPageLayout title="Ajustes" headerActions={<button type="button">Guardar</button>}>
  Contenido
</BasicPageLayout>
```
