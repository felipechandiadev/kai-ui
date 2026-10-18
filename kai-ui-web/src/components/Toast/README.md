# Toast

Aviso breve que se apila en un viewport. `Toast` es un mensaje. `ToastViewport` es el contenedor.

## Import

```ts
import { Toast, ToastViewport } from "@felipechandiadev/ui";
```

## Props

- `Toast`: `message`, `variant` (`success`, `info`, `warning`, `error`) y `onClose`.
- `exiting`: se aplica mientras el toast sale de la pila.
- `ToastViewport`: recibe los toasts como `children`. Si no hay hijos, no renderiza nada.

## Servidor

Client components. Un toast montado entra en el HTML. La salida animada corre en el cliente.

## Ejemplo mínimo

```tsx
<ToastViewport>
  <Toast variant="success" message="Guardado" onClose={() => {}} />
</ToastViewport>
```
