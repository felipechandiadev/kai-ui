//! /customers/:id: rename a customer in the shared state.

use kai::components::{ButtonVariant, TextField, button};
use kai::element::{Element, div};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

pub struct Screen {
    index: usize,
    draft: String,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Draft(String),
    Save,
}

impl Page for Screen {
    type Msg = Msg;
    type Params = usize;
    type State = State;

    fn init(index: &usize, state: &State) -> (Self, Effects<Msg>) {
        let draft = state.customers.get(*index).cloned().unwrap_or_default();
        (Self { index: *index, draft }, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Draft(draft) => self.draft = draft,
            Msg::Save => {
                if let Some(name) = cx.state().customers.get_mut(self.index) {
                    name.clone_from(&self.draft);
                }
                cx.back();
            }
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        div().children([
            TextField::new("Nombre", &self.draft)
                .on_input(Msg::Draft)
                .test_id("customer.name")
                .build(),
            button("Guardar", ButtonVariant::Primary)
                .on_click(Msg::Save)
                .test_id("customer.save"),
        ])
    }

    fn title(&self, state: &State) -> String {
        state.customers.get(self.index).cloned().unwrap_or_default()
    }
}
