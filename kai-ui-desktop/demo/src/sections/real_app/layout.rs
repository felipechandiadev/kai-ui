//! Around every page of «App real»: the navigation between its panel, its
//! clients and its map ([`section_tabs`], chosen from the route) in the
//! center of the window's top bar, its actions on the right of it and its
//! note in the bottom bar; the "Aurora 2" background, still, under it all.

use kai::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
use kai::components::{section_tabs, status_text, toolbar_item};
use kai::element::{Element, div};
use kai::icons::Icon;
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
    /// The bell of the bar (no notices yet).
    Notices,
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
            Msg::Notices => {}
        }
        Effects::none()
    }

    /// Its tabs in the center of the root's top bar, its bell on the right
    /// of it and its note in the bottom bar.
    fn slot(&self, name: &str, _: &State, route: &RouteInfo<'_>) -> Option<Element<Msg>> {
        use crate::layout::{SLOT_BOTTOM_LEADING, SLOT_TOP_CENTER, SLOT_TOP_TRAILING};
        match name {
            SLOT_TOP_CENTER => Some(
                section_tabs(&PARTS, route.path, Msg::Go)
                    .label("Secciones de App real")
                    .test_id("real_app.nav"),
            ),
            SLOT_TOP_TRAILING => Some(
                toolbar_item(Icon::Bell, "Avisos")
                    .on_click(Msg::Notices)
                    .test_id("real_app.notices"),
            ),
            SLOT_BOTTOM_LEADING => Some(
                status_text("App real · datos de ejemplo en memoria").test_id("real_app.status"),
            ),
            _ => None,
        }
    }

    /// "Aurora 2", still: drawn once and cached (glass goes on the bars,
    /// the content stays solid).
    fn background(&self, _: &State) -> Option<BackgroundEffect> {
        Some(
            BackgroundEffect::new(BackgroundKind::Aurora2).with_motion(BackgroundMotion::Static),
        )
    }

    fn view(&self, _: &State, _: &RouteInfo<'_>) -> Element<Msg> {
        div().class("flex flex-col flex-1 min-h-0").child(outlet())
    }
}
