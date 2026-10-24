# MediaSlot

Campo de imagen de un formulario: la vista previa es el control. Sirve para
foto de perfil (círculo), portada 16:9 o logo (cuadrado, imagen completa).

Código: `src/components/media.rs`.

## Uso

```rust
use kai::components::{MediaSlot, MediaVariant, AvatarSize, validate_image_file};
use kai::runtime::Effects;

MediaSlot::new(MediaVariant::Avatar(AvatarSize::Lg))
    .current(self.foto.clone())
    .on_pick(Msg::ElegirFoto)
    .on_drop(Msg::FotoSoltada)
    .error(self.error_foto.clone())
    .test_id("perfil.foto")
    .build()

// En update:
Msg::ElegirFoto => return Effects::pick_file(pedido, Msg::FotoElegida),
Msg::FotoElegida(Some(ruta)) | Msg::FotoSoltada(ruta) => {
    match validate_image_file(&ruta, 5) {
        Ok(()) => self.foto = Some(ruta.into()),
        Err(e) => self.error_foto = Some(e),
    }
}
```

## API

| Método | Qué hace |
|---|---|
| `new(MediaVariant)` | `Avatar(AvatarSize)`, `Banner` (16:9) o `Logo` (cuadrado). |
| `current(Option<ImageSource>)` | Imagen mostrada. |
| `on_pick(msg)` | Clic, Espacio o Return sobre la vista previa. |
| `on_drop(f)` | Primer archivo soltado desde Finder. |
| `error(Option<String>)` | Alerta de error bajo el campo. |
| `disabled`, `test_id`, `build()` | — |
| `validate_image_file(path, max_mb)` | Extensión de imagen y tamaño máximo, con los mensajes del componente web. |

## Comportamiento

Bajo el puntero (y mientras se arrastra un archivo encima) toma el aspecto
*choice*: borde y tinte sobre la imagen, con una píldora que nombra la acción
(«Agregar imagen» o «Cambiar imagen»).

## Partes

`x.preview`, `x.tint`, `x.hint`, `x.error`.

## Relacionados

[Avatar](avatar.md) · `Effects::pick_file`
