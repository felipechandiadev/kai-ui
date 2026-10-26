//! /containers: a full card, group boxes, tabs and stats.

use kai::components::{
    ButtonVariant, SwitchDensity, Tabs, button, caption, card, card_actions, card_content,
    card_header, group_box, heading, page, page_body, paragraph, stat, switch_labeled,
};
use kai::element::{Element, div};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

const TABS: [&str; 3] = ["General", "Cuenta", "Avanzado"];

#[derive(Default)]
pub struct Screen {
    sync: bool,
    tab: usize,
    accepted: u32,
    discarded: u32,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Accept,
    Discard,
    ToggleSync,
    Tab(usize),
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
            Msg::Accept => self.accepted += 1,
            Msg::Discard => self.discarded += 1,
            Msg::ToggleSync => self.sync = !self.sync,
            Msg::Tab(index) => self.tab = index,
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let sync = || {
            switch_labeled(
                "Sincronizar",
                self.sync,
                Msg::ToggleSync,
                false,
                SwitchDensity::Default,
            )
        };
        page().child(page_body().children([
            card()
                .class("w-full max-w-3xl")
                .test_id("containers.card")
                .children([
                    card_header("Card completa", Some("Encabezado, contenido y acciones")),
                    card_content().children([
                        heading("Título de sección"),
                        paragraph(
                            "Un párrafo de cuerpo: el texto largo se ajusta al ancho de la \
                             card y mantiene el interlineado del tema.",
                        ),
                        caption("Una leyenda, para notas y metadatos."),
                    ]),
                    card_actions().children([
                        button("Descartar", ButtonVariant::Text)
                            .on_click(Msg::Discard)
                            .test_id("containers.discard"),
                        button("Aceptar", ButtonVariant::Primary)
                            .on_click(Msg::Accept)
                            .test_id("containers.accept"),
                    ]),
                ]),
            div()
                .class("grid grid-cols-1 sm:grid-cols-2 gap-4 w-full max-w-3xl")
                .children([
                    group_box(Some("Preferencias"), [sync().test_id("containers.sync")])
                        .test_id("containers.group"),
                    group_box(
                        None,
                        [caption("Un group box sin título agrupa sin encabezado.")],
                    )
                    .test_id("containers.untitled"),
                ]),
            Tabs::new(self.tab, Msg::Tab)
                .tab(
                    TABS[0],
                    paragraph("Las pestañas muestran un panel a la vez; ← y → cambian de pestaña."),
                )
                .tab(TABS[1], sync().test_id("containers.tab_sync"))
                .tab(
                    TABS[2],
                    caption("Tab entra al panel y Mayúsculas+Tab vuelve a la pestaña."),
                )
                .build()
                .class("w-full max-w-3xl")
                .test_id("containers.tabs"),
            div()
                .class("grid grid-cols-3 gap-4 w-full max-w-3xl")
                .test_id("containers.stats")
                .children([
                    stat("Aceptadas", self.accepted.to_string(), None),
                    stat("Descartadas", self.discarded.to_string(), None),
                    stat("Pestaña", TABS[self.tab], Some("abierta")),
                ]),
        ]))
    }

    fn title(&self, _: &State) -> String {
        "Contenedores".into()
    }
}
