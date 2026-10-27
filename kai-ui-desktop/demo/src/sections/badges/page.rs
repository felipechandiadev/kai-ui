//! /badges: solid and outlined badges, and alerts.

use kai::components::{
    AlertVariant, BadgeVariant, alert, badge, caption, card, card_content, card_header, page,
    page_body,
};
use kai::element::{Element, div};
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

/// Nothing to keep: badges and alerts only show.
pub struct Screen;

#[derive(Clone, Debug)]
pub enum Msg {}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        (Self, Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {}
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let row = |badges: [Element<Msg>; 6]| {
            div()
                .class("flex flex-wrap items-center gap-2")
                .children(badges)
        };
        page().child(
            page_body().child(
                card()
                    .class("w-full max-w-3xl")
                    .test_id("badges")
                    .children([
                        card_header(
                            "Badges y Alertas",
                            Some("Indicadores de estado y mensajes del sistema"),
                        ),
                        card_content().class("gap-4").children([
                            div().class("flex flex-col gap-2").children([
                                caption("Badges sólidos:"),
                                row([
                                    badge("Primary", BadgeVariant::Primary),
                                    badge("Secondary", BadgeVariant::Secondary),
                                    badge("Success", BadgeVariant::Success),
                                    badge("Error", BadgeVariant::Error),
                                    badge("Warning", BadgeVariant::Warning),
                                    badge("Info", BadgeVariant::Info),
                                ])
                                .test_id("badges.solid"),
                                caption("Badges con borde:"),
                                row([
                                    badge("Primary", BadgeVariant::PrimaryOutlined),
                                    badge("Secondary", BadgeVariant::SecondaryOutlined),
                                    badge("Success", BadgeVariant::SuccessOutlined),
                                    badge("Error", BadgeVariant::ErrorOutlined),
                                    badge("Warning", BadgeVariant::WarningOutlined),
                                    badge("Info", BadgeVariant::InfoOutlined),
                                ])
                                .test_id("badges.outlined"),
                            ]),
                            div()
                                .class("grid grid-cols-2 gap-3 pt-2")
                                .test_id("badges.alerts")
                                .children([
                                    alert(AlertVariant::Success, "Operación guardada con éxito"),
                                    alert(AlertVariant::Warning, "Revisa los campos requeridos"),
                                ]),
                        ]),
                    ]),
            ),
        )
    }

    fn title(&self, _: &State) -> String {
        "Badges y alertas".into()
    }
}
