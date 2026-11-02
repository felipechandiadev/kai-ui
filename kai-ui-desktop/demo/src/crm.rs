//! The data of «App real» (/real-app): a small CRM kept in memory, as a
//! server would keep it, and shared by its pages through [`State`]. The
//! pages ask for it with a simulated delay ([`LATENCY`]) so the loading
//! states are real.
//!
//! [`State`]: crate::state::State

use std::time::Duration;

use kai::datetime::Date;
use kai::forms::{phone, rut};
use kai::geo::LatLon;

use crate::sections::map::page::SECTORS;

pub mod cards;
pub mod editor;

/// What the simulated server takes to answer.
pub const LATENCY: Duration = Duration::from_millis(300);

/// The demo's «today»: fixed, so the screens and the tests do not change
/// with the date.
pub fn today() -> Date {
    Date::new(2026, 11, 2).expect("fecha válida")
}

/// Communes of the Maule region the client form offers.
pub const COMMUNES: &[&str] = &[
    "Cauquenes",
    "Colbún",
    "Curicó",
    "Linares",
    "Longaví",
    "Parral",
    "Retiro",
    "San Javier",
    "Talca",
    "Villa Alegre",
    "Yerbas Buenas",
];

/// Where a client is in the sales funnel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Prospect,
    Active,
    Inactive,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Prospect => "Prospecto",
            Self::Active => "Activo",
            Self::Inactive => "Inactivo",
        }
    }
}

/// A client of the CRM.
#[derive(Clone, Debug, PartialEq)]
pub struct Client {
    pub id: u32,
    pub name: String,
    /// As shown: `12.345.678-5`.
    pub rut: String,
    pub email: String,
    pub phone: String,
    pub commune: String,
    /// Code of its DSP sector (`AOP26000001`…).
    pub sector: String,
    pub referred_by: Option<u32>,
    /// Next visit, ISO (`2026-11-05`), or empty.
    pub visit: String,
    /// Time of the next visit (`09:30`), or empty.
    pub visit_time: String,
    pub notes: String,
    pub stage: Stage,
    /// Monthly sales, in pesos.
    pub monthly: u32,
    pub location: LatLon,
}

/// Something that happened, newest first in [`Crm::activity`].
#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub when: String,
    pub text: String,
    pub client: Option<u32>,
}

/// What the client form sends to the server.
#[derive(Clone, Debug, PartialEq)]
pub struct ClientDraft {
    /// The client being edited; `None` for a new one.
    pub id: Option<u32>,
    pub name: String,
    pub rut: String,
    pub email: String,
    pub phone: String,
    pub commune: String,
    pub sector: String,
    pub referred_by: Option<u32>,
    pub visit: String,
    pub visit_time: String,
    pub notes: String,
}

/// The CRM: clients, what happened, and what one page asks another to
/// show (the map's client, the list's last saved).
pub struct Crm {
    pub clients: Vec<Client>,
    pub activity: Vec<Activity>,
    /// The client the map selects when it opens («Ver en mapa»).
    pub focus: Option<u32>,
    /// The client saved last: the list selects it.
    pub last_saved: Option<u32>,
    /// Saves so far, to tell activity entries apart.
    saves: u32,
}

impl Default for Crm {
    fn default() -> Self {
        Self::sample()
    }
}

/// A sample client: name, RUT body, commune, sector index, referrer,
/// next visit, stage and monthly sales.
type Seed = (
    &'static str,
    u32,
    &'static str,
    usize,
    Option<u32>,
    &'static str,
    Stage,
    u32,
);

/// `body` with its check digit, as shown: `12.345.678-5`.
fn rut_of(body: u32) -> String {
    let body = body.to_string();
    let digit = rut::check_digit(&body).expect("solo dígitos");
    rut::format(&format!("{body}{digit}"))
}

/// The name of DSP sector `code`, or the code when there is none.
pub fn sector_name(code: &str) -> String {
    SECTORS
        .iter()
        .find(|sector| sector.code == code)
        .map_or_else(|| code.to_owned(), |sector| sector.name.clone())
}

/// A spot inside sector `code` for client `id`: around its centroid, a
/// little apart from the others so their pins do not overlap.
pub fn spot_in(code: &str, id: u32) -> LatLon {
    let Some(sector) = SECTORS.iter().find(|sector| sector.code == code) else {
        return crate::sections::map::page::MAIN_SQUARE;
    };
    let center = sector.layer.centroid();
    let angle = f64::from(id) * 2.399_963; // golden angle: spread evenly
    let near = LatLon::new(
        center.lat + 0.0012 * angle.sin(),
        center.lon + 0.0016 * angle.cos(),
    );
    if sector.layer.contains(near) {
        near
    } else {
        center
    }
}

impl Crm {
    /// Eight clients over the five sectors of Parral.
    pub fn sample() -> Self {
        let seed: [Seed; 8] = [
            (
                "Ana Pérez",
                11_111_111,
                "Parral",
                0,
                None,
                "2026-11-04",
                Stage::Active,
                1_250_000,
            ),
            (
                "Andrés Soto",
                12_345_678,
                "Parral",
                1,
                Some(1),
                "2026-11-06",
                Stage::Active,
                820_000,
            ),
            (
                "Bruno Díaz",
                7_654_321,
                "Retiro",
                2,
                None,
                "",
                Stage::Prospect,
                0,
            ),
            (
                "Camila Fuentes",
                15_222_333,
                "Parral",
                3,
                Some(2),
                "2026-11-03",
                Stage::Active,
                2_100_000,
            ),
            (
                "Carlos Muñoz",
                16_444_555,
                "Longaví",
                4,
                None,
                "",
                Stage::Inactive,
                0,
            ),
            (
                "Daniela Rojas",
                17_890_123,
                "Parral",
                0,
                Some(1),
                "2026-11-10",
                Stage::Prospect,
                0,
            ),
            (
                "Esteban Vera",
                9_876_543,
                "Parral",
                2,
                None,
                "2026-11-05",
                Stage::Active,
                640_000,
            ),
            (
                "Francisca Lagos",
                18_765_432,
                "Parral",
                4,
                Some(4),
                "",
                Stage::Active,
                1_480_000,
            ),
        ];
        let clients = seed
            .iter()
            .zip(1..)
            .map(
                |(&(name, rut, commune, sector, referred_by, visit, stage, monthly), id)| {
                    let sector = SECTORS[sector % SECTORS.len()].code.clone();
                    let user = name
                        .split(' ')
                        .next()
                        .unwrap_or(name)
                        .to_lowercase()
                        .replace('á', "a")
                        .replace('é', "e")
                        .replace('í', "i");
                    Client {
                        id,
                        name: name.to_owned(),
                        rut: rut_of(rut),
                        email: format!("{user}@ejemplo.cl"),
                        phone: phone::format(&format!(
                            "9{:04}{:04}",
                            5000 + id * 37,
                            1000 + id * 113
                        )),
                        commune: commune.to_owned(),
                        location: spot_in(&sector, id),
                        sector,
                        referred_by,
                        visit: visit.to_owned(),
                        visit_time: if visit.is_empty() {
                            String::new()
                        } else {
                            format!("{:02}:30", 8 + id)
                        },
                        notes: String::new(),
                        stage,
                        monthly,
                    }
                },
            )
            .collect();
        let activity = [
            ("hoy 09:12", "Visita registrada a Camila Fuentes", Some(4)),
            ("hoy 08:40", "Cotización enviada a Ana Pérez", Some(1)),
            (
                "ayer 17:05",
                "Daniela Rojas llegó referida por Ana Pérez",
                Some(6),
            ),
            ("ayer 11:30", "Pago recibido de Francisca Lagos", Some(8)),
            ("30/10 16:20", "Carlos Muñoz pasó a inactivo", Some(5)),
        ]
        .into_iter()
        .map(|(when, text, client)| Activity {
            when: when.to_owned(),
            text: text.to_owned(),
            client,
        })
        .collect();
        Self {
            clients,
            activity,
            focus: None,
            last_saved: None,
            saves: 0,
        }
    }

    pub fn client(&self, id: u32) -> Option<&Client> {
        self.clients.iter().find(|client| client.id == id)
    }

    /// Store `draft` as the server would: a RUT already taken by another
    /// client is refused. Returns the client's id.
    pub fn save(&mut self, draft: ClientDraft) -> Result<u32, String> {
        let key = rut::normalize(&draft.rut);
        if let Some(other) = self
            .clients
            .iter()
            .find(|client| Some(client.id) != draft.id && rut::normalize(&client.rut) == key)
        {
            return Err(format!(
                "El RUT {} ya es de {}",
                rut::format(&draft.rut),
                other.name
            ));
        }
        self.saves += 1;
        let when = format!("hoy 10:{:02}", self.saves.min(59));
        let (id, text) = match draft
            .id
            .and_then(|id| self.clients.iter_mut().find(|c| c.id == id))
        {
            Some(client) => {
                if client.sector != draft.sector {
                    client.location = spot_in(&draft.sector, client.id);
                }
                client.name = draft.name;
                client.rut = rut::format(&draft.rut);
                client.email = draft.email;
                client.phone = phone::format(&draft.phone);
                client.commune = draft.commune;
                client.sector = draft.sector;
                client.referred_by = draft.referred_by;
                client.visit = draft.visit;
                client.visit_time = draft.visit_time;
                client.notes = draft.notes;
                (client.id, format!("Ficha de {} actualizada", client.name))
            }
            None => {
                let id = self.clients.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                let text = format!("Cliente nuevo: {}", draft.name);
                self.clients.push(Client {
                    id,
                    location: spot_in(&draft.sector, id),
                    name: draft.name,
                    rut: rut::format(&draft.rut),
                    email: draft.email,
                    phone: phone::format(&draft.phone),
                    commune: draft.commune,
                    sector: draft.sector,
                    referred_by: draft.referred_by,
                    visit: draft.visit,
                    visit_time: draft.visit_time,
                    notes: draft.notes,
                    stage: Stage::Prospect,
                    monthly: 0,
                });
                (id, text)
            }
        };
        self.activity.insert(
            0,
            Activity {
                when,
                text,
                client: Some(id),
            },
        );
        self.last_saved = Some(id);
        Ok(id)
    }

    /// Visits from today to `days` days ahead.
    pub fn visits_within(&self, days: i64) -> usize {
        let today = today();
        let last = today.add_days(days).unwrap_or(today);
        self.clients
            .iter()
            .filter_map(|client| Date::parse(&client.visit))
            .filter(|visit| (today..=last).contains(visit))
            .count()
    }

    /// Monthly sales of the active clients, in pesos.
    pub fn monthly_sales(&self) -> u64 {
        self.clients
            .iter()
            .filter(|client| client.stage == Stage::Active)
            .map(|client| u64::from(client.monthly))
            .sum()
    }
}

/// `1250000` → `$ 1.250.000`.
pub fn pesos(amount: u64) -> String {
    let digits = amount.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push('.');
        }
        out.push(digit);
    }
    format!("$ {out}")
}

/// An ISO date as the screens show it (`04/11/2026`), or «—».
pub fn day(iso: &str) -> String {
    Date::parse(iso).map_or_else(|| "—".to_owned(), |date| date.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_is_valid_and_inside_its_sectors() {
        let crm = Crm::sample();
        for client in &crm.clients {
            assert!(rut::is_valid(&client.rut), "{}", client.rut);
            let sector = SECTORS.iter().find(|s| s.code == client.sector).unwrap();
            assert!(sector.layer.contains(client.location), "{}", client.name);
        }
        assert_eq!(pesos(1_250_000), "$ 1.250.000");
        assert_eq!(pesos(640), "$ 640");
    }

    #[test]
    fn saving_refuses_a_taken_rut_and_records_activity() {
        let mut crm = Crm::sample();
        let ana = crm.client(1).unwrap().clone();
        let draft = ClientDraft {
            id: None,
            name: "Otra Ana".into(),
            rut: ana.rut.replace('.', ""),
            email: "otra@ejemplo.cl".into(),
            phone: String::new(),
            commune: "Parral".into(),
            sector: ana.sector.clone(),
            referred_by: None,
            visit: String::new(),
            visit_time: String::new(),
            notes: String::new(),
        };
        assert_eq!(
            crm.save(draft.clone()),
            Err(format!("El RUT {} ya es de Ana Pérez", ana.rut))
        );
        let edit = ClientDraft {
            id: Some(1),
            name: "Ana María Pérez".into(),
            ..draft
        };
        assert_eq!(crm.save(edit), Ok(1));
        assert_eq!(crm.client(1).unwrap().name, "Ana María Pérez");
        assert_eq!(crm.activity[0].text, "Ficha de Ana María Pérez actualizada");
        assert_eq!(crm.last_saved, Some(1));
    }
}
