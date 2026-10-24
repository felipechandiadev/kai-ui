# Avatar

Foto de una persona: un círculo con el icono `User` y, encima, su imagen
recortada al círculo cuando carga.

Código: `src/components/media.rs` (`avatar`, `Avatar`).

## Uso

```rust
use kai::components::{avatar, Avatar, AvatarSize};

avatar("Ana Pérez", self.foto.clone())

Avatar::new("Ana Pérez")
    .image(ImageSource::Url("https://…/ana.jpg".into()))
    .size(AvatarSize::Xs)
    .build()
```

## API

| Método | Qué hace |
|---|---|
| `avatar(name, image)` | Tamaño medio. |
| `Avatar::new(name)` | `name` vacío: anónimo. |
| `image(source)` | Archivo, URL (feature `net`), bytes o píxeles. |
| `size(AvatarSize)` | `Xs` (1,5 rem, para filas), `Sm`, `Md`, `Lg` (4, 6 y 7 rem). |
| `test_id`, `build()` | — |

La imagen se carga fuera del hilo de UI; hasta entonces (o si falla) se ve el
icono.

## Accesibilidad

Rol `Image` con el nombre de la persona.

## Relacionados

[MediaSlot](media-slot.md) · [Table](table.md) (avatar en celdas)
