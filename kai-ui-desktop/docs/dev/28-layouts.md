# 28 — Layouts: marcos por carpeta

**Estado:** propuesta (28-oct-2026). L0 a L3 implementados (29-30 oct). Extiende
[26](26-router-y-kai-build.md) (router y `kai-build`).

**Norma:** igual que en 26: todo concepto de código va en inglés; solo el
texto que ve la persona usuaria puede ir en español.

## 1. Problema

La raíz que genera `kai-build` dibuja un único marco fijo: barra lateral a
la izquierda y la página a la derecha. No hay dónde poner una barra superior
(`toolbar`, que existe como componente) ni un marco propio de una sección.
La demo antigua (`src/demo.rs`) dibujaba su barra a mano; al migrar a
secciones (R5) se perdió, y el bundle `.app` salió sin ella.

Una `Page` solo aporta su `view`, `title`, `overlay`, tema y fondo. Dibujar la
barra dentro de cada `view` la repetiría en todas las páginas y la dejaría
dentro del área de la página, no a lo ancho de la ventana.

## 2. Objetivo

Como en Next.js, un `layout.rs` opcional en cada carpeta envuelve a las
páginas de esa carpeta y de las de abajo, y **se anida**:

```
my-app/
  src/
    layout.rs                 ← raíz: envuelve a todo el contenido
    sections/
      customers/
        layout.rs             ← envuelve /customers y /customers/:id
        page.rs
        [id]/
          layout.rs           ← envuelve solo /customers/:id (opcional)
          page.rs
      sales/
        page.rs               ← solo el layout raíz
```

- **Persiste mientras la ruta siga dentro de su carpeta.** Navegar de
  `/customers` a `/customers/42` no recrea el layout de `customers`: conserva
  su estado (un campo de búsqueda, un filtro, una pestaña). Al salir de la
  carpeta se descarta, igual que hoy las páginas.
- **Convención con salida:** todo se puede escribir a mano con los mismos
  traits (26 §1). Sin ningún `layout.rs`, la raíz generada es la de hoy.

## 3. Contrato

### 3.1 `outlet()`: dónde va el contenido

El obstáculo de tipos: la página es un `Element<PageMsg>` y el layout, un
`Element<LayoutMsg>`; no se puede meter uno dentro del otro. Next lo resuelve
con `children`; aquí, con un **hueco** que la raíz rellena después de mapear:

```rust
/// Where the layout's content goes. Exactly one per layout. A flexible
/// column, the same box the root gives a page today.
pub fn outlet<Msg>() -> Element<Msg>;

impl<Msg: 'static> Element<Msg> {
    /// Puts `content` inside the first `outlet()`, which stays as its container.
    pub fn fill_outlet(self, content: Element<Msg>) -> Element<Msg>;
}
```

La raíz hace, de dentro hacia fuera:

```rust
let mut content = page.view(&state).map(RootMsg::Page);
for layout in in_scope.rev() {                       // innermost first
    content = layout.view(..).map(RootMsg::Layout).fill_outlet(content);
}
```

Así el layout decide **dónde** va el contenido (arriba, abajo, entre dos
paneles), no solo que lo envuelve. Reutiliza `Element::map` (R1); su costo
medido (26 §2) aplica a la parte del layout, que es pequeña.

`kai-build` comprueba que cada `layout.rs` contenga una llamada a `outlet(`
y falla con el nombre del archivo si no (el mismo tipo de error que 26 §5.1).
En ejecución, un layout sin hueco o con dos cae en `debug_assert`.

### 3.2 `Layout`

```rust
pub trait Layout: Sized + 'static {
    type Msg: Clone + Debug + 'static;     // `Infallible` si no tiene mensajes
    type State: 'static;                    // el mismo estado compartido

    fn init(state: &Self::State) -> (Self, Effects<Self::Msg>);
    fn update(&mut self, msg: Self::Msg, cx: &mut Context<'_, Self::State>)
        -> Effects<Self::Msg>;
    /// Must contain exactly one `outlet()`.
    fn view(&self, state: &Self::State, route: &RouteInfo) -> Element<Self::Msg>;
}

/// What a layout may read of where the app is.
pub struct RouteInfo<'a> {
    pub path: &'a str,          // "/customers/42"
    pub title: &'a str,         // the page's `title()`
    pub can_back: bool,
    pub can_forward: bool,
    pub sidebar_collapsed: bool,
}
```

- Misma forma que `Page` a propósito: `init`, `update`, `view`, `Context`.
  Un layout puede navegar (`cx.navigate`), mostrar toasts y cambiar el estado
  compartido, igual que una página.
- Sus botones suelen disparar **comandos** con `Element::run_command("kai.back")`
  (también `kai.forward`, `kai.toggle_sidebar`), que ya existen; no necesitan
  `Msg` propio. No sirve `on_command`: ata el mensaje del elemento al de la app,
  y la vista de un layout tiene el suyo. Por eso `Msg = Infallible` es lo habitual.
- Convención de `kai-build`: cada `layout.rs` define `pub struct Frame`
  (implementa `kai::router::Layout`), igual que `Screen` para las páginas.

### 3.3 Qué envuelve el layout raíz

Decisión: el layout raíz (`src/layout.rs`) envuelve **la columna de
contenido**, a la derecha de la barra lateral. La barra lateral sigue a toda
la altura y la barra superior queda sobre el contenido, como Finder, Notas y
Mail. Esto deja intacta la barra lateral generada y su plegado a riel.

Una barra a todo el ancho de la ventana (sobre la barra lateral) es la otra
convención de macOS (barra de título unificada). Hecho con
`const WRAPS_SIDEBAR: bool = true` en el layout raíz: su `outlet()` recibe
la fila barra lateral + contenido y sus barras cruzan la ventana. La demo
lo usa: barra superior con `toolbar_areas` (inicio: plegado, atrás,
adelante, título; centro y final: `slot`s `top.center`/`top.trailing`) y
barra inferior `status_bar` (delgada y cuadrada) con `bottom.leading` y
`bottom.trailing`. «App real» (`sections/real_app/layout.rs`) pone en
`top.center` sus `section_tabs` (píldora Glass, elegida por ruta), su
campana en `top.trailing`, su nota abajo y el fondo «Aurora 2» estático.
Regla HIG: cristal solo en barras y controles; el contenido es sólido.

### 3.4 Accesibilidad

El hueco no declara ningún rol. Hoy tampoco lo declara la página: ningún
componente pone `Role::Main` (solo un árbol de prueba), así que falta el punto de
referencia «contenido principal» con o sin layouts; queda como pendiente de
accesibilidad, a resolver una vez en la raíz y no en cada layout anidado. La barra superior ya declara `Role::Toolbar`
(`toolbar()`), con ←/→ entre sus botones. El anunciador de cambio de vista
(`kai::router::announcer`) sigue siendo de la raíz.

## 4. Generación en `kai-build`

`scan` registra los `layout.rs` junto a los `page.rs` y, de cada carpeta, su
alcance (las rutas que cuelgan de ella). Para cada layout la raíz gana:

- una ranura `layout_<ruta>: Option<Frame>` y una variante
  `RootMsg::Layout<Ruta>(Msg)`;
- **entrada y salida:** tras cada navegación compara los layouts en alcance
  de la ruta anterior y de la nueva; descarta los que salen y hace `init` de
  los que entran, juntando sus efectos con los de la página (hoy `navigate`
  ya hace esto para páginas);
- en `view`, el plegado de §3.1 con los layouts en alcance, del más interno
  al más externo;
- un `update` que reenvía `RootMsg::Layout*` a su ranura (se ignora si el
  layout ya salió; falta comprobar que hoy las páginas se comporten igual).

`kai new layout [<sección>[/<carpeta>]]` crea el `layout.rs` con un esqueleto
que ya compila (sin argumento, el de toda la app, con una barra con el título;
los botones atrás/adelante llegan en L2). `kai doctor` valida la
convención con la misma función que `kai-build`.

## 5. Lo que cada página puede aportar a su marco

Un layout con barra superior pide, tarde o temprano, acciones propias de la
página ("Nuevo cliente", un buscador que filtra la tabla): la demo antigua
las tenía. Next lo resuelve con rutas paralelas o portales; aquí, con un
segundo hueco con nombre, del mismo mecanismo:

```rust
pub fn slot<Msg>(name: &'static str) -> Element<Msg>;   // en el layout

trait Page {
    /// What the page puts in the layout's slot `name`, if it asks.
    fn slot(&self, name: &str, state: &Self::State) -> Option<Element<Self::Msg>> { None }
}
```

Los nombres son texto por simplicidad; el layout dice `slot("actions")` y la
página responde a `"actions"`. Si no responde, el hueco queda vacío. No entra
en la primera entrega (§6, L3, ya hecho): sin esto, la barra lleva título, atrás,
adelante y plegado de la barra lateral, y nada propio de la página.

Un layout también responde a los huecos de los layouts que lo rodean, con
`Layout::slot(name, state, route)`: primero la página, luego los layouts
del más interno al externo. Así una sección llena la barra del raíz sin que
cada página lo repita (y con `route` elige su pestaña activa).

## 6. Plan

| Corte | Contenido | Verificación |
|---|---|---|
| L0 ✅ | `outlet()`, `Element::fill_outlet`, `Layout`, `RouteInfo`, con una raíz escrita a mano en un test (`src/router/frame.rs`). | Tests de `kai`; `bench_layout_fold`: unos 5,6 µs por layout con barra de 5 elementos, 2–13 % de construir la vista de la demo (release). |
| L1 ✅ | `kai-build`: escaneo, ranuras, entrada y salida, plegado. `kai new layout`. | Fixture `sections-app` con layout raíz, de sección y anidado; el estado del layout de `customers` sobrevive `/customers` → `/customers/1`. |
| L2 ✅ | La demo recupera su barra como `demo/src/layout.rs` (título, atrás/adelante, plegar barra lateral) y una sección con layout propio. | Tests de la demo; capturas de revisión; abrir el bundle. |
| L3 ✅ | `slot(name)`, `Element::fill_slots` y `Page::slot`; el buscador de Tabla vuelve a la barra. | Test de la demo: escribir en la barra filtra la tabla. |

Tras cada corte: `scripts/check.sh` completo y actualizar 26, `CLAUDE.md`,
el CHANGELOG y la página de `docs/componentes/` de `outlet`.

**Cambios respecto al diseño, al implementar L0:** `Layout::update` tiene una
implementación por defecto (no hace nada), para que un layout sin mensajes
solo escriba `init` y `view`; `Element::outlet_count()` sirve a `kai-build` y a
los tests; el hueco es un `div` con la clave reservada `kai.outlet`, así que
no hizo falta una variante nueva de `Kind`. **Para L2:** el catálogo de iconos
no tiene flecha o chevron a la izquierda (solo `ChevronRight`), y la barra
necesita «Atrás»; hay que añadirlo con `scripts/gen-icons.mjs`.

**Cambios respecto al diseño, al implementar L1:**

- `fill_outlet` ya no *reemplaza* el hueco: lo deja como contenedor del contenido
  y le quita la clave. Su clase (`flex-1 min-w-0 min-h-0 flex flex-col`) es la del
  envoltorio que la raíz da hoy a la página, así que una app sin layouts se ve
  igual y una con layouts no cambia el tamaño de sus páginas.
- **Un `layout.rs` dentro de una carpeta `[parámetro]`** se rechazó en L1 y se admite desde L3 (ver abajo).
- `kai-build` vigila ahora toda `src/` (antes solo `src/sections` y
  `src/state.rs`), para ver aparecer un `src/layout.rs` nuevo.
- `layout` pasa a ser nombre reservado: no puede haber una sección o carpeta
  llamada así (chocaría con el módulo del layout).
- `kai doctor` valida los layouts sin cambios: usa el mismo `scan`.
- Un mensaje para un layout que ya salió se ignora, igual que ya hacen las
  páginas (comprobado en el código generado: `None => Effects::none()`).
- Los layouts todavía no registran comandos propios (pregunta 3 de §8).

**Al implementar L2:** `run_command` ya existía (lo usa la paleta); solo se
amplió su documentación. `RouteInfo::can_back` y `can_forward` quedan sin uso en
la demo, porque los comandos se deshabilitan solos según su disponibilidad; se
mantienen para layouts que dibujen algo propio con ellos. Se añadió el icono
`chevron-left` al catálogo (`scripts/gen-icons.mjs`). La línea base de capturas
(`snapshots/baseline`, local) quedó vieja: toda captura de la demo lleva ahora la
barra; hay que regenerarla con `scripts/snapshot-baseline.sh` cuando se acepte.

## 7. Alternativas descartadas

- **Solo una barra opcional en la raíz generada.** Resuelve el síntoma, pero
  cada app que quiera otro marco (un panel derecho, un pie) vuelve a escribir
  la raíz a mano, y no hay marcos por sección.
- **Layouts sin estado propio (función pura).** Más simple, pero pierde lo que
  hace útil a un layout persistente: el texto de un buscador o un filtro no
  sobreviven a la navegación.
- **`children: Element<Msg>` como en Next.** No compila aquí: el contenido
  tiene otro `Msg` que el del layout. El hueco rellenado tras `map` lo
  resuelve sin cambiar los tipos de las páginas.

## 8. Preguntas abiertas

1. **¿Barra sobre la columna de contenido o sobre toda la ventana?** Propongo
   lo primero (§3.3); la segunda cambia el aspecto del bundle.
2. **¿`slot` por nombre de texto o por tipo?** Un tipo por hueco da
   comprobación en compilación, pero obliga a cada layout y a cada página a
   compartir ese tipo. Propongo texto y revisar si duele.
3. **Comandos del layout.** Hoy `Page::commands` registra los de la página.
   Un layout que quiera comandos propios (un atajo para su filtro) necesitaría
   lo mismo; propongo esperar a necesitarlo.

**Cambios respecto al diseño, al implementar L3 (30-oct):**

- `slot(name)` devuelve un `div` con la clave reservada `kai.slot.<nombre>` y
  `flex flex-row items-center`. `Element::fill_slots(&mut |name| …)` rellena todos
  los huecos de un árbol con la respuesta de la página; el hueco sin respuesta
  queda vacío. La raíz generada lo llama en la vista de cada layout, antes de
  `fill_outlet`, con una clausura `slot_of` que pregunta a `Page::slot` de la
  página de la ruta actual (su `Msg` se mapea a `RootMsg` como el de la vista).
- Como los nombres son texto, un layout puede pedir un hueco que ninguna página
  responde, y una página responder a un nombre que ningún layout dibuja: no hay
  error en ninguno de los dos casos.
- **Layouts en carpetas `[parámetro]`.** El layout vive mientras la ruta tenga el
  mismo valor del parámetro: otro valor lo descarta y crea uno nuevo, y salir de
  la carpeta lo descarta. La clave es el valor formateado con `{:?}` (el `Params`
  de cada página ya es `Debug` por `Route`). Limitación: `Layout::init` no recibe
  el parámetro; si el layout lo necesita, lo lee de `RouteInfo::path`.
- `RouteInfo::can_back` y `can_forward` ya no quedan sin uso: la fixture
  `sections-app` los muestra y un test comprueba lo que genera `kai-build`.
- El contenido principal (`Role::Main`) lo marca ahora `kai-build` en la columna
  que envuelve a la página (y a los layouts), con un test de la fixture.
