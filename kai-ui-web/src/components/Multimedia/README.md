# Multimedia

Campo de imagen única: una vista previa (clic para elegir, o arrastrar y soltar un archivo) con un botón de acción, «Agregar foto» (`Plus`) sin archivo y «Cambiar foto» (`Camera`) con uno. `MultimediaSingleSlot` es el componente base; `MultimediaUpdater` lo envuelve con la API de realEstatePlatform-3 / BShop. Valida tipo y tamaño y muestra el error en un `Alert`.

## Import

```ts
import { MultimediaSingleSlot, MultimediaUpdater } from "@felipechandiadev/ui";
```

## Props

### `MultimediaSingleSlot`
- `variant`: `banner` (16:9), `avatar` (círculo) o `logo` (cuadrado, imagen completa). Obligatorio.
- `currentUrl`: URL del archivo actual ya cargado.
- `currentType`: `'image' | 'video'`. Default: `'image'`.
- `disabled`: desactiva interacción y carga de archivos.
- `allowDragDrop`: permite arrastrar y soltar archivos. Default: `true`.
- `acceptedTypes`: lista MIME de tipos aceptados. Default: `['image/*', 'video/*']`.
- `maxSizeMb`: tamaño máximo permitido en megabytes. Default: `9`.
- `avatarSize`: `'sm' | 'md' | 'lg'` (solo para `variant="avatar"`). Default: `'md'`.
- `actionPlacement`: `'below' | 'edge'` (solo para `variant="avatar"`): botón debajo del círculo o badge en su borde. Default: `'below'`.
- `onFileChange`: callback con el archivo elegido o soltado, ya validado.
- `className`, `data-test-id` (default `multimedia-single-slot`).

### `MultimediaUpdater`
- `variant`: `default`, `banner`, `avatar` o `logo`; `default` usa el slot `banner`. Default: `'default'`.
- `currentUrl`, `currentType`, `onFileChange`, `avatarSize`, `actionPlacement`, `disabled` y `className`: como en `MultimediaSingleSlot`.
- `acceptedTypes`: default `['image/*']`.
- `maxSize`: tamaño máximo en megabytes. Default: `8`.
- `allowDragDrop`: default `false`.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<MultimediaSingleSlot
  variant="avatar"
  actionPlacement="edge"
  currentUrl="/avatar.jpg"
  onFileChange={(file) => console.log(file)}
/>
```

En Kai UI Desktop el equivalente es `MediaSlot` (`MediaVariant::Avatar/Banner/Logo`, `ActionPlacement::Below/Edge`), con `Avatar` aparte para mostrar sin editar.
