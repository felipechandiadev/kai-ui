# AutoComplete

Campo de búsqueda con lista de opciones filtrable.

## Import

```ts
import { AutoComplete } from "@felipechandiadev/ui";
```

## Props

- `options`: lista `{ id, label }`.
- `value`: opción seleccionada, o `null`.
- `onChange`: recibe la opción o `null`.
- `density`: `default` o `compact` (misma altura que un `TextField` compacto).

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata. La lista desplegable se abre en el cliente.

## Ejemplo mínimo

```tsx
<AutoComplete
  label="Ciudad"
  options={[{ id: "scl", label: "Santiago" }]}
  value={null}
  onChange={() => {}}
/>
```
