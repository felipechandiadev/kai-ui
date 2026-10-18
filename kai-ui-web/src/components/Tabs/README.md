# Tabs

Pestañas de navegación. Cada ítem es un link.

## Import

```ts
import { Tabs, type TabItem } from "@felipechandiadev/ui";
```

## Props

- `items`: `{ url, label }`.
- `activeTab`: url activa. Si no se pasa, se infiere del pathname.

Usa el `Link` de la librería, así que la página tiene que estar dentro de `NextNavProvider` o `MemoryNavProvider`.

## Servidor

Client component. Next lo incluye en el HTML. La pestaña activa depende del pathname del provider.

## Ejemplo mínimo

```tsx
<Tabs
  items={[
    { url: "/datos", label: "Datos" },
    { url: "/pagos", label: "Pagos" },
  ]}
  activeTab="/datos"
/>
```
