//! /real-app/map: the clients of «App real» on the local map of Parral
//! (PMTiles) with the DSP sectors over it. Choosing a client, in the list
//! or by its pin, highlights its pin and its sector and frames it;
//! choosing a sector frames it and counts its clients. «Ver en mapa» in
//! the CRM opens it on a client (`crm.focus`).

use kai::components::{
    ButtonVariant, CHOICE_ACTIVE, CHOICE_HOVER, button, caption, card, card_content, card_header,
    page, page_body,
};
use kai::element::{Element, Role, div, icon, text};
use kai::geo::{LatLonBounds, MapView};
use kai::icons::Icon;
use kai::maps::style::MapStyle;
use kai::maps::tiles::TileSource;
use kai::maps::{Anchor, map, marker};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::crm::sector_name;
use crate::sections::map::page::{INITIAL_VIEW, SECTORS};
use crate::state::State;

/// The same Protomaps extract as /map.
const PARRAL_PMTILES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/maps/parral.pmtiles");
/// Margin around a framed sector, as a fraction of its size per side.
const MARGIN: f64 = 0.2;

pub struct Screen {
    view: MapView,
    /// The chosen client.
    client: Option<u32>,
    /// Code of the highlighted sector.
    area: Option<String>,
    /// The box to frame and a serial, so the same box frames again.
    fit_request: Option<(LatLonBounds, u64)>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Moved(MapView),
    Select(u32),
    Area(String),
    Open(u32),
}

impl Screen {
    fn frame(&mut self, code: &str) {
        if let Some(sector) = SECTORS.iter().find(|s| s.code == code) {
            let serial = self.fit_request.map_or(1, |(_, serial)| serial + 1);
            self.fit_request = Some((sector.layer.bounds().padded(MARGIN), serial));
        }
    }

    fn select(&mut self, id: u32, state: &State) {
        let Some(client) = state.crm.client(id) else {
            return;
        };
        let code = client.sector.clone();
        self.client = Some(id);
        self.frame(&code);
        self.area = Some(code);
    }

    /// What the map shows, for the status line (and screen readers).
    fn status(&self, state: &State) -> String {
        let crm = &state.crm;
        match (self.client.and_then(|id| crm.client(id)), &self.area) {
            (Some(client), _) => format!(
                "{} · sector {} · {}",
                client.name,
                sector_name(&client.sector),
                client.commune
            ),
            (None, Some(code)) => {
                let count = crm.clients.iter().filter(|c| &c.sector == code).count();
                format!(
                    "Sector {} · {count} {}",
                    sector_name(code),
                    if count == 1 { "cliente" } else { "clientes" }
                )
            }
            (None, None) => "Elige un cliente en la lista o en el mapa".to_owned(),
        }
    }
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), state: &State) -> (Self, Effects<Msg>) {
        let mut screen = Self {
            view: INITIAL_VIEW,
            client: None,
            area: None,
            fit_request: None,
        };
        if let Some(id) = state.crm.focus {
            screen.select(id, state);
        }
        (screen, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Moved(view) => self.view = view,
            Msg::Select(id) => {
                self.select(id, cx.state());
                cx.state().crm.focus = self.client;
            }
            Msg::Area(code) => {
                let inside = self
                    .client
                    .and_then(|id| cx.state().crm.client(id))
                    .is_some_and(|c| c.sector == code);
                if !inside {
                    self.client = None;
                }
                self.frame(&code);
                self.area = Some(code);
            }
            Msg::Open(id) => cx.navigate(format!("/real-app/clients/{id}")),
        }
        Effects::none()
    }

    fn view(&self, state: &State) -> Element<Msg> {
        let crm = &state.crm;
        let area = self.area.as_deref();
        let layers = SECTORS.iter().map(|sector| {
            let color = sector.color();
            sector
                .layer
                .clone()
                .id(&sector.code)
                .fill(color, 0.12)
                .stroke(color, 2.0)
                .label(&sector.name)
                .selected(area == Some(sector.code.as_str()))
        });
        let pins = crm.clients.iter().map(|client| {
            let chosen = self.client == Some(client.id);
            let look = if chosen {
                "size-10 text-primary"
            } else {
                "size-7 text-muted-foreground"
            };
            marker(
                client.location,
                Anchor::Bottom,
                div()
                    .role(Role::Button)
                    .pressed(chosen)
                    .label(client.name.as_str())
                    .on_click(Msg::Select(client.id))
                    .test_id(format!("real_app.map.pin.{}", client.id))
                    .class("cursor-pointer")
                    .child(icon(Icon::MapPin).class(look)),
            )
            .alt(client.name.as_str())
        });
        let mut map_view = map(&self.view)
            .basemap(TileSource::pmtiles(PARRAL_PMTILES, MapStyle::protomaps()))
            .layers(layers)
            .animate_view(true)
            .on_view_change(Msg::Moved)
            .on_area_click(Msg::Area)
            .label("Clientes en el mapa de Parral")
            .test_id("real_app.map.view")
            .class("flex-1 min-w-0 h-[480px] border border-border")
            .children(pins);
        if let Some((bounds, serial)) = self.fit_request {
            map_view = map_view.fit_bounds(bounds, serial);
        }
        let list = div()
            .role(Role::List)
            .label("Clientes")
            .test_id("real_app.map.clients")
            .class("flex flex-col gap-1 w-56")
            .children(crm.clients.iter().map(|client| {
                let chosen = self.client == Some(client.id);
                div()
                    .role(Role::Button)
                    .pressed(chosen)
                    .label(client.name.as_str())
                    .on_click(Msg::Select(client.id))
                    .test_id(format!("real_app.map.client.{}", client.id))
                    .class(&format!(
                        "flex flex-col px-2 py-1 rounded-md border cursor-pointer \
                         focus-visible:ring-2 focus-visible:ring-primary/40 {}",
                        if chosen { CHOICE_ACTIVE } else { CHOICE_HOVER }
                    ))
                    .children([
                        text(client.name.as_str()).class("text-sm text-foreground"),
                        text(sector_name(&client.sector)).class("text-xs text-muted-foreground"),
                    ])
            }));
        let open = self.client.map(|id| {
            button("Abrir ficha", ButtonVariant::Outlined)
                .on_click(Msg::Open(id))
                .test_id("real_app.map.open")
        });
        page().child(
            page_body().child(
                card().class(&kai::components::themed("", "bg-window")).class("w-full").test_id("real_app.map").children([
                    card_header(
                        "Mapa de clientes",
                        Some("Mapa local (PMTiles) con los sectores DSP de Parral"),
                    ),
                    card_content().class("gap-3").children([
                        div().class("flex gap-4").children([list, map_view]),
                        div()
                            .class("flex items-center justify-between gap-3")
                            .children([caption(self.status(state))
                                .role(Role::Status)
                                .test_id("real_app.map.status")])
                            .children(open),
                    ]),
                ]),
            ),
        )
    }

    fn title(&self, _: &State) -> String {
        "Mapa de clientes".into()
    }
}
