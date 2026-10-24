# Map

Mapa vectorial interactivo dibujado por Kai: mapa base de teselas PMTiles,
capas de polígonos y líneas, marcadores, zoom y edición de áreas.

Código: `src/maps/` (`map`, `marker`, `layers`, `editor`). Guía:
`docs/dev/10-mapas.md`.

## Uso

```rust
use kai::element::icon;
use kai::geo::LatLon;
use kai::icons::Icon;
use kai::maps::{map, marker, style::MapStyle, tiles::TileSource, Anchor};

map(&self.vista)
    .basemap(TileSource::pmtiles("assets/maps/parral.pmtiles", MapStyle::protomaps()))
    .layers(self.areas.iter().map(|a| {
        a.capa.clone().fill(a.color, 0.35).stroke(a.color, 2.0).label(&a.nombre)
            .selected(Some(&a.id) == self.elegida.as_ref())
    }))
    .on_view_change(Msg::Vista)
    .on_area_click(Msg::ElegirArea)
    .fit_bounds(self.encuadre, self.serial_encuadre)
    .label("Mapa de Parral")
    .class("flex-1 min-h-0")
    .children([marker(
        LatLon { lat: -36.14, lon: -71.82 },
        Anchor::Bottom,
        icon(Icon::MapPin).class("size-6 text-accent"),
    )])
```

## API

| Método | Qué hace |
|---|---|
| `map(&MapView)` | Mapa en la vista dada (`center: LatLon`, `zoom` 0–20). La sesión reemplaza la vista con la de su cámara antes del layout. |
| `on_view_change(f)` | Recibe la vista al terminar un gesto (arrastre, scroll, pinza, teclas, botones de zoom). |
| `animate_view(bool)` | `false`: salta a una vista nueva en vez de deslizarse. |
| `basemap(TileSource)` | Teselas vectoriales bajo los marcadores (`TileSource::pmtiles(ruta, estilo)`). |
| `layers(iter)` | Capas sobre el mapa base, de abajo a arriba. |
| `on_area_click(f)` | Id de la capa clicada (sin arrastre). Las capas con id muestran cursor de mano y se iluminan. |
| `fit_bounds(bounds, serial)` | Encuadra `bounds` una vez por `serial` nuevo y reporta la vista. |
| `marker(at, anchor, child)` | Hijo fijado a una coordenada; `Anchor::Bottom` (pin), `Center`, `Top`, `TopLeft`. |

### `MapLayer`

`polygons(geojson)` lee una colección GeoJSON; `MapLayer::polygon`,
`ring(&[LatLon])`, `line(&[LatLon])` crean capas. Modificadores: `id`,
`fill(color, opacidad)`, `stroke(color, ancho)`, `label`, `selected`.
Consultas: `bounds`, `centroid`, `outer_ring`, `contains`.

## Teclado y accesibilidad

Flechas desplazan, `+`/`-` hacen zoom; botones de zoom propios. Dale nombre
accesible con `.label(..)`.

## Partes

`zoom_in`, `zoom_out`, `attribution` (crédito `ATTRIBUTION`, siempre visible).

## Relacionados

[ColorField](color-field.md) · `docs/dev/10-mapas.md`
