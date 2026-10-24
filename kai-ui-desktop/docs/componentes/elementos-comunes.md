# Elementos y modificadores comunes

Todo componente de Kai devuelve (o construye con `.build()`) un
`Element<Msg>`. Los modificadores de esta página sirven sobre cualquiera, y
los componentes los usan por dentro. Están en `src/element.rs`.

## Dos formas de componente

- **Función**: `button("Guardar", ButtonVariant::Primary)` devuelve un
  `Element` listo; se encadenan modificadores (`.on_click(..)`).
- **Builder**: `TextField::new("Nombre", &valor) … .build()` cuando hay
  muchas opciones. Hasta `.build()` no es un `Element`.

## Estilo

| Método | Qué hace |
|---|---|
| `.class("flex gap-2 …")` | Clases tipo Tailwind (`tw.rs`). Se pueden llamar varias veces; se suman. |
| `themed(kai, appkit)` | Una cadena de clases por tema: prefija `kai:` y `appkit:`. |
| Variantes | `hover:`, `active:`, `focus-visible:`, `keyboard-focus:`, `selected:`, `disabled:`, `busy:`, `window-inactive:`, `motion-reduce:`, `contrast-more:`, `transparency-reduce:`, `glass:`, `dark:`, y los breakpoints de ventana `sm:` (640), `md:` (768), `lg:` (1024), `xl:` (1280). |

Una clase desconocida no rompe nada, pero `Session::unknown_classes()` la
lista; los tests de la demo exigen que esté vacía.

## Mensajes y comandos

| Método | Qué hace |
|---|---|
| `.on_click(msg)` | Envía `msg` al hacer clic, o con Espacio/Return si tiene foco. |
| `.on_command(Accion)` | Ejecuta un comando del registro (`kai::commands`). Mientras el comando no está disponible el control queda deshabilitado y su motivo es la descripción accesible. Su `test_id` por defecto es el id del comando y, sin texto, su nombre es el título del comando. |
| `.on_input(f)` | Campos: cada edición. |
| `.on_submit(msg)` | Campos: Return (⌘Return en `TextArea`). |
| `.on_select(f)` | Elección de una lista (`options`) o categoría de un gráfico, por índice. |
| `.on_drop_files(f)` | Archivos soltados desde Finder. |

## Identidad para tests y accesibilidad

| Método | Qué hace |
|---|---|
| `.test_id("seccion.cosa")` | Id estable para `TestApp` (`By::test_id`). |
| `.part("name")` / `.parts_root()` | Partes internas de un componente: con `test_id("x")`, la parte queda `x.name`. |
| `.role(Role::…)` | Rol de AccessKit. |
| `.label("…")` | Nombre accesible. |
| `.description("…")` | Descripción accesible. |
| `.tooltip("…")` | Globo al pasar el puntero. |
| `.key("…")` | Identidad estable entre reconstrucciones (listas). |

## Estado

`.disabled(bool)`, `.pressed(bool)` (toggle, activa `selected:`),
`.busy(bool)`, `.required(bool)`, `.read_only(bool)`,
`.control_size(ControlSize)` (Mini, Small, Regular, Large, ExtraLarge; los
hijos lo heredan).

## Foco y teclado

| Método | Qué hace |
|---|---|
| `.focus_group()` | Los controles de dentro son una sola parada de Tab; las flechas se mueven entre ellos. |
| `.selecting_focus_group()` | Igual, y mover con flechas también elige (barra lateral, listas). |
| `.tabs_focus_group()` | Para pestañas: ← y → eligen. |
| `.key_equivalent(..)` | Atajo que activa el control sin foco. |
| `.default_button()` / `.cancel_button()` | Return / Escape y ⌘. lo activan. |
| `.modal()` | El foco queda atrapado dentro hasta que la app deja de mostrarlo. |

## Listas desplegables y menús contextuales

| Método | Qué hace |
|---|---|
| `.options(items, selected)` | Convierte el elemento en un botón desplegable; la sesión muestra y maneja la lista (flechas, Return, Escape, clic fuera). Elegir envía `on_select(índice)`. |
| `.check_options(items, checked)` | Lista de casillas: la lista queda abierta para marcar varias. |
| `.disabled_options(vec)` | Ítems que no se pueden elegir ahora. |
| `.context_menu()` | La lista se abre con **clic derecho** (o Ctrl+clic) donde está el puntero, o con la acción «mostrar menú» de VoiceOver, en vez de al activarlo. |

Ejemplo de menú contextual sobre una fila:

```rust
div()
    .child(text(&cliente.nombre))
    .options(vec!["Abrir".into(), "Duplicar".into(), "Eliminar".into()], None)
    .on_select(move |i| Msg::MenuCliente(id, i))
    .context_menu()
```

Hoy el menú es una lista de textos (con casillas y deshabilitados); no tiene
separadores, iconos, atajos ni submenús. La tabla lo usa en su encabezado
para ocultar columnas ([`column_menu`](table.md)).
