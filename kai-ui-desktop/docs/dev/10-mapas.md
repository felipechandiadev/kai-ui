# 10 — Mapas interactivos

**Fase:** pista C de la hoja de ruta [13](13-hoja-de-ruta-plataforma.md) (§8.1): M0 es el Paso 14b, M1 el 15b, M1b el 15c, M2 el 15d, M3 va en la Etapa 7 y M4 con los Pasos 16 y 27. Depende de primitivas de renderer pendientes en [04](04-renderer-y-texto.md) (imágenes, transformaciones, clip redondeado), de la etapa de identificación y pruebas de 13 y, para la automatización, del registro de comandos (13, Pasos 7–9) y MCP (13, Pasos 12–13). La app guía es **DSP desktop** (sección 1.1). Esta guía evalúa cómo dibujar mapas interactivos (pan/zoom, marcadores, polilíneas/polígonos, popups) **dentro del árbol Kai**, con buen rendimiento, preferiblemente teselas vectoriales, capacidad offline y licencias permisivas.

> **Todo el código de esta guía es API propuesta, no existente.** Lo que sí existe se cita con su archivo (`render/wgpu_backend.rs`, `quad.wgsl`, `layout.rs`, `runtime.rs`…).

> **Verificación.** Versiones, estrellas, fechas y licencias se comprobaron en crates.io, GitHub y en el código de los crates el **18-oct-2026**. Las marcas **(?)** indican datos no verificados del todo o inferencias. Las cifras de estrellas y actividad cambian; revisarlas antes de decidir.

## 1. Recomendación

- **v1 — raster propio dentro de Kai.** Matemática de teselas Web Mercator, caché de teselas, cuadros con textura en el renderer `wgpu` y pan/zoom con inercia. Marcadores y popups son **elementos Kai normales** posicionados sobre el mapa por proyección. Es la única opción que encaja hoy con `wgpu 30` sin acoplar otro framework, y obliga a construir primitivas (imágenes, clip, transformaciones) que Kai necesita igualmente ([04](04-renderer-y-texto.md), «Pendiente»).
- **Largo plazo — teselas vectoriales propias.** Archivos PMTiles (Protomaps u OpenFreeMap) locales o autoalojados, decodificados con `pmtiles` + `mvt-reader`, teselados con `lyon` y dibujados en un pipeline propio sobre el mismo `wgpu::Device`. Da offline real, estilo propio y dependencias MIT/Apache.
- **Spike paralelo con galileo.** Ya renderiza a textura con un `Device` externo (`WgpuRenderer::new_with_device_and_texture`), pero su `main` usa `wgpu 29` y la versión publicada `wgpu 24`: habría que portarlo a `wgpu 30` (fork) antes de compartir dispositivo.
- **Planes B:** MapLibre Native vía FFI (máxima fidelidad, coste C++ y lectura de píxeles por CPU) y webview con MapLibre GL JS (rápido de prototipar, rompe el modelo del árbol Kai).
- **Edición de geometrías en v1** (M1b): la app guía la necesita desde el primer corte (sección 1.1); se diseña en la sección 4.8.

### 1.1 Requisitos de la app guía: DSP

DSP-27 (`/Users/felipe/dev/dsp-27`) usa hoy Leaflet 1.9 + `react-leaflet` 5 + `leaflet-draw` 1.0 en dos pantallas. DSP desktop ([13](13-hoja-de-ruta-plataforma.md), Paso 14) debe cubrir lo mismo:

| Necesidad | Dónde está en DSP web | Qué pide a Kai |
|---|---|---|
| Teselas raster OSM | `TileLayer` con `tile.openstreetmap.org` en ambos mapas | `TileSource` HTTP XYZ, caché, User-Agent propio, **atribución visible** (DSP web la oculta con `attributionControl={false}`, contra la política de OSM) |
| Centro y zoom por defecto | `PARRAL_CENTER = (-36.1415, -71.8265)`, `DEFAULT_MAP_ZOOM = 13` (`territory-types.ts`) | Cámara inicial configurable |
| Áreas de operación como polígonos de contexto | `GeoJSON` con color por área (`areaColor`), grosor 1–2 y opacidad de relleno 0,08–0,15 | `MapLayer::polygon` con estilo por capa; GeoJSON `Polygon` de entrada |
| Editar el polígono de un área | `OperationAreasMap.tsx`: dibujar polígono sin autointersección y con área visible; editar vértices; borrar; «nueva área» con polígono por defecto | Editor de geometrías (4.8), modo Área, validación y área geodésica |
| Ubicación de incidencias, requerimientos y operaciones | `LocationGeometryMap.tsx` + `LocationGeometryPicker.tsx`: switch Punto · Línea · Área, una geometría por vez, áreas de contexto en solo lectura, modo solo lectura | Editor en los tres modos; `readOnly`; capa de contexto |
| Encuadrar la geometría | `setView` a zoom ≥ 15 para un punto; `fitBounds` con 20 % de margen para línea y área | `fit_bounds(bounds, padding)` y `set_view` |
| Intercambio con el core | GeoJSON `Point` · `LineString` · `Polygon` en `locationGeometry` y `geometry` (PostGIS `geography`, ver `dsp-27/dev-docs/12-areas-operacion.md` y `13-flujo-operativo.md`) | Tipos GeoJSON en el modelo de la app; conversión con el crate `geojson` |
| Futuro (no en el primer corte) | «Mapa global de incidencias», mapas de riesgo GRD, visitas con GPS (móvil) | Muchos marcadores (agrupación), capas temáticas; GPS queda fuera del escritorio |

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
| Clip | Rectangular por instancia en `quad.wgsl`, pila resuelta en `resolve` | Reutilizar el clip rectangular; añadir clip redondeado (máscara SDF por instancia) para `rounded-xl overflow-hidden` ([13](13-hoja-de-ruta-plataforma.md), Paso 14b). |
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
- **Cámara (recomendación de [13](13-hoja-de-ruta-plataforma.md) §8.1):** durante el gesto la cámara es estado de UI por `WidgetId`, igual que el scroll, y no provoca `update`; al terminar se emite un solo `MapaMovido(MapView)`. La app mueve la cámara con `MapView` en el `Model` (o la acción `mapa.set_view`); el runtime aplica el cambio cuando la revisión de la cámara del `Model` cambia, para no pisar un gesto en curso.

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

- La descarga y la decodificación (`image`, o `mvt-reader` + `lyon` en fase 2) corren fuera del hilo de UI, en el **Tokio de hilo de servicio** que fija [13](13-hoja-de-ruta-plataforma.md) para MCP y efectos (Pasos 12 y 15), con concurrencia acotada (4–6) y un **User-Agent propio** de la app (`[maps] user-agent` en `kai.toml`). El resultado llega como evento interno `TileReady` por el `EventLoopProxy` que `runtime.rs` ya usa para AccessKit, y provoca `request_redraw`. No es un `Msg` de la app.
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

Con el registro de [13](13-hoja-de-ruta-plataforma.md) §7.2 (diseño base en [09](09-framework-y-plataforma.md) §4), las operaciones del mapa son comandos tipados: el mismo `Msg` sirve para botones, menús, paleta, tests y herramientas MCP, y la UI refleja lo que hace el agente. Se distinguen **consultas** (sin efectos, sin `Msg`) y **acciones** (producen un `Msg`); `Safety` describe el efecto sobre datos de dominio, así que mover la cámara es una acción de UI y no de negocio.

| Id | Tipo | Seguridad | Entrada | Salida |
|---|---|---|---|---|
| `mapa.get_view` | Consulta | — | `{}` | `{ view, visible_bounds }` |
| `mapa.screen_to_latlon` | Consulta | — | `{ x, y }` (puntos lógicos) | `{ lat, lon }` |
| `mapa.set_view` | Acción de UI | `Mutating` | `{ center: {lat, lon}, zoom, animate? }` | `MapView` |
| `mapa.fit_bounds` | Acción de UI | `Mutating` | `{ bounds: [[lat, lon], [lat, lon]], padding? }` | `MapView` |
| `mapa.add_marker` | Acción | `Mutating` | `{ position, title, id? }` | `{ id }` |
| `mapa.remove_marker` | Acción | `Mutating` (undo: volver a añadirlo) | `{ id }` | `()` |
| `mapa.draw_route` | Acción | `Mutating` | `{ points: [{lat, lon}], id? }` | `{ id }` |
| `mapa.clear_layers` | Acción | `Destructive` si borra datos de usuario | `{ ids? }` | `()` |

Los comandos `mapa.*` son genéricos de Kai. Las acciones de dominio que guardan geometrías son de la app; en DSP, por ejemplo, `areas.editar_geometria { code, geometry }` (`Mutating`) e `incidencias.set_ubicacion { code, geometry }` (`Mutating`), con `geometry` GeoJSON validado (rangos de coordenadas, polígono cerrado y sin autointersección, límite de vértices).

```rust
// API propuesta (módulo kai::commands; en 09 §4.2 aparece como kai-commands)
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

### 4.8 Edición de geometrías

Sustituye lo que hace `leaflet-draw` en DSP (sección 1.1). Alcance deliberadamente acotado: **una geometría por vez**, tres modos, sin círculos ni rectángulos.

```rust
// API propuesta
map(&self.vista)
    .source(self.osm)
    .layers(self.areas.iter().map(|a| MapLayer::polygon(&a.geometry).class(a.clases())))
    .child(
        geometry_editor(GeometryMode::Area, self.borrador.as_ref())
            .read_only(!self.puede_editar)
            .on_change(Msg::GeometriaEditada)      // Option<Geometry> al confirmar
            .test_id("area.editor"),
    )
    .test_id("areas.mapa")
```

- **Modelo**: `Geometry = Point | LineString | Polygon`, en grados WGS84, convertible a y desde GeoJSON con el crate `geojson` (1.0.0, georust, MIT/Apache (?)). La app guarda solo la geometría confirmada; el borrador en curso es **estado de UI** del editor (por `WidgetId`), como el texto de un input antes de confirmar.
- **Dibujo**:

| Modo | Crear | Terminar | Editar |
|---|---|---|---|
| Punto | Clic en el mapa | Inmediato | Arrastrar el marcador |
| Línea | Clic por vértice | Retorno, doble clic o clic en el último vértice | Arrastrar vértices; clic en el punto medio de un tramo inserta un vértice |
| Área | Clic por vértice (mínimo 3) | Retorno, doble clic o clic en el primer vértice | Igual que línea; el anillo se cierra solo |

- **Vértices como elementos Kai**: cada vértice y cada punto medio es un `marker` con un botón pequeño (`role = Button`, nombre accesible «Vértice 3 de 7», `test_id` `"<editor>.vertex.<n>"` y `"<editor>.midpoint.<n>"`). Así reciben hit-test, foco, teclado y VoiceOver sin código especial.
- **Teclado**: con un vértice enfocado, las flechas lo mueven 1 px (Shift: 10 px); Backspace o Supr lo borran (respetando el mínimo del modo); Esc cancela el dibujo en curso sin emitir `Msg`; Retorno confirma; Cmd+Z deshace la última operación de vértice **dentro del borrador**.
- **Validación**: polígonos sin autointersección (en DSP `allowIntersection: false`) con el crate `geo` (0.33.1, georust, MIT/Apache (?)); un intento inválido marca el tramo en rojo y no confirma. Área geodésica visible mientras se dibuja (en DSP `showArea: true`), con `geo` y formato en m² o ha.
- **Controles**: barra pequeña sobre el mapa con los modos (segmented control, cuando la app no lo controla desde fuera), «Terminar», «Cancelar» y «Borrar geometría». Borrar emite `on_change(None)`; si borra datos guardados, la app decide si confirma.
- **Encuadre**: al cargar una geometría existente, el editor puede pedir `fit_bounds` con margen (DSP usa 20 % y zoom ≥ 15 para puntos).
- **Coste**: los vértices son pocos (decenas); no hace falta pipeline especial. Las aristas del borrador usan el pipeline de geometría de M0.

### 4.9 Pruebas

Los mapas siguen la etapa de identificación y pruebas de [13](13-hoja-de-ruta-plataforma.md) (Etapa 1): ningún test depende de píxeles de teselas reales ni de la red.

- **Identificación** (convención de 13 §6.4): `map(..).test_id("x")` genera `x` (raíz, rol `Group` con nombre accesible), `x.attribution`, `x.zoom_in`, `x.zoom_out`; cada `marker` lleva `key(id)` para conservar su estado y su propio `test_id` (por ejemplo, `"x.marker.<id>"`); el editor genera `"<editor>.vertex.<n>"`, `"<editor>.midpoint.<n>"`, `"<editor>.finish"`, `"<editor>.cancel"`, `"<editor>.clear"`.
- **Fuente falsa**: `FakeTileSource` devuelve teselas sólidas o con un patrón por `(z, x, y)`, sin latencia o con latencia controlada por el reloj de `TestApp`. Las capturas de mapa son deterministas.
- **Gestos en `TestApp`** (API propuesta): `drag(elemento, dx, dy)`, `scroll(elemento, delta)`, `pinch(elemento, factor)`, `double_click`, `advance(Duration)` para inercia y animaciones, `run_until_idle()` para teselas pendientes.
- **Consultas**: `app.query(GetView {})` para la cámara; `app.query(ScreenToLatLon { .. })` para comprobar el anclaje del zoom.
- **Ejemplos de tests**:

```rust
// API propuesta
#[test]
fn dibujar_area_y_confirmar() {
    let mut app = TestApp::new(AreasPantalla::nueva()).with_tiles(FakeTileSource::solid());
    let mapa = app.get_by_test_id("areas.mapa");
    for (x, y) in [(100.0, 100.0), (300.0, 100.0), (200.0, 250.0)] {
        app.click_at(mapa, x, y);
    }
    app.press(Key::Enter, Modifiers::empty());
    let Some(Msg::GeometriaEditada(Some(Geometry::Polygon(p)))) = app.messages().last().cloned() else {
        panic!("se esperaba un polígono");
    };
    assert_eq!(p.exterior().0.len(), 4); // anillo cerrado
}

#[test]
fn poligono_que_se_cruza_no_se_confirma() { /* cuatro clics en «8», Retorno, sin Msg */ }

#[test]
fn marcador_sigue_su_coordenada_en_cada_zoom() { /* bounds() del marcador vs. proyección, ±0,5 px */ }

#[test]
fn mover_vertice_con_teclado() {
    // foco en "area.editor.vertex.1", flecha derecha ×10, Retorno → la longitud del vértice aumenta
}
```

- **Accesibilidad**: `get_by_role(Role::Button).name("Vértice 1 de 3")` y el recorrido con Tab cubren lo que VoiceOver anuncia; el `test_id` llega a `accessibilityIdentifier` para XCUITest y Appium, como en el resto de Kai.

## 5. Hoja de ruta por fases

Cada fase conserva el gate de [README](README.md): `cargo fmt`, `check`, `test` y `clippy -D warnings` en verde, más su criterio propio. Cada hito corresponde a un paso de [13](13-hoja-de-ruta-plataforma.md) (pista C, §8.1) y no empieza antes de que exista la etapa de identificación y pruebas de 13.

### M0 — Primitivas de renderer (13, Paso 14b)

- `DrawOp::Image` con `TextureArray`/atlas RGBA y sampler lineal; lotes `Images` en `resolve`; uniforme de transformación; pipeline de geometría con vértices de `lyon`; clip redondeado.
- **Terminado cuando:** las capturas offscreen ([04](04-renderer-y-texto.md), «Capturas») muestran una imagen recortada por `rounded-xl overflow-hidden`, una polilínea con AA y un polígono semitransparente en el orden de pintor correcto, y los tests puros de `resolve` cubren los nuevos lotes.

### M1 — Mapa raster (v1) (13, Paso 15b)

- Matemática Mercator y `TileId` con tests (proyectar/desproyectar, teselas visibles, envoltura, anclaje del zoom). `TileSource` HTTP XYZ y PMTiles raster local; caché GPU/RAM/disco; carga en el Tokio de servicio con `EventLoopProxy`; elemento `map` con `marker`, capas de polígonos GeoJSON con estilo por capa y atribución; entrada con inercia y zoom anclado; `FakeTileSource` y gestos en `TestApp` (4.9).
- **Terminado cuando:** pan y pinch fluidos a 120 Hz en Apple M4 con 4 mapas abiertos (medido); sin huecos al hacer zoom (se ve el ancestro); un marcador sigue a su coordenada a ±0,5 px en todos los niveles (test de layout); VoiceOver recorre los marcadores; con la red cortada se ve lo cacheado; la atribución siempre es visible; la pantalla de áreas de DSP desktop muestra todas las áreas con su color.

### M1b — Edición de geometrías (13, Paso 15c)

- Editor de 4.8: modos Punto · Línea · Área, vértices accesibles, validación de autointersección, área geodésica, solo lectura, conversión GeoJSON.
- **Terminado cuando:** los tests de 4.9 pasan; el editor de áreas y el selector de ubicación de DSP desktop cubren lo que hacen `OperationAreasMap.tsx` y `LocationGeometryMap.tsx`; dibujar y editar un polígono se puede hacer solo con teclado.

### M2 — Capas de la app y comandos (13, Paso 15d)

- Polilíneas, polígonos y popups como API estable; comandos `mapa.*` en el registro (13, Pasos 7–9) y recurso de vista (13, Paso 24); acciones de dominio con geometría en la app guía.
- **Terminado cuando:** `TestApp::invoke(SetView { .. })` mueve la cámara; un cliente MCP (13, Paso 12) llama `incidencias.set_ubicacion` con un punto y la UI lo refleja; una geometría inválida vuelve como error de herramienta sin tocar el modelo.

### M3 — Vectorial (13, Etapa 7)

- PMTiles vectorial, subconjunto de estilo Protomaps/OpenMapTiles, teselado en workers, etiquetas básicas. Spike de galileo en paralelo para comparar coste y calidad.
- **Terminado cuando:** un extracto regional `.pmtiles` local (para DSP, Parral y alrededores a z0–17) se ve sin red con calles, agua, edificios y nombres; un frame medio durante el pan cuesta menos de 4 ms de CPU en M4; la memoria de la caché queda acotada.

### M4 — Offline y multiplataforma (13, Pasos 16 y 27)

- Gestión de regiones offline (descarga de extractos propios, no de servicios que lo prohíben), actualización de archivos con `Store`; validación en Windows y Linux con el Paso 27 de [13](13-hoja-de-ruta-plataforma.md).
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

- ~~¿Cámara siempre controlada por el `Model` o modo no controlado?~~ Recomendación en 4.2: estado de UI durante el gesto y `Msg` al terminar ([13](13-hoja-de-ruta-plataforma.md) §8.1).
- ¿Pan con dos dedos o zoom con la rueda del trackpad por defecto? ¿Cmd + rueda para zoom?
- ¿`TextureArray` de tamaño fijo o atlas con huecos? ¿512 px (pantallas Retina) o 256 px?
- ¿Teselas vectoriales dibujadas directamente con transformación, o rasterizadas a textura por tesela (más simple, menos nítido)?
- ¿Portar galileo a `wgpu 30` (fork que mantener) o construir el vectorial propio?
- ¿Qué proveedor raster de producción para v1 y con qué condiciones offline? (?)
- ~~¿Runtime de carga?~~ Tokio en hilo de servicio, el mismo de MCP y efectos ([13](13-hoja-de-ruta-plataforma.md), Pasos 12 y 15).
- ¿El editor de geometrías debe permitir varias geometrías por entidad (multipolígonos)? DSP hoy usa una por vez; fuera de alcance salvo necesidad.
- ¿Qué fuente usará DSP en producción: OSM estándar (uso bajo, con User-Agent y atribución) o un PMTiles propio de la región?
- ¿Proyecciones distintas de Web Mercator? (Fuera de alcance salvo necesidad concreta.)
