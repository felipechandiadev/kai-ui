# LocationPicker

Mapa Leaflet para elegir o mostrar una coordenada. Exporta el picker, un wrapper sin SSR y una vista de solo lectura.

## Import

```ts
import { LocationPicker, LocationPickerWrapper, LocationPreview } from "@felipechandiadev/ui";
```

## Props

- `mode`: `viewer`, `edit` o `update`.
- `initialLat` / `initialLng`: centro inicial.
- `onChange`: `{ lat, lng }` o `null`.
- `draggable`: permite mover el marcador.
- `LocationPreview`: `latitude`, `longitude`, `zoom`, `height`. Solo lectura.

## Servidor

El mapa no entra en el HTML. `LocationPicker`, `LocationPickerWrapper` y `LocationPreview` cargan Leaflet con `next/dynamic` y `ssr: false`. Hasta hidratar se ve el loading (`Cargando mapa` en el wrapper).

## Ejemplo mínimo

```tsx
<LocationPickerWrapper
  mode="edit"
  initialLat={-33.45}
  initialLng={-70.66}
  onChange={(coords) => console.log(coords)}
/>
```
