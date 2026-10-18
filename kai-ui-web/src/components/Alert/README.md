# Alert

Mensaje de estado a ancho completo. Variantes: `success`, `info`, `warning`, `error`.

## Import

```ts
import { Alert } from "@felipechandiadev/ui";
```

## Props

- `variant`: tono del mensaje. Default `info`.
- `children`: contenido.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<Alert variant="error">No se pudo guardar.</Alert>
```
