# Demo «App real»: panel, CRM y mapa

Estado: **implementado** (parche 08). Código: `demo/src/sections/real_app/`
(sección, layout y páginas), `demo/src/crm.rs` (datos en memoria),
`demo/src/crm/editor.rs` (formulario de cliente) y `demo/src/crm/cards.rs`
(ficha). Pruebas: `demo/tests/real_app.rs` y `crm::tests`.

## 1. Para qué

Las demás secciones de la demo muestran **un** componente cada una. «App
real» arma una aplicación chica de punta a punta, como la escribiría
quien usa Kai: rutas con subpáginas, un layout común, estado compartido,
carga con latencia, validación, autocompletado y mapa. Sólo usa
componentes que ya existen (más la capa flotante del parche 06 y el
autocompletar del 07); no agrega API a la biblioteca.

## 2. Rutas

kai-build genera las rutas a partir de las carpetas (ver
[26](26-router-y-kai-build.md)); el nombre de la carpeta, en
`snake_case`, da la ruta en `kebab-case`:

| Carpeta | Ruta | Qué muestra |
|---|---|---|
| `real_app/page.rs` | `/real-app` | Panel: cifras, ventas por mes, actividad reciente |
| `real_app/clients/page.rs` | `/real-app/clients` | Lista de clientes (`Table`) + ficha (`MasterDetailPage`) |
| `real_app/clients/new/page.rs` | `/real-app/clients/new` | Formulario de cliente nuevo |
| `real_app/clients/[id]/page.rs` | `/real-app/clients/:id` | Ficha de un cliente, su actividad y «Editar» |
| `real_app/map/page.rs` | `/real-app/map` | Clientes y sectores DSP en el mapa local |
| `real_app/layout.rs` | todas | Barra `segmented` Panel · Clientes · Mapa |

`new` es una carpeta fija y gana sobre `[id]`. La sección aparece en el
grupo «Demo» de la barra lateral, debajo de Inicio (`order: 5`).

El layout elige la opción de la barra según `RouteInfo::path` y navega con
`cx.navigate`. Como toda página que no es la raíz de su sección, las
subpáginas se crean de nuevo cada vez que se abren: la lista vuelve a
pedir los clientes y el mapa lee qué cliente mostrar.

## 3. Datos: `crm.rs`

- `State::crm` (`demo/src/state.rs`) guarda el CRM en memoria: ocho
  clientes de Parral repartidos en los cinco sectores de
  `assets/maps/dsp-areas.geojson`, con RUT válidos, y la actividad
  reciente.
- La «fecha de hoy» de la demo es fija (`crm::today()`, 02/11/2026) para
  que pantallas y pruebas no cambien con el calendario.
- La ubicación de cada cliente es un punto dentro de su sector (alrededor
  del centroide, separado de los demás).
- `Crm::save` hace de servidor: rechaza un RUT que ya es de otro cliente,
  crea o actualiza, anota la actividad y recuerda `last_saved` (la lista
  lo selecciona al volver).
- `Crm::focus` es el cliente que el mapa debe mostrar al abrirse («Ver en
  mapa» lo fija; ir al mapa desde la barra lo borra).

### Latencia simulada

Todo lo que «va al servidor» usa `Effects::perform` con una espera de
`crm::LATENCY` (300 ms) en el pool de tareas:

| Dónde | Qué espera | Mientras tanto |
|---|---|---|
| Panel (la primera vez) | la carga | `loading_placeholder` |
| Lista de clientes | una copia de los clientes | `loading_placeholder` en la lista |
| Guardar | la respuesta de `Crm::save` | botón «Guardando…» deshabilitado |
| «Referido por» | la búsqueda de clientes | estado «Buscando…» del autocompletar |

## 4. Panel («Resumen»)

Layout Glass de una app: tres paneles sólidos (`panes`, `pane`) sobre el
fondo «Aurora 2» estático, con la sidebar propia de la app
(`Layout::sidebar` → `app_sidebar`: Resumen, Clientes, Mapa, Oportunidades
y Laboratorio aún deshabilitados, y «Volver a la demo» → `/home`) en lugar
de las secciones de la demo. Las cifras y el gráfico de ventas de la
versión anterior salieron del panel.

### Patrones de diseño

Base del laboratorio de patrones; cada pieza ilustra uno:

- **Lista con chips de estado** (panel izquierdo): fila con iniciales,
  nombre, sector y un chip de color por etapa; pie con el conteo y acciones
  rápidas (buscar, filtros).
- **Maestro-detalle** (panel central): la fila elegida en la lista llena el
  detalle (datos de contacto, actividad reciente, «Editar» abre la ficha).
- **Barra de herramientas de lista** (pie de la lista, `list_toolbar`):
  info a la izquierda («8 clientes» / «N de M clientes»), acciones a la
  derecha (Descargar, Ordenar, Filtrar, Añadir) con punto si Ordenar o
  Filtrar están activos, y búsqueda al extremo derecho que se despliega
  animada sobre las acciones (foco al abrir; Escape o X cierran; filtra
  por nombre o sector en vivo). Si el cliente elegido queda fuera, el
  detalle muestra el primero visible. Overflow a un menú «más»: pendiente.
- **Confirmación destructiva** (iconos Editar/Eliminar del detalle):
  la papelera abre un `Dialog` con título, qué se pierde, «Eliminar» en
  rojo (no es el botón por defecto: Return no destruye) y «Cancelar»; al
  confirmar se borra el cliente en memoria y se elige otro (o «No quedan
  clientes»).
- **Inspector** (panel derecho): propiedades editables del elemento elegido
  (estado con selector, responsable, etiquetas como chips, «Añadir
  etiqueta»).
- **Navegación por ruta en la barra** (`section_tabs`, píldora centrada) y
  **sidebar flotante de cristal** (`app_sidebar`): el cristal solo en
  navegación y controles; el contenido es sólido.
- **Barra de estado** (`status_bar`): delgada, cuadrada, a todo el ancho:
  sincronización y conteo.

## 5. CRM

**Lista.** `Table` con búsqueda global (`search_field` →
`TableView::query`), orden por columna, la próxima visita como columna de
fecha y el estado como `badge`. Sin coincidencias muestra
`EmptyState::filter` con «Limpiar búsqueda». A la derecha, la ficha del
cliente elegido con «Abrir ficha» (→ `/real-app/clients/<id>`) y «Ver en
mapa».

**Ficha (`[id]`).** `Params = u32`. Datos, actividad del cliente, a quién
refirió y un botón a quien lo refirió. Un id que no existe muestra
`EmptyState::resource`. «Editar» cambia la página al mismo formulario de
`new`, ya lleno; al guardar vuelve a la ficha con un toast.

**Formulario (`ClientEditor`).** Un `FormState` con:

| Campo | Componente | Reglas |
|---|---|---|
| Nombre | `text_field` | obligatorio, mínimo 3 |
| RUT | `text_field` + `Format::Rut` | obligatorio, RUT válido; el servidor revisa que no se repita |
| Correo | `text_field` + `Format::Email` | obligatorio, correo |
| Teléfono | `text_field` + `Format::TelefonoCl` | teléfono chileno si se escribe |
| Comuna | `FormState::autocomplete` (lista de comunas del Maule) | obligatorio: vale lo elegido, no lo escrito (`write_to`) |
| Sector DSP | `select` | uno de los cinco |
| Referido por | autocompletar con búsqueda (`AutocompleteState::search`) | opcional; un texto sin elegir de la lista no se envía |
| Próxima visita | `FormState::date_field` + `.calendar(…)` | entre hoy y un año más; el calendario marca hoy y deshabilita el pasado |
| Hora | `text_field` + `Format::HORA` | entre 08:00 y 19:00 |
| Notas | `text_field` | máximo 200 |

Al guardar con errores, `FormState::submit` los muestra todos y no se
envía nada. Si pasa, el borrador viaja al «servidor»; un RUT repetido
vuelve como `alert` de error arriba del formulario. Si se guarda, toast
«<nombre> guardado» y vuelta a la lista, con el cliente nuevo elegido.

**Sin tocar `date.rs`.** El `DateField` se usa tal cual existe. Sin el
parche 06b su calendario es un panel `absolute z-50` dentro del campo
(ver [32](32-capa-flotante.md) §1.1), que un campo posterior puede tapar:
por eso «Próxima visita» y «Hora» van en la última fila del formulario.
Con 06b el calendario va en la capa flotante y no depende del orden.

## 6. Mapa

El mapa usa el mismo extracto local que `/map`
(`assets/maps/parral.pmtiles`, estilo Protomaps) y los sectores de
`sections::map::page::SECTORS`. Cada cliente es un `marker` con un pin que
se puede pulsar (rol botón, nombre del cliente); a la izquierda, la lista
de clientes.

- Elegir un cliente (en la lista o su pin) agranda y colorea su pin,
  marca su sector (`MapLayer::selected`), lo encuadra (`fit_bounds`) y la
  línea de estado dice «Cliente · sector · comuna».
- Un clic en un sector lo marca, lo encuadra y cuenta sus clientes.
- «Abrir ficha» lleva a la ficha del cliente elegido.

## 7. Pruebas

`demo/tests/real_app.rs` (sin GPU):

- el panel espera la carga y luego muestra cifras, barras y actividad;
- la barra, la lista, la ficha, el referidor, «Volver a la lista», la
  búsqueda sin resultados y un id inexistente;
- guardar vacío muestra los errores (y nada se envía); RUT, fecha y hora
  fuera de rango; referido escrito sin elegir;
- el autocompletar de comuna y el de referido (con su espera), la fecha
  desde el calendario, «Guardando…», el toast y la lista con el cliente
  nuevo elegido; el panel lo cuenta;
- el servidor rechaza un RUT repetido; editar guarda en la ficha;
- elegir en el mapa envía `Msg::Select`, marca el cliente y su pin, y la
  línea de estado lo dice; «Ver en mapa» abre el mapa en el cliente.

`demo/tests/snapshots.rs` agrega capturas (`demo-real-app*`) para la
revisión visual con GPU.

No hay comandos de página nuevos: `demo/mcp-schema.json` no cambia.
