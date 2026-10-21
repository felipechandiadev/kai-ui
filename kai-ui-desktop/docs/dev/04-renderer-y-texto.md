# 04 — Renderer y texto

**Fase:** L5 (capa de renderizado desacoplada). El trabajo de renderer desacoplado comienza después de que L0–L4 ofrezcan contratos estables.

## Objetivo

Convertir el árbol ya medido en una lista de operaciones visuales que Kai presenta en una superficie GPU, manteniendo el dibujo de todos los controles dentro del renderer propio.

## División de responsabilidades

- **Plataforma/ventana:** superficie, escala, eventos y ciclo de vida.
- **Layout:** calcula cajas, clipping y restricciones.
- **Árbol de widgets:** produce nodos visuales/operaciones semánticas de dibujo.
- **Renderer:** transforma esas operaciones en recursos y comandos GPU; no decide reglas de negocio ni propiedad del estado.
- **Texto:** shaping, fuentes, edición, caret, selección, composición y métricas, coordinados con el renderer.

## Pasos

1. Definir operaciones de dibujo independientes de `wgpu`: rectángulo, borde, clip, texto y orden de capas.
2. Encapsular surface/device/pipeline/buffers y recursos GPU en el backend; validar Metal, resize, formato, recreación y errores.
3. Hacer que L6 produzca una lista de dibujo; el backend la interpreta sin asumir estado de negocio ni API pública GPU.
4. Implementar clipping y orden de capas antes de sombras, filtros y efectos avanzados.
5. Mantener `glyphon` como presentación de texto y `cosmic-text::Editor` para edición hasta que haya evidencia para cambiar. La entrada `winit::Ime::Preedit`/`Commit` conserva su frontera en plataforma y debe pasar pruebas macOS explícitas.
	- Los buffers y layout de Kai se expresan en unidades lógicas. Al crear `TextArea`, convertir los orígenes y límites al espacio físico de la superficie; aplicar el factor de escala una sola vez a la escala del texto y a su posición.
	- La edición activa IME mientras el campo tiene foco y lo desactiva al perderlo. El clipboard está adaptado por `arboard`; probar fallos/permisos del sistema y no bloquear el ciclo gráfico con trabajo lento.
6. Definir política de recursos: carga de fuentes, texturas, cacheo de glifos, invalidación y destrucción al cerrar/cambiar de escala.
7. Evitar trabajo GPU y asignaciones innecesarias por frame solo tras medir; registrar benchmarks en hardware y versión conocidos.
8. Añadir capturas/pruebas visuales para primitivas y widgets base; tolerar diferencias de rasterización con comparaciones adecuadas.

## Estado actual de L5

### Operaciones de dibujo (`render/mod.rs`)

- `Color` (sRGB u8 con alfa directo, `hex`, `mix` como `color-mix`, `with_alpha`, conversión a lineal) y `TextId { widget, part }` (contenido, valor o placeholder de un elemento).
- `Quad { rect, fill, border, border_width, radius, blur, mask }`: una caja con esquinas redondeadas, borde interior y desenfoque opcional; sirve para fondos, bordes, sombras, anillos de foco, caret, selección y scrollbar. Con `mask: Some(Mask { icon, stroke_width })` el quad es un icono: `fill` se multiplica por la cobertura del icono rasterizado (abajo).
- `DrawOp { Quad, Image, Geometry, PushClip, PopClip, Text { id, origin, color } }` y `DrawList` con `clear` (color de fondo de la raíz). Los `PushClip` anidados se intersecan.
- `ImageDraw { rect, bitmap, source, radius, opacity }`: un `Bitmap` RGBA decodificado sobre `rect`; `source` es la parte visible en fracciones `0..1` (el recorte de `object-cover`) y `radius` redondea como un quad (`rounded-full` da un círculo).
- `GeometryDraw { mesh, transform, color, width }`: una malla de triángulos (abajo) colocada con una `Affine2`; `width` es el ancho del trazo en puntos lógicos (0 en los rellenos).
- `resolve(&DrawList) -> Vec<Batch>` resuelve la pila de clips en CPU y agrupa las operaciones en lotes alternos `Quads`/`Texts`/`Images`/`Geometry` **en orden de pintor**: un texto pintado antes que una caja queda debajo de ella. Descarta lo totalmente recortado o transparente.

### Paint (`render/paint.rs`)

`paint(&PaintInput { layout, theme, focused, text_fields, time }) -> DrawList` es puro. Recorre el árbol en `paint_order` (z-index) y, por nodo, pinta en el orden de CSS:

1. `box-shadow` (quad desenfocado, desplazado y expandido).
2. `ring` de foco por fuera del borde, con `ring-offset`.
3. Fondo y borde: con borde uniforme se usa un único quad SDF con radio; con lados distintos (`border-b`, `border-t`…), un rectángulo por lado.
4. Texto, icono (un quad con máscara, cuadrado y centrado en la caja de contenido, con el color de texto heredado y su `stroke_width`), imagen (`fit_image` según `object-fill`, `object-cover` o `object-contain`; radio `min(rounded-*, lado corto / 2)`), o el contenido del input (selección, valor o placeholder y caret), recortado a su interior.
5. Hijos, dentro del clip del nodo si tiene `overflow`.
6. Barra de scroll superpuesta (4 pt, `muted-foreground` translúcido) cuando hay overflow.

La opacidad efectiva multiplica el alfa de todo lo que pinta el nodo. No hay colores literales: todo sale del `Style` resuelto, que a su vez sale del tema.

**Animación al pintar.** Un elemento con `.motion(Motion::Wave { period, delay, lift })` se pinta desplazado en `y` por `offset(time) × alto`, donde `time` es `PaintInput::time` en segundos. Layout, hit-test y AccessKit no cambian: el painter dibuja una copia del nodo con sus cajas trasladadas. `Motion::wave(i)` reproduce `fs-dot-wave` de la web (lo usa `DotProgress`).

- La `Session` cuenta el tiempo desde su creación (`epoch`). `animating()` es verdadero mientras haya un nodo visible con `motion`, así la ventana pide frames.
- Con `Theme.prefs.reduce_motion`, `time` es `None`: nada se mueve y no se piden frames.
- `TestApp` arranca el reloj en el `epoch`, así los frames son iguales en cada corrida y `advance` mueve la animación. `snapshot::render_with_background(.., seconds)` pinta los elementos en ese instante.

### Backend (`render/wgpu_backend.rs` y `quad.wgsl`)

- `WgpuRenderer` encapsula device, pipeline, glyphon y la superficie, o una textura offscreen `Rgba8UnormSrgb` (`new_offscreen` + `render_to_rgba`, usado por las capturas).
- Quads instanciados: cada instancia lleva rect, relleno, borde, clip y `(radio, ancho de borde, blur)`. El shader evalúa la SDF de un rectángulo redondeado, descarta fuera del clip, suaviza el borde con antialiasing y aplica el blur con `smoothstep`. La mezcla es en alfa premultiplicado. El buffer de instancias crece a la siguiente potencia de dos.
- Iconos: `MaskAtlas` (`render/icons.rs`, puro) empaqueta por estantes las máscaras en una textura `R8Unorm` de 1024², con 1 px de margen. La clave es `(icono, tamaño en px físicos, trazo × 100)`, así que cada tamaño y escala se rasteriza nítido una sola vez. `rasterize` arma el `<svg>` de Lucide con el `stroke-width` pedido, lo convierte con `resvg`/`tiny-skia` a la escala física y guarda el canal alfa. En `prepare` se juntan las claves del frame, se empaquetan las nuevas (si el atlas se llena se vacía y se re-empaqueta una vez; una máscara más grande que el atlas se omite) y se suben con `queue.write_texture`. La instancia lleva `uv = (x, y, tamaño)` y `params.w = 1`. El rect se ajusta a píxeles físicos enteros y el shader lee la cobertura con `textureLoad` (un texel por píxel, sin sampler ni filtrado), así el icono queda tan nítido como el raster.
- Imágenes (`render/images.rs` e `image.wgsl`): `ImageAtlas` empaqueta por estantes los `Bitmap` en una textura `Rgba8Unorm` de 4096² con margen de 1 px; si se llena, se vacía y se re-empaqueta con las imágenes del frame. La clave es `Bitmap::id`. Antes de subir, cada imagen se reduce a 1024 px de lado como máximo (promediando) y se premultiplica. El shader muestrea con filtro lineal, recorta con la SDF del rectángulo redondeado y el clip de la instancia, y multiplica por la opacidad.
- Cada lote de texto tiene su propio `TextRenderer` de glyphon (todos comparten el atlas), así los textos se intercalan con las cajas en el orden correcto. Los `TextBounds` salen del clip resuelto en píxeles físicos.
- **MSAA 4×** (`SAMPLE_COUNT`): el pase principal dibuja todos los pipelines (quads, iconos, imágenes, texto y geometría) en una textura multisample de 4 muestras, recreada al cambiar de tamaño, que se resuelve en la superficie o en la textura de la captura. Los quads, iconos y texto ya suavizaban sus bordes en el shader; el MSAA es lo que antialiasa las aristas de las mallas.
- Los colores se convierten a lineal cuando el formato es sRGB, igual que glyphon. `render` devuelve `Ok(false)` cuando hay que redibujar (superficie perdida u obsoleta).

### Geometría (`render/geometry.rs` y `geometry.wgsl`)

- `Mesh` es una malla inmutable con un id único (la clave de sus buffers en GPU) y sus límites. `Mesh::fill(rings, pixel)` tesela anillos cerrados con `lyon` (par-impar: los interiores son huecos); `Mesh::stroke(lines, closed, StrokeStyle, pixel)` deja los vértices sobre el trazado con la normal de extrusión, y el shader los desplaza `normal × width / 2` después de la transformación. Así un trazo conserva su ancho en píxeles a cualquier zoom, y la misma malla sirve para el borde de una calle y su relleno. `MeshBuilder` junta varios rellenos y trazos en una malla; `pixel` (unidades de malla por píxel al zoom esperado) fija la tolerancia del teselado.
- `Affine2` lleva la malla de sus coordenadas propias (0..4096 en una tesela, relativas a un origen en una capa del mapa) a puntos lógicos; mover o hacer zoom solo cambia la transformación, sin volver a teselar ni a subir buffers.
- Backend: una llamada indexada por dibujo. Los buffers de vértices e índices se guardan por id de malla y se liberan cuando la malla ya no existe o lleva 600 frames sin dibujarse. La transformación, el ancho y el color de cada dibujo van en un uniforme con offset dinámico, y su clip es el scissor.
- Lo usan los mapas ([10](10-mapas.md)): teselas vectoriales, estilo del mapa base y capas de polígonos de la app. Los gráficos nativos (`chart`, barras y sparkline) pintan con los mismos `Quad` y `GeometryDraw`.

### Fondos de ventana (`background.rs` y `background.wgsl`)

- API: `App::background()` devuelve un `BackgroundEffect` (`kind`: `None`, `Mesh`, `Aurora`, `Glow`; `motion`: `Static` o `Animated`; `palette`, `speed`, `blur` en puntos, `intensity`). El runtime lo lee tras cada `update`, así que cambia en vivo. Por defecto es `NONE` (color sólido, sin capa).
- Paint convierte el efecto en un `BackgroundLayer` sobre el color de fondo de la raíz y el tema (claro u oscuro), lo pone en `DrawList::background` y no pinta el relleno de la raíz. El renderer lo dibuja primero, en coordenadas de ventana (no se desplaza con el scroll).
- Sigue la referencia GlowStudio (CSS): orbes `radial-gradient` con `filter: blur()` mezclados en *multiply* sobre blanco (mesh), un `conic-gradient` desenfocado que gira con un velo blanco al 40 % (aurora), y un glow central más dos orbes flotantes (glow). Los keyframes (`float-1/2/3`, `aurora-wave`) y las paletas son los de la referencia. En oscuro, tonos más profundos de los mismos matices, *screen* en lugar de *multiply* y opacidad al 65 %.
- El desenfoque es analítico, sin pases de blur: cada orbe es un perfil radial de tramos lineales convolucionado con una gaussiana (forma cerrada con la CDF normal), los bordes de la caja de la aurora usan la CDF y su degradado cónico se promedia sobre el ángulo que cubre el blur. Se evalúa a 1/8 de la resolución física en una textura `Rgba16Float` y se estira con filtrado bilineal y un dither de ±½ paso de 8 bits.
- Costo: estático, la textura se calcula una vez y se reutiliza mientras no cambien efecto, tema o tamaño; no pide frames. Animado, `about_to_wait` pide frames a 60 Hz solo con la ventana visible (`Occluded`), activa y sin movimiento reducido (`Theme.prefs.reduce_motion`, que el runtime lee de macOS al arrancar y cada vez que la ventana se activa: Ajustes › Accesibilidad › Movimiento). Esos frames recalculan solo la textura chica y vuelven a presentar los quads y textos ya preparados (`redraw_background`), sin `view`, layout ni paint; el reloj de animación se detiene mientras tanto, así que retoma sin saltos.
- AppKit no tiene un equivalente directo; lo más cercano es un `NSVisualEffectView` del tamaño de la ventana bajo el contenido, pero aquí no hay material del sistema ni desenfoque de lo que está detrás (no se reproduce `backdrop-filter`).

### Texto (`render/text.rs` y `text_field.rs`)

- `TextStore` guarda un buffer cosmic-text por `TextId`; `sync_text` solo re-modela si cambian el texto o el estilo e implementa `TextMeasure`. Salto solo entre palabras (`Wrap::Word`); `whitespace-nowrap` usa `Wrap::None`.
- El input es de una línea (`Wrap::None`) con `scroll_x` propio, porque glyphon ignora el scroll horizontal del buffer. El preedit IME se muestra subrayado sin modificar el valor. Las métricas del editor siguen al estilo resuelto del input.
- `set_masked(true)` (password): el editor guarda el valor real y `display()` arma un buffer con un `•` por carácter, remapeando caret, selección y preedit de bytes del texto a bytes de la máscara. Los clics y arrastres se resuelven sobre las viñetas visibles y se mapean de vuelta. Copiar y cortar no hacen nada, como en el navegador.
- `set_read_only(true)`: navegación, selección y copia funcionan; insertar, borrar, pegar e IME no. Cortar se comporta como copiar.
- `select_all()` selecciona todo el valor (lo usa `select_on_focus`). Los clics por encima o por debajo de la línea (delegados desde el contorno del campo) se acotan a la línea.

### Imágenes (`media.rs` y `element::image`)

- `image(src)` es una hoja con `Role::Image`; `.alt("…")` le da nombre (sin nombre es decorativa y AccessKit no la expone). `ImageSource` es `Path`, `Url`, `Bytes` o `Rgba(Bitmap)`.
- La sesión tiene un `ImageStore`: tras cada `view` rellena las imágenes ya decodificadas y pide las nuevas al pool de tareas (`Effects::perform`, 4 hilos `std`), que leen con un `ImageLoader` y decodifican con `image` (png, jpeg y webp). `SystemLoader` lee archivos y, con la feature `net` (por defecto), descarga con `ureq` (20 s de timeout, 20 MB como máximo). El resultado invalida el frame. El almacén guarda 256 imágenes y desaloja las que llevan más tiempo sin usarse. Si una carga falla, el elemento sigue sin imagen (el avatar muestra el ícono `User` de Lucide).
- Layout: una imagen cargada toma su aspecto intrínseco (`aspect_ratio` de Taffy) salvo que el estilo fije uno (`aspect-square`, `aspect-video`); sin ancho ni alto, mide su tamaño natural.
- Pruebas: `TestApp` usa un `FakeImageLoader` (respuestas por URL o ruta; `.images(loader)` en el builder) y `run_until_idle()` espera las cargas. Las capturas construidas fuera de una sesión llaman a `ImageStore::load_now(&mut tree)`.

### Capturas

`cargo test --manifest-path kai-ui-desktop/Cargo.toml -- --ignored snapshot_demo` renderiza la demo offscreen en la GPU local (2×) y escribe BMP en `kai-ui-desktop/target/snapshots/`: reposo, interacción (hover, foco y valor), ventana estrecha, scroll al final con anillo de foco, la tarjeta de variantes de TextField en reposo (`demo-fields`, con password enmascarado) y con valores y foco en el password visible (`demo-fields-focus`), y la sección Multimedia en reposo y con un archivo arrastrado sobre la portada (`demo-multimedia`, `demo-multimedia-drag`). `images_draw_rounded_cropped_and_clipped` escribe `images-primitive.bmp` (imagen entera, en círculo, `rounded-xl`, semitransparente y recortada) y `geometry_draws_antialiased_strokes_and_fills_in_painter_order` escribe `geometry-primitive.bmp` (una polilínea de 3 px con uniones redondeadas y un polígono semitransparente debajo y encima de un quad, también recortado y ampliado). La sección Mapa aporta `demo-mapa` (solo el mapa base), `demo-mapa-areas` (los sectores de DSP) y `demo-mapa-seleccion` (un sector elegido y encuadrado); sus teselas se cargan de forma síncrona y sus etiquetas aparecen ya fundidas. Para verlas como PNG: `sips -s format png demo-idle.bmp --out demo-idle.png`. Es `#[ignore]` porque necesita GPU.

### Pendiente

- [x] Comparación de capturas con la baseline en local: `scripts/snapshot-baseline.sh --check` (SHA de 21 capturas) y `snapshot::diff` con tolerancia.
- [ ] Comparación automática en CI (requiere un runner con GPU y tolerancia de rasterización).
- [x] Iconos SVG (Lucide) en un atlas de máscaras.
- [x] Imágenes rasterizadas (atlas RGBA, `object-*`, radio propio).
- [x] Geometría vectorial con `lyon` (rellenos y trazos de ancho constante en pantalla), transformación afín por dibujo y MSAA 4×.
- [ ] Sombras múltiples, gradientes y clip redondeado de contenedores (`rounded-xl overflow-hidden` recorta sus hijos con un rectángulo).
- [ ] Política de recursos de fuentes (carga propia, invalidación al cambiar de escala) más allá del `FontSystem` del sistema.

## Accesibilidad

Ver [08-accesibilidad.md](08-accesibilidad.md): el árbol AccessKit se genera desde el layout del árbol `Element`. Contexto original:

Investigar `accesskit` y el puente con la API de accesibilidad de macOS desde el prototipo. Los widgets dibujados a mano deben exponer roles, nombres, estados, acciones, foco y límites. Si el puente no cubre lo requerido, es un riesgo bloqueante que debe resolverse antes del lanzamiento, no una tarea cosmética. Estado actual en [08-accesibilidad.md](08-accesibilidad.md).

## Criterios de aceptación

- Renderer presenta primitivas y texto sin fugas ni fallos al resize/cierre.
- Coordenadas y escala coinciden con el layout y el hit-testing.
- Edición de texto y entrada IME tienen una estrategia documentada y probada.
- No se depende de `egui`/`iced` para renderizar widgets de producción.
