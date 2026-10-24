//! /sales: loads its total when it opens.

use kai::element::{Element, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

pub struct Screen {
    total: Option<u32>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Loaded(u32),
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self { total: None }, Effects::perform(|| 1250, Msg::Loaded))
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Loaded(total) => self.total = Some(total),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let total = self.total.map_or("Cargando…".into(), |total| format!("Total {total}"));
        text(total).test_id("sales.total")
    }

    fn title(&self, _: &State) -> String {
        "Ventas".into()
    }
}
