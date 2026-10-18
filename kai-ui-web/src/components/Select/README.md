# Select

Selector de una opción, con campo de texto y lista desplegable.

## Import

```ts
import { Select, type SelectOption } from "@felipechandiadev/ui";
```

## Props

- `options`: lista `{ id, label }`.
- `value`: id seleccionado, o `null`.
- `onChange`: recibe el id o `null`.
- `variant`: `default` o `minimal`.
- `density`: `default` o `compact`.
- `labelLayout`: `stack` o `inline`.
- `allowClear`: permite vaciar la selección.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata. La lista se abre en el cliente.

## Ejemplo mínimo

```tsx
<Select
  label="Ciudad"
  options={[{ id: "scl", label: "Santiago" }]}
  value={null}
  onChange={() => {}}
/>
```
