//! How «App real» shows a client: its stage as a badge and its card of
//! facts, shared by the list's detail and the client's page.

use kai::components::{BadgeVariant, badge, caption, heading};
use kai::element::{Element, div, text};

use super::{Client, Crm, Stage, day, pesos, sector_name};

pub fn stage_badge<Msg>(stage: Stage) -> Element<Msg> {
    let variant = match stage {
        Stage::Active => BadgeVariant::Success,
        Stage::Prospect => BadgeVariant::Info,
        Stage::Inactive => BadgeVariant::SecondaryOutlined,
    };
    badge(stage.label(), variant)
}

/// One «label: value» line of a client card.
fn fact<Msg>(label: &str, value: String, test_id: String) -> Element<Msg> {
    div().class("flex items-baseline gap-2").children([
        text(label).class("w-28 shrink-0 text-sm text-muted-foreground"),
        text(value)
            .class("text-sm text-foreground")
            .test_id(test_id),
    ])
}

/// The facts of `client`; its parts' ids start with `test_id`.
pub fn client_facts<Msg: 'static>(crm: &Crm, client: &Client, test_id: &str) -> Element<Msg> {
    let referrer = client
        .referred_by
        .and_then(|id| crm.client(id))
        .map_or_else(|| "—".to_owned(), |by| by.name.clone());
    let visit = match (day(&client.visit).as_str(), client.visit_time.as_str()) {
        ("—", _) => "Sin visita agendada".to_owned(),
        (date, "") => date.to_owned(),
        (date, time) => format!("{date} a las {time}"),
    };
    let or_dash = |value: &str| {
        if value.is_empty() {
            "—".to_owned()
        } else {
            value.to_owned()
        }
    };
    let id = |part: &str| format!("{test_id}.{part}");
    div().class("flex flex-col gap-3").children([
        div().class("flex items-center gap-3").children([
            heading(client.name.as_str()).test_id(id("name")),
            stage_badge(client.stage),
        ]),
        caption(format!("Cliente N.º {}", client.id)),
        div().class("flex flex-col gap-1.5").children([
            fact("RUT", client.rut.clone(), id("rut")),
            fact("Correo", client.email.clone(), id("email")),
            fact("Teléfono", or_dash(&client.phone), id("phone")),
            fact("Comuna", client.commune.clone(), id("commune")),
            fact("Sector DSP", sector_name(&client.sector), id("sector")),
            fact("Referido por", referrer, id("referrer")),
            fact("Próxima visita", visit, id("visit")),
            fact(
                "Ventas del mes",
                pesos(u64::from(client.monthly)),
                id("sales"),
            ),
            fact("Notas", or_dash(&client.notes), id("notes")),
        ]),
    ])
}
