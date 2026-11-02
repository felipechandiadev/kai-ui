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

### Formato y validación (`kai::forms`)

```rust
use kai::forms::{Format, ValidateOn, Validators};

TextField::new("RUT", &self.rut)              // "12345678-5"
    .format(Format::Rut)                      // muestra "12.345.678-5"
    .validate(Validators::required().and(Validators::rut()))
    .submitted(self.enviado)
    .helper_text("Con dígito verificador")
    .on_input(Msg::Rut)                       // recibe "12345678-5"
    .on_blur(Msg::RutTocado)
    .build()

TextField::new("Cupo", &self.cupo)          // "1234567"
    .format(Format::Clp)                      // muestra "1.234.567"
    .start_symbol("$")
    .validate(Validators::clp_range(1_000, 10_000_000))
    .on_input(Msg::Cupo)
    .build()

TextField::new("Teléfono", &self.fono)       // "+56912345678"
    .format(Format::TelefonoCl)               // muestra "+56 9 1234 5678"
    .validate(Validators::phone_cl())
    .on_input(Msg::Fono)
    .build()

TextField::new("Correo", &self.correo)
    .format(Format::Email)
    .validate(Validators::email().message("Revisa el correo"))
    .validate_on(ValidateOn::Change)
    .on_input(Msg::Correo)
    .build()
```

- Con `format`, el campo muestra el texto formateado mientras se escribe o
  pega (⌘V con puntos, espacios o guiones) y **`on_input` recibe el valor
  normalizado** (`Format::Rut`: sin puntos, `K` mayúscula; `Format::Email`:
  sin espacios y dominio en minúsculas; `Format::Clp`: solo dígitos, sin
  ceros a la izquierda, hasta 12; `Format::TelefonoCl`: `+56` y hasta 9
  dígitos nacionales, E.164). El cursor queda tras el mismo
  carácter significativo aunque se agreguen o quiten puntos. La app puede
  guardar el valor normalizado: el campo lo vuelve a formatear.
- Con `validate`, si la regla falla y el error debe verse, el campo toma el
  estado `error(true)` y el mensaje reemplaza al `helper_text` (también como
  descripción accesible); corregir el valor lo limpia. Se valida el valor
  normalizado. Un valor vacío solo falla `Validators::required()`.
- `Format::Clp` no escribe el `$`: usa `start_symbol("$")`. Pegar
  `$ 1.234.567`, `1,234,567` o con espacios funciona.
- `Format::TelefonoCl` agrupa el móvil y Santiago como `+56 9 1234 5678` /
  `+56 2 2345 6789` y otras áreas como `+56 32 234 5678`. Se escribe el
  número nacional; el `56` se quita si el texto empieza con `+` o trae 11
  dígitos o más. `Validators::phone_cl()` pide 9 dígitos nacionales que
  empiecen entre 2 y 9 (no revisa la lista oficial de códigos de área).
- `validate_on` decide cuándo mostrar el error: `Change` (apenas hay texto,
  o tocado/enviado), `Blur` (por defecto: tocado o enviado) y `Submit` (solo
  enviado). La sesión recuerda qué campos perdieron el foco (por `test_id`,
  o por etiqueta sin él), así que `Blur` funciona sin estado en la app; lo
  olvida cuando la app cambia el valor por su cuenta (limpiar el
  formulario). `touched(true)` y `submitted(true)` lo fuerzan; `submitted`
  lo guarda la app al intentar enviar.
- Al enviar, valida el modelo con el mismo `Validator`
  (`validador.validate(&self.rut)`), y `validation_error()` da el error
  visible de un `TextField` ya armado.

### Formularios (`FormState`)

`kai::forms::FormState` guarda el modelo de todo el formulario: valores
normalizados, formato y reglas por campo, tocados, enviado y errores. Es
puro (sin renderer) y se prueba sin ventana.

Antes, un `String`, un `Msg` y un `TextField` por campo, y la validación
repetida al guardar:

```rust
Msg::Rut(rut) => self.rut = rut,
Msg::Save if Validators::rut().validate(&self.rut).is_err() => self.enviado = true,
// …
TextField::new("RUT", &self.rut).format(Format::Rut).validate(Validators::rut())
    .submitted(self.enviado).on_input(Msg::Rut).build()
```

Después, un `FormState` y un solo mensaje:

```rust
use kai::forms::{Field, Format, FormMsg, FormState, Validators};

let form = FormState::new()
    .field("correo", Field::new().format(Format::Email)
        .validator(Validators::required().and(Validators::email())))
    .field("rut", Field::new().format(Format::Rut).validator(Validators::rut()));

// update
Msg::Form(msg) => if let Some(Ok(valores)) = self.form.update(msg) {
    guardar(valores);
    self.form.reset();
}
// view: etiqueta, valor formateado, reglas, tocado/enviado, test_id = nombre
self.form.text_field("rut", "RUT", Msg::Form).helper_text("Opcional").build()
button("Guardar", ButtonVariant::Primary).on_click(Msg::Form(FormMsg::Submit))
```

- `set`, `value` (normalizado), `display` (formateado), `values()`,
  `error(nombre)` (visible según `ValidateOn`), `errors()` (todos, en orden
  de registro), `is_valid`, `is_dirty`, `touch`, `touch_all`, `reset`.
- `submit()` marca enviado y todo tocado: `Ok(valores)` o `Err(errores)`;
  `errores.first_field()` es el campo al que llevar el foco (la app aún no
  puede mover el foco por código).
- **Pendiente (i18n):** los mensajes de las reglas y los formatos están fijos en español es-CL; un catálogo de mensajes vendrá con la tarea de i18n ([13](../dev/13-hoja-de-ruta-plataforma.md)).
- `text_field` usa `remember_touched(false)`: el tocado lo lleva el
  `FormState` (por `FormMsg::Blur`), no la sesión.

#### Comandos para agentes

La página implementa `kai::forms::commands::HasForm` y los registra:

```rust
impl HasForm for Screen {
    const IDS: FormIds = kai::form_ids!("client");
    fn form(&self) -> &FormState { &self.form }
    fn form_msg(msg: FormMsg) -> Msg { Msg::Form(msg) }
}
// Page::commands
kai::forms::commands::register::<Self, A>(registry).expose();
```

| Id | Tipo | Entrada | Salida |
|---|---|---|---|
| `<p>.set_field` | acción | `{ "name": string, "value": string }` | errores actuales |
| `<p>.errors` | consulta | `{}` | `[{ "field": string, "message": string }]` |
| `<p>.values` | consulta | `{}` | `{ campo: valor normalizado }` |
| `<p>.submit` | acción | `{}` | errores del envío (`[]` = enviado) |
| `<p>.last_submit` | consulta | `{}` | errores del último envío o `null` |
| `<p>.reset` | acción destructiva | `{}` | — |

El prefijo es por formulario porque los ids son globales (la demo usa
`client.*`; `form.*` es del formulario de Inicio). Sin `.expose()` no se
publican a agentes.

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
| `on_input(f)`, `on_submit(msg)` | Edición y Return. Con `format`, `f` recibe el valor normalizado. |
| `on_focus(msg)`, `on_blur(msg)` | Al recibir o perder el foco (clic, Tab). |
| `format(Format)` | Formato al escribir o pegar: `Rut`, `Email`, `Clp`, `TelefonoCl`. |
| `validate(Validator)` | Reglas de `Validators` (`required`, `email`, `rut`, `phone_cl`, `clp_range`, `min_len`, `max_len`, `custom`); se acumulan. |
| `validate_on(ValidateOn)` | `Change`, `Blur` (por defecto) o `Submit`. |
| `remember_touched(bool)` | `false`: ignora los tocados que recuerda la sesión. |
| `touched(bool)`, `submitted(bool)` | Fuerzan el error (la sesión ya recuerda los campos tocados). |
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
