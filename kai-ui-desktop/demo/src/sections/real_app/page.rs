//! /real-app: the «Resumen» of «App real», three solid panes over the
//! window background (docs/dev/28 §3.3, design patterns in
//! docs/dev/33-app-real.md, «Patrones de diseño»):
//!
//! - the list of clients with a colored chip of their stage (list with
//!   status chips) and a footer with the count and quick actions;
//! - the detail of the client chosen (master–detail): contact facts and
//!   its recent activity;
//! - the inspector on the right (inspector): stage, owner and tags.
//!
//! The first visit waits for the «server».

use kai::components::{AlertVariant, ListAction, SearchReveal, Toast, list_toolbar};
use kai::components::{
    ButtonVariant, Dialog, DialogWidth, IconButtonSize, IconButtonVariant, button, caption,
    heading, icon_button_sized, loading_placeholder, page, page_body, pane, panes, select, themed,
};
use kai::element::{Element, Role, div, icon, text};
use kai::icons::Icon;
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::crm::{Client, LATENCY, Stage, sector_name};
use crate::state::State;

const STAGES: [Stage; 3] = [Stage::Prospect, Stage::Active, Stage::Inactive];

/// Orders of the list's menu; none chosen keeps the CRM's order.
const SORTS: [&str; 3] = ["Nombre A–Z", "Nombre Z–A", "Etapa"];
/// Filters by stage; 0 shows them all.
const FILTERS: [&str; 4] = ["Todos", "Activo", "Prospecto", "Inactivo"];

#[derive(Default)]
pub struct Screen {
    loaded: bool,
    /// The client in the detail; the last saved, the one of the map or the
    /// first when none was chosen.
    selected: Option<u32>,
    /// The client whose deletion waits for confirmation.
    deleting: Option<u32>,
    /// Text of the list's search, and how open its field is.
    query: String,
    reveal: SearchReveal,
    /// Chosen order ([`SORTS`]) and stage filter ([`FILTERS`]).
    sort: Option<usize>,
    filter: usize,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Loaded,
    Select(u32),
    Open(u32),
    Stage(u32, usize),
    Query(String),
    OpenSearch,
    CloseSearch,
    /// A frame of the search's animation.
    Reveal,
    Sort(usize),
    Filter(usize),
    Download,
    NewClient,
    AddTag,
    /// Asks to delete the client (opens the confirmation).
    Delete(u32),
    ConfirmDelete,
    CancelDelete,
}

/// Initials of `name` in a small circle (the client has no photo).
pub fn initials<Msg>(name: &str, large: bool) -> Element<Msg> {
    let letters: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect();
    div()
        .class(if large {
            "flex items-center justify-center size-12 shrink-0 rounded-full bg-info/15 \
             text-info text-base font-semibold"
        } else {
            "flex items-center justify-center size-8 shrink-0 rounded-full bg-info/15 \
             text-info text-xs font-semibold"
        })
        .alt(name.to_owned())
        .child(text(letters))
}

/// The chip of a stage: its color, soft.
pub fn stage_chip<Msg>(stage: Stage) -> Element<Msg> {
    let tone = match stage {
        Stage::Prospect => "bg-info/15 text-info".to_owned(),
        Stage::Active => "bg-success/15 text-success".to_owned(),
        Stage::Inactive => themed(
            "bg-muted text-muted-foreground",
            "bg-fill-3 text-secondary-label",
        ),
    };
    div()
        .class(&format!(
            "inline-flex items-center h-[20px] px-2 shrink-0 rounded-[6px] text-xs font-medium {tone}"
        ))
        .role(Role::Status)
        .child(text(stage.label()))
}

fn section_title<Msg>(title: &str) -> Element<Msg> {
    text(title).class("text-sm font-semibold text-foreground")
}

impl Screen {
    /// The clients the list shows: filtered by stage and by the search
    /// (name or sector), in the chosen order.
    pub fn visible<'a>(&self, state: &'a State) -> Vec<&'a Client> {
        let query = self.query.trim().to_lowercase();
        let stage = match self.filter {
            1 => Some(Stage::Active),
            2 => Some(Stage::Prospect),
            3 => Some(Stage::Inactive),
            _ => None,
        };
        let mut clients: Vec<&Client> = state
            .crm
            .clients
            .iter()
            .filter(|c| stage.is_none_or(|stage| c.stage == stage))
            .filter(|c| {
                query.is_empty()
                    || c.name.to_lowercase().contains(&query)
                    || sector_name(&c.sector).to_lowercase().contains(&query)
            })
            .collect();
        match self.sort {
            Some(0) => clients.sort_by_key(|c| c.name.to_lowercase()),
            Some(1) => clients.sort_by_key(|c| std::cmp::Reverse(c.name.to_lowercase())),
            Some(2) => clients.sort_by_key(|c| STAGES.iter().position(|s| *s == c.stage)),
            _ => {}
        }
        clients
    }

    /// The client in the detail, among those the list shows: when the
    /// chosen one is filtered out, the first one shown.
    fn chosen<'a>(&self, state: &'a State, visible: &[&'a Client]) -> Option<&'a Client> {
        let crm = &state.crm;
        let id = self.selected.or(crm.last_saved).or(crm.focus);
        id.and_then(|id| visible.iter().copied().find(|c| c.id == id))
            .or(visible.first().copied())
    }

    fn list(&self, state: &State, visible: &[&Client], chosen: Option<u32>) -> Element<Msg> {
        let total = state.crm.clients.len();
        let narrowed = self.filter != 0 || !self.query.trim().is_empty();
        let info = if narrowed {
            format!("{} de {total} clientes", visible.len())
        } else {
            format!("{total} clientes")
        };
        let rows = visible.iter().map(|client| {
            let on = Some(client.id) == chosen;
            div()
                .class(&themed(
                    "flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer \
                     hover:bg-muted/40 selected:bg-choice/15",
                    "flex items-center gap-3 px-2.5 py-2 rounded-[10px] cursor-default \
                     hover:bg-fill-3 selected:bg-choice/15",
                ))
                .role(Role::Button)
                .label(client.name.as_str())
                .pressed(on)
                .test_id(format!("real_app.list.{}", client.id))
                .on_click(Msg::Select(client.id))
                .children([
                    initials(&client.name, false),
                    div().class("flex flex-col flex-1 min-w-0").children([
                        text(client.name.as_str())
                            .class("text-sm font-medium text-foreground truncate"),
                        caption(sector_name(&client.sector)),
                    ]),
                    stage_chip(client.stage),
                ])
        });
        pane()
            .class("w-[300px] shrink-0")
            .label("Clientes")
            .test_id("real_app.list")
            .children([
                div()
                    .class("flex flex-col gap-0.5 p-2 flex-1 min-h-0 overflow-y-auto")
                    .children(rows),
                list_toolbar(info)
                    .action(
                        ListAction::button(Icon::FileDown, "Descargar", Msg::Download)
                            .test_id("real_app.list.download"),
                    )
                    .action(
                        ListAction::menu(
                            Icon::ListOrdered,
                            "Ordenar",
                            &SORTS,
                            self.sort,
                            Msg::Sort,
                        )
                        .active(self.sort.is_some())
                        .test_id("real_app.list.sort"),
                    )
                    .action(
                        ListAction::menu(
                            Icon::Funnel,
                            "Filtrar",
                            &FILTERS,
                            Some(self.filter),
                            Msg::Filter,
                        )
                        .active(self.filter != 0)
                        .test_id("real_app.list.filter"),
                    )
                    .action(
                        ListAction::button(Icon::Plus, "Añadir cliente", Msg::NewClient)
                            .test_id("real_app.list.add"),
                    )
                    .search(
                        self.query.clone(),
                        self.reveal,
                        Msg::Query,
                        Msg::OpenSearch,
                        Msg::CloseSearch,
                    )
                    .placeholder("Nombre o sector")
                    .test_id("real_app.list.bar")
                    .build(),
            ])
    }

    fn detail(&self, state: &State, client: &Client) -> Element<Msg> {
        let fact = |label: &str, value: String, id: &str| {
            div().class("flex items-center gap-3 py-1").children([
                caption(label.to_owned()).class("w-28 shrink-0"),
                text(value)
                    .class("text-sm text-foreground truncate")
                    .test_id(format!("real_app.detail.{id}")),
            ])
        };
        let visit = if client.visit.is_empty() {
            "Sin visita".to_owned()
        } else {
            format!("{} {}", client.visit, client.visit_time)
        };
        let activity: Vec<Element<Msg>> = state
            .crm
            .activity
            .iter()
            .filter(|entry| entry.client == Some(client.id))
            .take(3)
            .enumerate()
            .map(|(index, entry)| {
                div()
                    .class("flex items-center gap-3 py-1.5")
                    .test_id(format!("real_app.detail.activity.{index}"))
                    .children([
                        div()
                            .class("flex items-center justify-center size-7 shrink-0 rounded-full bg-info/15 text-info")
                            .child(icon(Icon::MessageSquare).class("size-3.5")),
                        text(entry.text.as_str()).class("flex-1 min-w-0 text-sm text-foreground truncate"),
                        caption(entry.when.as_str()),
                    ])
            })
            .collect();
        let activity = if activity.is_empty() {
            vec![caption("Sin actividad todavía")]
        } else {
            activity
        };
        pane()
            .class("flex-1 min-w-[320px]")
            .label("Detalle del cliente")
            .test_id("real_app.detail")
            .child(
                div()
                    .class("flex flex-col gap-4 p-5 flex-1 min-h-0 overflow-y-auto")
                    .children([
                        div().class("flex items-center gap-3").children([
                            initials(&client.name, true),
                            div().class("flex flex-col flex-1 min-w-0").children([
                                heading(client.name.as_str()).test_id("real_app.detail.name"),
                                caption(sector_name(&client.sector)),
                            ]),
                            icon_button_sized(
                                Icon::Pencil,
                                "Editar",
                                IconButtonVariant::Neutral,
                                IconButtonSize::Sm,
                            )
                            .tooltip("Editar")
                            .on_click(Msg::Open(client.id))
                            .test_id("real_app.detail.edit"),
                            // Destructive, in a subtle danger tone.
                            icon_button_sized(
                                Icon::Trash2,
                                "Eliminar",
                                IconButtonVariant::Neutral,
                                IconButtonSize::Sm,
                            )
                            .class("text-error hover:bg-error/10")
                            .tooltip("Eliminar")
                            .on_click(Msg::Delete(client.id))
                            .test_id("real_app.detail.delete"),
                        ]),
                        div().class("flex flex-col").children([
                            fact("Correo", client.email.clone(), "email"),
                            fact("Teléfono", client.phone.clone(), "phone"),
                            fact("Ubicación", format!("{}, Maule", client.commune), "commune"),
                            fact("Tipo", sector_name(&client.sector), "sector"),
                            fact("Próxima visita", visit, "visit"),
                        ]),
                        div().class(&themed("h-px bg-border", "h-px bg-separator")),
                        section_title("Actividad reciente"),
                        div().class("flex flex-col").children(activity),
                        button("Ver toda la actividad ›", ButtonVariant::Text)
                            .on_click(Msg::Open(client.id))
                            .test_id("real_app.detail.all"),
                    ]),
            )
    }

    fn inspector(&self, client: &Client) -> Element<Msg> {
        let id = client.id;
        let labels = STAGES.map(Stage::label);
        let stage = STAGES.iter().position(|s| *s == client.stage);
        let tags = [
            client.commune.clone(),
            sector_name(&client.sector),
            client.stage.label().to_owned(),
        ];
        let divider = || div().class(&themed("h-px bg-border", "h-px bg-separator"));
        pane()
            .class("w-[240px] shrink-0")
            .label("Inspector")
            .test_id("real_app.inspector")
            .child(div().class("flex flex-col gap-3 p-4").children([
                section_title("Estado"),
                select(&labels, stage, "Estado", move |index| Msg::Stage(id, index))
                    .label("Estado")
                    .test_id("real_app.inspector.stage"),
                divider(),
                section_title("Responsable"),
                div().class("flex items-center gap-2").children([
                    initials("Tú", false),
                    div().class("flex flex-col min-w-0").children([
                        text("Tú").class("text-sm font-medium text-foreground"),
                        caption("tu@kaicrm.cl"),
                    ]),
                ]),
                divider(),
                section_title("Etiquetas"),
                div()
                    .class("flex flex-wrap gap-1.5")
                    .test_id("real_app.inspector.tags")
                    .children(tags.into_iter().map(|tag| {
                        div()
                            .class("inline-flex items-center h-[20px] px-2 rounded-[6px] bg-info/10 text-info text-xs")
                            .child(text(tag))
                    })),
                div()
                    .class(&themed(
                        "flex items-center justify-center gap-2 h-9 rounded-lg border \
                         border-border text-sm text-muted-foreground cursor-pointer hover:bg-muted/40",
                        "flex items-center justify-center gap-2 h-[34px] rounded-[10px] border \
                         border-separator text-control text-secondary-label \
                         cursor-default hover:bg-fill-3",
                    ))
                    .role(Role::Button)
                    .label("Añadir etiqueta")
                    .on_click(Msg::AddTag)
                    .test_id("real_app.inspector.add_tag")
                    .children([icon(Icon::Plus).class("size-3.5"), text("Añadir etiqueta")]),
            ]))
    }
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        let load = Effects::perform(|| std::thread::sleep(LATENCY), |()| Msg::Loaded);
        (Self::default(), load)
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Loaded => self.loaded = true,
            Msg::Select(id) => self.selected = Some(id),
            Msg::Open(id) => cx.navigate(format!("/real-app/clients/{id}")),
            Msg::Stage(id, index) => {
                if let (Some(client), Some(stage)) = (
                    cx.state().crm.clients.iter_mut().find(|c| c.id == id),
                    STAGES.get(index),
                ) {
                    client.stage = *stage;
                }
            }
            Msg::Query(query) => self.query = query,
            Msg::OpenSearch => return self.reveal.open(Msg::Reveal),
            Msg::CloseSearch => {
                self.query.clear();
                return self.reveal.close(Msg::Reveal);
            }
            Msg::Reveal => return self.reveal.tick(Msg::Reveal),
            Msg::Sort(index) => self.sort = Some(index),
            Msg::Filter(index) => self.filter = index,
            Msg::Download => cx.toast(Toast::new(
                0,
                AlertVariant::Info,
                "Exportación de ejemplo (datos en memoria)",
            )),
            Msg::NewClient => cx.navigate("/real-app/clients/new"),
            // Tags are not stored yet (the pattern lab will add them).
            Msg::AddTag => {}
            Msg::Delete(id) => self.deleting = Some(id),
            Msg::CancelDelete => self.deleting = None,
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting.take() {
                    let crm = &mut cx.state().crm;
                    crm.clients.retain(|c| c.id != id);
                    crm.activity.retain(|entry| entry.client != Some(id));
                    if crm.last_saved == Some(id) {
                        crm.last_saved = None;
                    }
                    if crm.focus == Some(id) {
                        crm.focus = None;
                    }
                    // The next one in the list is chosen (the first).
                    self.selected = None;
                }
            }
        }
        Effects::none()
    }

    fn view(&self, state: &State) -> Element<Msg> {
        if !self.loaded {
            return page().child(page_body().child(loading_placeholder("Cargando el panel…")));
        }
        if state.crm.clients.is_empty() {
            return page()
                .child(page_body().child(caption("No quedan clientes").test_id("real_app.empty")));
        }
        let visible = self.visible(state);
        let client = self.chosen(state, &visible);
        let list = self.list(state, &visible, client.map(|c| c.id));
        let rest = match client {
            Some(client) => vec![self.detail(state, client), self.inspector(client)],
            None => vec![
                pane()
                    .class("flex-1 min-w-[320px] items-center justify-center")
                    .test_id("real_app.detail")
                    .child(caption("Ningún cliente coincide").test_id("real_app.none")),
            ],
        };
        panes(std::iter::once(list).chain(rest)).test_id("real_app.panes")
    }

    /// Destructive confirmation: title, what is lost, a red Eliminar and
    /// Cancelar. Eliminar is not the default button (Return does not
    /// destroy; in AppKit a default button is always the accent color).
    fn overlay(&self, state: &State) -> Option<Element<Msg>> {
        let id = self.deleting?;
        let name = state
            .crm
            .clients
            .iter()
            .find(|c| c.id == id)
            .map_or("el cliente", |c| c.name.as_str());
        Some(
            Dialog::new(format!("¿Eliminar a {name}?"))
                .message("Se borrarán el cliente y su actividad. Esta acción no se puede deshacer.")
                .width(DialogWidth::Sm)
                .button(
                    button("Eliminar", ButtonVariant::Danger)
                        .on_click(Msg::ConfirmDelete)
                        .test_id("real_app.delete.confirm"),
                )
                .cancel_button("Cancelar", Msg::CancelDelete)
                .test_id("real_app.delete")
                .build(),
        )
    }

    fn title(&self, _: &State) -> String {
        "App real".into()
    }
}
