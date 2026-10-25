//! A small app the framework's tests share: a counter and a form, with the
//! commands most tests need (with and without input, destructive, with a
//! shortcut, in menus or not, a query, a task) and a link to the palette. It replaces the demo as the test
//! bench for commands, the palette, agents and MCP (docs/dev/27).

use schemars::JsonSchema;
use serde::Deserialize;

use crate::commands::{
    Action, Availability, CommandError, CommandRegistry, PALETTE, Query, Safety, Shortcut,
};
use crate::components::{
    BezelStyle, Button, ButtonVariant, FieldKind, TextField, button, card, card_actions,
    card_content, toolbar, toolbar_item, toolbar_spacer, toolbar_title,
};
use crate::element::{Element, Role, div, text};
use crate::icons::Icon;
use crate::runtime::{App, Effects, current_invocation};
use crate::ui_engine::Key;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sample {
    pub clicks: u32,
    pub name: String,
    pub email: String,
    pub password: String,
    pub show_password: bool,
    pub submissions: Vec<String>,
    /// Whether a [`Wait`] task has finished.
    pub waited: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Msg {
    Increment,
    Add(u32),
    Reset,
    NameChanged(String),
    EmailChanged(String),
    PasswordChanged(String),
    Submit,
    Clear,
    TogglePassword,
    Wait,
    WaitDone,
}

impl Sample {
    fn can_submit(&self) -> bool {
        !self.name.trim().is_empty()
    }

    fn clear_form(&mut self) {
        self.name.clear();
        self.email.clear();
        self.password.clear();
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct Increment;

impl Action for Increment {
    type App = Sample;
    const ID: &'static str = "counter.increment";
    const TITLE: &'static str = "Incrementar";
    fn description() -> &'static str {
        "Suma un clic al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Increment
    }
}

/// Adds `count` clicks at once; only scripts and agents call it.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Add {
    #[schemars(description = "Clics que se suman")]
    pub count: u32,
}

impl Action for Add {
    type App = Sample;
    const ID: &'static str = "counter.add";
    const TITLE: &'static str = "Sumar";
    fn description() -> &'static str {
        "Suma varios clics al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Add(self.count)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct Reset;

impl Action for Reset {
    type App = Sample;
    const ID: &'static str = "counter.reset";
    const TITLE: &'static str = "Reiniciar";
    /// The clicks are lost: an agent needs the person's confirmation.
    const SAFETY: Safety = Safety::Destructive;
    fn description() -> &'static str {
        "Vuelve el contador a cero"
    }
    fn available(app: &Sample) -> Availability {
        if app.clicks == 0 {
            Availability::disabled("El contador ya está en cero")
        } else {
            Availability::Enabled
        }
    }
    fn into_msg(self) -> Msg {
        Msg::Reset
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct Submit;

impl Action for Submit {
    type App = Sample;
    const ID: &'static str = "form.submit";
    const TITLE: &'static str = "Enviar";
    fn description() -> &'static str {
        "Registra el nombre y limpia el formulario"
    }
    fn available(app: &Sample) -> Availability {
        if app.can_submit() {
            Availability::Enabled
        } else {
            Availability::disabled("Falta el nombre")
        }
    }
    fn into_msg(self) -> Msg {
        Msg::Submit
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct ClearForm;

impl Action for ClearForm {
    type App = Sample;
    const ID: &'static str = "form.clear";
    const TITLE: &'static str = "Limpiar";
    fn description() -> &'static str {
        "Vacía el formulario"
    }
    fn into_msg(self) -> Msg {
        Msg::Clear
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct TogglePasswordAction;

impl Action for TogglePasswordAction {
    type App = Sample;
    const ID: &'static str = "form.toggle_password";
    const TITLE: &'static str = "Mostrar u ocultar contraseña";
    fn into_msg(self) -> Msg {
        Msg::TogglePassword
    }
}

/// Runs a short task; the command stays in course until it ends.
#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct Wait;

impl Action for Wait {
    type App = Sample;
    const ID: &'static str = "task.wait";
    const TITLE: &'static str = "Esperar";
    fn description() -> &'static str {
        "Espera un momento en una tarea; mientras, el comando está en curso"
    }
    fn into_msg(self) -> Msg {
        Msg::Wait
    }
}

/// The last name sent, if any.
#[derive(Deserialize, JsonSchema)]
pub(crate) struct LastSubmission;

impl Query for LastSubmission {
    type App = Sample;
    type Output = Option<String>;
    const ID: &'static str = "form.last_submission";
    const TITLE: &'static str = "Último envío";
    fn run(app: &Sample, _: Self) -> Result<Option<String>, CommandError> {
        Ok(app.submissions.last().cloned())
    }
}

impl App for Sample {
    type Msg = Msg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry
            .action::<Increment>()
            .shortcut(Shortcut::cmd(Key::Char('i')))
            .menu("Contador")
            .expose();
        registry.action::<Add>().expose();
        registry
            .action::<Reset>()
            .shortcut(Shortcut::cmd(Key::Char('0')))
            .menu("Contador")
            .expose();
        registry
            .action::<Submit>()
            .menu("Formulario")
            .returns::<LastSubmission>()
            .expose();
        registry
            .action::<ClearForm>()
            .shortcut(Shortcut::ctrl_cmd(Key::Char('l')))
            .menu("Formulario")
            .expose();
        // Showing the password is the user's business, not an agent's.
        registry.action::<TogglePasswordAction>().menu("Formulario");
        registry.query::<LastSubmission>().expose();
        registry.action::<Wait>().expose();
    }

    fn title(&self) -> String {
        "Muestra".into()
    }

    fn update(&mut self, msg: Msg) -> Effects<Msg> {
        match msg {
            Msg::Increment => self.clicks += 1,
            Msg::Add(amount) => self.clicks = self.clicks.saturating_add(amount),
            Msg::Reset => self.clicks = 0,
            Msg::NameChanged(name) => self.name = name,
            Msg::EmailChanged(email) => self.email = email,
            Msg::PasswordChanged(password) => self.password = password,
            Msg::Submit => {
                if self.can_submit() {
                    self.submissions.push(self.name.trim().to_owned());
                    self.clear_form();
                }
            }
            Msg::Clear => self.clear_form(),
            Msg::TogglePassword => self.show_password = !self.show_password,
            Msg::Wait => {
                let wait = std::time::Duration::from_millis(50);
                let effect = Effects::perform(move || std::thread::sleep(wait), |()| Msg::WaitDone);
                // The command that started it stays in course until it ends.
                return match current_invocation() {
                    Some(invocation) => effect.completes(invocation),
                    None => effect,
                };
            }
            Msg::WaitDone => self.waited = true,
        }
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        let header = toolbar([
            toolbar_title("Muestra", None),
            toolbar_spacer(),
            toolbar_item(Icon::Plus, "Sumar clic")
                .on_command(Increment)
                .test_id("bar.add"),
        ]);
        let counter = card().test_id("counter").children([
            card_content().child(text(self.clicks.to_string()).role(Role::Status)),
            card_actions().children([
                button("Reiniciar", ButtonVariant::Outlined).on_command(Reset),
                button("Incrementar", ButtonVariant::Primary).on_command(Increment),
            ]),
        ]);
        let form = card().test_id("form").children([
            card_content().children([
                // Return here activates «Enviar», the default button.
                TextField::new("Nombre", &self.name)
                    .test_id("form.name")
                    .field_icon(FieldKind::Name)
                    .on_input(Msg::NameChanged)
                    .build(),
                // Its own on_submit wins over the default button.
                TextField::new("Correo", &self.email)
                    .test_id("form.email")
                    .field_icon(FieldKind::Email)
                    .on_input(Msg::EmailChanged)
                    .on_submit(Msg::Submit)
                    .build(),
                TextField::new("Contraseña", &self.password)
                    .test_id("form.password")
                    .field_icon(FieldKind::Password)
                    .password_command(self.show_password, TogglePasswordAction)
                    .on_input(Msg::PasswordChanged)
                    .build(),
            ]),
            card_actions().children([
                button("Limpiar", ButtonVariant::Text).on_command(ClearForm),
                button("Enviar", ButtonVariant::Primary)
                    .on_command(Submit)
                    .default_button(true),
            ]),
        ]);
        let palette = Button::new("Abrir la paleta (⇧⌘P)")
            .bezel(BezelStyle::Borderless)
            .build()
            .run_command(PALETTE)
            .test_id("palette.open");
        div()
            .class("flex flex-col gap-4 p-4")
            .children([header, counter, form, palette])
    }
}
