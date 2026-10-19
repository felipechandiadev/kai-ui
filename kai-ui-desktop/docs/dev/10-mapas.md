# 10 — Mapas interactivos

**Fase:** posterior a L6; depende de primitivas de renderer pendientes en [04](04-renderer-y-texto.md) (imágenes, transformaciones, clip redondeado) y, para la automatización, del registro de comandos de [09](09-framework-y-plataforma.md). Esta guía evalúa cómo dibujar mapas interactivos (pan/zoom, marcadores, polilíneas/polígonos, popups) **dentro del árbol Kai**, con buen rendimiento, preferiblemente teselas vectoriales, capacidad offline y licencias permisivas.

> **Todo el código de esta guía es API propuesta, no existente.** Lo que sí existe se cita con su archivo (`render/wgpu_backend.rs`, `quad.wgsl`, `layout.rs`, `runtime.rs`…).

> **Verificación.** Versiones, estrellas, fechas y licencias se comprobaron en crates.io, GitHub y en el código de los crates el **18-oct-2026**. Las marcas **(?)** indican datos no verificados del todo o inferencias. Las cifras de estrellas y actividad cambian; revisarlas antes de decidir.

## 1. Recomendación

- **v1 — raster propio dentro de Kai.** Matemática de teselas Web Mercator, caché de teselas, cuadros con textura en el renderer `wgpu` y pan/zoom con inercia. Marcadores y popups son **elementos Kai normales** posicionados sobre el mapa por proyección. Es la única opción que encaja hoy con `wgpu 30` sin acoplar otro framework, y obliga a construir primitivas (imágenes, clip, transformaciones) que Kai necesita igualmente ([04](04-renderer-y-texto.md), «Pendiente»).
- **Largo plazo — teselas vectoriales propias.** Archivos PMTiles (Protomaps u OpenFreeMap) locales o autoalojados, decodificados con `pmtiles` + `mvt-reader`, teselados con `lyon` y dibujados en un pipeline propio sobre el mismo `wgpu::Device`. Da offline real, estilo propio y dependencias MIT/Apache.
- **Spike paralelo con galileo.** Ya renderiza a textura con un `Device` externo (`WgpuRenderer::new_with_device_and_texture`), pero su `main` usa `wgpu 29` y la versión publicada `wgpu 24`: habría que portarlo a `wgpu 30` (fork) antes de compartir dispositivo.
- **Planes B:** MapLibre Native vía FFI (máxima fidelidad, coste C++ y lectura de píxeles por CPU) y webview con MapLibre GL JS (rápido de prototipar, rompe el modelo del árbol Kai).

## 2. Restricción de versiones de `wgpu`

Kai usa **`wgpu 30`**: `glyphon 0.12.0` exige `wgpu ^30.0.0` y la última versión publicada es `wgpu 30.0.1` (22-ago-2026). Dos versiones mayores de `wgpu` en el mismo binario producen tipos distintos (`Device`, `Texture`, `TextureView`) que no se pueden mezclar: compartir dispositivo o texturas exige **la misma versión mayor**; si no, solo queda copiar píxeles por CPU.

| Crate | `wgpu` que exige | ¿Comparte `Device` con Kai? |
|---|---|---|
| `glyphon` 0.12.0 (Kai) | `^30.0.0` | — |
| `vello` 0.11.0 | `^30.0.0` (opcional) | Sí |
| `walkers` 0.63.0 | vía `egui-wgpu ^0.37` → `wgpu ^30.0` (solo feature `mvt`) | Mismo `wgpu`, pero acoplado a egui |
| `galileo` 0.2.1 (crates.io) | `^24` | No |
| `galileo` `main` (GitHub) | `29.0.0` | No sin portar |
| `maplibre-rs` `main` | `22.1.0` | No |
| `maplibre_native` 0.10.0 | no usa `wgpu` (C++ con Metal/Vulkan/OpenGL) | No; entrega imagen RGBA en CPU |

## 3. Comparativa de opciones

| Opción | Estado (18-oct-2026) | Licencia | Integración con el renderer Kai | Vectorial / offline | Veredicto |
|---|---|---|---|---|---|
| **walkers** | v0.63.0 (16-oct-2026), 289★, muy activo | MIT | Acoplado a egui (`Context`, `Painter`, `Mesh`) | MVT + PMTiles | Referencia y código copiable (MIT) |
| **galileo** | crates.io 0.2.1 (jul-2025); `main` con push 30-jul-2026; 651★ | MIT/Apache (crate) | Render a textura con `Device` externo | MVT con estilo MapLibre | Spike; requiere fork a `wgpu 30` |
| **maplibre-rs** | «stability-experimental», «proof-of-concept» | Apache-2.0 | `wgpu 22.1` | Sí, experimental | Descartado |
| **MapLibre Native (FFI)** | `maplibre_native` 0.10.0 (2-oct-2026); núcleo 2.275★ | Apache-2.0 (bindings), BSD-2 (núcleo) | Imagen RGBA en CPU → subida a textura | Completo | Plan B de fidelidad |
| **Webview (wry) + MapLibre GL JS** | wry 0.57.0 (8-sep-2026); GL JS 11,9k★ | Apache/MIT; BSD (?) | Vista nativa superpuesta | Completo | Plan B de prototipado |
| **Raster propio** | — | propia | Nativa (cuadros con textura) | Raster; offline según la fuente | **v1** |
| **Vectorial propio** | crates base activos | MIT/Apache | Nativa (pipeline de geometría) | Sí | **Largo plazo** |
| **vello** | 0.11.0 (2-oct-2026), 4,4k★ | Apache/MIT | `wgpu 30`; renderer de cómputo, escena → textura | Rasterizar teselas vectoriales | Complemento opcional |

### 3.1 walkers

- https://github.com/podusowski/walkers · https://crates.io/crates/walkers
- «Slippy map widget for egui», al estilo de Leaflet. Funciones: teselas XYZ por HTTP, archivos `.pmtiles` locales, raster, MVT con estilo parecido a MapLibre (features `mvt` y `pmtiles`), GeoJSON experimental. Fuentes incluidas: OpenStreetMap, Mapbox, OpenFreeMap, OpenTopoMap, Geoportal.
- **No reutilizable sin egui:** `tiles.rs`, `http_tiles.rs`, `io/fetch.rs`, `io/tiles_io.rs`, `mvt.rs` y el render usan tipos de egui. Sí son independientes `mercator.rs` (proyección, `tile_id`, `total_tiles`) y la lógica de `TileId` (vecinos, proyección), útiles como referencia o para copiar con atribución MIT.

### 3.2 galileo

- https://github.com/galileo-map/galileo (antes `Maximkaaa/galileo`) · https://crates.io/crates/galileo
- Motor de mapas de propósito general: raster, MVT (`galileo-mvt`), estilos MapLibre (`galileo-maplibre`), capas de features, proyecciones distintas de Mercator, ejemplo web con 3,6 millones de puntos.
- `WgpuRenderer` admite `new_with_device_and_surface` y `new_with_device_and_texture(device, queue, size)`: el modelo encaja con «Kai pinta la textura del mapa como una imagen».
- Riesgos: pocas versiones publicadas (5), la última en jul-2025; API inestable (?); `wgpu` desalineado; dependencias propias de texto (`rustybuzz`, `font-kit`) que duplican lo que hace glyphon.

### 3.3 maplibre-rs

- https://github.com/maplibre/maplibre-rs (1.583★). El README dice que la implementación actual es un *proof-of-concept* y que no está claro si el rendimiento requerido es alcanzable con su stack. `wgpu 22.1.0`. Commits de 2026 sobre todo de mantenimiento. **No viable.**

### 3.4 MapLibre Native vía FFI

- https://github.com/maplibre/maplibre-native-rs · https://crates.io/crates/maplibre_native · núcleo https://github.com/maplibre/maplibre-native (BSD-2, 2.275★).
- Backends: Metal en macOS ARM (probado en CI), Vulkan/OpenGL en Linux; Windows marcado como «debería funcionar, aún no» en su tabla.
- La API de Rust renderiza a `Image` (`ImageBuffer<Rgba<u8>>`) en modos estático, por teselas y continuo. No encontré forma de compartir textura con `wgpu` (revisión por `grep` del crate, no exhaustiva (?)): cada frame implica lectura de GPU → CPU → `queue.write_texture`.
- Coste: compilación C++ o binario precompilado, tamaño del bundle, otra pila de texto y estilos. A cambio, fidelidad total del estilo MapLibre, etiquetas y offline.

### 3.5 Webview (wry) con MapLibre GL JS

- https://github.com/tauri-apps/wry (`WebViewBuilder::build_as_child`) · https://github.com/maplibre/maplibre-gl-js
- Es lo más rápido para tener todo (estilos, etiquetas, gestos). Pero es una vista nativa superpuesta a la superficie de Kai: los elementos Kai no se dibujan encima (los popups tendrían que ser HTML), el clip y las esquinas redondeadas no se integran, AccessKit ve una caja opaca, la entrada va por IPC y se añade un runtime web completo. Contradice [00](00-vision-y-arquitectura.md) (árbol y renderer propios).

### 3.6 Raster propio

- Unos cientos de líneas: proyección, selección de teselas visibles, caché, descarga, atlas/arreglo de texturas y entrada.
- Control total, sin conflictos de `wgpu`, y base del camino vectorial (las teselas vectoriales rasterizadas a textura reutilizan todo).
- Límites: nitidez en zoom fraccional (escalado bilineal), sin estilo propio, offline solo si la fuente lo permite (sección 6).

### 3.7 Vectorial propio

| Crate | Versión | Fecha | Licencia | Papel |
|---|---|---|---|---|
| [`pmtiles`](https://crates.io/crates/pmtiles) | 0.25.0 | 15-oct-2026 | MIT/Apache | Lectura de archivos PMTiles (features `mmap-async-tokio`, `http-async`, `write`…) |
| [`mvt-reader`](https://crates.io/crates/mvt-reader) | 2.5.0 | 6-sep-2026 | MIT | Decodificar Mapbox Vector Tiles (lo usa walkers) |
| [`mvt`](https://crates.io/crates/mvt) | 0.15.0 | 1-ago-2026 | MIT/Apache | Alternativa (lectura/escritura MVT) (?) |
| [`geozero`](https://crates.io/crates/geozero) | 0.15.1 | 11-dic-2025 | MIT/Apache | Conversión de geometrías (GeoJSON, MVT…) |
| [`lyon_tessellation`](https://crates.io/crates/lyon_tessellation) | 1.0.22 | 6-sep-2026 | MIT/Apache | Triangulación de rellenos y trazos |
| [`vello`](https://crates.io/crates/vello) | 0.11.0 | 2-oct-2026 | Apache/MIT | Rasterizado 2D con AA en GPU, `wgpu 30` |

- La parte cara no es la geometría sino el **estilo** (un subconjunto de la spec de MapLibre) y las **etiquetas** (colocación, colisiones, texto a lo largo de líneas). El texto puede reutilizar glyphon/cosmic-text.
- Evitar `slippy-map-tiles` (GPL-3.0, sin versiones desde 2020).

## 4. Diseño de integración (enfoque recomendado)

```mermaid
flowchart LR
  Model[Model de la app: MapView, marcadores, rutas] --> View[view: map(..) con hijos Kai]
  View --> Layout[layout.rs: rect del mapa y de los marcadores]
  Layout --> Paint[paint: DrawOp::Map + hijos]
  Paint --> Backend[wgpu_backend: teselas, geometría, quads, texto]
  Source[TileSource: HTTP XYZ / PMTiles] --> Worker[workers de carga y decodificación]
  Worker -->|EventLoopProxy: TileReady| Runtime[runtime.rs]
  Runtime --> Cache[TileCache: GPU / RAM / disco]
  Cache --> Backend
  Input[winit: arrastre, rueda, pellizco] --> Runtime
  Runtime -->|Msg::MapMoved| Model
  Mcp[registro de comandos / MCP] -->|mapa.set_view, mapa.add_marker...| Model
```

### 4.1 Lo que necesita el renderer

| Pieza | Estado actual | Propuesta |
|---|---|---|
| Imágenes rasterizadas | Pendiente en [04](04-renderer-y-texto.md); solo existe `MaskAtlas` `R8Unorm` para iconos | `DrawOp::Image { rect, texture, uv, opacity }`. Almacén de teselas en un `TextureArray` RGBA (por ejemplo 256 capas de 256 o 512 px) o en un atlas de 4096² con huecos y desalojo LRU. Útil también para `<img>` y avatares. Con sampler lineal (a diferencia de los iconos, que usan `textureLoad`). |
| Clip | Rectangular por instancia en `quad.wgsl`, pila resuelta en `resolve` | Reutilizar el clip rectangular; añadir clip redondeado (máscara SDF por instancia) para `rounded-xl overflow-hidden`, pendiente según [09](09-framework-y-plataforma.md) F5. |
| Transformaciones | Pendiente | Uniforme por lote con matriz afín 2D (o traslación + escala). La geometría del mapa vive en coordenadas Mercator del mundo y pan/zoom solo cambian el uniforme: no hace falta reconstruir buffers aunque el árbol se reconstruya cada frame. |
| Geometría | Solo quads SDF | Pipeline de triángulos (posición en mundo, color, `width`/normal para trazos) con vértices de `lyon`. AA por MSAA 4× o por distancia en el shader. Imprescindible para polilíneas y polígonos de la app desde v1. |
| Orden de pintor | `resolve` alterna lotes `Quads`/`Texts` | Añadir lotes `Images` y `Geometry` en la misma secuencia, para que fondo del mapa, capas vectoriales e hijos Kai respeten el orden. |

### 4.2 Elemento `Map`

```rust
// API propuesta
#[derive(Clone, Debug, PartialEq)]
pub struct MapView {
    pub center: LatLon,   // grados WGS84
    pub zoom: f64,        // fraccional; 0 = mundo en una tesela
    pub bearing: f64,     // 0 en v1
}

fn view(&self) -> Element<Msg> {
    map(&self.vista)
        .source(self.fuente)                       // TileSourceId registrado al arrancar
        .layers([
            MapLayer::polyline(&self.ruta).class("stroke-primary stroke-[3]"),
            MapLayer::polygon(&self.zona).class("fill-primary/20 stroke-primary"),
        ])
        .on_view_change(Msg::MapaMovido)            // cámara controlada por el Model
        .label("Mapa de entregas")
        .class("w-full h-96 rounded-xl overflow-hidden")
        .children(self.paradas.iter().map(|p| {
            marker(p.posicion, Anchor::Bottom,
                   button(icon(Icon::MapPin)).label(&p.nombre)
                       .on_click(Msg::Seleccionar(p.id)))
                .key(p.id)
        }))
        .children(self.popup.as_ref().map(|p| {
            marker(p.posicion, Anchor::Top, card(..).class("p-3 shadow-lg"))
        }))
}
```

- El `Element` solo **describe** el mapa: cámara, fuente, capas e hijos. Caché, texturas y mallas viven en el runtime/renderer, indexadas por `(TileSourceId, TileId)`, y sobreviven a la reconstrucción del árbol cada frame.
- **Layout:** Taffy da el rect del nodo `map`. Tras el layout, cada `marker(pos, anchor, hijo)` se coloca en posición absoluta en `proyectar(pos, zoom) − proyectar(centro, zoom) + centro_del_rect`, ajustada por `anchor`. Se reutiliza el soporte de `position: absolute` de [02](02-geometria-y-layout.md).
- **Paint:** fondo del mapa (teselas), capas vectoriales, atribución obligatoria (texto pequeño abajo a la derecha) y luego los hijos Kai, todo dentro del clip del nodo.
- **Accesibilidad ([08](08-accesibilidad.md)):** el mapa es un grupo con nombre accesible; cada marcador es un botón normal con foco y teclado. Flechas y `+`/`−` mueven y hacen zoom cuando el mapa tiene foco.
- **Cámara controlada o no controlada:** con `on_view_change` el `Model` es la fuente de verdad (Elm puro). Para no forzar un `update` por cada evento de ratón, un modo no controlado guarda la cámara en estado de UI por `WidgetId` y solo emite `MapaMovido` al terminar el gesto (decisión abierta, sección 8).

### 4.3 Lógica de teselas

- **Proyección:** Web Mercator (EPSG:3857); `x = (lon + 180) / 360 · 2^z`, `y = (1 − ln(tan φ + sec φ) / π) / 2 · 2^z`, con latitud acotada a ±85,0511°.
- **Zoom fraccional:** nivel de teselas `z_t = floor(zoom)` (acotado al `max_zoom` de la fuente) y escala de dibujo `2^(zoom − z_t)`. Con teselas de 512 px se resta 1 al nivel (como `zoom_offset` en `walkers/src/mercator.rs`).
- **Teselas visibles:** el rect del nodo convertido a coordenadas de tesela, más un margen de una tesela; envoltura horizontal en `x` (mundo repetido) y recorte en `y`.
- **Sin huecos:** para cada tesela visible sin cargar, buscar el ancestro más cercano en caché y dibujarlo con `uv` como sub-rectángulo; opcionalmente, hijos ya cargados al alejar. Fundido de 150 ms al llegar.
- **Prioridad de carga:** desde el centro hacia fuera; cancelar peticiones de teselas que salen de la vista o de niveles abandonados.

### 4.4 Caché y carga

| Nivel | Contenido | Política |
|---|---|---|
| GPU | Capas del `TextureArray` o huecos del atlas | LRU por último frame de uso (256–512 teselas) |
| RAM | Bytes codificados o RGBA decodificado; mallas vectoriales en fase 2 | LRU por tamaño (por ejemplo 64–128 MB) |
| Disco | `~/Library/Caches/<bundle-id>/tiles/<fuente>/{z}/{x}/{y}` | Respeta `Cache-Control`, `ETag` y peticiones condicionales; para OSM, mínimo 7 días |

```rust
// API propuesta
pub trait TileSource: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn tile_size(&self) -> u32;            // 256 o 512
    fn zoom_range(&self) -> RangeInclusive<u8>;
    fn attribution(&self) -> &str;         // siempre visible en el widget
    fn kind(&self) -> TileKind;            // Raster(Png|Jpeg|Webp) | Vector(Mvt)
    fn fetch(&self, tile: TileId) -> TileFuture;   // HTTP XYZ, PMTiles local o PMTiles HTTP
}
```

- La descarga y la decodificación (`image`, o `mvt-reader` + `lyon` en fase 2) corren en un pool fuera del hilo de UI, con concurrencia acotada (4–6) y un **User-Agent propio** de la app. El resultado llega como evento interno `TileReady` por el `EventLoopProxy` que `runtime.rs` ya usa para AccessKit, y provoca `request_redraw`. No es un `Msg` de la app: no depende de `Task`/`Subscription` (F4 de [09](09-framework-y-plataforma.md)).
- La subida a GPU (`queue.write_texture`) se limita por frame (por ejemplo 8 teselas) para no bajar de 60/120 Hz.
- PMTiles local: lectura por mmap; la cabecera y los directorios se cachean una vez por archivo.

### 4.5 Entrada

| Gesto (winit 0.30) | Comportamiento |
|---|---|
| `MouseInput` + `CursorMoved` (arrastre) | Pan 1:1 en píxeles. Se guarda la velocidad de los últimos ~100 ms; al soltar empieza la **inercia**: `v ← v · e^(−k·dt)` y un redraw por frame mientras `‖v‖` supere un umbral. |
| `MouseWheel` con `PixelDelta` (trackpad) | Pan con dos dedos (como en Apple Maps), o zoom continuo con Cmd (decisión abierta). |
| `MouseWheel` con `LineDelta` (ratón) | ±1 nivel animado (unos 200 ms, `ease-out`). |
| `PinchGesture` (macOS) | Zoom continuo `zoom += log2(1 + delta)`. |
| Doble clic / Shift + doble clic | +1 / −1 nivel animado. |
| Teclado con foco en el mapa | Flechas mueven, `+`/`−` hacen zoom, Esc cierra el popup. |

- **Zoom anclado al cursor:** el punto geográfico bajo el puntero no se mueve; `centro' = ancla + (centro − ancla) · 2^(zoom − zoom')` en coordenadas de mundo.
- Los clics sobre marcadores los resuelve el hit-test normal de `layout.rs` antes que el mapa, porque son hijos con mayor orden de pintura.
- Mientras haya animación o inercia, el runtime pide frames continuos; en reposo, ninguno.

### 4.6 Comandos de mapa en el registro (MCP)

Con el registro de [09](09-framework-y-plataforma.md) §4, las acciones del mapa son comandos tipados: el mismo `Msg` sirve para botones, menús, paleta, tests y herramientas MCP, y la UI refleja lo que hace el agente.

| Id | Seguridad | Entrada | Salida |
|---|---|---|---|
| `mapa.set_view` | `Mutating` | `{ center: {lat, lon}, zoom, animate? }` | `MapView` |
| `mapa.fit_bounds` | `Mutating` | `{ bounds: [[lat, lon], [lat, lon]], padding? }` | `MapView` |
| `mapa.get_view` | `ReadOnly` | `{}` | `{ view, visible_bounds }` |
| `mapa.add_marker` | `Mutating` | `{ position, title, id? }` | `{ id }` |
| `mapa.remove_marker` | `Mutating` (undo: volver a añadirlo) | `{ id }` | `()` |
| `mapa.draw_route` | `Mutating` | `{ points: [{lat, lon}], id? }` | `{ id }` |
| `mapa.clear_layers` | `Destructive` si borra datos de usuario | `{ ids? }` | `()` |
| `mapa.screen_to_latlon` | `ReadOnly` | `{ x, y }` (puntos lógicos) | `{ lat, lon }` |

```rust
// API propuesta (kai-commands, ver 09 §4.2)
/// Centra el mapa en `center` con el nivel `zoom`. Con `animate`, la transición dura ~300 ms.
#[derive(Clone, Debug, Deserialize, JsonSchema, Command)]
#[command(id = "mapa.set_view", title = "Mover el mapa", safety = Mutating,
          output = MapView, msg = Msg::SetView)]
pub struct SetView { pub center: LatLon, pub zoom: f64, #[serde(default)] pub animate: bool }
```

- Recurso propuesto: `kai://<app>/mapa/vista` (cámara y límites visibles), con notificación cuando cambia su revisión tras un `update`.
- Validar entradas: latitud en ±85,0511 (Mercator), zoom dentro del rango de la fuente, límite de puntos por ruta.
- Las descargas offline masivas no deben exponerse como herramienta sin revisar los términos de la fuente (sección 6).

### 4.7 Fase 2: teselas vectoriales

1. `pmtiles` entrega los bytes de la tesela, que se descomprimen (gzip).
2. `mvt-reader` saca capas y features; se filtran con un **subconjunto de estilo**: Protomaps basemap v4 u OpenMapTiles (OpenFreeMap). Solo `fill`, `line` y `symbol` básicos al principio.
3. `lyon` tesela rellenos y trazos en un worker; la malla se cachea por `(TileId, versión de estilo)` en coordenadas locales de tesela (0–4096).
4. Dibujo con el pipeline de geometría y el uniforme de transformación; sobrezoom de la última tesela disponible (z15 en Protomaps).
5. Etiquetas con glyphon y un índice de colisiones propio, recalculado al terminar el gesto (no por frame).

Alternativa más barata: rasterizar cada tesela vectorial a una textura (con `vello` en `wgpu 30`, o con el pipeline de `lyon`) y reutilizar el camino raster de v1, a costa de nitidez en zoom fraccional. Si el spike de galileo funciona tras portarlo a `wgpu 30`, puede sustituir los pasos 1–5 renderizando a textura con el `Device` de Kai.

## 5. Hoja de ruta por fases

Cada fase conserva el gate de [README](README.md): `cargo fmt`, `check`, `test` y `clippy -D warnings` en verde, más su criterio propio.

### M0 — Primitivas de renderer

- `DrawOp::Image` con `TextureArray`/atlas RGBA y sampler lineal; lotes `Images` en `resolve`; uniforme de transformación; pipeline de geometría con vértices de `lyon`; clip redondeado.
- **Terminado cuando:** las capturas offscreen ([04](04-renderer-y-texto.md), «Capturas») muestran una imagen recortada por `rounded-xl overflow-hidden`, una polilínea con AA y un polígono semitransparente en el orden de pintor correcto, y los tests puros de `resolve` cubren los nuevos lotes.

### M1 — Mapa raster (v1)

- Matemática Mercator y `TileId` con tests (proyectar/desproyectar, teselas visibles, envoltura, anclaje del zoom). `TileSource` HTTP XYZ y PMTiles raster local; caché GPU/RAM/disco; worker con `EventLoopProxy`; elemento `map` con `marker` y atribución; entrada con inercia y zoom anclado.
- **Terminado cuando:** pan y pinch fluidos a 120 Hz en Apple M4 con 4 mapas abiertos (medido); sin huecos al hacer zoom (se ve el ancestro); un marcador sigue a su coordenada a ±0,5 px en todos los niveles (test de layout); VoiceOver recorre los marcadores; con la red cortada se ve lo cacheado; la atribución siempre es visible.

### M2 — Capas de la app y comandos

- Polilíneas, polígonos y popups como API estable; comandos `mapa.*` en el registro (cuando exista F1 de [09](09-framework-y-plataforma.md)) y recurso de vista.
- **Terminado cuando:** `TestApp::invoke(SetView { .. })` mueve la cámara y el recurso cambia de revisión; un cliente MCP (F2 de [09](09-framework-y-plataforma.md)) llama `mapa.add_marker` y el marcador aparece en la UI.

### M3 — Vectorial

- PMTiles vectorial, subconjunto de estilo Protomaps/OpenMapTiles, teselado en workers, etiquetas básicas. Spike de galileo en paralelo para comparar coste y calidad.
- **Terminado cuando:** un extracto regional `.pmtiles` local (por ejemplo, Santiago a z0–15) se ve sin red con calles, agua, edificios y nombres; un frame medio durante el pan cuesta menos de 4 ms de CPU en M4; la memoria de la caché queda acotada.

### M4 — Offline y multiplataforma

- Gestión de regiones offline (descarga de extractos propios, no de servicios que lo prohíben), actualización de archivos; validación en Windows/Linux con F5 de [09](09-framework-y-plataforma.md).
- **Terminado cuando:** una región se descarga, verifica (hash) y borra desde la app; los tests y capturas de mapa pasan en macOS y Windows.

## 6. Datos de teselas y licencias

| Fuente | Tipo | Offline | Condiciones clave | Fuente verificada |
|---|---|---|---|---|
| `tile.openstreetmap.org` | Raster | **Prohibido** | User-Agent propio, atribución visible «© OpenStreetMap contributors», caché según cabeceras o ≥ 7 días, HTTPS, sin precarga; pueden bloquear sin aviso. Recomiendan que la URL sea configurable. | https://operations.osmfoundation.org/policies/tiles/ |
| Protomaps basemap (PMTiles) | Vectorial | **Sí** (archivo propio) | ODbL *Produced Work*: atribución a OSM obligatoria. Planeta ≈ 120 GB (z0–15); recortes con `pmtiles extract --maxzoom`. No enlazar sus builds: copiarlos a almacenamiento propio. Formato PMTiles: spec CC0, implementaciones BSD-3. | https://docs.protomaps.com/basemaps/downloads · https://github.com/protomaps/PMTiles |
| OpenFreeMap | Vectorial (sin raster) | Limitado | Gratis, sin clave, uso comercial permitido. Descargas offline ≤ 3.000 teselas o 50 MB por descarga, ≤ 4 peticiones paralelas, User-Agent único. Planeta semanal en PMTiles/MBTiles para autoalojar. | https://openfreemap.org/tos/ |
| MapTiler Cloud | Raster y vectorial | Restringido | Plan gratuito solo no comercial o I+D. Caché temporal en el dispositivo del usuario; sin caché en servidor, redistribución ni descarga masiva. Offline completo con licencia de MapTiler Server + Data. | https://www.maptiler.com/terms/cloud/ |
| Mapbox | Raster y vectorial | Restringido | Caché offline solo con peticiones directas de cada dispositivo, para un usuario, con tope de teselas (6.000 por defecto). Sin precargar, empaquetar ni redistribuir. | https://docs.mapbox.com/ios/maps/guides/offline/concepts/ |

Consejos:

- **Desarrollo:** OSM estándar con User-Agent propio y caché en disco. **Producción v1 (raster):** un proveedor comercial compatible con OSM o raster propio; no verifiqué una fuente raster abierta que permita offline (?).
- **Largo plazo:** PMTiles autoalojado (Protomaps o el PMTiles de OpenFreeMap) en almacenamiento propio, más extractos regionales locales para offline.
- La **atribución** es parte del widget, no de la app: siempre visible y fuera de menús o tooltips.
- **Licencias de código:** todo lo recomendado es MIT, Apache o BSD. Copiar código de walkers (MIT) exige conservar su aviso de copyright.

## 7. Riesgos

- **Desalineación de `wgpu`** con cualquier librería de mapas externa; cada salto mayor de `wgpu` en Kai obliga a revisar las dependencias que compartan `Device`.
- **Coste de etiquetas y estilos vectoriales:** es el grueso del trabajo de M3; acotar el subconjunto de estilo y no aspirar a paridad con MapLibre.
- **Frame por evento:** la cámara en el `Model` provoca `update` + `view` + layout por cada evento de puntero; medir con el benchmark de [02](02-geometria-y-layout.md) (≈130 µs p95 hoy) antes de optimizar.
- **Memoria GPU** con varios mapas y teselas de 512 px; política LRU compartida entre mapas.
- **Términos de servicio** cambiantes de los proveedores; URL y fuente siempre configurables.

## 8. Preguntas abiertas

- ¿Cámara siempre controlada por el `Model` (Elm puro) o modo no controlado con estado de UI por `WidgetId` y un `Msg` solo al terminar el gesto?
- ¿Pan con dos dedos o zoom con la rueda del trackpad por defecto? ¿Cmd + rueda para zoom?
- ¿`TextureArray` de tamaño fijo o atlas con huecos? ¿512 px (pantallas Retina) o 256 px?
- ¿Teselas vectoriales dibujadas directamente con transformación, o rasterizadas a textura por tesela (más simple, menos nítido)?
- ¿Portar galileo a `wgpu 30` (fork que mantener) o construir el vectorial propio?
- ¿Qué proveedor raster de producción para v1 y con qué condiciones offline? (?)
- ¿Runtime de carga: hilos propios, `tokio` en un hilo de servicio (como el de MCP en [09](09-framework-y-plataforma.md) §13) o el futuro `Task`?
- ¿Proyecciones distintas de Web Mercator? (Fuera de alcance salvo necesidad concreta.)
