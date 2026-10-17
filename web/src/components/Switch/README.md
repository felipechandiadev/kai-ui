# Switch

Interruptor booleano, con una etiqueta o con par off/on.

## Import

```ts
import { Switch } from "@felipechandiadev/ui";
```

## Props

- `checked` y `onChange`.
- `label` y `labelPosition` (`left` o `right`) para una sola etiqueta.
- `optionLabels`: `{ off, on }` muestra las dos y resalta la activa.
- `density`: `default` o `compact`.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<Switch checked={on} onChange={setOn} label="Activo" />
```
