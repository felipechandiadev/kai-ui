# 02 — Geometría y layout

**Fases:** L0 (base geométrica) y L4 (maquetación incremental/caché). Implementar L0 y validarlo antes de L1; L4 se inicia tras L1 y L2–L3.

## Estado actual de L0

- [x] Módulo geométrico puro inicial en `kai-ui-desktop/src/geometry.rs`, con `Point`, `Size`, `Rect`, `Insets`, `PhysicalPoint` y `ScaleFactor`.
- [x] Hit-testing, conversión lógico/físico y transformación de rectángulos a NDC comparten esas primitivas.
- [x] Tests de bordes, rectángulos inválidos/vacíos, intersección, traslación, insets y escala Retina.
- [x] Verificación visual manual de render e interacción tras la extracción, ejecutando la app en macOS; el usuario confirmó que está correcta.
- [ ] Revisar si las operaciones geométricas adicionales requeridas por layout justifican otro tipo/dependencia; no añadir una crate geométrica por anticipado.

La extracción L0 queda aceptada: las pruebas automáticas pasan y la revisión visual manual fue confirmada. IME y portapapeles mantienen su estado de validación independiente de las capas geométricas.

## Objetivo

Definir coordenadas coherentes entre ventana, layout, hit-testing y renderer; después habilitar composiciones básicas para los widgets.

## Pasos

1. **L0:** documentar espacio y unidades: puntos lógicos, origen, eje Y, límites y conversión a píxeles físicos por escala de ventana.
2. **L0:** consolidar tipos mínimos `Point`, `Size`, `Rect`, `Insets` y `Scale` (o equivalente); elegir tipos propios inicialmente y comparar `kurbo`/`euclid` solo si una necesidad real lo justifica.
3. **L0:** especificar y probar intersección, contención, traslación y conversión lógico/físico; definir semántica de tamaños no positivos, valores no finitos y rectángulos vacíos.
4. **L0:** migrar el hit-testing y las cajas de presentación para que render e interacción usen la misma geometría.
5. **Gate L0:** tests unitarios deterministas de geometría, bordes, valores límite, escala Retina y transformación a NDC; la app debe conservar comportamiento.
6. **L4:** usar CSS Grid como modelo principal de composición, con tracks, auto-placement, gap, padding, alineación y restricciones explícitas. Taffy es el algoritmo candidato para aproximar la semántica CSS.
7. **L4:** mantener el layout desacoplado del renderer; compartir resultados entre render y hit-testing, y probar árbol conocido, resize y escala.
8. **L4:** empezar con layout completo correcto; añadir invalidación/caché solo tras benchmark reproducible y pruebas de paridad frente a recomputación completa.

## Contrato esperado

El layout calcula cajas; no dibuja ni procesa eventos. El renderer consume las cajas. El sistema de interacción pregunta a esas mismas cajas qué nodo está bajo el puntero. Así se evita que la apariencia y los clics diverjan.

## Modelo de cajas (tipo DOM/CSS)

- La vista es un árbol `Element<Msg>` (`element.rs`) con tres tipos de nodo: `div`, `text` e `input`. Cada nodo lleva una cadena de clases tipo Tailwind; `tw.rs` la convierte en un `Style` tipo CSS (`style.rs`) y `Style::to_taffy` en un `taffy::Style`.
- **La raíz es la ventana.** `Layout::compute` fuerza el tamaño de la raíz al viewport lógico, sin importar sus clases: el elemento `main` ocupa siempre toda la ventana y todo lo demás vive dentro de él.
- `display` admite `block` (por defecto, como en CSS), `flex`, `grid` (`grid-cols-N` = `repeat(N, 1fr)`) y `none`. Taffy calcula flex, grid y block con la semántica CSS: `box-sizing: border-box`, padding, borde, márgenes (incluidos negativos y `mx-auto`), `gap`, `min-*`/`max-*`, fracciones y `position: absolute` con `inset`.
- Medida intrínseca: el texto se mide con cosmic-text (`TextMeasure`), con salto de línea solo entre palabras (`overflow-wrap: normal`). El tamaño medido se redondea hacia arriba para que el redondeo a píxel de Taffy no recorte el último glifo. Un `input` tiene un ancho intrínseco de 180 pt y su línea de texto se centra verticalmente en la caja de contenido.
- Tras el layout, cada texto se vuelve a envolver al ancho final, así que paint dibuja exactamente lo medido.
- **Scroll por nodo.** `overflow-auto`/`overflow-y-auto` se traduce a `Overflow::Scroll` con `scrollbar_width = 0`, porque la barra se pinta superpuesta. `Layout::scroll_height()` da el desplazamiento máximo; los offsets viven en `ScrollOffsets` (`HashMap<WidgetId, f32>`), propiedad del runtime, y se acotan en cada frame.
- **Clip por nodo.** Solo los contenedores con `overflow` distinto de `visible` recortan, y lo hacen a su *padding box*, como CSS. Por eso los bordes, anillos de foco y sombras de los hijos no se cortan salvo que un ancestro con overflow lo exija.
- **Orden de pintura.** Los hermanos se ordenan por `z-index` de forma estable (orden de documento a igualdad). `paint_order` y `hit_path` usan el mismo orden, así que lo que se ve encima es lo que recibe el clic.
- La reducción de altura no comprime controles, tipografía ni gaps: el contenedor con `overflow-y-auto` se desplaza. El mínimo de ventana es **420×340 puntos lógicos**.

```text
Layout (por frame)
└── main (raíz = ventana, flex col)
    ├── header (h-14, shadow-md, z-10)
    └── div.flex-1.overflow-y-auto       ← scroll + clip a su padding box
        └── div.items-center.gap-6.p-6
            ├── card (rounded-lg border shadow-sm)
            │   ├── card_header → text …
            │   └── card_actions → button …
            └── div.grid.grid-cols-3 → stat …
```

## API de `Layout`

- `Layout::compute(&root, &LayoutInput { viewport, theme, interaction, scroll }, &mut StyleCache, &mut dyn TextMeasure)` resuelve el estilo de cada nodo según su estado (hover sobre toda la ruta, foco, `focus-within`, presión, deshabilitado y las variantes `group-*`), hereda las propiedades de texto (`ComputedText`) y multiplica la opacidad hacia abajo en el árbol.
- `LayoutNode` expone `bounds`, `padding_box`, `content_box`, `clip`, `scroll`, `text_box`, el `Style` resuelto, `ComputedText`, la opacidad efectiva y `hidden` (`display: none` o un ancestro oculto).
- `hit_path(point)` devuelve la ruta raíz → nodo más interno bajo el puntero, respetando clips, z-index y `pointer-events-none`. `ancestors`, `reveal(id, margen, offsets)` y `scroll_target(path, delta)` sirven a foco y rueda.
- `StyleCache` memoiza el `ClassList` parseado por cadena de clases y reúne las clases desconocidas.

## Criterios de aceptación

- Tests deterministas para geometría, transforms, escala y cajas vacías.
- Layout reproducible para un conjunto fijo de restricciones.
- Resize actualiza layout y hit-testing de forma consistente.
- No hay promesa de latencia fija sin benchmark documentado.
- L0 no depende de `winit` ni de `wgpu`; la conversión de plataforma ocurre en límites explícitos.

## Estado actual de L4

- [x] `layout.rs` traduce el árbol `Element` a un `TaffyTree` con flex, grid y block, y calcula el layout completo y determinista en unidades lógicas.
- [x] Raíz = ventana, scroll y clip por nodo con `overflow`, z-index en pintura y hit-test, `position: absolute` (etiqueta flotante del campo).
- [x] Hit-testing, paint, AccessKit, caret/IME y mapeo del puntero usan los mismos `LayoutNode`.
- [x] Tests (con `testing::FixedMeasure`): raíz que llena la ventana aunque no tenga clases de tamaño, columna flex con padding, gap y botones `inline-flex`, scroll de `overflow-auto` con clip al padding box, bordes de hijos sin clip que nunca se cortan, `hit_path` con z-index, clip, `display: none` y `pointer-events-none`, variantes de estado y `group-*` en descendientes, y `reveal`. `style.rs` prueba la conversión a Taffy y la herencia de texto.
- [x] `grid_layout.rs` conserva el helper de grid uniforme como referencia probada de la API Grid de Taffy.
- [ ] Scroll horizontal, `position: fixed`/`sticky` y áreas de grid nombradas.
- [ ] Sin caché ni invalidación incremental: la vista y el layout se recalculan completos en cada frame, y el benchmark muestra que hoy no hace falta.

### Benchmark de vista + layout + paint

`cargo test --manifest-path kai-ui-desktop/Cargo.toml --release -- --ignored --nocapture bench_layout_and_paint` ejecuta 400 iteraciones de `view` + `Layout::compute` + `paint` sobre la demo, con medida real de cosmic-text y alternando cuatro anchos (1024, 800, 600 y 420 pt) para forzar el re-envolvido del texto.

Resultado de referencia (Apple M4, macOS 15, release, 18-10-2026): media ≈ 196 µs, p50 ≈ 114 µs, p95 ≈ 132 µs, 142 operaciones de dibujo por frame. El máximo (≈ 32 ms) es la primera iteración, en frío, cuando cosmic-text carga las fuentes. Un frame a 120 Hz dispone de 8,3 ms, así que un caché de layout todavía no se justifica.
