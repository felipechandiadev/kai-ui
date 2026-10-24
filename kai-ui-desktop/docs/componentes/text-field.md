# TextField

Campo de texto de una línea con etiqueta, adornos al inicio y al final,
contraseña, ayuda y error. Equivale al `TextField` de `kai-ui-web`.

Código: `src/components.rs` (`TextField`), iconos en
`src/components/field_icons.rs`.

## Uso

```rust
use kai::components::{TextField, FieldKind};

TextField::new("Correo", &self.correo)
    .field_icon(FieldKind::Email)
    .required(true)
    .helper_text(self.error_correo.as_deref().unwrap_or("Te escribiremos aquí"))
    .error(self.error_correo.is_some())
    .on_input(Msg::Correo)
    .on_submit(Msg::Enviar)
    .test_id("form.correo")
    .build()

TextField::new("Contraseña", &self.clave)
    .password(self.ver_clave, Msg::AlternarClave)
    .on_input(Msg::Clave)
    .build()

TextField::new("Monto", &self.monto).start_symbol("$").end_symbol("CLP").build()
```

La app guarda el valor: cada edición llega por `on_input` y se vuelve a pasar
en `new(label, value)`.

## API

| Método | Qué hace |
|---|---|
| `new(label, value)` | Crea el campo. |
| `placeholder(text)` | Texto con el campo vacío y sin foco (por defecto, la etiqueta). |
| `placeholder_color(token)` | Color del placeholder (`primary`, `error/60`…). |
| `variant(FieldVariant)` | `Contrast` (sobre barra primaria) o `Autocomplete` (sin borde, dentro de un combobox). |
| `density(FieldDensity)` | `Default` (etiqueta flotante sobre el borde) o `Compact` (etiqueta arriba). |
| `label_layout(LabelLayout::Inline)` | Solo compacto: la etiqueta dentro del borde, antes del texto. |
| `bezel(FieldBezel)` | Forma AppKit: `Square` o `Rounded`. Kai lo ignora. |
| `start_symbol` / `end_symbol` | Texto corto: `$`, `%`. |
| `start_icon` / `end_icon` | Icono Lucide. |
| `start_adornment` / `end_adornment` | Cualquier `Element`. |
| `inline_leading(el)` | Solo layout inline: elemento antes de la etiqueta. |
| `field_icon(FieldKind)` | Aplica el icono canónico del tipo de campo como `start_icon`. |
| `password(revealed, on_toggle)` | Viñetas y botón de ojo que no quita el foco. |
| `password_command(revealed, Comando)` | Igual, pero el ojo ejecuta un comando. |
| `required(bool)` | Asterisco rojo en etiqueta y placeholder. |
| `helper_text(text)` | Ayuda bajo el campo; también su descripción accesible. |
| `error(bool)` | Borde y ayuda en color de peligro. Úsalo con `helper_text`. |
| `read_only`, `disabled`, `select_on_focus`, `always_show_label` | — |
| `on_input(f)`, `on_submit(msg)` | Edición y Return. |
| `test_id(id)`, `build()` | — |

### Iconos por tipo (`FieldKind`)

`field_icon_for(kind)` devuelve el icono canónico, o `None` cuando conviene un
símbolo de texto. La tabla estable está en `FIELD_ICON_MAP`:

| `FieldKind` | Icono |
|---|---|
| `Name`, `User` | `User` |
| `Email` | `Mail` |
| `Password` | `Lock` |
| `Search` | `Search` |
| `Phone` | `Phone` |
| `Date` | `CalendarDays` |
| `Url`, `Amount` | sin icono: usa un símbolo (`https://`, `$`) |

Detalle y criterios: `docs/dev/19-iconografia-campos.md`.

## Comportamiento

- Un clic en cualquier parte del campo (etiqueta, iconos, relleno) enfoca el
  texto.
- Edición por palabra y línea (⌥←, ⌘←), deshacer/rehacer (⌘Z / ⇧⌘Z), IME,
  portapapeles.
- El contorno es una fila `[inicio][texto][fin][ojo]`: los adornos ocupan su
  propio espacio en vez de superponerse.

## Accesibilidad

Rol `TextInput`; nombre = etiqueta; descripción = ayuda o error. `required`
se expone. El ojo de contraseña tiene nombre «Mostrar/Ocultar contraseña».

## Partes (con `test_id("x")`)

`x.input`, `x.label`, `x.floating`, `x.shell`, `x.placeholder`, `x.helper`,
`x.start`, `x.end`, `x.toggle`.

```rust
app.type_text(By::test_id("form.correo.input"), "ana@ejemplo.cl");
```

## Temas

- **Kai**: etiqueta flotante sobre el borde (densidad por defecto).
- **AppKit**: etiqueta según `FieldLabels` del tema (`Near`, `AppKit`,
  `Top`, `Tab`; variable `KAI_LABEL` en la demo).

## Relacionados

[TextArea](text-area.md) · [SearchField](search-field.md) ·
[ColorField](color-field.md)
