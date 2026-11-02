//! Around /customers and every page below it. No messages: it only draws,
//! but it lives while the route stays in the folder. It also gives the
//! window its theme and background on those routes (docs/dev/31).

use std::convert::Infallible;

use kai::background::{BackgroundEffect, BackgroundKind};
use kai::element::{Element, div, text};
use kai::router::{Layout, RouteInfo, outlet};
use kai::runtime::Effects;
use kai::theme::{Density, MixStyle, Theme};

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

    /// The app's Kai theme, marked with a mix that Kai only records: the
    /// tests can tell it apart and nothing on screen moves.
    fn theme(&self, _: &State) -> Option<Theme> {
        Some(Theme::kai(Density::Comfortable).with_mix(MixStyle::A))
    }

    fn background(&self, _: &State) -> Option<BackgroundEffect> {
        Some(BackgroundEffect::new(BackgroundKind::Glow))
    }

    fn view(&self, _: &State, _: &RouteInfo<'_>) -> Element<Infallible> {
        div().class("flex flex-col flex-1 min-h-0").children([
            text(format!("Visto al abrir: {}", self.opened_with)).test_id("customers.frame"),
            outlet(),
        ])
    }
}
