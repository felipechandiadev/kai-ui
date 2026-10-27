//! /template-form: the form template (`FormPage`).

use kai::components::{
    AlertVariant, ButtonVariant, FormPage, PageHeader, TextField, Toast, button,
    form_field_cell, form_full_cell,
};
use kai::element::Element;
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

#[derive(Default)]
pub struct Screen {
    name: String,
    email: String,
    notes: String,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Name(String),
    Email(String),
    Notes(String),
    Save,
    Cancel,
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
            Msg::Name(name) => self.name = name,
            Msg::Email(email) => self.email = email,
            Msg::Notes(notes) => self.notes = notes,
            Msg::Save => {
                let name = self.name.trim();
                let title = if name.is_empty() {
                    "Cliente guardado".to_owned()
                } else {
                    format!("{name} guardado")
                };
                cx.toast(Toast::new(0, AlertVariant::Success, title));
                *self = Self::default();
            }
            Msg::Cancel => *self = Self::default(),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        FormPage::new()
            .header(
                PageHeader::new("Nuevo cliente")
                    .subtitle("Densidad comfortable · gap-5 (step 5) · campos span 6 / 12")
                    .breadcrumbs(["Demo", "Plantillas", "Formulario"]),
            )
            .fields([
                form_field_cell(
                    TextField::new("Nombre", &self.name)
                        .placeholder("Ana Ruiz")
                        .on_input(Msg::Name)
                        .test_id("template.form.name")
                        .build(),
                ),
                form_field_cell(
                    TextField::new("Correo", &self.email)
                        .placeholder("ana@ejemplo.cl")
                        .on_input(Msg::Email)
                        .test_id("template.form.email")
                        .build(),
                ),
                form_full_cell(
                    TextField::new("Notas", &self.notes)
                        .placeholder("Contexto, preferencias…")
                        .on_input(Msg::Notes)
                        .test_id("template.form.notes")
                        .build(),
                ),
            ])
            .actions([
                button("Cancelar", ButtonVariant::Text)
                    .on_click(Msg::Cancel)
                    .test_id("template.form.cancel"),
                button("Guardar", ButtonVariant::Primary)
                    .on_click(Msg::Save)
                    .test_id("template.form.save"),
            ])
            .test_id("template.form")
            .build()
    }

    fn title(&self, _: &State) -> String {
        "Formulario".into()
    }
}
