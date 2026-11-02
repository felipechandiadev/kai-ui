//! /real-app: the panel of «App real». Figures computed from the CRM in
//! the state (so a saved client shows here), monthly sales as bars and
//! the latest activity. The first visit waits for the «server».

use kai::components::{
    ButtonVariant, ChartKind, button, caption, card, card_content, card_header, chart,
    loading_placeholder, page, page_body, page_grid, page_grid_item, stat,
};
use kai::element::{Element, Role, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::crm::{LATENCY, Stage, pesos};
use crate::state::State;

const MONTHS: [&str; 6] = ["Jun", "Jul", "Ago", "Sep", "Oct", "Nov"];
/// Sales of the months before this one, in millions of pesos.
const PAST_SALES: [f32; 5] = [4.1, 4.8, 5.2, 4.6, 5.9];

#[derive(Default)]
pub struct Screen {
    loaded: bool,
    month: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Loaded,
    Month(usize),
    Open(u32),
    NewClient,
}

fn sales(state: &State) -> [f32; 6] {
    let mut all = [0.0; 6];
    all[..5].copy_from_slice(&PAST_SALES);
    all[5] = state.crm.monthly_sales() as f32 / 1_000_000.0;
    all
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
            Msg::Month(index) => self.month = Some(index),
            Msg::Open(id) => cx.navigate(format!("/real-app/clients/{id}")),
            Msg::NewClient => cx.navigate("/real-app/clients/new"),
        }
        Effects::none()
    }

    fn view(&self, state: &State) -> Element<Msg> {
        if !self.loaded {
            return page().child(page_body().child(loading_placeholder("Cargando el panel…")));
        }
        let crm = &state.crm;
        let count = |stage| crm.clients.iter().filter(|c| c.stage == stage).count();
        let sales = sales(state);
        let chosen = self.month.map_or("Elige un mes".to_owned(), |index| {
            format!("{}: {:.1} millones", MONTHS[index], sales[index])
        });
        let activity = crm
            .activity
            .iter()
            .take(6)
            .enumerate()
            .map(|(index, entry)| {
                let row = div()
                    .class("flex items-center justify-between gap-3 px-3 py-2 rounded-md")
                    .test_id(format!("real_app.activity.{index}"))
                    .children([
                        text(entry.text.as_str()).class("text-sm text-foreground"),
                        caption(entry.when.as_str()),
                    ]);
                match entry.client {
                    Some(id) => row
                        .class("cursor-pointer hover:bg-muted/40")
                        .role(Role::Button)
                        .label(format!("Abrir {}", entry.text))
                        .on_click(Msg::Open(id)),
                    None => row,
                }
            });
        page().child(
            page_body().child(
                page_grid().children([
                    page_grid_item(3).child(
                        stat(
                            "Clientes",
                            crm.clients.len().to_string(),
                            Some(&format!("{} activos", count(Stage::Active))),
                        )
                        .test_id("real_app.stat.clients"),
                    ),
                    page_grid_item(3).child(
                        stat(
                            "Prospectos",
                            count(Stage::Prospect).to_string(),
                            Some("por convertir"),
                        )
                        .test_id("real_app.stat.prospects"),
                    ),
                    page_grid_item(3).child(
                        stat(
                            "Visitas",
                            crm.visits_within(7).to_string(),
                            Some("próximos 7 días"),
                        )
                        .test_id("real_app.stat.visits"),
                    ),
                    page_grid_item(3).child(
                        stat(
                            "Ventas del mes",
                            pesos(crm.monthly_sales()),
                            Some("clientes activos"),
                        )
                        .test_id("real_app.stat.sales"),
                    ),
                    page_grid_item(7).child(
                        card().test_id("real_app.sales").children([
                            card_header(
                                "Ventas por mes",
                                Some("Millones de pesos; elige una barra"),
                            ),
                            card_content().children([
                                chart(ChartKind::Bar)
                                    .categories(MONTHS)
                                    .series("Ventas", &sales)
                                    .selected(self.month)
                                    .on_select(Msg::Month)
                                    .label("Ventas por mes")
                                    .test_id("real_app.sales.plot")
                                    .class("h-48 w-full"),
                                caption(chosen)
                                    .role(Role::Status)
                                    .test_id("real_app.sales.selected"),
                            ]),
                        ]),
                    ),
                    page_grid_item(5).child(
                        card().test_id("real_app.activity").children([
                            card_header(
                                "Actividad reciente",
                                Some("Lo último que pasó con los clientes"),
                            ),
                            card_content().class("gap-1").children(activity).child(
                                div().class("pt-2").child(
                                    button("Nuevo cliente", ButtonVariant::Outlined)
                                        .on_click(Msg::NewClient)
                                        .test_id("real_app.new"),
                                ),
                            ),
                        ]),
                    ),
                ]),
            ),
        )
    }

    fn title(&self, _: &State) -> String {
        "App real".into()
    }
}
