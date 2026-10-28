//! Around every page: a bar with the title and a counter whose state must
//! survive navigation (docs/dev/28).

use kai::components::{ButtonVariant, button};
use kai::element::{Element, div, text};
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

    fn view(&self, _: &State, route: &RouteInfo<'_>) -> Element<Msg> {
        div().class("flex flex-col w-full h-full").children([
            div().class("flex flex-row gap-2").test_id("frame.bar").children([
                text(route.title).test_id("frame.title"),
                text(route.path).test_id("frame.path"),
                button(format!("Pulsado {}", self.presses), ButtonVariant::Secondary)
                    .test_id("frame.press")
                    .on_click(Msg::Press),
            ]),
            outlet(),
        ])
    }
}
