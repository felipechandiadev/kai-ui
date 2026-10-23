# 19 — Iconografía de campos

Mapa documentado nombre-de-campo → icono Lucide del catálogo Kai. Evita pares incorrectos (p. ej. lupa en «Correo»).

## Mapa

| Kind (`FieldKind`) | Uso | Icono |
| --- | --- | --- |
| `Name` / `User` | Nombre, usuario | `user` |
| `Email` | Correo | `mail` |
| `Password` | Contraseña / PIN | `lock` |
| `Search` | Búsqueda / filtro | `search` |
| `Phone` | Teléfono | `phone` |
| `Date` | Fecha (reserva) | `calendar-days` |
| `Url` | URL | — (adorno texto `https://`) |
| `Amount` | Monto | — (símbolo `$` / `%`) |

Fuente de verdad en código: `field_icon_for` y `FIELD_ICON_MAP` (`src/components/field_icons.rs`). Iconos `lock` y `phone` añadidos al catálogo Lucide.

## API

```rust
use kai::components::{FieldKind, TextField, field_icon_for};

TextField::new("Correo", &email)
    .field_icon(FieldKind::Email) // = .start_icon(Icon::Mail)
    .build();

assert_eq!(field_icon_for(FieldKind::Search), Some(Icon::Search));
assert_eq!(field_icon_for(FieldKind::Amount), None);
```

## Demo

- Formulario de **Inicio**: Nombre → `User`, Correo → `Mail`.
- **Campos** → card «Estados de TextField» y variantes usan el mapa.
