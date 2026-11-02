# Autocomplete

Campo de texto que ofrece opciones mientras se escribe (combobox), desde una
lista en memoria o desde una búsqueda fuera del hilo de la UI, en una lista
que flota bajo el campo ([capa flotante](../dev/32-capa-flotante.md)). Es el
componente real detrás de lo que antes era solo la variante visual
`FieldVariant::Autocomplete`.

Código: `src/components/autocomplete.rs`.

## Uso

La app guarda un `AutocompleteState` por campo, le pasa sus mensajes en
`update` y lo dibuja con `autocomplete(...)`:

```rust
use kai::components::{autocomplete, AutocompleteItem, AutocompleteMsg, AutocompleteState};

#[derive(Clone)]
struct Cliente { nombre: String, rut: String }

impl AutocompleteItem for Cliente {
    fn label(&self) -> String { self.nombre.clone() }
    fn detail(&self) -> Option<String> { Some(self.rut.clone()) }
}

struct Pantalla {
    cliente: AutocompleteState<Cliente>,
    comuna: AutocompleteState<&'static str>,
}

enum Msg { Cliente(AutocompleteMsg<Cliente>), Comuna(AutocompleteMsg<&'static str>) }

// Asíncrono: la búsqueda corre en un hilo (`Effects::perform`) cuando el
// tipeo hace una pausa. Ok(opciones) o Err(mensaje para la lista).
let cliente = AutocompleteState::search("clientes.buscar", |q| api::buscar_clientes(q));
// Síncrono: filtra una lista sin distinguir mayúsculas ni tildes.
let comuna = AutocompleteState::list("despacho.comuna", COMUNAS.iter().copied())
    .min_chars(0);

fn update(&mut self, msg: Msg) -> Effects<Msg> {
    match msg {
        Msg::Cliente(m) => self.cliente.update(m).map(Msg::Cliente),
        Msg::Comuna(m) => self.comuna.update(m).map(Msg::Comuna),
    }
}

autocomplete("Cliente", &self.cliente, Msg::Cliente)
    .test_id("clientes.buscar")
    .placeholder("Nombre del cliente")
    .build()

// La opción elegida (None mientras el texto no sea una opción tomada).
self.cliente.selected()
```

Demo: `/fields`, tarjeta «Autocompletar» (`demo/src/sections/fields/page.rs`).

## API

### `AutocompleteState<T: AutocompleteItem>`

| Método | Qué hace |
|---|---|
| `list(key, items)` | Opciones en memoria; filtra por «contiene» con `fold` (sin mayúsculas ni tildes, la ñ sigue siendo ñ). `min_chars` 0. |
| `search(key, f)` | `f(&str) -> Result<Vec<T>, String>`, `Send + Sync`, corre fuera del hilo de la UI. `min_chars` 2. |
| `min_chars(n)` | Caracteres antes de buscar; con 0, ↓ en el campo vacío muestra la lista. |
| `debounce(d)` | Pausa del tipeo antes de buscar (`AUTOCOMPLETE_DEBOUNCE`, 250 ms). Solo búsqueda. |
| `limit(n)` | Opciones que muestra (`AUTOCOMPLETE_LIMIT`, 8). |
| `with_selected(item)` | Parte con una opción tomada (editar un registro). |
| `update(msg) -> Effects<AutocompleteMsg<T>>` | Aplica un mensaje; devuelve timers y tareas. |
| `query`, `options`, `status`, `is_open`, `active`, `selected`, `generation` | Lectura. |
| `write_to(&mut form, name)` | Escribe en el `FormState` la etiqueta elegida, o "" si no hay. |

`key` nombra los efectos (`kai.autocomplete.<key>.debounce` y
`kai.autocomplete.<key>.search`): debe ser único por ventana.

`AutocompleteItem` pide `label()` y, opcional, `detail()` (segunda línea
tenue). Viene implementado para `String` y `&'static str`.

`AutocompleteStatus`: `Idle`, `Loading`, `Ready`, `Failed(mensaje)`.

### `autocomplete(label, &state, wrap) -> Autocomplete<Msg>`

| Método | Qué hace |
|---|---|
| `test_id(id)` | Partes `id.input`, `id.shell`, `id.list`, `id.option.<i>`, `id.status`. |
| `placeholder`, `helper_text`, `required`, `disabled` | Como en `TextField`. |
| `field(\|f\| f...)` | Acceso al `TextField` de adentro (íconos, ancho, `on_submit`…). |
| `empty_text(t)` | Texto sin resultados (por defecto «Sin resultados»). |
| `loading_text(t)` | Texto mientras busca (por defecto «Buscando…»). |
| `build()` | El elemento. |

### Con `FormState`

```rust
// En update: después de pasar el mensaje, copiar la opción al formulario.
Msg::Ciudad(m) => {
    let fx = self.ciudad.update(m).map(Msg::Ciudad);
    self.ciudad.write_to(&mut self.form, "ciudad");
    fx
}

// En view: error y «tocado» vienen del formulario; salir del campo marca Blur.
self.form.autocomplete("ciudad", "Ciudad", &self.ciudad, Msg::Form, Msg::Ciudad).build()
```

El valor del formulario es la etiqueta elegida, no el texto tipeado: si se
escribe sin elegir, queda "" y `required` lo reclama. Vive en
`src/components/autocomplete.rs` (como `DateField`), sin tocar `src/forms/`.

## Teclado y ratón

El foco se queda siempre en el campo; la opción activa se anuncia con
`aria-activedescendant`.

| Tecla | Lista cerrada | Lista abierta |
|---|---|---|
| ↓ | Abre (si hay `min_chars`), activa la primera al siguiente ↓ | Siguiente opción (se detiene en la última) |
| ↑ | Lo normal del campo (inicio del texto) | Anterior; desde la primera, ninguna |
| Retorno | `on_submit` del campo (envía el formulario) | Con opción activa la toma; sin ella, `on_submit` |
| Escape | Con texto lo borra; vacío sigue la cadena (cierra el diálogo) | Cierra la lista (y cancela una búsqueda en curso) |
| Tab / ⇧Tab | Siguiente / anterior control | **Cierra sin elegir** y pasa al siguiente / anterior control |

Ratón: hover resalta, clic toma la opción (sin quitar el foco al campo); un
clic afuera cierra la lista.

**Por qué Tab no acepta:** tomar una opción con Tab hace que salir de un
campo cambie su valor sin una decisión explícita (y en un formulario es
la tecla con la que se recorre). Aceptar es Retorno o clic; Tab solo sale,
como `NSComboBox` y el patrón combobox de WAI-ARIA sin selección
automática. Las opciones son controles, pero Tab nunca entra a la lista.

## Estados

- **Inactivo**: menos de `min_chars`, sin lista.
- **Buscando**: tras tipear, durante la pausa y la búsqueda; la lista muestra
  «Buscando…» (`Role::Status`).
- **Listo**: opciones, o «Sin resultados» (`Role::Status`).
- **Error**: el `Err(mensaje)` de la búsqueda (`Role::Alert`).

Respuestas viejas: cada tecla sube una generación y vuelve a programar los
efectos con clave (`after(..).keyed`, `perform(..).keyed`), que descartan el
timer o la tarea pendiente. Si igual llega un `Loaded` de una generación
anterior, `update` lo ignora: nunca se ve la respuesta lenta de una consulta
ya reemplazada.

## Accesibilidad

- Campo: `Role::ComboBox` (AccessKit `EditableComboBox`), `expanded` según la
  lista, `active_descendant` a la opción activa.
- Lista: `Role::ListBox` con el nombre del campo.
- Opción: `Role::ListBoxOption`, `selected` la activa, nombre = etiqueta.

## Partes

`x.shell`, `x.input`, `x.list`, `x.option.0…`, `x.status`.

## Relacionados

[TextField](text-field.md) · [Select](select.md) (opción fija, sin texto) ·
[SearchField](search-field.md) · [Capa flotante](../dev/32-capa-flotante.md)
