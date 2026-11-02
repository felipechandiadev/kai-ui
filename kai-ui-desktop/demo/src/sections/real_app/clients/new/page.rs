//! /real-app/clients/new: the client form, empty. Saved, it goes back to
//! the list, which selects the new client.

use kai::components::{AlertVariant, Toast};
use kai::element::Element;
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::crm::editor::{ClientEditor, EditorMsg, Outcome};
use crate::state::State;

pub struct Screen {
    editor: ClientEditor,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Editor(EditorMsg),
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), state: &State) -> (Self, Effects<Msg>) {
        let editor = ClientEditor::new(&state.crm, None);
        (Self { editor }, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        let Msg::Editor(msg) = msg;
        let (effects, outcome) = self.editor.update(msg, &mut cx.state().crm);
        match outcome {
            Outcome::Editing => {}
            Outcome::Saved(id) => {
                let name = cx
                    .state()
                    .crm
                    .client(id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                cx.toast(Toast::new(
                    0,
                    AlertVariant::Success,
                    format!("{name} guardado"),
                ));
                cx.navigate("/real-app/clients");
            }
            Outcome::Cancelled => cx.navigate("/real-app/clients"),
        }
        effects.map(Msg::Editor)
    }

    fn view(&self, state: &State) -> Element<Msg> {
        self.editor.view(&state.crm).map(Msg::Editor)
    }

    fn title(&self, _: &State) -> String {
        "Nuevo cliente".into()
    }
}
