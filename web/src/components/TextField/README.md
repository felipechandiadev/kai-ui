# TextField

Campo de texto con label. También exporta `resolveTouchInputMode` para decidir el teclado en pantallas táctiles.

## Import

```ts
import { TextField } from "@felipechandiadev/ui";
```

## Props

- `label`, `value` y `onChange`. Obligatorios.
- `type`: tipo del input. En puntero grueso, los numéricos pueden pasar a teclado de texto.
- `variante`: `normal`, `contrast` o `autocomplete`.
- `startSymbol` / `endSymbol`, o `startAdornment` / `endAdornment`.
- `rows`: si es mayor que 1, el campo es un textarea.

## Servidor

Client component. Next lo incluye en el HTML y lo hidrata.

## Ejemplo mínimo

```tsx
<TextField label="Nombre" value={name} onChange={(e) => setName(e.target.value)} />
```
