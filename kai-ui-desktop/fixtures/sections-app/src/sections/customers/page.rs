//! /customers: the list, with a search that stays while other sections show.

use kai::components::{ButtonVariant, TextField, button};
use kai::element::{Element, div};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

pub struct Screen {
    query: String,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Query(String),
    Open(usize),
    New,
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self { query: String::new() }, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Query(query) => self.query = query,
            Msg::Open(index) => cx.navigate(format!("/customers/{index}")),
            Msg::New => cx.navigate("/customers/new"),
        }
        Effects::none()
    }

    fn view(&self, state: &State) -> Element<Msg> {
        let query = self.query.to_lowercase();
        let rows = state
            .customers
            .iter()
            .enumerate()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .map(|(index, name)| {
                button(name.as_str(), ButtonVariant::Text)
                    .on_click(Msg::Open(index))
                    .test_id(format!("customers.row.{index}"))
            });
        div().children(
            [
                TextField::new("Buscar", &self.query)
                    .on_input(Msg::Query)
                    .test_id("customers.find")
                    .build(),
                button("Nuevo", ButtonVariant::Primary)
                    .on_click(Msg::New)
                    .test_id("customers.new"),
            ]
            .into_iter()
            .chain(rows),
        )
    }

    fn title(&self, _: &State) -> String {
        "Clientes".into()
    }
}
