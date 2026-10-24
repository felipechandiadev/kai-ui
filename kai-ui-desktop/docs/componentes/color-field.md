# ColorField

Campo para un color en hex CSS (`#1e73ae`, `#f80`), con una muestra del color
y una fila de colores sugeridos.

Código: `src/components/color.rs`.

## Uso

```rust
use kai::components::color_field;
use kai::Color;

color_field("Color del sector", &self.color_hex, Msg::ColorSector)
    .palette([
        ("Azul", Color::hex(0x1e73ae)),
        ("Verde", Color::hex(0x2e9e5b)),
        ("Rojo", Color::hex(0xd64545)),
    ])
    .allow_empty(true)
    .test_id("mapa.color")
    .build()

// Leer el color cuando haga falta:
let color = Color::parse_hex(&self.color_hex);
```

La app guarda el **texto**, no un `Color`: cada edición y cada elección pasa
por `on_input`, así un hex a medio escribir se queda en el campo.

## API

| Método | Qué hace |
|---|---|
| `color_field(label, value, on_input)` | Crea el campo. |
| `palette(colores)` | Colores sugeridos: pares `(nombre, Color)`. |
| `allow_empty(bool)` | Ofrece «Sin color», que vacía el campo. |
| `disabled`, `test_id`, `build()` | — |

Un texto que no es vacío ni un color muestra bajo el campo `INVALID_COLOR`
(«Escribe el color como #RGB o #RRGGBB»).

## Accesibilidad

La paleta es un grupo de radio: una parada de Tab y flechas para recorrer.
Cada color se lee por su nombre («Azul») y su hex como descripción.

## Partes

`x.hex` (el TextField; su texto es `x.hex.input`), `x.swatch`, `x.palette`,
`x.palette.<i>`, `x.clear`, `x.error`.

## Relacionados

[TextField](text-field.md) · [Map](map.md) (color de áreas en la demo)
