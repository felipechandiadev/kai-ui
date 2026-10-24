# Tabs

Pestañas sobre el panel de la elegida, como `NSTabView` (pestañas ARIA con
activación automática).

Código: `src/components/tabs.rs`.

## Uso

```rust
use kai::components::Tabs;

Tabs::new(self.pestana, Msg::Pestana)
    .tab("General", self.general())
    .tab("Facturación", self.facturacion())
    .build()
```

## API

| Método | Qué hace |
|---|---|
| `Tabs::new(selected, on_select)` | Crea las pestañas. |
| `tab(label, content)` | Una pestaña y el contenido que muestra si está elegida. |
| `build()` | — |

Solo el panel elegido está en el árbol.

## Teclado y accesibilidad

La fila es una parada de Tab que entra por la pestaña elegida; ← y → eligen;
Tab sigue al panel. AccessKit ve un `TabList` de `Tab`s y un `TabPanel`
nombrado como la pestaña.

## Temas

Kai: pestañas sobre una pista tenue, la elegida en relieve. AppKit: centradas
sobre una caja con borde, como el sistema.

## Partes

`tab.<i>` por pestaña y `panel`.

## Relacionados

[Segmented](segmented.md) · [Document](document.md) (la ribbon usa pestañas propias)
