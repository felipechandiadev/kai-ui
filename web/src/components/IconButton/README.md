# IconButton

Botón que muestra un icono de `lucide-react` por nombre.

## Import

```ts
import { IconButton } from "@felipechandiadev/ui";
```

## Props

- `icon`: nombre de un icono Lucide, por ejemplo `"Plus"`.
- `variant`: `action`, `primary`, `secondary`, `primaryCircle`, `secondaryCircle`, `neutral`, `text`, `outlined`.
- `size`: `xs`, `sm`, `md`, `lg`, `xl`, o un número en px.
- `ariaLabel`: texto accesible. Hace falta cuando el botón no tiene texto visible.
- `isLoading`: reemplaza el icono por un estado de carga.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<IconButton icon="Plus" variant="action" ariaLabel="Agregar" onClick={() => {}} />
```
