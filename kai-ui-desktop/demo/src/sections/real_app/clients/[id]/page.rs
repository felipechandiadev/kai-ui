//! /real-app/clients/:id: a client's page, its activity and, with
//! «Editar», the same form as /real-app/clients/new filled with it.

use kai::components::{
    AlertVariant, ButtonVariant, EmptyState, PageHeader, Toast, button, caption, card,
    card_content, card_header, page, page_body, page_grid, page_grid_item,
};
use kai::element::{Element, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::crm::cards::client_facts;
use crate::crm::editor::{ClientEditor, EditorMsg, Outcome};
use crate::state::State;

pub struct Screen {
    id: u32,
    /// The form, while editing.
    editor: Option<ClientEditor>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Edit,
    Editor(EditorMsg),
    Open(u32),
    ShowOnMap,
    List,
}

impl Page for Screen {
    type Msg = Msg;
    type Params = u32;
    type State = State;

    fn init(id: &u32, _: &State) -> (Self, Effects<Msg>) {
        (
            Self {
                id: *id,
                editor: None,
            },
            Effects::none(),
        )
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Edit => self.editor = Some(ClientEditor::new(&cx.state().crm, Some(self.id))),
            Msg::Editor(msg) => {
                let Some(editor) = &mut self.editor else {
                    return Effects::none();
                };
                let (effects, outcome) = editor.update(msg, &mut cx.state().crm);
                match outcome {
                    Outcome::Editing => return effects.map(Msg::Editor),
                    Outcome::Saved(_) => {
                        cx.toast(Toast::new(0, AlertVariant::Success, "Ficha guardada"));
                        self.editor = None;
                    }
                    Outcome::Cancelled => self.editor = None,
                }
            }
            Msg::Open(id) => cx.navigate(format!("/real-app/clients/{id}")),
            Msg::ShowOnMap => {
                cx.state().crm.focus = Some(self.id);
                cx.navigate("/real-app/map");
            }
            Msg::List => cx.navigate("/real-app/clients"),
        }
        Effects::none()
    }

    fn view(&self, state: &State) -> Element<Msg> {
        if let Some(editor) = &self.editor {
            return editor.view(&state.crm).map(Msg::Editor);
        }
        let crm = &state.crm;
        let Some(client) = crm.client(self.id) else {
            return page().child(
                page_body().child(
                    EmptyState::resource("Cliente no encontrado")
                        .description(format!("No hay un cliente N.º {}.", self.id))
                        .action("Volver a la lista", Msg::List)
                        .test_id("real_app.client.missing")
                        .build(),
                ),
            );
        };
        let mut actions = vec![
            button("Editar", ButtonVariant::Primary)
                .on_click(Msg::Edit)
                .test_id("real_app.client.edit"),
            button("Ver en mapa", ButtonVariant::Outlined)
                .on_click(Msg::ShowOnMap)
                .test_id("real_app.client.map"),
        ];
        if let Some(by) = client.referred_by.and_then(|id| crm.client(id)) {
            actions.push(
                button(format!("Ver a {}", by.name), ButtonVariant::Text)
                    .on_click(Msg::Open(by.id))
                    .test_id("real_app.client.referrer"),
            );
        }
        let referred: Vec<_> = crm
            .clients
            .iter()
            .filter(|c| c.referred_by == Some(client.id))
            .collect();
        let activity: Vec<Element<Msg>> = crm
            .activity
            .iter()
            .filter(|entry| entry.client == Some(client.id))
            .map(|entry| {
                div().class("flex justify-between gap-3").children([
                    text(entry.text.as_str()).class("text-sm text-foreground"),
                    caption(entry.when.as_str()),
                ])
            })
            .collect();
        let activity = if activity.is_empty() {
            vec![caption("Sin actividad registrada")]
        } else {
            activity
        };
        let header = PageHeader::new(client.name.as_str())
            .subtitle(format!("{} · {}", client.rut, client.commune))
            .breadcrumbs(["App real", "Clientes", client.name.as_str()])
            .action(
                button("Volver a la lista", ButtonVariant::Outlined)
                    .on_click(Msg::List)
                    .test_id("real_app.client.back"),
            );
        page().child(page_body().children(
            [
                header.build(),
                page_grid().children(
                    [
                        page_grid_item(7).child(card().test_id("real_app.client").children([
                            card_header("Ficha", None),
                            card_content().class("gap-4").children([
                                client_facts(crm, client, "real_app.client.facts"),
                                div().class("flex flex-wrap gap-2").children(actions),
                            ]),
                        ])),
                        page_grid_item(5).child(
                            card().test_id("real_app.client.activity").children(
                                [
                                    card_header("Actividad", None),
                                    card_content().class("gap-2").children(activity),
                                    card_header(
                                        "Clientes que refirió",
                                        Some(&format!("{} en total", referred.len())),
                                    ),
                                    card_content().class("gap-1").children(referred.iter().map(
                                        |c| {
                                            button(c.name.as_str(), ButtonVariant::Text)
                                                .on_click(Msg::Open(c.id))
                                                .test_id(format!(
                                                    "real_app.client.referred.{}",
                                                    c.id
                                                ))
                                        },
                                    )),
                                ],
                            ),
                        ),
                    ],
                ),
            ],
        ))
    }

    fn title(&self, state: &State) -> String {
        state
            .crm
            .client(self.id)
            .map_or_else(|| "Cliente".to_owned(), |c| c.name.clone())
    }
}
