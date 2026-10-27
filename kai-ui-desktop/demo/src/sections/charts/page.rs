//! /charts: bars whose selection the page keeps, and a sparkline.

use kai::components::{
    ChartKind, caption, card, card_content, card_header, chart, page, page_body, stat,
};
use kai::element::{Element, Role};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

const MONTHS: [&str; 6] = ["Ene", "Feb", "Mar", "Abr", "May", "Jun"];
const CLICKS_PER_MONTH: [f32; 6] = [4.0, 12.0, 7.0, 15.0, 9.0, 11.0];
const TREND: [f32; 12] = [
    2.0, 5.0, 4.0, 8.0, 6.0, 9.0, 7.0, 12.0, 10.0, 14.0, 11.0, 16.0,
];

#[derive(Default)]
pub struct Screen {
    /// The selected bar, by month index.
    month: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Month(usize),
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self::default(), Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Month(index) => self.month = Some(index),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let chosen = self.month.map_or("Elige una barra".to_owned(), |index| {
            format!(
                "Seleccionado: {} ({})",
                MONTHS[index], CLICKS_PER_MONTH[index] as i32
            )
        });
        page().child(page_body().children([
            card()
                .class("w-full max-w-3xl")
                .test_id("charts.bars")
                .children([
                    card_header("Barras", Some("Clics por mes; elige una barra")),
                    card_content().children([
                        chart(ChartKind::Bar)
                            .categories(MONTHS)
                            .series("Clics", &CLICKS_PER_MONTH)
                            .selected(self.month)
                            .on_select(Msg::Month)
                            .label("Clics por mes")
                            .test_id("charts.bars.plot")
                            .class("h-48 w-full"),
                        caption(chosen)
                            .role(Role::Status)
                            .test_id("charts.bars.selected"),
                    ]),
                ]),
            card()
                .class("w-full max-w-3xl")
                .test_id("charts.sparkline")
                .children([
                    card_header("Sparkline", Some("Tendencia compacta, sin ejes")),
                    card_content().class("flex items-center gap-4").children([
                        stat("Tendencia", "16", Some("último mes")),
                        chart(ChartKind::Sparkline)
                            .series("Tendencia", &TREND)
                            .label("Tendencia")
                            .test_id("charts.sparkline.plot")
                            .class("h-8 flex-1"),
                    ]),
                ]),
        ]))
    }

    fn title(&self, _: &State) -> String {
        "Gráficos".into()
    }
}
