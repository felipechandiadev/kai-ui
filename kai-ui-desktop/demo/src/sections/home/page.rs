//! /home: a counter and a controlled form, with their commands for the
//! menu, the palette, scripts and agents.

use kai::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
use kai::commands::{
    Availability, CommandError, CommandRegistry, Key, PageAction, PageQuery, Safety, Shortcut,
};
use kai::components::{
    AlertVariant, ButtonVariant, FieldKind, SwitchDensity, TextField, alert, button, caption,
    card, card_actions, card_content, card_header, checkbox, page, page_body, page_grid,
    page_grid_item, paragraph, segmented, select, stat, switch_labeled,
};
use kai::element::{ControlState, Element, Role, div, text};
use kai::router::{Context, HasPage, Page};
use kai::runtime::Effects;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::State;

const ROLES: [&str; 3] = ["Diseño", "Desarrollo", "Producto"];

#[derive(Default)]
pub struct Screen {
    clicks: u32,
    name: String,
    email: String,
    role: Option<usize>,
    terms: bool,
    notifications: bool,
    /// Names sent, oldest first.
    submissions: Vec<String>,
    /// The window's background effect while Inicio is on screen.
    background: BackgroundKind,
    motion: BackgroundMotion,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Increment,
    Add(u32),
    Reset,
    Name(String),
    Email(String),
    Role(usize),
    ToggleTerms,
    ToggleNotifications,
    Submit,
    Clear,
    Background(BackgroundKind),
    ToggleMotion,
}

impl Screen {
    fn can_submit(&self) -> bool {
        !self.name.trim().is_empty()
    }

    fn clear_form(&mut self) {
        self.name.clear();
        self.email.clear();
        self.role = None;
        self.terms = false;
    }
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
            Msg::Increment => self.clicks += 1,
            Msg::Add(count) => self.clicks = self.clicks.saturating_add(count),
            Msg::Reset => self.clicks = 0,
            Msg::Name(name) => self.name = name,
            Msg::Email(email) => self.email = email,
            Msg::Role(index) => self.role = Some(index),
            Msg::ToggleTerms => self.terms = !self.terms,
            Msg::ToggleNotifications => self.notifications = !self.notifications,
            Msg::Submit => {
                if self.can_submit() {
                    self.submissions.push(self.name.trim().to_owned());
                    self.clear_form();
                }
            }
            Msg::Clear => self.clear_form(),
            Msg::Background(kind) => self.background = kind,
            Msg::ToggleMotion => {
                self.motion = match self.motion {
                    BackgroundMotion::Static => BackgroundMotion::Animated,
                    BackgroundMotion::Animated => BackgroundMotion::Static,
                };
            }
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().children([
            self.background_panel(),
            div().class("w-full max-w-3xl").child(alert(
                AlertVariant::Info,
                "Componentes sincronizados con la biblioteca web de Kai (React/TypeScript)",
            )),
            self.counter(),
            self.form(),
            self.stats(),
        ]))
    }

    fn title(&self, _: &State) -> String {
        "Inicio".into()
    }

    fn background(&self, _: &State) -> BackgroundEffect {
        BackgroundEffect::new(self.background).with_motion(self.motion)
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        registry
            .page_action::<Increment>()
            .shortcut(Shortcut::cmd(Key::Char('i')))
            .menu("Contador")
            .expose();
        registry.page_action::<Add>().expose();
        registry
            .page_action::<Reset>()
            .shortcut(Shortcut::cmd(Key::Char('0')))
            .menu("Contador")
            .expose();
        registry
            .page_action::<Submit>()
            .menu("Formulario")
            .returns_page::<LastSubmission>()
            .expose();
        registry.page_action::<ClearForm>().menu("Formulario").expose();
        registry.page_query::<LastSubmission>().expose();
    }
}

impl Screen {
    /// The effect under the content, static or animated.
    fn background_panel(&self) -> Element<Msg> {
        let options = [
            (BackgroundKind::None, "Ninguno"),
            (BackgroundKind::Mesh, "Mesh"),
            (BackgroundKind::Aurora, "Aurora"),
            (BackgroundKind::Glow, "Glow"),
        ];
        let selected = options
            .iter()
            .position(|(kind, _)| *kind == self.background)
            .unwrap_or(0);
        card()
            .class("w-full max-w-3xl")
            .test_id("background")
            .children([
                card_header(
                    "Fondo de ventana",
                    Some("Efecto bajo el contenido; estático o animado"),
                ),
                card_content().child(
                    div()
                        .class("flex flex-wrap items-center justify-between gap-3")
                        .children([
                            segmented(&options.map(|(_, label)| label), selected, |index| {
                                Msg::Background(options[index].0)
                            })
                            .label("Efecto")
                            .test_id("background.effect"),
                            switch_labeled(
                                "Animado",
                                self.motion == BackgroundMotion::Animated,
                                Msg::ToggleMotion,
                                self.background == BackgroundKind::None,
                                SwitchDensity::Default,
                            )
                            .test_id("background.animated"),
                        ]),
                ),
            ])
    }

    fn counter(&self) -> Element<Msg> {
        card()
            .class("w-full max-w-3xl")
            .test_id("counter")
            .children([
                card_header(
                    "Contador reactivo",
                    Some("El estado vive en el modelo; cada mensaje vuelve a ejecutar la vista"),
                ),
                card_content().children([
                    div().class("flex items-end gap-2").children([
                        text(self.clicks.to_string())
                            .class("text-3xl font-bold text-foreground")
                            .role(Role::Status)
                            .test_id("counter.value"),
                        caption(if self.clicks == 1 { "clic" } else { "clics" }).class("pb-1"),
                    ]),
                    paragraph(
                        "Los botones usan las mismas clases que Kai web: hover, foco con anillo, \
                         estado activo y deshabilitado.",
                    ),
                ]),
                card_actions().children([
                    button("Reiniciar", ButtonVariant::Outlined).on_page_command(Reset),
                    button("Incrementar", ButtonVariant::Primary).on_page_command(Increment),
                ]),
            ])
    }

    fn form(&self) -> Element<Msg> {
        let greeting = if self.name.trim().is_empty() {
            caption("Escribe tu nombre para ver la vista reaccionar.")
        } else {
            text(format!("Hola, {} 👋", self.name.trim()))
                .class("text-base font-medium text-primary")
        };
        card().class("w-full max-w-3xl").test_id("form").children([
            card_header(
                "Formulario controlado",
                Some("Cada tecla emite un mensaje con el nuevo valor"),
            ),
            card_content().class("gap-4").children([
                page_grid().children([
                    // Return here activates «Enviar», the default button.
                    page_grid_item(6).child(
                        TextField::new("Nombre", &self.name)
                            .test_id("form.name")
                            .field_icon(FieldKind::Name)
                            .on_input(Msg::Name)
                            .build(),
                    ),
                    page_grid_item(6).child(
                        TextField::new("Correo", &self.email)
                            .test_id("form.email")
                            .field_icon(FieldKind::Email)
                            .on_input(Msg::Email)
                            .on_submit(Msg::Submit)
                            .build(),
                    ),
                    page_grid_item(6).child(
                        select(&ROLES, self.role, "Elegir rol", Msg::Role)
                            .label("Rol")
                            .test_id("form.role"),
                    ),
                    page_grid_item(6).child(
                        checkbox(
                            "Acepto los términos",
                            if self.terms {
                                ControlState::On
                            } else {
                                ControlState::Off
                            },
                            Msg::ToggleTerms,
                        )
                        .test_id("form.terms"),
                    ),
                ]),
                div()
                    .class("flex items-center justify-between pt-1")
                    .children([
                        greeting.role(Role::Status).test_id("form.greeting"),
                        switch_labeled(
                            "Notificaciones",
                            self.notifications,
                            Msg::ToggleNotifications,
                            false,
                            SwitchDensity::Compact,
                        )
                        .test_id("form.notifications"),
                    ]),
            ]),
            card_actions().children([
                button("Limpiar", ButtonVariant::Text).on_page_command(ClearForm),
                button("Enviar", ButtonVariant::Primary)
                    .on_page_command(Submit)
                    .default_button(true),
            ]),
        ])
    }

    fn stats(&self) -> Element<Msg> {
        let last = self
            .submissions
            .last()
            .map_or("Sin envíos todavía".to_owned(), |name| format!("Último: {name}"));
        div()
            .class("grid grid-cols-3 gap-4 w-full max-w-3xl")
            .test_id("stats")
            .children([
                stat("Clics", self.clicks.to_string(), None),
                stat(
                    "Caracteres",
                    self.name.chars().count().to_string(),
                    Some("en el nombre"),
                ),
                stat("Envíos", self.submissions.len().to_string(), Some(&last)),
            ])
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub struct Increment;

impl PageAction for Increment {
    type Page = Screen;
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
pub struct Add {
    #[schemars(description = "Clics que se suman")]
    count: u32,
}

impl PageAction for Add {
    type Page = Screen;
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
pub struct Reset;

impl PageAction for Reset {
    type Page = Screen;
    const ID: &'static str = "counter.reset";
    const TITLE: &'static str = "Reiniciar";
    /// The clicks are lost: an agent needs the person's confirmation.
    const SAFETY: Safety = Safety::Destructive;
    fn description() -> &'static str {
        "Vuelve el contador a cero"
    }
    fn available(page: &Screen) -> Availability {
        if page.clicks == 0 {
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
pub struct Submit;

impl PageAction for Submit {
    type Page = Screen;
    const ID: &'static str = "form.submit";
    const TITLE: &'static str = "Enviar";
    fn description() -> &'static str {
        "Registra el nombre y limpia el formulario"
    }
    fn available(page: &Screen) -> Availability {
        if page.can_submit() {
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
pub struct ClearForm;

impl PageAction for ClearForm {
    type Page = Screen;
    const ID: &'static str = "form.clear";
    const TITLE: &'static str = "Limpiar";
    fn description() -> &'static str {
        "Borra el nombre, el correo, el rol y los términos"
    }
    fn into_msg(self) -> Msg {
        Msg::Clear
    }
}

/// The last name sent, if any.
#[derive(Deserialize, JsonSchema)]
pub struct LastSubmission;

impl PageQuery for LastSubmission {
    type Page = Screen;
    type Output = Option<String>;
    const ID: &'static str = "form.last_submission";
    const TITLE: &'static str = "Último envío";
    fn run(page: &Screen, _: &State, _: Self) -> Result<Option<String>, CommandError> {
        Ok(page.submissions.last().cloned())
    }
}
