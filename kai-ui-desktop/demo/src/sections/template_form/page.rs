//! /template-form: the form template (`FormPage`), with its model in a
//! `kai::forms::FormState` and the `client.*` form commands for agents.

use kai::commands::CommandRegistry;
use kai::components::{
    AlertVariant, ButtonVariant, FormPage, PageHeader, Toast, button, form_field_cell,
    form_full_cell,
};
use kai::element::Element;
use kai::forms::commands::{FormIds, HasForm};
use kai::forms::{Field, FormMsg, FormState, Format, Validators, Values};
use kai::router::{Context, HasPage, Page};
use kai::runtime::Effects;

use crate::state::State;

pub struct Screen {
    form: FormState,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            form: client_form(),
        }
    }
}

/// Nombre libre, correo obligatorio; RUT, teléfono y cupo opcionales pero
/// válidos si se escriben.
fn client_form() -> FormState {
    FormState::new()
        .field("nombre", Field::new())
        .field(
            "correo",
            Field::new()
                .format(Format::Email)
                .validator(Validators::required().and(Validators::email())),
        )
        .field(
            "rut",
            Field::new()
                .format(Format::Rut)
                .validator(Validators::rut()),
        )
        .field(
            "telefono",
            Field::new()
                .format(Format::TelefonoCl)
                .validator(Validators::phone_cl()),
        )
        .field(
            "cupo",
            Field::new()
                .format(Format::Clp)
                .validator(Validators::clp_range(1_000, 10_000_000)),
        )
        .field("notas", Field::new())
}

#[derive(Clone, Debug)]
pub enum Msg {
    Form(FormMsg),
    Cancel,
}

impl Screen {
    fn saved(values: &Values, cx: &mut Context<'_, State>) {
        let name = values.get("nombre").map(|name| name.trim()).unwrap_or("");
        let title = if name.is_empty() {
            "Cliente guardado".to_owned()
        } else {
            format!("{name} guardado")
        };
        cx.toast(Toast::new(0, AlertVariant::Success, title));
    }
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
            // Con errores, `submit` deja todo tocado y enviado: se ven.
            Msg::Form(msg) => {
                if let Some(Ok(values)) = self.form.update(msg) {
                    Self::saved(&values, cx);
                    self.form.reset();
                }
            }
            Msg::Cancel => self.form.reset(),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        let field = |name: &str, label: &str| {
            self.form
                .text_field(name, label, Msg::Form)
                .test_id(format!("template.form.{name}"))
        };
        FormPage::new()
            .header(
                PageHeader::new("Nuevo cliente")
                    .subtitle("Densidad comfortable · gap-5 (step 5) · campos span 6 / 12")
                    .breadcrumbs(["Demo", "Plantillas", "Formulario"]),
            )
            .fields([
                form_field_cell(field("nombre", "Nombre").placeholder("Ana Ruiz").build()),
                form_field_cell(
                    field("correo", "Correo")
                        .placeholder("ana@ejemplo.cl")
                        .required(true)
                        .build(),
                ),
                form_field_cell(
                    field("rut", "RUT")
                        .placeholder("12.345.678-5")
                        .helper_text("Opcional, con dígito verificador")
                        .build(),
                ),
                form_field_cell(
                    field("telefono", "Teléfono")
                        .placeholder("+56 9 1234 5678")
                        .build(),
                ),
                form_field_cell(
                    field("cupo", "Cupo")
                        .placeholder("500.000")
                        .start_symbol("$")
                        .build(),
                ),
                form_full_cell(
                    field("notas", "Notas")
                        .placeholder("Contexto, preferencias…")
                        .build(),
                ),
            ])
            .actions([
                button("Cancelar", ButtonVariant::Text)
                    .on_click(Msg::Cancel)
                    .test_id("template.form.cancel"),
                button("Guardar", ButtonVariant::Primary)
                    .on_click(Msg::Form(FormMsg::Submit))
                    .test_id("template.form.save"),
            ])
            .test_id("template.form")
            .build()
    }

    fn title(&self, _: &State) -> String {
        "Formulario".into()
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        kai::forms::commands::register::<Self, A>(registry).expose();
    }
}

/// `client.set_field`, `client.errors`, `client.values`, `client.submit`,
/// `client.reset` y `client.last_submit` (`form.*` ya lo usa el
/// formulario de Inicio).
impl HasForm for Screen {
    const IDS: FormIds = kai::form_ids!("client");
    fn form(&self) -> &FormState {
        &self.form
    }
    fn form_msg(msg: FormMsg) -> Msg {
        Msg::Form(msg)
    }
}
