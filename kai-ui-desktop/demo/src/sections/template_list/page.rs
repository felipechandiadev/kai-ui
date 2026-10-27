//! /template-list: the list + detail template (`MasterDetailPage`).

use kai::components::{
    AlertVariant, ButtonVariant, MasterDetailPage, PageHeader, Toast, button, caption, heading,
    paragraph,
};
use kai::element::{Element, Role, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

const CLIENTS: [(&str, &str, &str); 5] = [
    ("Ana Ruiz", "ana.ruiz@ejemplo.cl", "Activa"),
    ("Bruno Vega", "bruno.vega@ejemplo.cl", "Activo"),
    ("Carla Soto", "carla.soto@ejemplo.cl", "Pendiente"),
    ("Diego Núñez", "diego.nunez@ejemplo.cl", "Activo"),
    ("Elena Paredes", "elena.paredes@ejemplo.cl", "Inactiva"),
];

#[derive(Default)]
pub struct Screen {
    selected: usize,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Select(usize),
    /// A button of the template: what it would do, as a toast.
    Said(&'static str),
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self::default(), Effects::none())
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Select(index) => self.selected = index.min(CLIENTS.len() - 1),
            Msg::Said(what) => cx.toast(Toast::new(0, AlertVariant::Info, what)),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let list = div()
            .class(
                "flex flex-col gap-1 w-full rounded-lg border border-border/70 \
                 appkit:border-separator appkit:rounded-md overflow-hidden bg-background \
                 appkit:bg-control",
            )
            .part("items")
            .children(CLIENTS.iter().enumerate().map(|(index, (name, email, _))| {
                div()
                    .class(if index == self.selected {
                        "flex flex-col gap-0.5 px-3 py-2 cursor-pointer bg-accent/15 \
                         appkit:bg-selection"
                    } else {
                        "flex flex-col gap-0.5 px-3 py-2 cursor-pointer hover:bg-muted/40 \
                         appkit:hover:bg-fill-3"
                    })
                    .role(Role::Button)
                    .pressed(index == self.selected)
                    .on_click(Msg::Select(index))
                    .test_id(format!("template.items.row.{index}"))
                    .children([
                        text(*name).class("text-sm font-medium text-foreground appkit:text-label"),
                        text(*email)
                            .class("text-xs text-muted-foreground appkit:text-secondary-label"),
                    ])
            }));
        let (name, email, status) = CLIENTS[self.selected];
        let detail = div()
            .class(
                "flex flex-col gap-3 w-full min-h-40 rounded-lg border border-border/70 \
                 appkit:border-separator appkit:rounded-md p-4 bg-background appkit:bg-control",
            )
            .part("panel")
            .children([
                heading(name).test_id("template.items.name"),
                caption(format!("{email} · {status}")),
                paragraph(
                    "Panel de detalle: en una app real aquí van fichas, historial o acciones \
                     sobre el ítem seleccionado en la lista.",
                ),
                div().class("flex flex-wrap gap-2 pt-1").children([
                    button("Editar", ButtonVariant::Primary)
                        .on_click(Msg::Said("Editar cliente"))
                        .test_id("template.items.edit"),
                    button("Archivar", ButtonVariant::Outlined)
                        .on_click(Msg::Said("Cliente archivado"))
                        .test_id("template.items.archive"),
                ]),
            ]);
        MasterDetailPage::new(list, detail)
            .header(
                PageHeader::new("Clientes")
                    .subtitle("Lista + detalle · span 4/8 · gutter gap-4 (step 4)")
                    .breadcrumbs(["Demo", "Plantillas", "Lista + detalle"])
                    .action(
                        button("Nuevo", ButtonVariant::Primary)
                            .on_click(Msg::Said("Nuevo cliente"))
                            .test_id("template.items.new"),
                    ),
            )
            .list_span(4)
            .test_id("template.items")
            .build()
    }

    fn title(&self, _: &State) -> String {
        "Lista + detalle".into()
    }
}
