//! The client form of «App real», shared by /real-app/clients/new and
//! the editor of /real-app/clients/[id]: `FormState` with validators,
//! two autocompletes (commune from a list, referrer searched with
//! latency), the DSP sector, and the next visit with `DateField` and its
//! calendar plus a time field.

use std::sync::Arc;

use kai::components::{
    AlertVariant, AutocompleteItem, AutocompleteMsg, AutocompleteState, ButtonVariant, FormPage,
    PageHeader, alert, button, caption, form_field_cell, form_full_cell, select,
};
use kai::datetime::{Date, Time};
use kai::element::{Element, div};
use kai::forms::{Field, FormMsg, FormState, Format, Validators};
use kai::runtime::Effects;

use super::{COMMUNES, ClientDraft, Crm, LATENCY, today};
use crate::sections::map::page::SECTORS;

/// A client offered as referrer: searched off the UI thread.
#[derive(Clone, Debug, PartialEq)]
pub struct Referral {
    pub id: u32,
    pub name: String,
    pub rut: String,
}

impl AutocompleteItem for Referral {
    fn label(&self) -> String {
        self.name.clone()
    }

    fn detail(&self) -> Option<String> {
        Some(self.rut.clone())
    }
}

#[derive(Clone, Debug)]
pub enum EditorMsg {
    Form(FormMsg),
    Commune(AutocompleteMsg<&'static str>),
    Referral(AutocompleteMsg<Referral>),
    Sector(usize),
    ToggleCalendar,
    Month(Date),
    Pick(Date),
    Save,
    /// The server answered the save of this draft.
    Saved(Box<ClientDraft>),
    Cancel,
}

/// What the page does after an [`EditorMsg`].
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Editing,
    /// Stored with this id.
    Saved(u32),
    Cancelled,
}

pub struct ClientEditor {
    id: Option<u32>,
    form: FormState,
    commune: AutocompleteState<&'static str>,
    referral: AutocompleteState<Referral>,
    referral_error: Option<String>,
    sector: usize,
    calendar: bool,
    shown: Date,
    saving: bool,
    failure: Option<String>,
}

fn form(crm: &Crm, id: Option<u32>) -> FormState {
    let client = id.and_then(|id| crm.client(id));
    let initial = |pick: fn(&super::Client) -> &str| client.map(pick).unwrap_or("").to_owned();
    let today = today();
    let last = today.add_days(365).unwrap_or(today);
    FormState::new()
        .field(
            "nombre",
            Field::new()
                .validator(
                    Validators::required()
                        .message("Escribe el nombre")
                        .and(Validators::min_len(3)),
                )
                .initial(initial(|c| &c.name)),
        )
        .field(
            "rut",
            Field::new()
                .format(Format::Rut)
                .validator(Validators::required().and(Validators::rut()))
                .initial(initial(|c| &c.rut)),
        )
        .field(
            "correo",
            Field::new()
                .format(Format::Email)
                .validator(Validators::required().and(Validators::email()))
                .initial(initial(|c| &c.email)),
        )
        .field(
            "telefono",
            Field::new()
                .format(Format::TelefonoCl)
                .validator(Validators::phone_cl())
                .initial(initial(|c| &c.phone)),
        )
        .field(
            "comuna",
            Field::new()
                .validator(Validators::required().message("Elige una comuna de la lista"))
                .initial(initial(|c| &c.commune)),
        )
        .field(
            "visita",
            Field::new()
                .format(Format::Fecha)
                .validator(Validators::date_range(today, last))
                .initial(initial(|c| &c.visit)),
        )
        .field(
            "hora",
            Field::new()
                .format(Format::HORA)
                .validator(Validators::time_range(
                    Time::new(8, 0).expect("hora"),
                    Time::new(19, 0).expect("hora"),
                ))
                .initial(initial(|c| &c.visit_time)),
        )
        .field(
            "notas",
            Field::new()
                .validator(Validators::max_len(200))
                .initial(initial(|c| &c.notes)),
        )
}

impl ClientEditor {
    /// The form for client `id`, or for a new one.
    pub fn new(crm: &Crm, id: Option<u32>) -> Self {
        let client = id.and_then(|id| crm.client(id));
        let mut commune = AutocompleteState::list("comuna", COMMUNES.iter().copied()).min_chars(1);
        if let Some(name) = client.and_then(|c| COMMUNES.iter().find(|n| **n == c.commune)) {
            commune = commune.with_selected(name);
        }
        let others: Arc<Vec<Referral>> = Arc::new(
            crm.clients
                .iter()
                .filter(|c| Some(c.id) != id)
                .map(|c| Referral {
                    id: c.id,
                    name: c.name.clone(),
                    rut: c.rut.clone(),
                })
                .collect(),
        );
        let search_in = Arc::clone(&others);
        let mut referral = AutocompleteState::search("referido", move |query: &str| {
            std::thread::sleep(LATENCY);
            let query = query.to_lowercase();
            Ok(search_in
                .iter()
                .filter(|r| r.name.to_lowercase().contains(&query) || r.rut.contains(&query))
                .cloned()
                .collect())
        })
        .limit(5);
        if let Some(by) = client
            .and_then(|c| c.referred_by)
            .and_then(|by| others.iter().find(|r| r.id == by))
        {
            referral = referral.with_selected(by.clone());
        }
        let sector = client
            .and_then(|c| SECTORS.iter().position(|s| s.code == c.sector))
            .unwrap_or(0);
        let shown = client
            .and_then(|c| Date::parse(&c.visit))
            .unwrap_or_else(today)
            .first_of_month();
        Self {
            id,
            form: form(crm, id),
            commune,
            referral,
            referral_error: None,
            sector,
            calendar: false,
            shown,
            saving: false,
            failure: None,
        }
    }

    pub fn is_saving(&self) -> bool {
        self.saving
    }

    pub fn update(&mut self, msg: EditorMsg, crm: &mut Crm) -> (Effects<EditorMsg>, Outcome) {
        let effects = match msg {
            EditorMsg::Form(msg) => {
                let _ = self.form.update(msg);
                Effects::none()
            }
            EditorMsg::Commune(msg) => {
                let effects = self.commune.update(msg).map(EditorMsg::Commune);
                self.commune.write_to(&mut self.form, "comuna");
                effects
            }
            EditorMsg::Referral(msg) => {
                let effects = self.referral.update(msg).map(EditorMsg::Referral);
                self.referral_error = None;
                effects
            }
            EditorMsg::Sector(index) => {
                self.sector = index;
                Effects::none()
            }
            EditorMsg::ToggleCalendar => {
                self.calendar = !self.calendar;
                Effects::none()
            }
            EditorMsg::Month(month) => {
                self.shown = month;
                Effects::none()
            }
            EditorMsg::Pick(date) => {
                self.form.set("visita", &date.iso());
                self.form.touch("visita");
                self.calendar = false;
                Effects::none()
            }
            EditorMsg::Save => return (self.save(), Outcome::Editing),
            EditorMsg::Saved(draft) => {
                self.saving = false;
                return match crm.save(*draft) {
                    Ok(id) => (Effects::none(), Outcome::Saved(id)),
                    Err(message) => {
                        self.failure = Some(message);
                        (Effects::none(), Outcome::Editing)
                    }
                };
            }
            EditorMsg::Cancel => return (Effects::none(), Outcome::Cancelled),
        };
        (effects, Outcome::Editing)
    }

    /// Validate and, when it passes, send the draft to the «server».
    fn save(&mut self) -> Effects<EditorMsg> {
        if self.saving {
            return Effects::none();
        }
        self.commune.write_to(&mut self.form, "comuna");
        let typed_referral = !self.referral.query().trim().is_empty();
        self.referral_error = (typed_referral && self.referral.selected().is_none())
            .then(|| "Elige un cliente de la lista o borra el texto".to_owned());
        let values = self.form.submit();
        let (Ok(values), None) = (values, &self.referral_error) else {
            return Effects::none();
        };
        let get = |name: &str| {
            values
                .get(name)
                .map_or("", |v| v.as_str())
                .trim()
                .to_owned()
        };
        let draft = ClientDraft {
            id: self.id,
            name: get("nombre"),
            rut: get("rut"),
            email: get("correo"),
            phone: get("telefono"),
            commune: get("comuna"),
            sector: SECTORS[self.sector.min(SECTORS.len() - 1)].code.clone(),
            referred_by: self.referral.selected().map(|r| r.id),
            visit: get("visita"),
            visit_time: get("hora"),
            notes: get("notas"),
        };
        self.saving = true;
        self.failure = None;
        Effects::perform(
            move || {
                std::thread::sleep(LATENCY);
                Box::new(draft)
            },
            EditorMsg::Saved,
        )
    }

    pub fn view(&self, crm: &Crm) -> Element<EditorMsg> {
        let editing = self.id.and_then(|id| crm.client(id));
        let title = editing.map_or_else(
            || "Nuevo cliente".to_owned(),
            |c| format!("Editar a {}", c.name),
        );
        let mut crumbs = vec!["App real".to_owned(), "Clientes".to_owned()];
        if let Some(client) = editing {
            crumbs.push(client.name.clone());
        }
        crumbs.push(if editing.is_some() { "Editar" } else { "Nuevo" }.to_owned());
        let today = today();
        let sectors: Vec<&str> = SECTORS.iter().map(|s| s.name.as_str()).collect();
        let form = &self.form;
        let mut fields = vec![
            form_field_cell(
                form.text_field("nombre", "Nombre", EditorMsg::Form)
                    .required(true)
                    .placeholder("Nombre y apellido")
                    .test_id("real_app.form.nombre")
                    .build(),
            ),
            form_field_cell(
                form.text_field("rut", "RUT", EditorMsg::Form)
                    .required(true)
                    .placeholder("12.345.678-5")
                    .test_id("real_app.form.rut")
                    .build(),
            ),
            form_field_cell(
                form.text_field("correo", "Correo", EditorMsg::Form)
                    .required(true)
                    .placeholder("nombre@empresa.cl")
                    .test_id("real_app.form.correo")
                    .build(),
            ),
            form_field_cell(
                form.text_field("telefono", "Teléfono", EditorMsg::Form)
                    .placeholder("9 1234 5678")
                    .test_id("real_app.form.telefono")
                    .build(),
            ),
            form_field_cell(
                form.autocomplete(
                    "comuna",
                    "Comuna",
                    &self.commune,
                    EditorMsg::Form,
                    EditorMsg::Commune,
                )
                .required(true)
                .placeholder("Escribe para buscar")
                .test_id("real_app.form.comuna")
                .build(),
            ),
            form_field_cell(
                div()
                    .class("flex flex-col gap-1")
                    .child(caption("Sector DSP"))
                    .child(
                        select(
                            &sectors,
                            Some(self.sector),
                            "Elegir sector",
                            EditorMsg::Sector,
                        )
                        .label("Sector DSP")
                        .test_id("real_app.form.sector"),
                    ),
            ),
            form_field_cell(
                form.autocomplete(
                    "referido",
                    "Referido por",
                    &self.referral,
                    EditorMsg::Form,
                    EditorMsg::Referral,
                )
                .placeholder("Busca un cliente (nombre o RUT)")
                .test_id("real_app.form.referido")
                .field(|field| match &self.referral_error {
                    Some(message) => field.error(true).helper_text(message.clone()),
                    None => field.helper_text("Opcional"),
                })
                .build(),
            ),
            form_full_cell(
                form.text_field("notas", "Notas", EditorMsg::Form)
                    .test_id("real_app.form.notas")
                    .build(),
            ),
            form_field_cell(
                form.date_field("visita", "Próxima visita", EditorMsg::Form)
                    .today(today)
                    .min(today)
                    .calendar(
                        self.calendar,
                        self.shown,
                        EditorMsg::ToggleCalendar,
                        EditorMsg::Month,
                        EditorMsg::Pick,
                    )
                    .test_id("real_app.form.visita")
                    .build(),
            ),
            form_field_cell(
                form.text_field("hora", "Hora", EditorMsg::Form)
                    .placeholder("HH:MM")
                    .helper_text("Entre 08:00 y 19:00")
                    .test_id("real_app.form.hora")
                    .build(),
            ),
        ];
        if let Some(message) = &self.failure {
            fields.insert(
                0,
                form_full_cell(
                    alert(
                        AlertVariant::Error,
                        format!("No se pudo guardar: {message}"),
                    )
                    .test_id("real_app.form.failure"),
                ),
            );
        }
        let save = button(
            if self.saving {
                "Guardando…"
            } else {
                "Guardar"
            },
            ButtonVariant::Primary,
        )
        .on_click(EditorMsg::Save)
        .disabled(self.saving)
        .test_id("real_app.form.save");
        let cancel = button("Cancelar", ButtonVariant::Outlined)
            .on_click(EditorMsg::Cancel)
            .disabled(self.saving)
            .test_id("real_app.form.cancel");
        FormPage::new()
            .header(
                PageHeader::new(title)
                    .subtitle("Los campos con * son obligatorios; el servidor revisa que el RUT no se repita.")
                    .breadcrumbs(crumbs),
            )
            .fields(fields)
            .actions([cancel, save])
            .test_id("real_app.form")
            .build()
    }
}
