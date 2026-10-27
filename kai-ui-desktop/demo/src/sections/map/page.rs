//! /map: Parral with a pin on the plaza, the DSP sectors over it (chosen
//! from the list, the map or an agent; colored; their outline edited on
//! the map or replaced by an agent) and where the map is.

use std::sync::LazyLock;

use kai::color::Color;
use kai::commands::{CommandError, CommandRegistry, PageAction, PageQuery};
use kai::components::{
    ButtonVariant, CHOICE_ACTIVE, CHOICE_HOVER, SwitchDensity, button, caption, card,
    card_content, card_header, color_field, page, page_body, switch_labeled,
};
use kai::element::{Element, Role, div, icon, text};
use kai::geo::{LatLon, LatLonBounds, MapView};
use kai::icons::Icon;
use kai::maps::editor::{PolygonDraft, PolygonEdit, area_m2, crossing};
use kai::maps::layers::{MapLayer, polygons};
use kai::maps::style::MapStyle;
use kai::maps::tiles::TileSource;
use kai::maps::{Anchor, map, marker};
use kai::router::{Context, HasPage, Page};
use kai::runtime::Effects;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::State;

/// Protomaps extract of Parral (`scripts/extract-region.sh`).
const PARRAL_PMTILES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/maps/parral.pmtiles"
);
/// The five starting sectors of DSP Parral, as its seed draws them.
const DSP_AREAS: &str = include_str!("../../../../assets/maps/dsp-areas.geojson");
/// Plaza de Armas de Parral, where the map starts.
pub const MAIN_SQUARE: LatLon = LatLon::new(-36.1415, -71.8265);
pub const INITIAL_VIEW: MapView = MapView::new(MAIN_SQUARE, 13.0);
/// Margin around a selected sector, as a fraction of its size per side.
const SECTOR_MARGIN: f64 = 0.2;

/// The five starting colors of DSP's area types.
const DSP_PALETTE: [(&str, Color); 5] = [
    ("Azul", Color::hex(0x1e73ae)),
    ("Naranjo", Color::hex(0xc45c26)),
    ("Verde", Color::hex(0x2d6a4f)),
    ("Violeta", Color::hex(0x7c3aed)),
    ("Ámbar", Color::hex(0xb45309)),
];

/// An area of operation of DSP.
#[derive(Clone, Debug, PartialEq)]
pub struct Sector {
    pub code: String,
    pub name: String,
    kind_name: String,
    pub layer: MapLayer,
    /// Its color as typed: hex, or empty for the color of its type.
    color: String,
}

/// The color of an area type, as in DSP's `operation_area_types`.
fn color_for_kind(kind_name: &str) -> Color {
    match kind_name {
        "territorio" => Color::hex(0x1e73ae),
        "barrio" => Color::hex(0x2d6a4f),
        _ => Color::hex(0xc45c26),
    }
}

impl Sector {
    /// Its outline, without repeating the first vertex.
    pub fn ring_points(&self) -> Vec<LatLon> {
        self.layer.outer_ring()
    }

    /// Take `ring_points` as its outline; a ring a layer cannot be made of
    /// leaves it as it was.
    fn with_ring(&mut self, ring_points: &[LatLon]) {
        if let Ok(layer) = MapLayer::ring(ring_points) {
            self.layer = layer;
        }
    }

    /// The color typed for it, or else the one of its type (also while the
    /// typed hex is incomplete).
    pub fn color(&self) -> Color {
        Color::parse_hex(&self.color).unwrap_or_else(|| color_for_kind(&self.kind_name))
    }

    fn swatch(&self) -> String {
        format!("bg-[{}]", self.color().to_hex())
    }
}

/// The sectors as the seed draws them.
pub static SECTORS: LazyLock<Vec<Sector>> = LazyLock::new(|| {
    let text = |properties: &kai::serde_json::Map<String, kai::serde_json::Value>, key: &str| {
        properties
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    polygons(DSP_AREAS)
        .expect("assets/maps/dsp-areas.geojson")
        .into_iter()
        .map(|(layer, properties)| {
            let kind_name = text(&properties, "type");
            Sector {
                code: text(&properties, "code"),
                name: text(&properties, "name"),
                color: color_for_kind(&kind_name).to_hex(),
                kind_name,
                layer,
            }
        })
        .collect()
});

/// Area in hectares with a decimal comma.
fn hectares(area: f64) -> String {
    format!("{area:.1} ha").replace('.', ",")
}

pub struct Screen {
    view: MapView,
    animate: bool,
    sectors: Vec<Sector>,
    show_areas: bool,
    /// Code of the selected sector.
    area: Option<String>,
    /// The box to frame and a serial, so the same box frames again.
    fit_request: Option<(LatLonBounds, u64)>,
    /// The sector whose outline is being edited, and its draft.
    editing: Option<(String, PolygonDraft)>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    /// A gesture on the map ended there.
    Moved(MapView),
    SetView { view: MapView, animate: bool },
    Select(String),
    Fit(LatLonBounds),
    ToggleAreas,
    Color(String),
    Edit(String),
    Editing(PolygonEdit),
    /// Replaces the outline of a sector (an agent's edit, already valid).
    SaveGeometry { code: String, ring_points: Vec<LatLon> },
}

impl Screen {
    fn sector(&self, code: &str) -> Option<&Sector> {
        self.sectors.iter().find(|sector| sector.code == code)
    }

    /// Ask the map to show `bounds`. Boxes with NaN are ignored.
    fn fit_to(&mut self, bounds: LatLonBounds) {
        let Some(bounds) = LatLonBounds::around([
            LatLon::new(bounds.south, bounds.west),
            LatLon::new(bounds.north, bounds.east),
        ])
        .filter(|b| {
            [b.south, b.west, b.north, b.east]
                .iter()
                .all(|v| v.is_finite())
        }) else {
            return;
        };
        let serial = self.fit_request.map_or(1, |(_, serial)| serial + 1);
        self.fit_request = Some((bounds, serial));
        self.animate = true;
    }

    /// Select sector `code`, show the sectors and frame it with a margin.
    fn select(&mut self, code: &str) {
        let Some(bounds) = self.sector(code).map(|s| s.layer.bounds()) else {
            return;
        };
        self.area = Some(code.to_owned());
        self.show_areas = true;
        self.fit_to(bounds.padded(SECTOR_MARGIN));
    }

    fn save_ring(&mut self, code: &str, ring_points: &[LatLon]) {
        if let Some(sector) = self.sectors.iter_mut().find(|s| s.code == code) {
            sector.with_ring(ring_points);
        }
    }

    fn edit(&mut self, edit: PolygonEdit) {
        let Some((code, draft)) = &mut self.editing else {
            return;
        };
        match edit {
            PolygonEdit::Finish if draft.is_valid() => {
                let (code, ring_points) = (code.clone(), draft.ring().to_vec());
                self.editing = None;
                self.save_ring(&code, &ring_points);
            }
            PolygonEdit::Cancel => self.editing = None,
            edit => {
                draft.apply(edit);
            }
        }
    }
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        let screen = Self {
            view: INITIAL_VIEW,
            animate: true,
            sectors: SECTORS.clone(),
            show_areas: true,
            area: None,
            fit_request: None,
            editing: None,
        };
        (screen, Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Moved(view) => self.view = view,
            Msg::SetView { view, animate } => {
                self.view = view.clamped();
                self.animate = animate;
            }
            Msg::Select(code) => {
                if self.editing.is_none() {
                    self.select(&code);
                }
            }
            Msg::Fit(bounds) => self.fit_to(bounds),
            Msg::ToggleAreas => self.show_areas = !self.show_areas,
            Msg::Color(color) => {
                let code = self.area.clone();
                if let Some(sector) = self
                    .sectors
                    .iter_mut()
                    .find(|s| Some(&s.code) == code.as_ref())
                {
                    sector.color = color;
                }
            }
            Msg::Edit(code) => {
                if self.editing.is_none()
                    && let Some(ring_points) = self.sector(&code).map(Sector::ring_points)
                    && let Ok(draft) = PolygonDraft::from_ring(ring_points)
                {
                    self.select(&code);
                    self.editing = Some((code, draft));
                }
            }
            Msg::Editing(edit) => self.edit(edit),
            Msg::SaveGeometry { code, ring_points } => {
                if self
                    .editing
                    .as_ref()
                    .is_some_and(|(editing, _)| *editing == code)
                {
                    self.editing = None;
                }
                self.save_ring(&code, &ring_points);
                if self.editing.is_none() {
                    self.select(&code);
                }
            }
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().child(self.map_card()))
    }

    fn title(&self, _: &State) -> String {
        "Mapa".into()
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        registry.page_action::<SetMapView>().expose();
        registry.page_action::<FitBounds>().expose();
        registry.page_action::<SelectArea>().expose();
        registry.page_action::<EditGeometry>().expose();
        registry.page_query::<MapViewQuery>().expose();
    }
}

impl Screen {
    fn map_card(&self) -> Element<Msg> {
        let view = self.view;
        let selected = self.area.as_deref();
        let editing_code = self.editing.as_ref().map(|(code, _)| code.as_str());
        let layers = self
            .sectors
            .iter()
            .filter(|sector| self.show_areas && editing_code != Some(sector.code.as_str()))
            .map(|sector| {
                let color = sector.color();
                sector
                    .layer
                    .clone()
                    .id(&sector.code)
                    .fill(color, 0.12)
                    .stroke(color, 2.0)
                    .label(&sector.name)
                    .selected(selected == Some(sector.code.as_str()))
            });
        let mut map_view = map(&view)
            .basemap(TileSource::pmtiles(PARRAL_PMTILES, MapStyle::protomaps()))
            .layers(layers)
            .animate_view(self.animate)
            .on_view_change(Msg::Moved);
        map_view = match &self.editing {
            Some((code, draft)) => {
                let color = self
                    .sector(code)
                    .map_or(Color::hex(0xc45c26), Sector::color);
                map_view.polygon_editor(draft, color, Msg::Editing)
            }
            None => map_view.on_area_click(Msg::Select),
        };
        let mut map_view = map_view
            .label("Mapa de Parral")
            .test_id("map.view")
            .class("flex-1 min-w-0 h-96 border border-border")
            .children([marker(
                MAIN_SQUARE,
                Anchor::Bottom,
                icon(Icon::MapPin).class("size-8 text-primary"),
            )
            .alt("Plaza de Armas")]);
        if let Some((bounds, serial)) = self.fit_request {
            map_view = map_view.fit_bounds(bounds, serial);
        }
        let list = div()
            .role(Role::List)
            .label("Sectores")
            .test_id("map.sectors")
            .class("flex flex-col gap-1 w-44")
            .children(self.sectors.iter().map(|sector| {
                let chosen = selected == Some(sector.code.as_str());
                div()
                    .role(Role::Button)
                    .pressed(chosen)
                    .on_click(Msg::Select(sector.code.clone()))
                    .test_id(format!("map.sector.{}", sector.code))
                    .class(&format!(
                        "flex items-center gap-2 h-8 px-2 rounded-md border text-sm \
                         text-foreground cursor-pointer focus-visible:ring-2 \
                         focus-visible:ring-primary/40 {}",
                        if chosen {
                            format!("{CHOICE_ACTIVE} font-medium")
                        } else {
                            CHOICE_HOVER.to_owned()
                        }
                    ))
                    .children([
                        div().class(&format!("size-3 shrink-0 rounded-sm {}", sector.swatch())),
                        text(&sector.name),
                    ])
            }));
        let status = match (&self.editing, selected.and_then(|c| self.sector(c))) {
            (Some(_), Some(sector)) => format!(
                "Editando {}: arrastra los vértices, haz clic entre dos para agregar uno; \
                 Supr borra el vértice elegido y ⌘Z deshace",
                sector.name
            ),
            (None, Some(sector)) => format!(
                "Sector {} · {} · {}",
                sector.name,
                sector.code,
                hectares(area_m2(&sector.ring_points()) / 10_000.0)
            ),
            _ => "Elige un sector en la lista o en el mapa".to_owned(),
        };
        let editing = self.editing.as_ref().map(|(_, draft)| {
            let crosses = draft.crossing().is_some();
            div()
                .class("flex items-center gap-3")
                .test_id("map.editing")
                .children([
                    caption(format!("Superficie {}", hectares(draft.area_ha())))
                        .role(Role::Status)
                        .test_id("map.surface"),
                    if crosses {
                        caption("Los bordes se cruzan")
                            .class("text-error")
                            .role(Role::Alert)
                            .test_id("map.crossing")
                    } else {
                        div()
                    },
                    div().class("flex-1"),
                    button("Cancelar", ButtonVariant::Outlined)
                        .on_click(Msg::Editing(PolygonEdit::Cancel))
                        .test_id("map.cancel"),
                    button("Terminar", ButtonVariant::Primary)
                        .on_click(Msg::Editing(PolygonEdit::Finish))
                        .disabled(!draft.is_valid())
                        .test_id("map.finish"),
                ])
        });
        let color = selected
            .and_then(|code| self.sector(code))
            .filter(|_| self.editing.is_none())
            .map(|sector| {
                div().class("flex w-80").child(
                    color_field("Color del sector", &sector.color, Msg::Color)
                        .palette(DSP_PALETTE)
                        .allow_empty(true)
                        .test_id("map.color")
                        .build(),
                )
            });
        let edit_sector = button("Editar sector", ButtonVariant::Outlined)
            .on_click(Msg::Edit(self.area.clone().unwrap_or_default()))
            .disabled(self.area.is_none() || self.editing.is_some())
            .test_id("map.edit");
        card().class("w-full max-w-3xl").test_id("map").children([
            card_header(
                "Parral",
                Some("Arrastra, usa la rueda o pellizca; doble clic acerca y ⇧ doble clic aleja"),
            ),
            card_content().class("gap-3").children(
                [div().class("flex gap-3").children([
                    map_view,
                    div().class("flex flex-col gap-3 w-44").children([
                        list,
                        edit_sector,
                        switch_labeled(
                            "Sectores",
                            self.show_areas,
                            Msg::ToggleAreas,
                            false,
                            SwitchDensity::Compact,
                        )
                        .test_id("map.show_sectors"),
                    ]),
                ])]
                .into_iter()
                .chain(editing)
                .chain(color)
                .chain([
                    caption(status).test_id("map.sector"),
                    div()
                        .class("flex items-center justify-between gap-3")
                        .children([
                            caption(format!(
                                "{:.4}, {:.4} · zoom {:.1}",
                                view.center.lat, view.center.lon, view.zoom
                            ))
                            .role(Role::Status)
                            .test_id("map.position"),
                            button("Volver a la plaza", ButtonVariant::Outlined)
                                .on_click(Msg::SetView {
                                    view: INITIAL_VIEW,
                                    animate: true,
                                })
                                .test_id("map.back"),
                        ]),
                ]),
            ),
        ])
    }
}

/// `map.set_view`: moves the map, as a drag or the zoom buttons would.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetMapView {
    #[schemars(description = "Centro del mapa (grados)")]
    center: LatLon,
    #[schemars(description = "Zoom de 0 (el mundo entero) a 20 (una calle)")]
    zoom: f64,
    #[schemars(description = "Deslizarse hasta la vista (por omisión) o saltar")]
    #[serde(default)]
    animate: Option<bool>,
}

impl PageAction for SetMapView {
    type Page = Screen;
    const ID: &'static str = "map.set_view";
    const TITLE: &'static str = "Mover el mapa";
    fn description() -> &'static str {
        "Centra el mapa de la sección Mapa en un punto, con el zoom dado"
    }
    fn into_msg(self) -> Msg {
        Msg::SetView {
            view: MapView::new(self.center, self.zoom),
            animate: self.animate.unwrap_or(true),
        }
    }
}

/// `map.fit_bounds`: frames a box on the map.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FitBounds {
    #[schemars(description = "Caja a mostrar entera (grados)")]
    bounds: LatLonBounds,
}

impl PageAction for FitBounds {
    type Page = Screen;
    const ID: &'static str = "map.fit_bounds";
    const TITLE: &'static str = "Encuadrar el mapa";
    fn description() -> &'static str {
        "Mueve el mapa de la sección Mapa hasta mostrar entera una caja de latitud y longitud"
    }
    fn into_msg(self) -> Msg {
        Msg::Fit(self.bounds)
    }
}

/// `areas.select`: selects a sector, as a click on it or on the list.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectArea {
    #[schemars(
        description = "Código del sector: AOP26000001 (Plaza de Armas), AOP26000002 \
                       (Catillo), AOP26000003 (Villa San Pedro), AOP26000004 (Palma Rosa) \
                       o AOP26000005 (Terminal de Buses)"
    )]
    code: String,
}

impl PageAction for SelectArea {
    type Page = Screen;
    const ID: &'static str = "areas.select";
    const TITLE: &'static str = "Seleccionar sector";
    fn description() -> &'static str {
        "Selecciona un sector de DSP en el mapa de la sección Mapa y lo encuadra con un 20 % \
         de margen; un código desconocido, o un sector en edición, no cambia nada"
    }
    fn into_msg(self) -> Msg {
        Msg::Select(self.code)
    }
}

/// The outline an agent proposes for a sector, before it is checked.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProposedGeometry {
    #[schemars(description = "Código del sector, como en areas.select")]
    code: String,
    #[schemars(
        description = "Anillo exterior del polígono como en GeoJSON: [longitud, latitud] por \
                       vértice, al menos 3, sin bordes que se crucen; el primero puede repetirse \
                       al final"
    )]
    coordinates: Vec<[f64; 2]>,
}

/// `areas.edit_geometry`: replaces a sector's outline; the input is
/// checked as it is read, so an invalid one is an input error.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(try_from = "ProposedGeometry")]
pub struct EditGeometry {
    code: String,
    ring_points: Vec<LatLon>,
}

impl TryFrom<ProposedGeometry> for EditGeometry {
    type Error = String;

    fn try_from(proposed: ProposedGeometry) -> Result<Self, String> {
        if !SECTORS.iter().any(|sector| sector.code == proposed.code) {
            return Err(format!("no hay un sector {}", proposed.code));
        }
        let draft = PolygonDraft::from_ring(
            proposed
                .coordinates
                .iter()
                .map(|[lon, lat]| LatLon::new(*lat, *lon)),
        )?;
        if let Some(bad) = draft
            .ring()
            .iter()
            .find(|at| at.lat.abs() > 90.0 || at.lon.abs() > 180.0)
        {
            return Err(format!("[{}, {}] está fuera del mundo", bad.lon, bad.lat));
        }
        if let Some((a, b)) = crossing(draft.ring()) {
            return Err(format!("los bordes {} y {} se cruzan", a + 1, b + 1));
        }
        Ok(Self {
            code: proposed.code,
            ring_points: draft.ring().to_vec(),
        })
    }
}

impl PageAction for EditGeometry {
    type Page = Screen;
    const ID: &'static str = "areas.edit_geometry";
    const TITLE: &'static str = "Editar la geometría de un sector";
    fn description() -> &'static str {
        "Reemplaza el polígono de un sector de DSP en el mapa de la sección Mapa, lo selecciona y \
         lo encuadra; una geometría inválida es un error y no cambia nada"
    }
    fn into_msg(self) -> Msg {
        Msg::SaveGeometry {
            code: self.code,
            ring_points: self.ring_points,
        }
    }
}

/// `map.get_view`: what the map shows, as of the last gesture.
#[derive(Deserialize, JsonSchema)]
pub struct MapViewQuery;

impl PageQuery for MapViewQuery {
    type Page = Screen;
    type Output = MapView;
    const ID: &'static str = "map.get_view";
    const TITLE: &'static str = "Vista del mapa";
    fn run(page: &Screen, _: &State, _: Self) -> Result<MapView, CommandError> {
        Ok(page.view)
    }
}
