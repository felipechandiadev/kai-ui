//! /customers/new: a fixed path next to /customers/:id; it must win.

use kai::element::{Element, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

pub struct Screen;

#[derive(Clone, Debug)]
pub enum Msg {}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self, Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {}
    }

    fn view(&self, _: &State) -> Element<Msg> {
        text("Cliente nuevo").test_id("customers.new.form")
    }

    fn title(&self, _: &State) -> String {
        "Cliente nuevo".into()
    }
}
