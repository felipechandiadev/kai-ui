# Multimedia

Componentes para carga y gestión de archivos multimedia (imágenes y videos), incluyendo vista de ranura única (`MultimediaSingleSlot`) y subida/gestión múltiple (`MultimediaUpdater`).

## Import

```ts
import { MultimediaSingleSlot, MultimediaUpdater } from "@felipechandiadev/ui";
```

## Props

### `MultimediaSingleSlot`
- `variant`: `single-image`, `single-video`, `avatar` o `icon`. Obligatorio.
- `currentUrl`: URL del archivo actual ya cargado.
- `currentType`: `'image' | 'video'`. Default: `'image'`.
- `disabled`: desactiva interacción y carga de archivos.
- `allowDragDrop`: permite arrastrar y soltar archivos. Default: `true`.
- `acceptedTypes`: lista MIME de tipos aceptados.
- `maxSizeMb`: tamaño máximo permitido en megabytes.
- `avatarSize`: `'sm' | 'md' | 'lg'` (solo para `variant="avatar"`). Default: `'md'`.
- `actionPlacement`: `'bottom' | 'badge-edge'` (solo para `variant="avatar"`). Default: `'bottom'`.
- `onFileChange`: callback al seleccionar o limpiar un archivo.

### `MultimediaUpdater`
- `variant`: `gallery`, `banner` o `documents`.
- `items`: lista de elementos multimedia.
- `onItemsChange`: callback con la lista actualizada de elementos.
- `maxFiles`: número máximo de archivos permitidos.
- `disabled`: desactiva acciones de subida o edición.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<MultimediaSingleSlot
  variant="avatar"
  currentUrl="/avatar.jpg"
  onFileChange={(file) => console.log(file)}
/>
```
