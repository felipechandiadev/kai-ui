# 15 — Caso especial: tablas con muchos registros y datos remotos

Las tablas son un componente especial: son el único lugar donde la cantidad de datos no la decide la pantalla, sino el negocio. Un formulario tiene diez campos; una tabla de clientes puede tener cien mil filas. Esta guía explica qué hace `Table` con miles de filas, dónde está el límite y qué estrategia seguir cuando los datos vienen de una API. La referencia del componente está en [05](05-widgets-y-tema.md#table-y-virtual_list).

## 1. Miles de filas en memoria: funciona tal cual

`Table` va sobre `virtual_list`, que usa `Element::virtual_rows`: en cada frame la `Session` construye solo las filas que se ven, más `OVERSCAN` (6) por arriba y por abajo (`src/virtual_rows.rs`). Las demás se reemplazan por dos espaciadores con la altura de las filas que faltan, así la barra de scroll se ve como si estuvieran todas. Antes del primer layout, cuando la altura de la lista todavía no se conoce, construye `INITIAL_ROWS` (30).

Consecuencias:

- **El costo de `view`, layout y pintura depende del alto de la tabla, no de la cantidad de filas.** Da lo mismo tener 1 000, 10 000 o 100 000: se construyen unas 40.
- **Teclado y selección siguen funcionando.** La fila seleccionada y la fila a la que va el foco (↑/↓, Inicio/Fin, RePág/AvPág) se construyen siempre, aunque estén fuera de la vista, y la lista hace scroll hasta ellas.
- **Accesibilidad.** AccessKit recibe el total (`row_count`) y el índice real de cada fila (`row_index`): VoiceOver dice «fila 5 431 de 10 000» aunque el árbol tenga unas 40 filas.

La demo lo prueba con 10 000 clientes (sección Tabla, `CLIENTES` en `src/demo.rs`) y el test `the_toolbar_search_filters_and_the_headers_sort_the_table`.

### Límites

1. **Ordenar y filtrar es trabajo de la app.** `Table` solo pide `cell(fila, columna)` en el orden de pantalla. En la demo, cada búsqueda o cambio de orden recorre los 10 000 clientes, filtra y ordena (`clientes()` en `demo.rs`). Con 10 000 es instantáneo; con cientos de miles o millones conviene hacerlo fuera del hilo de la UI (`Effects::perform` con `.keyed(..)`, así una búsqueda nueva reemplaza la anterior) o pedirlo al servidor ya ordenado.
2. **Todas las filas miden lo mismo** (`row_height`). La posición de una fila es su índice por la altura; no hay filas de altura variable (texto que salta de línea).
3. **La tabla no pagina.** Llama a `cell(i, col)` por cada fila visible y espera la respuesta en el momento: los datos tienen que estar en memoria o ser instantáneos de consultar por índice.
4. **Una sola fila seleccionada.** No hay selección múltiple (⌘-clic, ⇧-clic).

## 2. Datos remotos: 100 000 registros en un servidor

**Nunca se bajan todos.** El servidor filtra, ordena y pagina; la app guarda solo el total y las páginas cercanas a lo que se ve.

### Estrategia

1. **El servidor manda.** La API recibe la búsqueda, el orden y la ventana (`?q=temuco&orden=monto&dir=desc&offset=4200&limit=100`) y devuelve esa página y el total (`{ "total": 100000, "filas": [...] }`).
2. **La tabla usa el total real.** `Table::new(columnas, total, cell)`: el scroll mide lo que corresponde, VoiceOver dice «fila 5 431 de 100 000» y la virtualización construye solo lo visible.
3. **Caché de páginas en el modelo.** Por ejemplo `HashMap<usize, Vec<Fila>>` con páginas de 100 filas y un tope (unas 20 páginas); se descartan las más lejanas a la vista.
4. **Una fila que no llegó se dibuja como pendiente** («…» en las celdas). `cell(i, col)` nunca hace una petición: solo lee el caché.
5. **Cuando se ve una página que falta, se pide en segundo plano** con `Effects::perform(..).keyed(format!("pagina.{n}"))`. La clave evita pedir dos veces la misma página. La respuesta llega como `Msg`, se guarda en el caché y la tabla se redibuja.
6. **Buscar u ordenar empieza de cero:**
   - Pausa al tipear (debounce) de unos 300 ms con `Effects::after(..).keyed("buscar")`: cada tecla reinicia la espera.
   - Luego se vacía el caché, se cancelan las peticiones viejas (`Effects::cancel`) y se pide la página 0.
   - Cada consulta lleva un número de generación; una respuesta de una búsqueda anterior se descarta.
7. **La selección se guarda por id del registro, no por índice.** El índice cambia al ordenar o filtrar (la demo ya lo hace así con `cliente`).

### Esbozo del modelo

```rust
const PAGINA: usize = 100;

struct Clientes {
    total: usize,
    paginas: HashMap<usize, Vec<Cliente>>,
    generacion: u64,
    query: String,
    sort: (usize, SortDirection),
    seleccionado: Option<ClienteId>,
}

enum Msg {
    Query(String),
    Buscar,
    Pagina { generacion: u64, numero: usize, total: usize, filas: Vec<Cliente> },
    FilasVisibles(Range<usize>),
    Error(String),
}

fn update(&mut self, msg: Msg) -> Effects<Msg> {
    match msg {
        Msg::Query(q) => {
            self.query = q;
            Effects::after(Duration::from_millis(300), Msg::Buscar).keyed("buscar")
        }
        Msg::Buscar => {
            self.generacion += 1;
            self.paginas.clear();
            self.pedir(0)
        }
        Msg::FilasVisibles(rango) => {
            let faltan = (rango.start / PAGINA..=rango.end / PAGINA)
                .filter(|n| !self.paginas.contains_key(n))
                .map(|n| self.pedir(n))
                .collect::<Vec<_>>();
            Effects::batch(faltan)
        }
        Msg::Pagina { generacion, numero, total, filas } if generacion == self.generacion => {
            self.total = total;
            self.paginas.insert(numero, filas);
            Effects::none()
        }
        _ => Effects::none(),
    }
}

fn pedir(&self, numero: usize) -> Effects<Msg> {
    let (q, sort, generacion) = (self.query.clone(), self.sort, self.generacion);
    Effects::perform(
        move || api::clientes(&q, sort, numero * PAGINA, PAGINA),
        move |r| match r {
            Ok(p) => Msg::Pagina { generacion, numero, total: p.total, filas: p.filas },
            Err(e) => Msg::Error(e.to_string()),
        },
    )
    .keyed(format!("pagina.{numero}"))
}
```

`Msg::FilasVisibles` todavía no lo emite Kai (ver la sección 3).

### Qué hay en Kai y qué falta

**Ya existe:**

- `Effects::perform`: la petición corre en el pool de 4 hilos sin congelar la ventana.
- `.keyed(..)` y `Effects::cancel`: no duplicar peticiones y descartar las viejas.
- `Effects::after`: la pausa al tipear.
- `ureq`, con la feature `net` (activa por defecto).
- `Table` acepta cualquier cantidad de filas y una función `cell` que solo lee el modelo.

**Falta: que la tabla avise qué filas muestra.** La `Session` lo sabe (`virtual_rows::materialize` devuelve el rango construido de cada lista), pero no hay un callback que lo convierta en `Msg`. La función que construye cada fila se llama mientras se arma el árbol y no puede mandar mensajes. El cambio es chico: algo como `Table::on_rows_needed(|rango| Msg::FilasVisibles(rango))`, que la `Session` emite cuando cambia el rango construido, con una pausa corta para no pedir páginas mientras se arrastra la barra a toda velocidad. Está anotado como pendiente en el Paso 14 de [13](13-hoja-de-ruta-plataforma.md).

### Mientras tanto: paginación clásica

Sin tocar el framework funciona la paginación por botones: una página de 100 filas con «Anterior / Siguiente» y «Página 12 de 1 000»; cada cambio pide una página con `Effects::perform`. Es menos fluida que el scroll continuo, pero en una app de gestión es aceptable.

## 3. Del lado del servidor

- **Offset** (`OFFSET 50000 LIMIT 100`): permite saltar a cualquier punto arrastrando la barra. Con 100 000 filas e índices adecuados anda bien en Postgres.
- **Cursor o keyset** (`WHERE (monto, id) > ($1, $2)`): más rápido y estable si los datos cambian mientras se navega, pero no permite saltar al medio. Encaja con «cargar más», no con una barra de scroll real.
- **El total** puede ser caro (`COUNT(*)` con filtros): usar un conteo aproximado o pedirlo una vez por búsqueda, no por página.
- **Datos que cambian mientras se mira** (otro usuario edita un registro): hace falta una suscripción (websockets) que invalide páginas del caché. Kai todavía no tiene suscripciones (Paso 15 de [13](13-hoja-de-ruta-plataforma.md)).

## Resumen

| Escenario | Qué hacer |
|---|---|
| Hasta decenas de miles de filas en memoria | `Table` tal cual; filtrar y ordenar en `update`. |
| Cientos de miles en memoria | Igual, pero filtrar y ordenar con `Effects::perform(..).keyed(..)`. |
| Datos en un servidor, hoy | Paginación por botones con `Effects::perform`. |
| Datos en un servidor, con `on_rows_needed` | Total real en la tabla, caché de páginas, peticiones por ventana visible, debounce y generación. |
