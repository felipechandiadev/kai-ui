//! The bar across the top of the window, around every page: the sidebar
//! toggle, back and forward, and the title of the page on screen. Its
//! buttons run the framework's commands, so it has no messages of its own.

use std::convert::Infallible;

use kai::components::{toolbar, toolbar_item, toolbar_title};
use kai::element::{Element, div};
use kai::icons::Icon;
use kai::router::{Layout, RouteInfo, outlet};
use kai::runtime::Effects;

use crate::state::State;

/// The frame around the content, to the right of the sidebar.
pub struct Frame;

impl Layout for Frame {
    type Msg = Infallible;
    type State = State;

    fn init(_: &State) -> (Self, Effects<Infallible>) {
        (Self, Effects::none())
    }

    fn view(&self, _: &State, route: &RouteInfo<'_>) -> Element<Infallible> {
        let app = format!("Kai UI Desktop v{}", kai::version::VERSION);
        let sidebar = if route.sidebar_collapsed {
            "Mostrar barra lateral"
        } else {
            "Ocultar barra lateral"
        };
        div().class("flex flex-col flex-1 min-h-0").children([
            toolbar([
                toolbar_item(Icon::PanelLeft, sidebar)
                    .run_command("kai.toggle_sidebar")
                    .test_id("bar.sidebar"),
                toolbar_item(Icon::ChevronLeft, "Atrás")
                    .run_command("kai.back")
                    .test_id("bar.back"),
                toolbar_item(Icon::ChevronRight, "Adelante")
                    .run_command("kai.forward")
                    .test_id("bar.forward"),
                toolbar_title(route.title, Some(&app)).test_id("bar.title"),
            ]),
            outlet(),
        ])
    }
}
