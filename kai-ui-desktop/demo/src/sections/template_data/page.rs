//! /template-data: the data view template (`DataPage`): metrics and a
//! dense table.

use kai::components::{
    AlertVariant, ButtonVariant, DataPage, PageHeader, Toast, button, caption, page_grid,
    page_grid_item,
};
use kai::element::{Element, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

const METRICS: [(&str, &str); 3] = [
    ("Ingresos", "$ 128.4k"),
    ("Conversiones", "3.2 %"),
    ("Tickets abiertos", "47"),
];

const ORDERS: [(&str, &str, &str, &str); 6] = [
    ("#1042", "Ana Ruiz", "Pagado", "$ 240"),
    ("#1043", "Bruno Vega", "Pendiente", "$ 180"),
    ("#1044", "Carla Soto", "Pagado", "$ 95"),
    ("#1045", "Diego Núñez", "Fallido", "$ 310"),
    ("#1046", "Elena Paredes", "Pagado", "$ 120"),
    ("#1047", "Felipe Chandía", "Pendiente", "$ 75"),
];

pub struct Screen;

#[derive(Clone, Debug)]
pub enum Msg {
    Export,
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self, Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Export => cx.toast(
                Toast::new(0, AlertVariant::Info, "Exportar")
                    .message(format!("{} operaciones", ORDERS.len())),
            ),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let metrics = page_grid().children(METRICS.map(|(label, value)| {
            page_grid_item(4).child(
                div()
                    .class(
                        "flex flex-col gap-1 rounded-md border border-border/70 \
                         appkit:border-separator p-3 bg-background appkit:bg-control",
                    )
                    .children([
                        caption(label),
                        text(value)
                            .class("text-lg font-semibold text-foreground appkit:text-label"),
                    ]),
            )
        }));
        let row_class = "grid grid-cols-4 gap-2 px-3 py-1.5";
        let header = div()
            .class(&format!(
                "{row_class} text-xs font-medium text-muted-foreground \
                 appkit:text-secondary-label bg-muted/30 appkit:bg-fill-3 border-b \
                 border-border/50 appkit:border-separator"
            ))
            .children(["Orden", "Cliente", "Estado", "Monto"].map(text));
        let last = ORDERS.len() - 1;
        let rows = ORDERS
            .iter()
            .enumerate()
            .map(|(index, (order, client, status, amount))| {
                // The last row has no line under it: the table's border
                // closes it.
                let line = if index == last {
                    ""
                } else {
                    "border-b border-border/40 appkit:border-separator"
                };
                div()
                    .class(&format!(
                        "{row_class} text-sm text-foreground appkit:text-label {line}"
                    ))
                    .test_id(format!("template.data.row.{index}"))
                    .children([*order, *client, *status, *amount].map(text))
            });
        let table = div()
            .class(
                "w-full rounded-md border border-border/70 appkit:border-separator \
                 overflow-hidden bg-background appkit:bg-control",
            )
            .part("table")
            .child(header)
            .children(rows);
        DataPage::new()
            .header(
                PageHeader::new("Operaciones")
                    .subtitle("Densidad compact · gap-3 (step 3) · métricas + tabla densa")
                    .breadcrumbs(["Demo", "Plantillas", "Vista de datos"])
                    .action(
                        button("Exportar", ButtonVariant::Outlined)
                            .on_click(Msg::Export)
                            .test_id("template.data.export"),
                    ),
            )
            .child(metrics)
            .child(table)
            .test_id("template.data")
            .build()
    }

    fn title(&self, _: &State) -> String {
        "Vista de datos".into()
    }
}
