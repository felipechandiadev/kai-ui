//! /table: ten thousand clients (search, filters, sort, columns, a detail
//! per row and export to Excel), a virtual list and the empty and loading
//! patterns.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use kai::commands::{Availability, CommandRegistry, PageAction};
use kai::components::{
    AlertVariant, Avatar, AvatarSize, BadgeVariant, ButtonSize, ButtonVariant, Column, EmptyState,
    IconButtonSize, IconButtonVariant, Table, TableView, Toast, ViewChange, badge, button_sized,
    caption, card, card_content, card_header, column_menu, icon_button_sized, loading_placeholder,
    page, page_body, search_field, virtual_list,
};
use kai::element::{Element, Role, div, text};
use kai::icons::Icon;
use kai::router::{Context, HasPage, Page};
use kai::runtime::{Effects, SaveFile};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::State;

/// Rows of the client table.
pub const CLIENT_COUNT: u32 = 10_000;

/// Pause in the typing of a column filter before the table refreshes.
pub const FILTER_PAUSE: Duration = Duration::from_millis(250);

/// One client of the generated table.
#[derive(Debug, PartialEq)]
pub struct Client {
    pub number: u32,
    pub name: String,
    pub city: &'static str,
    pub plan: &'static str,
    pub amount: u32,
}

/// Client `number` (1-based), the same every time.
pub fn client(number: u32) -> Client {
    const FIRST_NAMES: [&str; 12] = [
        "Ana", "Bruno", "Carla", "Diego", "Elena", "Felipe", "Gabriela", "Hugo", "Inés", "Javier",
        "Laura", "Matías",
    ];
    const SURNAMES: [&str; 10] = [
        "Rojas",
        "Muñoz",
        "Soto",
        "Contreras",
        "Silva",
        "Martínez",
        "Sepúlveda",
        "Morales",
        "Fuentes",
        "Vega",
    ];
    const CITIES: [&str; 8] = [
        "Santiago",
        "Valparaíso",
        "Concepción",
        "La Serena",
        "Antofagasta",
        "Temuco",
        "Puerto Montt",
        "Arica",
    ];
    const PLANS: [&str; 3] = ["Básico", "Pro", "Empresa"];
    let n = number as usize;
    let mix = n.wrapping_mul(2_654_435_761) >> 7;
    Client {
        number,
        name: format!(
            "{} {}",
            FIRST_NAMES[n % FIRST_NAMES.len()],
            SURNAMES[(n / FIRST_NAMES.len() + mix) % SURNAMES.len()]
        ),
        city: CITIES[mix % CITIES.len()],
        plan: PLANS[(mix / 7) % PLANS.len()],
        amount: (mix % 9_000 + 100) as u32 * 10,
    }
}

/// Columns: number, name with its avatar, city, plan as a badge, amount
/// and the row's actions.
fn columns() -> Vec<Column<Client, Msg>> {
    vec![
        Column::number("N.º", |c: &Client| f64::from(c.number))
            .fixed(72.0)
            .sortable(),
        Column::text("Nombre", |c: &Client| c.name.clone())
            .render(|c: &Client| {
                div().class("flex items-center gap-2 min-w-0").children([
                    Avatar::new(c.name.clone()).size(AvatarSize::Xs).build(),
                    text(c.name.as_str()).class("truncate"),
                ])
            })
            .flex(2.0)
            .min_width(160.0)
            .sortable()
            .filterable(),
        Column::text("Ciudad", |c: &Client| c.city.to_owned())
            .flex(1.5)
            .sortable()
            .filterable()
            .hideable(),
        Column::text("Plan", |c: &Client| c.plan.to_owned())
            .render(|c: &Client| badge(c.plan, plan_badge(c.plan)))
            .fixed(104.0)
            .sortable()
            .filterable()
            .hideable(),
        Column::number("Monto", |c: &Client| f64::from(c.amount))
            .format(|amount| format!("$ {amount}"))
            .fixed(112.0)
            .sortable()
            .filterable()
            .hideable(),
        Column::actions(|c: &Client| {
            icon_button_sized(
                Icon::Eye,
                format!("Ver {}", c.name),
                IconButtonVariant::Text,
                IconButtonSize::Sm,
            )
            .on_click(Msg::ShowClient(c.number as usize - 1))
            .test_id(format!("table.view.{}", c.number))
        })
        .fixed(64.0),
    ]
}

/// Panel of an expanded row: what the row leaves out, and a button.
fn client_detail(c: &Client) -> Element<Msg> {
    div()
        .class("flex items-center justify-between gap-3 h-full")
        .children([
            div().class("flex flex-col gap-0.5 min-w-0").children([
                text(format!("Cliente N.º {} de {}", c.number, c.city))
                    .class("text-sm font-medium truncate"),
                caption(format!("Plan {} · $ {} al mes", c.plan, c.amount)),
            ]),
            button_sized("Ver ficha", ButtonVariant::Outlined, ButtonSize::Sm)
                .on_click(Msg::ShowClient(c.number as usize - 1))
                .test_id(format!("table.card.{}", c.number)),
        ])
}

fn plan_badge(plan: &str) -> BadgeVariant {
    match plan {
        "Empresa" => BadgeVariant::Primary,
        "Pro" => BadgeVariant::Info,
        _ => BadgeVariant::Secondary,
    }
}

pub struct Screen {
    clients: Rc<[Client]>,
    view: TableView,
    query: String,
    /// Index into `clients` of the selected row.
    selected: Option<usize>,
    exporting: bool,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Query(String),
    Table(ViewChange),
    Refresh,
    Select(usize),
    ShowClient(usize),
    Export,
    SaveTo(Option<PathBuf>),
    Exported(Result<(String, usize), String>),
    Created,
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        let clients: Rc<[Client]> = (1..=CLIENT_COUNT).map(client).collect();
        let view = TableView::new(&clients, &columns());
        let screen = Self {
            clients,
            view,
            query: String::new(),
            selected: None,
            exporting: false,
        };
        (screen, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Query(query) => {
                self.view.query.clone_from(&query);
                self.query = query;
                self.view.refresh(&self.clients, &columns());
            }
            Msg::Table(change) => {
                let typing = matches!(change, ViewChange::Filter(..));
                self.view.apply(change);
                if typing {
                    return Effects::after(FILTER_PAUSE, Msg::Refresh).keyed("filter");
                }
                self.view.refresh(&self.clients, &columns());
            }
            Msg::Refresh => self.view.refresh(&self.clients, &columns()),
            Msg::Select(index) => self.selected = Some(index),
            Msg::ShowClient(index) => {
                if let Some(client) = self.clients.get(index) {
                    cx.toast(
                        Toast::new(0, AlertVariant::Info, client.name.as_str())
                            .message(format!("Cliente N.º {} de {}", client.number, client.city)),
                    );
                }
            }
            Msg::Export => {
                if self.exporting || self.view.is_empty() {
                    return Effects::none();
                }
                let request = SaveFile::new("Exportar a Excel", "clientes.xlsx").types(&["xlsx"]);
                return Effects::save_file(request, Msg::SaveTo);
            }
            Msg::SaveTo(None) => {}
            Msg::SaveTo(Some(path)) => {
                self.exporting = true;
                let sheet = self.view.export(&self.clients, &columns());
                return Effects::perform(
                    move || {
                        kai::xlsx::write(&sheet, "Clientes", &path).map_err(|e| e.to_string())?;
                        let name = path.file_name().map_or_else(
                            || path.display().to_string(),
                            |name| name.to_string_lossy().into_owned(),
                        );
                        Ok((name, sheet.rows.len()))
                    },
                    Msg::Exported,
                );
            }
            Msg::Exported(result) => {
                self.exporting = false;
                cx.toast(match result {
                    Ok((name, rows)) => Toast::new(0, AlertVariant::Success, "Excel exportado")
                        .message(format!("{rows} filas en {name}")),
                    Err(error) => {
                        Toast::new(0, AlertVariant::Error, "No se exportó").message(error)
                    }
                });
            }
            Msg::Created => {
                cx.toast(Toast::new(0, AlertVariant::Success, "Elemento creado"));
            }
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().children([self.table(), simple_list(), patterns()]))
    }

    fn title(&self, _: &State) -> String {
        "Tabla".into()
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        registry.page_action::<ExportTable>().menu("Tabla").expose();
    }
}

impl Screen {
    fn table(&self) -> Element<Msg> {
        let summary = match self.selected.and_then(|index| self.clients.get(index)) {
            Some(client) => format!("Seleccionado: {}", client.name),
            None => "Elige una fila con un clic o con ↑ ↓".to_owned(),
        };
        let shown = if !self.view.is_filtered() {
            format!("{} clientes", self.view.len())
        } else if self.query.trim().is_empty() {
            format!("{} de {CLIENT_COUNT} filtrados", self.view.len())
        } else {
            format!(
                "{} de {CLIENT_COUNT} con «{}»",
                self.view.len(),
                self.query.trim()
            )
        };
        let filtering = self.view.filtering;
        let filter_rows = icon_button_sized(
            if filtering { Icon::FunnelX } else { Icon::Funnel },
            "Filtrar",
            if self.view.has_filters() {
                IconButtonVariant::Primary
            } else {
                IconButtonVariant::Outlined
            },
            IconButtonSize::Sm,
        )
        .tooltip(if filtering {
            "Quitar los filtros"
        } else {
            "Filtrar por columna"
        })
        .pressed(filtering)
        .on_click(Msg::Table(ViewChange::Filtering(!filtering)))
        .test_id("table.filter");
        let columns_button = column_menu(
            icon_button_sized(
                Icon::Columns3,
                "Columnas",
                IconButtonVariant::Outlined,
                IconButtonSize::Sm,
            )
            .tooltip("Mostrar u ocultar columnas")
            .test_id("table.columns"),
            &columns(),
            &self.view,
            Msg::Table,
        );
        card().class("w-full max-w-3xl").test_id("table").children([
            card_header(
                "Clientes",
                Some("10 000 filas; solo se construyen las visibles"),
            ),
            card_content().class("gap-3").children([
                search_field(&self.query, Msg::Query)
                    .placeholder("Buscar clientes")
                    .label("Buscar clientes")
                    .test_id("table.find")
                    .build(),
                div()
                    .class("flex items-center justify-between gap-3")
                    .children([
                        caption(shown).role(Role::Status).test_id("table.total"),
                        div().class("flex items-center gap-3").children([
                            caption(summary).test_id("table.selection"),
                            filter_rows,
                            columns_button,
                            button_sized("Exportar", ButtonVariant::Outlined, ButtonSize::Sm)
                                .on_page_command(ExportTable)
                                .busy(self.exporting),
                        ]),
                    ]),
                Table::new(Rc::clone(&self.clients), columns(), &self.view)
                    .on_view(Msg::Table)
                    .selected(self.selected)
                    .on_select(Msg::Select)
                    .expandable(client_detail, 64.0)
                    .label("Clientes")
                    .test_id("table.clients")
                    .build()
                    .class("h-[360px] rounded-md border border-border appkit:border-separator"),
            ]),
        ])
    }
}

fn simple_list() -> Element<Msg> {
    card().class("w-full max-w-3xl").children([
        card_header("Lista virtualizada", Some("1 000 elementos de 28 pt")),
        card_content().child(
            virtual_list(
                1_000,
                28.0,
                Rc::new(|index| {
                    div()
                        .class("flex items-center px-3 text-sm text-foreground appkit:text-label")
                        .child(text(format!("Elemento {}", index + 1)))
                }),
            )
            .label("Elementos")
            .test_id("table.items")
            .class("h-48 rounded-md border border-border appkit:border-separator"),
        ),
    ])
}

fn patterns() -> Element<Msg> {
    let framed = |id: &str, child: Element<Msg>| {
        div()
            .class("rounded-md border border-border appkit:border-separator overflow-hidden")
            .test_id(id)
            .child(child)
    };
    card()
        .class("w-full max-w-3xl")
        .test_id("patterns.empty_loading")
        .children([
            card_header(
                "Empty y Loading",
                Some("Patrones reutilizables para listas y tablas vacías o en carga"),
            ),
            card_content().class("gap-4").children([
                caption("Empty filter — la consulta no devolvió filas"),
                framed(
                    "patterns.empty.filter",
                    EmptyState::filter("No hay resultados")
                        .description("Prueba otro filtro o límpialo.")
                        .action("Limpiar filtros", Msg::Query(String::new()))
                        .test_id("patterns.empty.filter.state")
                        .build(),
                ),
                caption("Empty resource — la colección nunca tuvo ítems"),
                framed(
                    "patterns.empty.resource",
                    EmptyState::resource("Sin elementos")
                        .description("Todavía no hay nada aquí.")
                        .action("Crear elemento", Msg::Created)
                        .test_id("patterns.empty.state")
                        .build(),
                ),
                framed("patterns.loading", loading_placeholder("Cargando filas…")),
            ]),
        ])
}

/// `table.export`: the rows the table shows (search, filters, sort; no
/// hidden columns) as an Excel file where the person chooses.
#[derive(Clone, Deserialize, JsonSchema)]
pub struct ExportTable;

impl PageAction for ExportTable {
    type Page = Screen;
    const ID: &'static str = "table.export";
    const TITLE: &'static str = "Exportar a Excel";
    fn description() -> &'static str {
        "Guarda en un Excel las filas que muestra la tabla de clientes, con su búsqueda, \
         filtros y orden"
    }
    fn available(page: &Screen) -> Availability {
        if page.exporting {
            Availability::disabled("Ya se está exportando")
        } else if page.view.is_empty() {
            Availability::disabled("No hay filas que exportar")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::Export
    }
}
