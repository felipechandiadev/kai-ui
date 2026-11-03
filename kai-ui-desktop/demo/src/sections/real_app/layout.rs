//! Around every page of «App real»: the navigation between its panel, its
//! clients and its map ([`section_tabs`], chosen from the route) in the
//! center of the window's top bar, its actions on the right of it and its
//! note in the bottom bar; the "Aurora 2" background, still, under it all.

use kai::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
use kai::components::{Avatar, AvatarSize, app_sidebar, section_tabs, status_text, toolbar_item};
use kai::element::{Element, div};
use kai::icons::Icon;
use kai::router::{Context, Layout, RouteInfo, outlet};
use kai::runtime::Effects;

use crate::state::State;

/// The app's own floating sidebar, in place of the demo's sections (the
/// last row goes back to the demo). An empty path: not there yet.
const NAV: [(Icon, &str, &str); 6] = [
    (Icon::LayoutDashboard, "Resumen", "/real-app"),
    (Icon::Users, "Clientes", "/real-app/clients"),
    (Icon::Map, "Mapa", "/real-app/map"),
    (Icon::FolderKanban, "Oportunidades", ""),
    (Icon::Layers, "Laboratorio", ""),
    (Icon::House, "Volver a la demo", "/home"),
];

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
    /// A row of the sidebar.
    Nav(usize),
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
            Msg::Nav(index) => {
                cx.state().crm.focus = None;
                if let Some((_, _, path)) = NAV.get(index).filter(|row| !row.2.is_empty()) {
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
                div().class("flex items-center gap-2").children([
                    toolbar_item(Icon::Bell, "Avisos")
                        .on_click(Msg::Notices)
                        .test_id("real_app.notices"),
                    Avatar::new("Tú")
                        .size(AvatarSize::Xs)
                        .test_id("real_app.me")
                        .build(),
                ]),
            ),
            SLOT_BOTTOM_LEADING => Some(
                status_text("App real · datos de ejemplo en memoria").test_id("real_app.status"),
            ),
            _ => None,
        }
    }

    /// Its own floating glass sidebar, only the rows of the app.
    fn sidebar(&self, _: &State, route: &RouteInfo<'_>) -> Option<Element<Msg>> {
        Some(
            app_sidebar(&NAV, route.path, Msg::Nav)
                .label("App real")
                .test_id("real_app.sidebar"),
        )
    }

    /// "Aurora 2", still: drawn once and cached (glass goes on the bars,
    /// the content stays solid).
    fn background(&self, _: &State) -> Option<BackgroundEffect> {
        Some(BackgroundEffect::new(BackgroundKind::Aurora2).with_motion(BackgroundMotion::Static))
    }

    fn view(&self, _: &State, _: &RouteInfo<'_>) -> Element<Msg> {
        div().class("flex flex-col flex-1 min-h-0").child(outlet())
    }
}
