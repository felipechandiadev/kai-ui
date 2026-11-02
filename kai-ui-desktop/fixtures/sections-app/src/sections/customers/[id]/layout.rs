//! Around one customer: a counter that lives while the route keeps the same
//! `[id]`; another customer, another layout.

use kai::components::{ButtonVariant, button};
use kai::element::{Element, div};
use kai::router::{Layout, RouteInfo, outlet};
use kai::runtime::Effects;

use crate::state::State;

pub struct Frame {
    presses: u32,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Press,
}

impl Layout for Frame {
    type Msg = Msg;
    type State = State;

    fn init(_: &State) -> (Self, Effects<Msg>) {
        (Self { presses: 0 }, Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut kai::router::Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Press => self.presses += 1,
        }
        Effects::none()
    }

    fn view(&self, _: &State, _: &RouteInfo<'_>) -> Element<Msg> {
        div().class("flex flex-col flex-1 min-h-0").children([
            button(format!("Visitas {}", self.presses), ButtonVariant::Secondary)
                .test_id("customer.visits")
                .on_click(Msg::Press),
            outlet(),
        ])
    }
}
