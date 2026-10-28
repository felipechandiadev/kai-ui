//! Around /customers and every page below it. No messages: it only draws,
//! but it lives while the route stays in the folder.

use std::convert::Infallible;

use kai::element::{Element, div, text};
use kai::router::{Layout, RouteInfo, outlet};
use kai::runtime::Effects;

use crate::state::State;

pub struct Frame {
    /// The second customer as it was when the layout was created.
    opened_with: String,
}

impl Layout for Frame {
    type Msg = Infallible;
    type State = State;

    fn init(state: &State) -> (Self, Effects<Infallible>) {
        (
            Self {
                opened_with: state.customers[1].clone(),
            },
            Effects::none(),
        )
    }

    fn view(&self, _: &State, _: &RouteInfo<'_>) -> Element<Infallible> {
        div().class("flex flex-col flex-1 min-h-0").children([
            text(format!("Visto al abrir: {}", self.opened_with)).test_id("customers.frame"),
            outlet(),
        ])
    }
}
