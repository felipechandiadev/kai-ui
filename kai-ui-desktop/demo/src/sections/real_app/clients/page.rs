//! /real-app/clients: the client list (a `Table` with search and sort)
//! beside the selected client (`MasterDetailPage`). It asks the «server»
//! for the list each time it opens, and selects the client saved last.

use std::rc::Rc;

use kai::components::{
    ButtonVariant, Column, EmptyState, MasterDetailPage, PageHeader, Table, TableView, Value,
    ViewChange, button, loading_placeholder, search_field,
};
use kai::element::{Element, div};
use kai::icons::Icon;
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::crm::cards::{client_facts, stage_badge};
use crate::crm::{Client, LATENCY, sector_name};
use crate::state::State;

pub struct Screen {
    /// `None` until the list arrives.
    clients: Option<Rc<[Client]>>,
    view: TableView,
    query: String,
    /// By index in `clients`.
    selected: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Loaded(Vec<Client>),
    Query(String),
    Table(ViewChange),
    Select(usize),
    Open(u32),
    ShowOnMap(u32),
    NewClient,
}

/// `2026-11-04` as a date of the table, or the far past when empty.
fn table_date(iso: &str) -> kai::components::Date {
    let mut parts = iso.split('-').map(|part| part.parse::<u32>().unwrap_or(0));
    let (year, month, day) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );
    kai::components::Date::new(year as i32, month as u8, day as u8)
}

fn columns() -> Vec<Column<Client, Msg>> {
    vec![
        Column::text("Nombre", |c: &Client| c.name.clone())
            .sortable()
            .flex(2.0),
        Column::text("RUT", |c: &Client| c.rut.clone()).fixed(120.0),
        Column::text("Comuna", |c: &Client| c.commune.clone())
            .sortable()
            .flex(1.0),
        Column::text("Sector", |c: &Client| sector_name(&c.sector))
            .sortable()
            .flex(1.2),
        Column::date("Próxima visita", |c: &Client| table_date(&c.visit))
            .format(|value| match value {
                Value::Date(date) if date.year == 0 => "—".to_owned(),
                Value::Date(date) => format!("{:02}/{:02}/{}", date.day, date.month, date.year),
                other => other.to_string(),
            })
            .sortable()
            .fixed(120.0),
        Column::text("Estado", |c: &Client| c.stage.label().to_owned())
            .render(|c: &Client| stage_badge(c.stage))
            .sortable()
            .fixed(110.0),
    ]
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), state: &State) -> (Self, Effects<Msg>) {
        let snapshot = state.crm.clients.clone();
        let load = Effects::perform(
            move || {
                std::thread::sleep(LATENCY);
                snapshot
            },
            Msg::Loaded,
        );
        let screen = Self {
            clients: None,
            view: TableView::new::<Client, Msg>(&[], &columns()),
            query: String::new(),
            selected: None,
        };
        (screen, load)
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Loaded(list) => {
                let last = cx.state().crm.last_saved;
                self.selected = last
                    .and_then(|id| list.iter().position(|c| c.id == id))
                    .or((!list.is_empty()).then_some(0));
                let clients: Rc<[Client]> = list.into();
                self.view = TableView::new(&clients, &columns());
                self.clients = Some(clients);
            }
            Msg::Query(query) => {
                self.view.query.clone_from(&query);
                self.query = query;
                self.refresh();
            }
            Msg::Table(change) => {
                self.view.apply(change);
                self.refresh();
            }
            Msg::Select(index) => self.selected = Some(index),
            Msg::Open(id) => cx.navigate(format!("/real-app/clients/{id}")),
            Msg::ShowOnMap(id) => {
                cx.state().crm.focus = Some(id);
                cx.navigate("/real-app/map");
            }
            Msg::NewClient => cx.navigate("/real-app/clients/new"),
        }
        Effects::none()
    }

    fn view(&self, state: &State) -> Element<Msg> {
        let header = PageHeader::new("Clientes")
            .subtitle("Busca, ordena y elige un cliente para ver su ficha")
            .breadcrumbs(["App real", "Clientes"])
            .action(
                button("Nuevo cliente", ButtonVariant::Primary)
                    .on_click(Msg::NewClient)
                    .test_id("real_app.clients.new"),
            );
        let Some(clients) = &self.clients else {
            let waiting = loading_placeholder("Cargando clientes…");
            return MasterDetailPage::new(waiting, div())
                .header(header)
                .list_span(7)
                .test_id("real_app.clients")
                .build();
        };
        let table = if self.view.is_empty() {
            EmptyState::filter("Ningún cliente coincide")
                .description(format!("No hay clientes con «{}».", self.query))
                .action("Limpiar búsqueda", Msg::Query(String::new()))
                .test_id("real_app.clients.none")
                .build()
        } else {
            Table::new(Rc::clone(clients), columns(), &self.view)
                .on_view(Msg::Table)
                .selected(self.selected)
                .on_select(Msg::Select)
                .label("Clientes")
                .test_id("real_app.clients.table")
                .build()
                .class("h-[420px] rounded-md border border-border appkit:border-separator")
        };
        let list = div().class("flex flex-col gap-3 w-full").children([
            search_field(self.query.as_str(), Msg::Query)
                .placeholder("Buscar por nombre, RUT, comuna…")
                .label("Buscar clientes")
                .test_id("real_app.clients.search")
                .build(),
            table,
        ]);
        let chosen = self
            .selected
            .and_then(|index| clients.get(index))
            .and_then(|c| state.crm.client(c.id));
        let detail = match chosen {
            Some(client) => div()
                .class(
                    "flex flex-col gap-4 w-full rounded-lg border border-border/70 \
                     appkit:border-separator appkit:rounded-md p-4 bg-background \
                     appkit:bg-control",
                )
                .test_id("real_app.clients.detail")
                .children([
                    client_facts(&state.crm, client, "real_app.clients.facts"),
                    div().class("flex flex-wrap gap-2").children([
                        button("Abrir ficha", ButtonVariant::Primary)
                            .on_click(Msg::Open(client.id))
                            .test_id("real_app.clients.open"),
                        button("Ver en mapa", ButtonVariant::Outlined)
                            .on_click(Msg::ShowOnMap(client.id))
                            .test_id("real_app.clients.map"),
                    ]),
                ]),
            None => EmptyState::new(Icon::Users, "Elige un cliente")
                .description("Su ficha aparece aquí.")
                .build(),
        };
        MasterDetailPage::new(list, detail)
            .header(header)
            .list_span(7)
            .test_id("real_app.clients")
            .build()
    }

    fn title(&self, _: &State) -> String {
        "Clientes".into()
    }
}

impl Screen {
    fn refresh(&mut self) {
        if let Some(clients) = &self.clients {
            self.view.refresh(clients, &columns());
        }
    }
}
