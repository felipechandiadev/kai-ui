//! Around every page of «App real»: the bar that moves between its panel,
//! its clients and its map, chosen from the route.

use kai::components::{caption, segmented};
use kai::element::{Element, div};
use kai::router::{Context, Layout, RouteInfo, outlet};
use kai::runtime::Effects;

use crate::state::State;

/// The parts of the app, in the order of the bar.
const PARTS: [(&str, &str); 3] = [
    ("Panel", "/real-app"),
    ("Clientes", "/real-app/clients"),
    ("Mapa", "/real-app/map"),
];

pub struct Frame;

#[derive(Clone, Debug)]
pub enum Msg {
    Go(usize),
}

/// The part of the bar `path` is in.
fn part_of(path: &str) -> usize {
    PARTS
        .iter()
        .rposition(|(_, prefix)| path == *prefix || path.starts_with(&format!("{prefix}/")))
        .unwrap_or(0)
}

impl Layout for Frame {
    type Msg = Msg;
    type State = State;

    fn init(_: &State) -> (Self, Effects<Msg>) {
        (Self, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Go(index) => {
                // From the bar the map opens without a client.
                cx.state().crm.focus = None;
                if let Some((_, path)) = PARTS.get(index) {
                    cx.navigate(*path);
                }
            }
        }
        Effects::none()
    }

    fn view(&self, _: &State, route: &RouteInfo<'_>) -> Element<Msg> {
        let names = PARTS.map(|(name, _)| name);
        div().class("flex flex-col flex-1 min-h-0").children([
            div()
                .class("flex items-center justify-between gap-3 px-6 pt-4")
                .children([
                    segmented(&names, part_of(route.path), Msg::Go)
                        .label("Secciones de App real")
                        .test_id("real_app.nav"),
                    caption("Datos de ejemplo en memoria · latencia simulada"),
                ]),
            outlet(),
        ])
    }
}
