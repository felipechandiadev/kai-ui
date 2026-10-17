# LoadingState

Estado de carga con `DotProgress` y un texto para lectores de pantalla.

## Import

```ts
import { LoadingState } from "@felipechandiadev/ui";
```

## Props

- `label`: texto de `aria-label`. Default `Cargando`.
- `size`: tamaño de los puntos.
- `className`: layout del contenedor.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<LoadingState label="Cargando mapa" />
```
