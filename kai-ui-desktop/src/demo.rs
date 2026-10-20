//! Kai UI Desktop demo, shared by the `kai-ui-desktop-prototype` binary
//! (`cargo run`) and `examples/demo.rs` (`cargo run --example demo`).

use schemars::JsonSchema;
use serde::Deserialize;

use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
use crate::commands::{
    Action, Availability, CommandError, CommandRegistry, Query, Safety, Shortcut, describe_json,
};
use crate::components::{
    AlertVariant, BadgeVariant, ButtonSize, ButtonVariant, FieldDensity, FieldVariant,
    IconButtonSize, IconButtonVariant, LabelLayout, SwitchDensity, TextField, alert, badge, button,
    button_sized, caption, card, card_actions, card_content, card_header, icon_button_sized,
    paragraph, stat, switch_labeled,
};
use crate::element::{ControlSize, Element, Role, div, icon, text};
use crate::icons::Icon;
use crate::runtime::{self, SimpleApp};
use crate::theme::{
    AppKitGeneration, Appearance, Density, FieldLabels, MacVersion, MixStyle, SystemInfo, Theme,
};
use crate::ui_engine::Key;
use crate::version;

/// Demo model: everything the screen shows derives from it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Demo {
    clicks: u32,
    name: String,
    email: String,
    submissions: Vec<String>,
    search: String,
    password: String,
    show_password: bool,
    amount: String,
    discount: String,
    commune: String,
    code: String,
    quantity: String,
    autocomplete: String,
    notifications: bool,
    /// Window background effect (top panel); `KAI_BG`, `KAI_BG_ANIM`.
    background: BackgroundKind,
    background_motion: BackgroundMotion,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            clicks: 0,
            name: String::new(),
            email: String::new(),
            submissions: Vec::new(),
            search: String::new(),
            password: String::new(),
            show_password: false,
            amount: String::new(),
            discount: String::new(),
            commune: String::new(),
            code: String::new(),
            quantity: String::new(),
            autocomplete: String::new(),
            notifications: true,
            background: BackgroundKind::None,
            background_motion: BackgroundMotion::Static,
        }
    }
}

/// Initial background from `KAI_BG=none|mesh|aurora|glow` and
/// `KAI_BG_ANIM=0|1` (unset: none, static), plus a warning per unknown
/// value.
fn background_from_env(
    kind: Option<&str>,
    animated: Option<&str>,
) -> (BackgroundKind, BackgroundMotion, Vec<String>) {
    let mut warnings = Vec::new();
    let kind = match kind {
        None => BackgroundKind::None,
        Some(value) => BackgroundKind::parse(value).unwrap_or_else(|| {
            warnings.push(format!(
                "KAI_BG={value} desconocido (none, mesh, aurora, glow); se usa none"
            ));
            BackgroundKind::None
        }),
    };
    let motion = match animated.map(str::trim) {
        None | Some("" | "0") => BackgroundMotion::Static,
        Some("1") => BackgroundMotion::Animated,
        Some(other) => {
            warnings.push(format!("KAI_BG_ANIM={other} desconocido (0, 1); se usa 0"));
            BackgroundMotion::Static
        }
    };
    (kind, motion, warnings)
}

/// Inputs of the TextField variants card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Search,
    Password,
    Amount,
    Discount,
    Commune,
    Code,
    Quantity,
    Autocomplete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Msg {
    Increment,
    Add(u32),
    Reset,
    NameChanged(String),
    EmailChanged(String),
    Submit,
    Clear,
    Edit(Field, String),
    TogglePassword,
    ToggleNotifications,
    SetBackground(BackgroundKind),
    ToggleBackgroundMotion,
}

impl Demo {
    fn can_submit(&self) -> bool {
        !self.name.trim().is_empty()
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct Incrementar;

impl Action for Incrementar {
    type App = Demo;
    const ID: &'static str = "contador.incrementar";
    const TITLE: &'static str = "Incrementar";
    fn description() -> &'static str {
        "Suma un clic al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Increment
    }
}

/// Adds `cantidad` clicks at once; only scripts and agents call it.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Sumar {
    #[schemars(description = "Clics que se suman")]
    cantidad: u32,
}

impl Action for Sumar {
    type App = Demo;
    const ID: &'static str = "contador.sumar";
    const TITLE: &'static str = "Sumar";
    fn description() -> &'static str {
        "Suma varios clics al contador"
    }
    fn into_msg(self) -> Msg {
        Msg::Add(self.cantidad)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct Reiniciar;

impl Action for Reiniciar {
    type App = Demo;
    const ID: &'static str = "contador.reiniciar";
    const TITLE: &'static str = "Reiniciar";
    /// The clicks are lost: an agent needs the person's confirmation.
    const SAFETY: Safety = Safety::Destructive;
    fn description() -> &'static str {
        "Vuelve el contador a cero"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.clicks == 0 {
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
struct Enviar;

impl Action for Enviar {
    type App = Demo;
    const ID: &'static str = "form.enviar";
    const TITLE: &'static str = "Enviar";
    fn description() -> &'static str {
        "Registra el nombre y limpia el formulario"
    }
    fn available(demo: &Demo) -> Availability {
        if demo.can_submit() {
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
struct Limpiar;

impl Action for Limpiar {
    type App = Demo;
    const ID: &'static str = "form.limpiar";
    const TITLE: &'static str = "Limpiar";
    fn description() -> &'static str {
        "Borra el nombre y el correo"
    }
    fn into_msg(self) -> Msg {
        Msg::Clear
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
struct AlternarContrasena;

impl Action for AlternarContrasena {
    type App = Demo;
    const ID: &'static str = "form.alternar_contrasena";
    const TITLE: &'static str = "Mostrar u ocultar contraseña";
    fn into_msg(self) -> Msg {
        Msg::TogglePassword
    }
}

/// The last name sent, if any.
#[derive(Deserialize, JsonSchema)]
struct Saludo;

impl Query for Saludo {
    type App = Demo;
    type Output = Option<String>;
    const ID: &'static str = "form.ultimo_envio";
    const TITLE: &'static str = "Último envío";
    fn run(demo: &Demo, _: Self) -> Result<Option<String>, CommandError> {
        Ok(demo.submissions.last().cloned())
    }
}

impl SimpleApp for Demo {
    type Msg = Msg;

    fn commands(registry: &mut CommandRegistry<Self>) {
        registry
            .action::<Incrementar>()
            .shortcut(Shortcut::cmd(Key::Char('i')))
            .menu("Contador")
            .expose();
        registry.action::<Sumar>().expose();
        registry
            .action::<Reiniciar>()
            .shortcut(Shortcut::cmd(Key::Char('0')))
            .menu("Contador")
            .expose();
        registry
            .action::<Enviar>()
            .menu("Formulario")
            .returns::<Saludo>()
            .expose();
        registry.action::<Limpiar>().menu("Formulario").expose();
        // Showing the password is the user's business, not an agent's.
        registry.action::<AlternarContrasena>().menu("Formulario");
        registry.query::<Saludo>().expose();
    }

    fn title(&self) -> String {
        let base = format!("Kai UI Desktop v{}", version::VERSION);
        if self.clicks == 0 {
            base
        } else {
            format!("{base} — {} clics", self.clicks)
        }
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Increment => self.clicks += 1,
            Msg::Add(amount) => self.clicks = self.clicks.saturating_add(amount),
            Msg::Reset => self.clicks = 0,
            Msg::NameChanged(name) => self.name = name,
            Msg::EmailChanged(email) => self.email = email,
            Msg::Submit => {
                if self.can_submit() {
                    self.submissions.push(self.name.trim().to_owned());
                    self.name.clear();
                    self.email.clear();
                }
            }
            Msg::Clear => {
                self.name.clear();
                self.email.clear();
            }
            Msg::Edit(field, value) => {
                *match field {
                    Field::Search => &mut self.search,
                    Field::Password => &mut self.password,
                    Field::Amount => &mut self.amount,
                    Field::Discount => &mut self.discount,
                    Field::Commune => &mut self.commune,
                    Field::Code => &mut self.code,
                    Field::Quantity => &mut self.quantity,
                    Field::Autocomplete => &mut self.autocomplete,
                } = value;
            }
            Msg::TogglePassword => self.show_password = !self.show_password,
            Msg::ToggleNotifications => self.notifications = !self.notifications,
            Msg::SetBackground(kind) => self.background = kind,
            Msg::ToggleBackgroundMotion => {
                self.background_motion = match self.background_motion {
                    BackgroundMotion::Static => BackgroundMotion::Animated,
                    BackgroundMotion::Animated => BackgroundMotion::Static,
                };
            }
        }
    }

    fn background(&self) -> BackgroundEffect {
        BackgroundEffect::new(self.background).with_motion(self.background_motion)
    }

    fn view(&self) -> Element<Msg> {
        let header = div()
            .class(
                "flex items-center justify-between gap-4 h-14 px-6 bg-background \
                 border-b border-border/70 shadow-md z-10",
            )
            .role(Role::Header)
            .children([
                div().class("flex items-center gap-3").children([
                    text("Kai UI Desktop")
                        .class("text-lg font-semibold text-primary whitespace-nowrap"),
                    badge(
                        format!("v{}", version::VERSION),
                        BadgeVariant::PrimaryOutlined,
                    ),
                ]),
                div().class("flex items-center gap-3").children([
                    caption("Elm · clases Tailwind · wgpu").class("truncate"),
                    icon_button_sized(
                        Icon::Plus,
                        "Sumar clic",
                        IconButtonVariant::Primary,
                        IconButtonSize::Sm,
                    )
                    .test_id("header.sumar")
                    .on_command(Incrementar),
                ]),
            ]);

        let alert_banner = alert(
            AlertVariant::Info,
            "Componentes sincronizados con la biblioteca web de Kai (React/TypeScript)",
        );

        let counter = card()
            .class("w-full max-w-3xl")
            .test_id("contador")
            .children([
                card_header(
                    "Contador reactivo",
                    Some("El estado vive en el modelo; cada mensaje vuelve a ejecutar la vista"),
                ),
                card_content().children([
                    div().class("flex items-end gap-2").children([
                        text(self.clicks.to_string())
                            .class("text-3xl font-bold text-foreground")
                            .role(Role::Status),
                        caption(if self.clicks == 1 { "clic" } else { "clics" }).class("pb-1"),
                    ]),
                    paragraph(
                        "Los botones usan las mismas clases que Kai web: hover, foco con anillo, \
                     estado activo y deshabilitado.",
                    ),
                ]),
                card_actions().children([
                    button("Reiniciar", ButtonVariant::Outlined).on_command(Reiniciar),
                    button("Incrementar", ButtonVariant::Primary).on_command(Incrementar),
                ]),
            ]);

        let greeting = if self.name.trim().is_empty() {
            caption("Escribe tu nombre para ver la vista reaccionar.")
        } else {
            text(format!("Hola, {} 👋", self.name.trim()))
                .class("text-base font-medium text-primary")
        };
        let form = card().class("w-full max-w-3xl").test_id("form").children([
            card_header(
                "Formulario controlado",
                Some("Cada tecla emite un mensaje con el nuevo valor"),
            ),
            card_content().class("gap-4").children([
                div()
                    .class("grid grid-cols-1 sm:grid-cols-2 gap-4")
                    .children([
                        // Return here activates «Enviar», the default button.
                        TextField::new("Nombre", &self.name)
                            .test_id("form.nombre")
                            .start_icon(Icon::User)
                            .on_input(Msg::NameChanged)
                            .build(),
                        TextField::new("Correo", &self.email)
                            .test_id("form.correo")
                            .start_icon(Icon::Search)
                            .on_input(Msg::EmailChanged)
                            .on_submit(Msg::Submit)
                            .build(),
                    ]),
                div()
                    .class("flex items-center justify-between pt-1")
                    .children([
                        greeting.role(Role::Status),
                        switch_labeled(
                            "Notificaciones",
                            self.notifications,
                            Msg::ToggleNotifications,
                            false,
                            SwitchDensity::Compact,
                        )
                        .test_id("form.notificaciones"),
                    ]),
            ]),
            card_actions().children([
                button("Limpiar", ButtonVariant::Text).on_command(Limpiar),
                button("Enviar", ButtonVariant::Primary)
                    .on_command(Enviar)
                    .default_button(true),
            ]),
        ]);

        let last = self
            .submissions
            .last()
            .map_or("Sin envíos todavía".to_owned(), |name| {
                format!("Último: {name}")
            });
        let stats = div()
            .class("grid grid-cols-3 gap-4 w-full max-w-3xl")
            .children([
                stat("Clics", self.clicks.to_string(), None),
                stat(
                    "Caracteres",
                    self.name.chars().count().to_string(),
                    Some("en el nombre"),
                ),
                stat("Envíos", self.submissions.len().to_string(), Some(&last)),
            ]);

        let badges_and_alerts = card().class("w-full max-w-3xl").children([
            card_header(
                "Badges y Alertas",
                Some("Indicadores de estado y mensajes del sistema"),
            ),
            card_content().class("gap-4").children([
                div().class("flex flex-col gap-2").children([
                    caption("Badges sólidos:"),
                    div().class("flex flex-wrap items-center gap-2").children([
                        badge("Primary", BadgeVariant::Primary),
                        badge("Secondary", BadgeVariant::Secondary),
                        badge("Success", BadgeVariant::Success),
                        badge("Error", BadgeVariant::Error),
                        badge("Warning", BadgeVariant::Warning),
                        badge("Info", BadgeVariant::Info),
                    ]),
                    caption("Badges con borde:"),
                    div().class("flex flex-wrap items-center gap-2").children([
                        badge("Primary", BadgeVariant::PrimaryOutlined),
                        badge("Secondary", BadgeVariant::SecondaryOutlined),
                        badge("Success", BadgeVariant::SuccessOutlined),
                        badge("Error", BadgeVariant::ErrorOutlined),
                        badge("Warning", BadgeVariant::WarningOutlined),
                        badge("Info", BadgeVariant::InfoOutlined),
                    ]),
                ]),
                div().class("grid grid-cols-2 gap-3 pt-2").children([
                    alert(AlertVariant::Success, "Operación guardada con éxito"),
                    alert(AlertVariant::Warning, "Revisa los campos requeridos"),
                ]),
            ]),
        ]);

        let buttons_section = card().class("w-full max-w-3xl").children([
            card_header(
                "Botones, Tamaños e IconButtons",
                Some("Variantes visuales, tamaños y botones de iconos"),
            ),
            card_content().class("gap-4").children([
                div().class("flex flex-wrap items-center gap-3").children([
                    button("Primary", ButtonVariant::Primary).on_click(Msg::Increment),
                    button("Secondary", ButtonVariant::Secondary)
                        .test_id("botones.secondary")
                        .on_click(Msg::Increment),
                    button("Outlined", ButtonVariant::Outlined).on_click(Msg::Increment),
                    button("Text", ButtonVariant::Text).on_click(Msg::Increment),
                    button("Danger", ButtonVariant::Danger).on_click(Msg::Reset),
                    button("Deshabilitado", ButtonVariant::Primary)
                        .on_click(Msg::Increment)
                        .disabled(true),
                ]),
                div().class("flex items-center gap-3 pt-2").children([
                    caption("Tamaños:"),
                    button_sized("Pequeño", ButtonVariant::Primary, ButtonSize::Sm),
                    button_sized("Mediano", ButtonVariant::Primary, ButtonSize::Md),
                    button_sized("Grande", ButtonVariant::Primary, ButtonSize::Lg),
                ]),
                div().class("flex items-center gap-3 pt-2").children([
                    caption("IconButtons:"),
                    icon_button_sized(
                        Icon::Search,
                        "Buscar",
                        IconButtonVariant::Primary,
                        IconButtonSize::Md,
                    ),
                    icon_button_sized(
                        Icon::User,
                        "Usuario",
                        IconButtonVariant::Secondary,
                        IconButtonSize::Md,
                    ),
                    icon_button_sized(
                        Icon::Info,
                        "Info",
                        IconButtonVariant::Action,
                        IconButtonSize::Md,
                    ),
                    icon_button_sized(
                        Icon::Trash2,
                        "Eliminar",
                        IconButtonVariant::Outlined,
                        IconButtonSize::Md,
                    ),
                ]),
            ]),
        ]);

        div()
            // `neutral` is systemGray in AppKit: use the window background.
            .class("w-full h-full flex flex-col bg-neutral appkit:bg-window")
            .role(Role::Main)
            .children([
                header,
                div()
                    .class("flex-1 overflow-y-auto")
                    .test_id("contenido")
                    .child(
                        div()
                            .class("flex flex-col items-center gap-6 p-6")
                            .children([
                                self.background_panel(),
                                div().class("w-full max-w-3xl").child(alert_banner),
                                counter,
                                form,
                                self.fields(),
                                badges_and_alerts,
                                buttons_section,
                                self.control_sizes(),
                                stats,
                            ]),
                    ),
            ])
    }
}

impl Demo {
    /// Card with every control size (`NSControl.ControlSize`, mini … extra
    /// large) for the main button variants, and the three icon-button
    /// sizes (Kai 28 / 40 / 48 pt, also in mix C).
    /// Top panel: window background effect and its motion, applied live.
    fn background_panel(&self) -> Element<Msg> {
        let options = [
            (BackgroundKind::None, "Ninguno"),
            (BackgroundKind::Mesh, "Mesh gradient"),
            (BackgroundKind::Aurora, "Aurora boreal"),
            (BackgroundKind::Glow, "Glow"),
        ];
        let animated = self.background_motion == BackgroundMotion::Animated;
        card().class("w-full max-w-3xl").children([
            card_header(
                "Fondo de ventana",
                Some("Efecto bajo el contenido; estático o animado"),
            ),
            card_content().child(
                div()
                    .class("flex flex-wrap items-center justify-between gap-3")
                    .children([
                        div()
                            .class("flex flex-wrap items-center gap-1")
                            .role(Role::Group)
                            .children(options.map(|(kind, label)| {
                                let selected = self.background == kind;
                                button(
                                    label,
                                    if selected {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Outlined
                                    },
                                )
                                .control_size(ControlSize::Small)
                                .pressed(selected)
                                .on_click(Msg::SetBackground(kind))
                            })),
                        switch_labeled(
                            "Animado",
                            animated,
                            Msg::ToggleBackgroundMotion,
                            self.background == BackgroundKind::None,
                            SwitchDensity::Default,
                        ),
                    ]),
            ),
        ])
    }

    fn control_sizes(&self) -> Element<Msg> {
        let sizes = [
            (ControlSize::Mini, "Mini"),
            (ControlSize::Small, "Small"),
            (ControlSize::Regular, "Regular"),
            (ControlSize::Large, "Large"),
            (ControlSize::ExtraLarge, "Extra large"),
        ];
        let row = |name: &str, variant: ButtonVariant| {
            div()
                .class("flex flex-wrap items-center gap-3")
                .child(caption(name).class("w-20 shrink-0"))
                .children(sizes.map(|(size, label)| {
                    button(label, variant)
                        .control_size(size)
                        .on_click(Msg::Increment)
                }))
        };
        let icons = |name: &str, variant: IconButtonVariant| {
            div()
                .class("flex flex-wrap items-center gap-3")
                .child(caption(name).class("w-20 shrink-0"))
                .children(
                    [
                        (IconButtonSize::Sm, Icon::Search),
                        (IconButtonSize::Md, Icon::User),
                        (IconButtonSize::Lg, Icon::Trash2),
                    ]
                    .map(|(size, kind)| icon_button_sized(kind, name, variant, size)),
                )
        };
        card().class("w-full max-w-3xl").children([
            card_header(
                "Tamaños de control",
                Some("Mini, small, regular, large y extra large; IconButtons Sm / Md / Lg"),
            ),
            card_content().class("gap-3").children([
                row("Primary", ButtonVariant::Primary),
                row("Secondary", ButtonVariant::Secondary),
                row("Outlined", ButtonVariant::Outlined),
                row("Text", ButtonVariant::Text),
                div().class("flex flex-col gap-3 pt-2").children([
                    icons("Primary", IconButtonVariant::Primary),
                    icons("Secondary", IconButtonVariant::Secondary),
                    icons("Outlined", IconButtonVariant::Outlined),
                    icons("Action", IconButtonVariant::Action),
                ]),
            ]),
        ])
    }

    /// Card showing the TextField props implemented so far.
    fn fields(&self) -> Element<Msg> {
        let edit = |field: Field| move |value: String| Msg::Edit(field, value);
        let contrast_bar = div()
            .class("flex items-center gap-4 px-4 py-3 rounded-md bg-primary")
            .children([
                text("Kai").class("text-base font-semibold text-white whitespace-nowrap"),
                TextField::new("Buscar en Kai", &self.search)
                    .variant(FieldVariant::Contrast)
                    .always_show_label(true)
                    .start_icon(Icon::Search)
                    .on_input(edit(Field::Search))
                    .build(),
            ]);

        let autocomplete_shell = div()
            .class(
                "relative flex items-center w-full rounded-md border border-border px-3 \
                 focus-within:border-primary bg-background",
            )
            .children([
                icon(Icon::Search).class("size-4 text-muted-foreground mr-1 shrink-0"),
                TextField::new("Buscar cliente...", &self.autocomplete)
                    .variant(FieldVariant::Autocomplete)
                    .density(FieldDensity::Compact)
                    .on_input(edit(Field::Autocomplete))
                    .build(),
            ]);

        card().class("w-full max-w-3xl").children([
            card_header(
                "Variantes de TextField",
                Some(
                    "Iconos Lucide, adornments, password, requerido, compacto, solo lectura, \
                     deshabilitado y autocomplete",
                ),
            ),
            card_content().class("gap-4").children([
                div()
                    .class("grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-5")
                    .children([
                        TextField::new("Buscar", &self.search)
                            .placeholder("Buscar productos")
                            .start_icon(Icon::Search)
                            .on_input(edit(Field::Search))
                            .build(),
                        TextField::new("Contraseña", &self.password)
                            .test_id("form.password")
                            .password_command(self.show_password, AlternarContrasena)
                            .required(true)
                            .helper_text("Mínimo 8 caracteres")
                            .on_input(edit(Field::Password))
                            .build(),
                        TextField::new("Monto", &self.amount)
                            .start_symbol("$")
                            .select_on_focus(true)
                            .on_input(edit(Field::Amount))
                            .build(),
                        TextField::new("Descuento", &self.discount)
                            .end_symbol("%")
                            .on_input(edit(Field::Discount))
                            .build(),
                        TextField::new("Comuna", &self.commune)
                            .required(true)
                            .helper_text("Donde despacharemos el pedido")
                            .on_input(edit(Field::Commune))
                            .build(),
                        TextField::new("Código", &self.code)
                            .density(FieldDensity::Compact)
                            .placeholder("Ej: A-12")
                            .on_input(edit(Field::Code))
                            .build(),
                        TextField::new("Cantidad", &self.quantity)
                            .density(FieldDensity::Compact)
                            .label_layout(LabelLayout::Inline)
                            .inline_leading(
                                icon(Icon::Info)
                                    .class("size-4 text-secondary appkit:text-secondary-label"),
                            )
                            .placeholder("0")
                            .on_input(edit(Field::Quantity))
                            .build(),
                        TextField::new("Folio", "F-2026-0042")
                            .read_only(true)
                            .end_icon(Icon::CircleCheckBig)
                            .build(),
                        TextField::new("Sucursal", "Santiago Centro")
                            .disabled(true)
                            .build(),
                        autocomplete_shell,
                    ]),
                contrast_bar,
            ]),
        ])
    }
}

/// Opens the demo window. Reads `KAI_THEME`, `KAI_HOVER`, `KAI_MIX`,
/// `KAI_LABEL`, `KAI_BG` and `KAI_BG_ANIM`. With `--describe-commands` it
/// prints the commands and their schemas (`mcp-schema.json`) instead.
pub fn run_from_env() -> Result<(), Box<dyn std::error::Error>> {
    let flag = |name: &str| std::env::args().skip(1).any(|arg| arg == name);
    if flag("--describe-commands") {
        println!("{}", describe_json::<Demo>());
        return Ok(());
    }
    let mcp = flag("--mcp");
    if mcp && !cfg!(feature = "mcp") {
        return Err("--mcp necesita compilar con --features mcp".into());
    }
    if !cfg!(target_os = "macos") {
        return Err("Kai UI Desktop comienza como prototipo para macOS".into());
    }
    let (background, background_motion, warnings) = background_from_env(
        std::env::var("KAI_BG").ok().as_deref(),
        std::env::var("KAI_BG_ANIM").ok().as_deref(),
    );
    for warning in warnings {
        eprintln!("{warning}");
    }
    let demo = Demo {
        background,
        background_motion,
        ..Demo::default()
    };
    #[cfg(feature = "mcp")]
    if mcp {
        return runtime::run_with_mcp(demo, demo_theme());
    }
    runtime::run(demo, demo_theme())
}

/// Theme of the demo, chosen with `KAI_THEME=kai|appkit|appkit-dark`
/// (default `kai`). Temporary: Paso 16 replaces it with `--theme appkit`
/// and the live system theme.
/// `KAI_THEME`: `appkit[-dark]` follows this Mac ([`Theme::appkit_current`]);
/// `aqua11[-dark]` and `glass27[-dark]` force a generation. `KAI_HOVER=0`
/// turns hover off (strict AppKit). `KAI_MIX=appkit|a|b|c` picks the mixed
/// candidate of the AppKit themes (default `c`); Kai ignores it.
/// `KAI_LABEL=near|appkit|top|tab` places AppKit field labels: above,
/// small and close (default), the original AppKit label, on the border
/// like Kai, or in a tab on the border.
fn demo_theme() -> Theme {
    let forced = |generation, appearance| Theme::appkit_static(generation, appearance);
    let theme = match std::env::var("KAI_THEME").as_deref() {
        Ok("appkit" | "appkit-light") => Theme::appkit_current(Appearance::Light),
        Ok("appkit-dark") => Theme::appkit_current(Appearance::Dark),
        Ok("aqua11") => forced(AppKitGeneration::Aqua11, Appearance::Light),
        Ok("aqua11-dark") => forced(AppKitGeneration::Aqua11, Appearance::Dark),
        Ok("glass27") => forced(AppKitGeneration::Glass27, Appearance::Light),
        Ok("glass27-dark") => forced(AppKitGeneration::Glass27, Appearance::Dark),
        Ok("kai" | "") | Err(_) => Theme::kai(Density::Comfortable),
        Ok(other) => {
            eprintln!(
                "KAI_THEME={other} desconocido (kai, appkit, appkit-dark, aqua11, aqua11-dark, \
                 glass27, glass27-dark); se usa kai"
            );
            Theme::kai(Density::Comfortable)
        }
    };
    let theme = theme.with_hover(std::env::var("KAI_HOVER").as_deref() != Ok("0"));
    let mix = match std::env::var("KAI_MIX").as_deref() {
        Ok("appkit" | "strict") => MixStyle::AppKit,
        Ok("a" | "A") => MixStyle::A,
        Ok("b" | "B") => MixStyle::B,
        Ok("c" | "C" | "") | Err(_) => MixStyle::C,
        Ok(other) => {
            eprintln!("KAI_MIX={other} desconocido (appkit, a, b, c); se usa c");
            MixStyle::C
        }
    };
    let theme = theme.with_mix(mix);
    // Experiment: where AppKit fields put their label.
    let labels = match std::env::var("KAI_LABEL").as_deref() {
        Ok("appkit") => FieldLabels::AppKit,
        Ok("top") => FieldLabels::Top,
        Ok("tab") => FieldLabels::Tab,
        Ok("near" | "") | Err(_) => FieldLabels::Near,
        Ok(other) => {
            eprintln!("KAI_LABEL={other} desconocido (near, appkit, top, tab); se usa near");
            FieldLabels::Near
        }
    };
    let theme = theme.with_field_labels(labels);
    if theme.is_appkit() {
        let system = SystemInfo::detect();
        let show = |v: Option<MacVersion>| v.map_or("?".to_owned(), |v| v.to_string());
        eprintln!(
            "tema {:?} {:?}, mezcla {:?}, hover {}, etiquetas {:?} (macOS {}, SDK {}: nativo {:?})",
            theme.kind,
            theme.appearance,
            theme.mix,
            if theme.hover_effects { "sí" } else { "no" },
            theme.field_labels,
            show(system.os),
            show(system.sdk),
            system.generation()
        );
    }
    theme
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use cosmic_text::{
        Action, Attrs, Buffer, Edit, Editor, FontSystem, Metrics, Motion, Selection, Shaping,
    };

    use super::{
        AlternarContrasena, Demo, Enviar, Field, Incrementar, Limpiar, Msg, Reiniciar, Saludo,
    };
    use serde_json::json;

    use crate::background::{BackgroundEffect, BackgroundKind, BackgroundMotion};
    use crate::commands::{
        Availability, CommandError, DIALECT, Origin, Outcome, PALETTE, describe_json,
    };
    use crate::element::{FocusKind, FocusOrder, KeyEquivalent, Kind, Role, WidgetId};
    use crate::geometry::Size;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::menu::{MenuEntry, menu_model};
    use crate::render::paint::{PaintInput, paint};
    use crate::render::text::TextStore;
    use crate::runtime::{SimpleApp, build_view};
    use crate::snapshot::{self, Shot};
    use crate::testing::{By, Confirm, Key, Modifiers, MsgSource, Screen, TestApp, layout_of};
    use crate::theme::{AppKitGeneration, Appearance, Density, Theme};

    /// Two-field rows (`grid-cols-1 sm:grid-cols-2`): side by side from
    /// 640 pt, one column below, never wider than the window (the fields
    /// keep their 180 pt minimum).
    #[test]
    fn field_rows_stack_in_a_narrow_window() {
        use crate::theme::MixStyle;
        let demo = Demo::default();
        let themes = [
            Theme::kai(Density::Comfortable),
            Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light).with_mix(MixStyle::C),
            Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark),
        ];
        for theme in themes {
            let at = |width: f32| {
                let tree = build_view(&demo);
                let layout = Layout::compute(
                    &tree,
                    &LayoutInput {
                        viewport: Size {
                            width,
                            height: 700.0,
                        },
                        theme: &theme,
                        interaction: &Interaction::default(),
                        scroll: &ScrollOffsets::new(),
                    },
                    &mut StyleCache::default(),
                    &mut FixedMeasure,
                );
                let bounds = |label| {
                    let input = find(&demo, By::label(label));
                    // The field's shell: the input's grid cell ancestor.
                    let path = tree.path_to(input).unwrap();
                    let cell = path.iter().rev().find(|id| {
                        layout
                            .node(**id)
                            .is_some_and(|n| n.style.min_width == crate::style::Length::Px(180.0))
                    });
                    layout.bounds(*cell.unwrap()).unwrap()
                };
                [("Nombre", "Correo"), ("Buscar", "Contraseña")]
                    .map(|(a, b)| (bounds(a), bounds(b)))
            };
            for (first, second) in at(1024.0) {
                assert_eq!(first.y, second.y, "{:?}", theme.kind);
                assert!(second.x >= first.x + first.width);
            }
            for width in [639.0, 420.0, 400.0] {
                for (first, second) in at(width) {
                    assert_eq!(first.x, second.x, "{width} {:?}", theme.kind);
                    assert!(second.y >= first.y + first.height);
                    assert!(second.x + second.width <= width, "{width} {:?}", theme.kind);
                }
            }
        }
    }

    /// A field and a button in an `items-end` row (mix C, `near` label
    /// above the field): same height, same bottom, for regular fields with
    /// regular buttons and compact fields with small ones, on Aqua11 and
    /// Glass27, light and dark.
    #[test]
    fn mix_c_buttons_line_up_with_fields_in_an_items_end_row() {
        use crate::components::{ButtonVariant, FieldDensity, TextField, button};
        use crate::element::{ControlSize, Element, div};
        for generation in [AppKitGeneration::Aqua11, AppKitGeneration::Glass27] {
            for appearance in [Appearance::Light, Appearance::Dark] {
                let theme = Theme::appkit_static(generation, appearance);
                assert_eq!(theme.mix, crate::theme::MixStyle::AppKit);
                let theme = theme.with_mix(crate::theme::MixStyle::C);
                for (density, size, height) in [
                    (FieldDensity::Default, ControlSize::Regular, 36.0),
                    (FieldDensity::Compact, ControlSize::Small, 32.0),
                ] {
                    let mut tree: Element<Msg> = div().class("w-full h-full p-4").child(
                        div().class("flex items-end gap-2").children([
                            TextField::new("Campo", "x")
                                .test_id("campo")
                                .density(density)
                                .build(),
                            button("Enviar", ButtonVariant::Primary)
                                .control_size(size)
                                .test_id("enviar"),
                        ]),
                    );
                    tree.assign_ids();
                    let layout = Layout::compute(
                        &tree,
                        &LayoutInput {
                            viewport: Size {
                                width: 600.0,
                                height: 200.0,
                            },
                            theme: &theme,
                            interaction: &Interaction::default(),
                            scroll: &ScrollOffsets::new(),
                        },
                        &mut StyleCache::default(),
                        &mut FixedMeasure,
                    );
                    let screen = Screen::new(&layout);
                    let field = screen.get_by_test_id("campo.shell").bounds();
                    let button = screen.get_by_test_id("enviar").bounds();
                    let at = format!("{generation:?} {appearance:?} {size:?}");
                    assert_eq!(field.height, height, "{at}");
                    assert_eq!(button.height, height, "{at}");
                    assert_eq!(button.y + button.height, field.y + field.height, "{at}");
                }
            }
        }
    }

    fn layout(demo: &Demo, viewport: Size, styles: &mut StyleCache) -> Layout {
        Layout::compute(
            &build_view(demo),
            &LayoutInput {
                viewport,
                theme: &Theme::kai(Density::Comfortable),
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            styles,
            &mut FixedMeasure,
        )
    }

    #[test]
    fn update_follows_messages_and_view_reflects_the_model() {
        let mut demo = Demo::default();
        demo.update(Msg::Increment);
        demo.update(Msg::Increment);
        assert_eq!(demo.clicks, 2);
        assert_eq!(
            demo.title(),
            format!("Kai UI Desktop v{} — 2 clics", crate::version::VERSION)
        );
        demo.update(Msg::Submit);
        assert!(demo.submissions.is_empty(), "empty names are not submitted");
        demo.update(Msg::NameChanged("  Ana ".to_owned()));
        let view = build_view(&demo);
        let mut texts = Vec::new();
        view.walk(&mut |element| {
            if let Kind::Text(text) = &element.kind {
                texts.push(text.clone());
            }
        });
        assert!(texts.contains(&"Hola, Ana 👋".to_owned()));
        assert!(texts.contains(&"2".to_owned()));
        demo.update(Msg::Submit);
        assert_eq!(demo.submissions, vec!["Ana".to_owned()]);
        assert!(demo.name.is_empty());
        demo.update(Msg::Reset);
        assert_eq!(demo.clicks, 0);
    }

    #[test]
    fn return_in_name_field_activates_the_default_button() {
        use crate::ui_engine::{Intent, Key, Modifiers, TextIntent, UiModel, dispatch};

        let return_key = Intent::Text(TextIntent::KeyDown {
            key: Key::Enter,
            modifiers: Modifiers::NONE,
            repeat: false,
        });
        let mut demo = Demo::default();
        let name = find(&demo, By::test_id("form.nombre.input"));
        let order = FocusOrder::from_tree(&build_view(&demo));
        let mut model = UiModel::default();
        dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
        let empty = dispatch(&mut model, &order, return_key.clone());
        assert_eq!(empty.activated, None, "«Enviar» is disabled without a name");

        demo.update(Msg::NameChanged("Ana".to_owned()));
        let send = find(&demo, By::test_id("form.enviar"));
        let order = FocusOrder::from_tree(&build_view(&demo));
        let result = dispatch(&mut model, &order, return_key.clone());
        assert_eq!(result.activated.map(|a| a.widget), Some(send));

        // «Correo» keeps its on_submit.
        let email = find(&demo, By::test_id("form.correo.input"));
        dispatch(&mut model, &order, Intent::SetFocus(Some(email)));
        let result = dispatch(&mut model, &order, return_key);
        assert_eq!(result.activated, None);
        assert_eq!(result.text_actions.len(), 1);
    }

    #[test]
    fn clicked_buttons_show_no_focus_ring_but_fields_keep_their_focus_look() {
        use crate::ui_engine::{Intent, Key, Modifiers, TextIntent, UiModel, dispatch};

        let tab = |shift| {
            Intent::Text(TextIntent::KeyDown {
                key: Key::Tab,
                modifiers: Modifiers {
                    shift,
                    ..Modifiers::NONE
                },
                repeat: false,
            })
        };
        let demo = Demo::default();
        let tree = build_view(&demo);
        let order = FocusOrder::from_tree(&tree);
        let theme = Theme::kai(Density::Comfortable);
        let mut styles = StyleCache::default();
        let mut compute = |model: &UiModel| {
            Layout::compute(
                &tree,
                &LayoutInput {
                    viewport: Size {
                        width: 1024.0,
                        height: 768.0,
                    },
                    theme: &theme,
                    interaction: &Interaction {
                        focused: *model.focused.get(),
                        focus_visible: *model.focus_visible.get(),
                        ..Interaction::default()
                    },
                    scroll: &ScrollOffsets::new(),
                },
                &mut styles,
                &mut FixedMeasure,
            )
        };
        let button = find(&demo, By::role(Role::Button).name("Incrementar"));
        let ring = |layout: &Layout| layout.node(button).unwrap().style.ring.width;
        let mut model = UiModel::default();

        dispatch(&mut model, &order, Intent::PointerDown(Some(button)));
        dispatch(&mut model, &order, Intent::PointerUp(Some(button)));
        assert_eq!(*model.focused.get(), Some(button));
        assert_eq!(ring(&compute(&model)), 0.0, "mouse click: no ring");

        dispatch(&mut model, &order, tab(false));
        dispatch(&mut model, &order, tab(true));
        assert_eq!(*model.focused.get(), Some(button));
        assert_eq!(ring(&compute(&model)), 2.0, "Tab: ring");

        // A clicked text field looks exactly as one reached with Tab.
        let name = find(&demo, By::label("Nombre"));
        let path = tree.path_to(name).unwrap();
        let field_look = |layout: &Layout| {
            path.iter()
                .map(|id| format!("{:?}", layout.node(*id).unwrap().style))
                .collect::<Vec<_>>()
        };
        let unfocused = field_look(&compute(&UiModel::default()));
        dispatch(&mut model, &order, Intent::PointerDown(Some(name)));
        assert!(!*model.focus_visible.get());
        let clicked = field_look(&compute(&model));
        dispatch(&mut model, &order, Intent::SetFocus(Some(name)));
        assert!(*model.focus_visible.get());
        assert_eq!(clicked, field_look(&compute(&model)));
        assert_ne!(clicked, unfocused, "the focused field is highlighted");
    }

    #[test]
    fn demo_focus_order_follows_document_order_and_skips_disabled() {
        let demo = Demo::default();
        let order = FocusOrder::from_tree(&build_view(&demo));
        let kinds: Vec<FocusKind> = order.ids().map(|id| order.kind(id).unwrap()).collect();
        assert_eq!(kinds[0], FocusKind::Button, "header quick action button");
        // Background panel: four effect buttons; the "Animado" switch starts
        // disabled (no effect).
        assert_eq!(kinds[1..5], [FocusKind::Button; 4], "background effects");
        assert_eq!(
            kinds[5],
            FocusKind::Button,
            "Incrementar; Reiniciar starts disabled"
        );
        assert_eq!(kinds[6], FocusKind::TextField, "Nombre");
        assert_eq!(kinds[7], FocusKind::TextField, "Correo");
        assert_eq!(kinds[8], FocusKind::Button, "switch notificaciones");
        assert_eq!(kinds[9], FocusKind::Button, "Limpiar");
        assert_eq!(kinds[10], FocusKind::TextField, "Buscar");
        assert_eq!(kinds[11], FocusKind::TextField, "Contraseña");
        assert_eq!(kinds[12], FocusKind::Button, "Toggle password");
        // With an effect the switch joins the order.
        let mut glow = Demo::default();
        glow.update(Msg::SetBackground(BackgroundKind::Glow));
        let order = FocusOrder::from_tree(&build_view(&glow));
        assert_eq!(order.ids().count(), kinds.len() + 1);
    }

    /// The background panel sets the effect and its motion; the app hands
    /// them to the runtime, `None` (static) by default.
    #[test]
    fn background_panel_messages_drive_the_window_background() {
        let mut demo = Demo::default();
        assert_eq!(demo.background(), BackgroundEffect::NONE);
        assert!(!demo.background().is_visible());
        demo.update(Msg::SetBackground(BackgroundKind::Aurora));
        assert_eq!(demo.background().kind, BackgroundKind::Aurora);
        assert!(!demo.background().is_animated());
        demo.update(Msg::ToggleBackgroundMotion);
        assert!(demo.background().is_animated());
        demo.update(Msg::SetBackground(BackgroundKind::None));
        assert!(
            !demo.background().is_animated(),
            "no effect, nothing to animate"
        );
        demo.update(Msg::ToggleBackgroundMotion);
        assert_eq!(demo.background_motion, BackgroundMotion::Static);
        // The selected option is pressed; clicking another sends its kind.
        demo.update(Msg::SetBackground(BackgroundKind::Mesh));
        let tree = build_view(&demo);
        let mut options = Vec::new();
        tree.walk(&mut |element| {
            if let Some(Msg::SetBackground(kind)) = &element.handlers.on_click {
                options.push((*kind, element.is_pressed()));
            }
        });
        assert_eq!(
            options,
            [
                (BackgroundKind::None, false),
                (BackgroundKind::Mesh, true),
                (BackgroundKind::Aurora, false),
                (BackgroundKind::Glow, false),
            ]
        );
    }

    #[test]
    fn background_env_parses_kind_and_motion_and_warns_on_unknown_values() {
        use super::background_from_env;
        assert_eq!(
            background_from_env(None, None),
            (BackgroundKind::None, BackgroundMotion::Static, vec![])
        );
        assert_eq!(
            background_from_env(Some("aurora"), Some("1")),
            (BackgroundKind::Aurora, BackgroundMotion::Animated, vec![])
        );
        assert_eq!(
            background_from_env(Some("GLOW"), Some("0")).0,
            BackgroundKind::Glow
        );
        let (kind, motion, warnings) = background_from_env(Some("bokeh"), Some("si"));
        assert_eq!(
            (kind, motion),
            (BackgroundKind::None, BackgroundMotion::Static)
        );
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].starts_with("KAI_BG=bokeh"));
    }

    #[test]
    fn demo_uses_only_supported_classes_and_fills_the_window() {
        let mut styles = StyleCache::default();
        let mut demo = Demo::default();
        demo.update(Msg::NameChanged("Ana".to_owned()));
        let layout = layout(
            &demo,
            Size {
                width: 420.0,
                height: 340.0,
            },
            &mut styles,
        );
        assert!(styles.unknown().is_empty(), "{:?}", styles.unknown());
        let root = layout.root().unwrap();
        assert_eq!((root.bounds.width, root.bounds.height), (420.0, 340.0));
        let screen = Screen::new(&layout);
        let content = screen.get_by_test_id("contenido").node();
        let scroll = content.scroll.expect("content scrolls");
        assert!(scroll.max > 0.0, "the small window overflows");
        let header = screen.get(By::role(Role::Header)).node();
        assert_eq!(header.bounds.y, 0.0, "header stays fixed");
        assert_eq!(content.clip, None);
    }

    /// Full layout + paint of the demo view with real cosmic-text
    /// measurement and no cache; widths alternate so text re-wraps every
    /// frame. Run with:
    /// `cargo test --release -- --ignored --nocapture bench_layout_and_paint`
    #[test]
    #[ignore = "benchmark; run explicitly in release mode"]
    fn bench_layout_and_paint() {
        const ITERATIONS: usize = 400;
        const WIDTHS: [f32; 4] = [1024.0, 800.0, 600.0, 420.0];
        let theme = Theme::kai(Density::Comfortable);
        let mut demo = Demo::default();
        demo.update(Msg::NameChanged("Ana".to_owned()));
        let mut text = TextStore::new(FontSystem::new());
        let mut styles = StyleCache::default();

        let mut samples: Vec<Duration> = Vec::with_capacity(ITERATIONS);
        let mut ops = 0;
        for index in 0..ITERATIONS {
            let viewport = Size {
                width: WIDTHS[index % WIDTHS.len()],
                height: 720.0,
            };
            let start = Instant::now();
            let tree = build_view(&demo);
            let layout = Layout::compute(
                &tree,
                &LayoutInput {
                    viewport,
                    theme: &theme,
                    interaction: &Interaction::default(),
                    scroll: &ScrollOffsets::new(),
                },
                &mut styles,
                &mut text,
            );
            let list = paint(&PaintInput {
                layout: &layout,
                theme: &theme,
                focused: None,
                window: Default::default(),
                text_fields: &[],
                background: Default::default(),
            });
            samples.push(start.elapsed());
            ops += list.ops.len();
        }
        samples.sort();
        let total: Duration = samples.iter().sum();
        eprintln!(
            "view+layout+paint: {ITERATIONS} iteraciones, media {:?}, p50 {:?}, p95 {:?}, máx {:?} ({} ops/frame)",
            total / ITERATIONS as u32,
            samples[ITERATIONS / 2],
            samples[ITERATIONS * 95 / 100],
            samples[ITERATIONS - 1],
            ops / ITERATIONS,
        );
        assert!(ops > 0);
    }

    /// The single element of the demo's view matching `by`.
    #[track_caller]
    fn find(demo: &Demo, by: By) -> WidgetId {
        let layout = layout_of(&build_view(demo));
        Screen::new(&layout).get(by).id()
    }

    /// Header block of the card titled `title`.
    fn card_header_of(demo: &Demo, title: &str) -> WidgetId {
        let layout = layout_of(&build_view(demo));
        let screen = Screen::new(&layout);
        screen.get_by_text(title).parent().unwrap().id()
    }

    const BUTTONS_CARD: &str = "Botones, Tamaños e IconButtons";

    #[test]
    fn test_ids_are_unique_and_point_at_the_controls() {
        let tree = build_view(&Demo::default());
        assert_eq!(tree.validate_ids(), Vec::<String>::new());
        let layout = layout_of(&tree);
        let screen = Screen::new(&layout);
        let send = screen.get_by_test_id("form.enviar");
        assert_eq!(send.role(), Role::Button);
        assert_eq!(send.name().as_deref(), Some("Enviar"));
        assert_eq!(
            screen.get(By::role(Role::Button).name("Enviar")).id(),
            send.id()
        );
        assert_eq!(
            screen.get(By::role(Role::Button).name("Limpiar")).test_id(),
            Some("form.limpiar")
        );
        assert_eq!(
            screen.get_by_label("Nombre").test_id(),
            Some("form.nombre.input")
        );
        assert_eq!(
            screen.get_by_label("Correo").test_id(),
            Some("form.correo.input")
        );
        assert_eq!(
            screen.get_by_label("Contraseña").test_id(),
            Some("form.password.input")
        );
        assert_eq!(
            screen
                .get(By::role(Role::Button).name("Mostrar contraseña"))
                .test_id(),
            Some("form.password.toggle")
        );
        assert_eq!(
            screen.get_by_test_id("header.sumar").name().as_deref(),
            Some("Sumar clic")
        );
    }

    #[test]
    fn typing_a_name_enables_send_and_the_click_submits_it() {
        let mut app = TestApp::new(Demo::default());
        let send = || By::test_id("form.enviar");
        assert!(!app.is_enabled(send()));
        app.click(send());
        assert_eq!(app.messages(), [], "a disabled button sends nothing");

        app.type_text(By::label("Nombre"), "Ana");
        assert_eq!(app.value(By::label("Nombre")), "Ana");
        assert_eq!(
            app.messages(),
            ["A", "An", "Ana"].map(|name| Msg::NameChanged(name.to_owned()))
        );
        assert!(app.is_enabled(send()));
        app.screen().get(By::text("Hola, Ana 👋"));

        app.click(send());
        assert_eq!(app.messages().last(), Some(&Msg::Submit));
        assert_eq!(app.app().submissions, ["Ana"]);
        assert_eq!(app.value(By::label("Nombre")), "");
        assert!(!app.is_enabled(send()));
        app.screen().get(By::text("Último: Ana"));
    }

    #[test]
    fn return_in_the_name_field_submits_through_the_default_button() {
        let mut app = TestApp::new(Demo::default());
        app.type_text(By::test_id("form.nombre"), "Ana");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages().last(), Some(&Msg::Submit));
        assert_eq!(app.app().submissions, ["Ana"]);
    }

    #[test]
    fn tab_moves_through_the_form_in_order_showing_the_ring() {
        let mut app = TestApp::new(Demo::default());
        app.click(By::test_id("form.nombre"));
        assert!(app.is_focused(By::test_id("form.nombre")));
        assert!(!app.focus_visible(), "a click does not show the ring");

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("form.correo")));
        assert!(app.focus_visible());

        let shift = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, shift);
        assert!(app.is_focused(By::test_id("form.nombre")));
    }

    #[test]
    fn the_password_eye_toggles_and_leaves_focus_in_the_field() {
        let mut app = TestApp::new(Demo::default());
        app.type_text(By::test_id("form.password"), "abc");
        assert!(!app.app().show_password);
        app.click(By::test_id("form.password.toggle"));
        assert!(app.app().show_password);
        assert!(app.is_focused(By::test_id("form.password")));
        assert_eq!(app.value(By::test_id("form.password")), "abc");
    }

    #[test]
    fn availability_follows_update() {
        let mut app = TestApp::new(Demo::default());
        let ids: Vec<&str> = app.commands().iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            [
                "contador.incrementar",
                "contador.sumar",
                "contador.reiniciar",
                "form.enviar",
                "form.limpiar",
                "form.alternar_contrasena",
                "form.ultimo_envio",
                "kai.paleta",
            ]
        );
        assert_eq!(
            app.available::<Reiniciar>(),
            Availability::disabled("El contador ya está en cero")
        );
        app.click(By::test_id("header.sumar"));
        assert!(app.available::<Reiniciar>().is_enabled());
        assert!(!app.available::<Enviar>().is_enabled());
        app.type_text(By::test_id("form.nombre"), "Ana");
        assert!(app.available::<Enviar>().is_enabled());
    }

    #[test]
    fn invoke_runs_enabled_actions_and_reports_disabled_ones() {
        let mut app = TestApp::new(Demo::default());
        let error = app.invoke(Reiniciar).unwrap_err();
        assert_eq!(error.message, "El contador ya está en cero");
        assert_eq!(app.messages(), [], "a disabled action sends nothing");

        app.invoke(Incrementar).unwrap();
        app.invoke(Incrementar).unwrap();
        assert_eq!(app.app().clicks, 2);
        let screen = app.screen();
        screen
            .within(screen.get_by_test_id("contador"))
            .get(By::role(Role::Status).name("2"));
        app.invoke(Reiniciar).unwrap();
        assert_eq!(app.messages(), [Msg::Increment, Msg::Increment, Msg::Reset]);
        assert_eq!(app.app().clicks, 0);
    }

    #[test]
    fn menu_bar_follows_the_registry() {
        let app = TestApp::new(Demo::default());
        let model = menu_model(app.commands());
        let titles: Vec<_> = model.iter().map(|group| group.title).collect();
        assert_eq!(titles, ["Contador", "Formulario", "Ver"]);
        assert_eq!(
            model[0].items[1],
            MenuEntry {
                id: "contador.reiniciar",
                title: "Reiniciar",
                enabled: false,
                shortcut: Some(KeyEquivalent::cmd(Key::Char('0'))),
            }
        );
        assert_eq!(model[2].items.len(), 1);
        assert_eq!(model[2].items[0].id, PALETTE);
        assert!(model[2].items[0].enabled);
    }

    const CMD: Modifiers = Modifiers {
        command: true,
        ..Modifiers::NONE
    };

    #[test]
    fn command_shortcut_sends_the_same_msg_as_the_button() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('i'), CMD);
        assert_eq!(app.messages(), [Msg::Increment]);
        app.click(By::role(Role::Button).name("Incrementar"));
        assert_eq!(app.messages(), [Msg::Increment, Msg::Increment]);
        assert_eq!(app.app().clicks, 2);
    }

    #[test]
    fn disabled_command_shortcut_sends_nothing() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('0'), CMD);
        assert_eq!(app.messages(), [], "nothing to reset");
        app.press(Key::Char('i'), CMD);
        app.press(Key::Char('0'), CMD);
        assert_eq!(app.messages(), [Msg::Increment, Msg::Reset]);
    }

    #[test]
    fn command_shortcuts_do_not_steal_copy_in_a_field() {
        let mut app = TestApp::new(Demo::default());
        app.type_text(By::test_id("form.nombre"), "Ana");
        let typed = app.messages().len();
        app.press(Key::A, CMD);
        app.press(Key::C, CMD);
        assert_eq!(app.clipboard().text().as_deref(), Some("Ana"));
        assert_eq!(app.messages().len(), typed, "copying sends nothing");
        // Cmd shortcuts still work while typing.
        app.press(Key::Char('i'), CMD);
        assert_eq!(app.messages().last(), Some(&Msg::Increment));
    }

    #[test]
    fn a_button_follows_its_command() {
        let mut app = TestApp::new(Demo::default());
        let reset = || By::test_id("contador.reiniciar");
        assert!(!app.is_enabled(reset()));
        assert_eq!(
            app.screen().get(reset()).node().description.as_deref(),
            Some("El contador ya está en cero")
        );
        assert_eq!(
            app.screen().get(reset()).name().as_deref(),
            Some("Reiniciar")
        );
        app.click(reset());
        assert_eq!(app.messages(), []);
        app.click(By::test_id("header.sumar"));
        assert!(app.is_enabled(reset()));
        assert_eq!(app.screen().get(reset()).node().description, None);
        app.click(reset());
        assert_eq!(app.messages().last(), Some(&Msg::Reset));
        assert_eq!(app.app().clicks, 0);
    }

    const CMD_SHIFT: Modifiers = Modifiers {
        command: true,
        shift: true,
        ..Modifiers::NONE
    };

    fn row(id: &str) -> By {
        By::test_id(format!("kai.paleta.{id}"))
    }

    fn search() -> By {
        By::test_id("kai.paleta.buscar")
    }

    #[test]
    fn palette_opens_with_focus_in_search_and_filters() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(app.session().palette_open());
        assert!(app.is_focused(search()));
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
        let dialog = app
            .screen()
            .get(By::role(Role::Dialog).name("Paleta de comandos"))
            .test_id()
            .map(str::to_owned);
        assert_eq!(dialog.as_deref(), Some("kai.paleta"));
        let rows = |app: &TestApp<Demo>| -> Vec<String> {
            let screen = app.screen();
            let panel = screen.get(By::test_id("kai.paleta"));
            screen
                .within(panel)
                .all(By::role(Role::Button))
                .iter()
                .filter_map(|found| found.test_id().map(str::to_owned))
                .collect()
        };
        assert_eq!(
            rows(&app).len(),
            5,
            "every app action, not kai.* or queries"
        );
        app.type_text(search(), "REIN");
        assert_eq!(rows(&app), ["kai.paleta.contador.reiniciar"]);
        assert_eq!(app.value(search()), "REIN");
        app.type_text(search(), "zzz");
        assert_eq!(rows(&app), Vec::<String>::new());
        assert!(app.screen().query(By::text("Sin resultados")).is_some());
    }

    #[test]
    fn disabled_command_stays_disabled_in_button_palette_and_menu() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        let reset = row("contador.reiniciar");
        assert!(!app.is_enabled(By::test_id("contador.reiniciar")));
        assert!(!app.is_enabled(reset.clone()));
        assert_eq!(
            app.screen().get(reset).node().description.as_deref(),
            Some("El contador ya está en cero")
        );
        let menu = menu_model(app.commands());
        let entry = menu[0]
            .items
            .iter()
            .find(|item| item.id == "contador.reiniciar");
        assert_eq!(entry.map(|item| item.enabled), Some(false));
    }

    #[test]
    fn enter_runs_the_selected_command_only_when_enabled() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        app.type_text(search(), "rein");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages(), [], "Reiniciar is disabled");
        assert!(app.session().palette_open());

        app.press(Key::Escape, Modifiers::NONE);
        app.click(By::test_id("header.sumar"));
        app.press(Key::Char('p'), CMD_SHIFT);
        assert_eq!(app.value(search()), "", "a new palette starts empty");
        app.type_text(search(), "rein");
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages(), [Msg::Increment, Msg::Reset]);
        assert!(!app.session().palette_open());
        assert_eq!(app.app().clicks, 0);
    }

    #[test]
    fn arrows_move_the_selection() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        app.press(Key::ArrowUp, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.messages(), [Msg::Increment], "first row: Incrementar");
        app.press(Key::Char('p'), CMD_SHIFT);
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(
            app.messages(),
            [Msg::Increment, Msg::Reset],
            "second row: Reiniciar"
        );
    }

    #[test]
    fn escape_gives_focus_back_and_app_ids_do_not_change() {
        let mut app = TestApp::new(Demo::default());
        let name = By::test_id("form.nombre");
        app.click(name.clone());
        let reset_id = app.screen().get(By::test_id("contador.reiniciar")).id();
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(!app.is_focused(name.clone()));
        assert_eq!(
            app.screen().get(By::test_id("contador.reiniciar")).id(),
            reset_id
        );
        app.press(Key::Escape, Modifiers::NONE);
        assert!(!app.session().palette_open());
        assert!(app.is_focused(name));
        assert!(app.screen().query(By::test_id("kai.paleta")).is_none());

        app.press(Key::Char('p'), CMD_SHIFT);
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(!app.session().palette_open(), "its shortcut toggles it");
    }

    #[test]
    fn tab_stays_inside_the_palette() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        for _ in 0..8 {
            app.press(Key::Tab, Modifiers::NONE);
            let focused = app.session().focused().expect("something has focus");
            let test_id = app
                .screen()
                .by_id(focused)
                .and_then(|found| found.test_id());
            assert!(
                test_id.is_some_and(|id| id.starts_with("kai.paleta.")),
                "focus left the palette: {test_id:?}"
            );
        }
    }

    #[test]
    fn clicks_run_a_row_or_close_on_the_backdrop() {
        let mut app = TestApp::new(Demo::default());
        app.press(Key::Char('p'), CMD_SHIFT);
        app.click(row("contador.incrementar"));
        assert_eq!(app.messages(), [Msg::Increment]);
        assert!(!app.session().palette_open());

        app.press(Key::Char('p'), CMD_SHIFT);
        app.click_at(crate::geometry::Point { x: 4.0, y: 4.0 });
        assert!(!app.session().palette_open());
        assert_eq!(
            app.messages(),
            [Msg::Increment],
            "the click hit nothing below"
        );
    }

    /// A way to run a command: as a script, with the mouse, the keyboard,
    /// the menu bar or the palette.
    #[derive(Clone, Copy, Debug)]
    enum Via {
        Invoke,
        /// `call(id, {})`, as an agent will.
        Json,
        Button(&'static str),
        Key(Key, Modifiers),
        Menu,
        /// Open the palette, type this and press Return.
        Palette(&'static str),
    }

    struct Case {
        id: &'static str,
        setup: fn(&mut TestApp<Demo>),
        invoke: fn(&mut TestApp<Demo>),
        vias: Vec<Via>,
    }

    /// Run `via` on a fresh demo after `setup`; the model and the messages
    /// sent by `via` alone.
    fn run_via(case: &Case, via: Via) -> (Demo, Vec<Msg>) {
        let mut app = TestApp::new(Demo::default());
        (case.setup)(&mut app);
        let before = app.messages().len();
        match via {
            Via::Invoke => (case.invoke)(&mut app),
            Via::Json => {
                let _ = app.call(case.id, json!({}));
            }
            Via::Button(test_id) => app.click(By::test_id(test_id)),
            Via::Key(key, modifiers) => app.press(key, modifiers),
            Via::Menu => {
                let menu = menu_model(app.commands());
                let entry = menu
                    .iter()
                    .flat_map(|group| &group.items)
                    .find(|e| e.id == case.id);
                assert!(entry.is_some(), "{} no está en el menú", case.id);
                app.perform(case.id);
            }
            Via::Palette(query) => {
                app.press(Key::Char('p'), CMD_SHIFT);
                app.type_text(search(), query);
                app.press(Key::Enter, Modifiers::NONE);
            }
        }
        (app.app().clone(), app.messages()[before..].to_vec())
    }

    fn type_name(app: &mut TestApp<Demo>) {
        app.type_text(By::test_id("form.nombre"), "Ana");
    }

    fn cases() -> Vec<Case> {
        vec![
            Case {
                id: "contador.incrementar",
                setup: |_| {},
                invoke: |app| app.invoke(Incrementar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("header.sumar"),
                    Via::Button("contador.incrementar"),
                    Via::Key(Key::Char('i'), CMD),
                    Via::Menu,
                    Via::Palette("Incrementar"),
                ],
            },
            Case {
                id: "contador.reiniciar",
                setup: |app| app.invoke(Incrementar).unwrap(),
                invoke: |app| app.invoke(Reiniciar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("contador.reiniciar"),
                    Via::Key(Key::Char('0'), CMD),
                    Via::Menu,
                    Via::Palette("Reiniciar"),
                ],
            },
            Case {
                id: "form.enviar",
                setup: type_name,
                invoke: |app| app.invoke(Enviar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.enviar"),
                    // Return in «Nombre»: «Enviar» is the default button.
                    Via::Key(Key::Enter, Modifiers::NONE),
                    Via::Menu,
                    Via::Palette("Enviar"),
                ],
            },
            Case {
                id: "form.limpiar",
                setup: type_name,
                invoke: |app| app.invoke(Limpiar).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.limpiar"),
                    Via::Menu,
                    Via::Palette("Limpiar"),
                ],
            },
            Case {
                id: "form.alternar_contrasena",
                setup: |_| {},
                invoke: |app| app.invoke(AlternarContrasena).unwrap(),
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.password.toggle"),
                    Via::Menu,
                    Via::Palette("contraseña"),
                ],
            },
        ]
    }

    #[test]
    fn every_action_does_the_same_from_every_via() {
        for case in cases() {
            let mut setup = TestApp::new(Demo::default());
            (case.setup)(&mut setup);
            let before = setup.app().clone();
            let (expected, messages) = run_via(&case, Via::Invoke);
            assert_eq!(messages.len(), 1, "{}: un mensaje", case.id);
            assert_ne!(expected, before, "{} cambia el modelo", case.id);
            for &via in &case.vias[1..] {
                let (model, sent) = run_via(&case, via);
                assert_eq!(sent, messages, "{} vía {via:?}", case.id);
                assert_eq!(model, expected, "{} vía {via:?}", case.id);
            }
        }
    }

    #[test]
    fn disabled_actions_do_nothing_from_any_via() {
        let disabled = [
            Case {
                id: "contador.reiniciar",
                setup: |_| {},
                invoke: |app| {
                    let error = app.invoke(Reiniciar).unwrap_err();
                    assert_eq!(error.message, "El contador ya está en cero");
                },
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("contador.reiniciar"),
                    Via::Key(Key::Char('0'), CMD),
                    Via::Menu,
                    Via::Palette("Reiniciar"),
                ],
            },
            Case {
                id: "form.enviar",
                setup: |app| app.click(By::test_id("form.nombre")),
                invoke: |app| {
                    let error = app.invoke(Enviar).unwrap_err();
                    assert_eq!(error.message, "Falta el nombre");
                },
                vias: vec![
                    Via::Invoke,
                    Via::Json,
                    Via::Button("form.enviar"),
                    Via::Key(Key::Enter, Modifiers::NONE),
                    Via::Menu,
                    Via::Palette("Enviar"),
                ],
            },
        ];
        for case in disabled {
            let mut setup = TestApp::new(Demo::default());
            (case.setup)(&mut setup);
            let before = setup.app().clone();
            for &via in &case.vias {
                let (model, sent) = run_via(&case, via);
                assert_eq!(sent, [], "{} vía {via:?}", case.id);
                assert_eq!(model, before, "{} vía {via:?}", case.id);
            }
        }
    }

    #[test]
    fn json_calls_run_actions_and_queries() {
        let mut app = TestApp::new(Demo::default());
        assert_eq!(
            app.call("contador.sumar", json!({"cantidad": 3})),
            Ok(json!(null))
        );
        assert_eq!(app.app().clicks, 3);
        assert_eq!(app.messages(), [Msg::Add(3)]);
        let error = app.call("contador.sumar", json!({})).unwrap_err();
        assert_eq!(
            error.message,
            "entrada inválida para contador.sumar: missing field `cantidad`"
        );
        assert!(app.call("contador.sumar", json!({"cantidad": -1})).is_err());

        assert_eq!(app.call("form.ultimo_envio", json!({})), Ok(json!(null)));
        let error = app.call("form.enviar", json!({})).unwrap_err();
        assert_eq!(error.message, "Falta el nombre");
        app.type_text(By::test_id("form.nombre"), "Ana");
        // «Enviar» answers with «Último envío», read after its update.
        assert_eq!(app.call("form.enviar", json!({})), Ok(json!("Ana")));
        assert_eq!(app.call("form.ultimo_envio", json!({})), Ok(json!("Ana")));
        assert_eq!(app.call("form.limpiar", json!({})), Ok(json!(null)));
        assert!(app.call("contador.nada", json!({})).is_err());

        let exposed: Vec<&str> = app
            .commands()
            .iter()
            .filter(|command| command.exposed)
            .map(|command| command.id)
            .collect();
        assert_eq!(
            exposed,
            [
                "contador.incrementar",
                "contador.sumar",
                "contador.reiniciar",
                "form.enviar",
                "form.limpiar",
                "form.ultimo_envio",
            ]
        );
    }

    fn agent(client: &str) -> Origin {
        Origin::Agent {
            client: client.to_owned(),
        }
    }

    fn denied() -> Option<Result<serde_json::Value, CommandError>> {
        Some(Err(CommandError::new("La persona rechazó «Reiniciar»")))
    }

    const DONE: Option<Result<serde_json::Value, CommandError>> = Some(Ok(serde_json::Value::Null));

    #[test]
    fn agents_confirm_destructive_actions_in_a_dialog() {
        let mut app = TestApp::new(Demo::default());
        app.invoke(Incrementar).unwrap();
        app.invoke(Incrementar).unwrap();

        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(reply.result(), None, "waits for the person");
        assert_eq!(app.app().clicks, 2);
        let screen = app.screen();
        let dialog = screen.get(By::role(Role::Dialog).name("¿Permitir «Reiniciar»?"));
        screen
            .within(dialog)
            .get(By::text("Cursor quiere ejecutar contador.reiniciar."));
        assert!(app.is_focused(By::test_id("kai.confirmar.rechazar")));
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
        assert!(!app.perform("contador.incrementar"), "the dialog is modal");
        app.press(Key::Char('p'), CMD_SHIFT);
        assert!(!app.session().palette_open());

        app.click(By::test_id("kai.confirmar.rechazar"));
        assert_eq!(reply.result(), denied());
        assert_eq!(app.app().clicks, 2);
        assert!(app.screen().query(By::role(Role::Dialog)).is_none());
        let last = app.audit_log().last().unwrap();
        assert_eq!(last.command, "contador.reiniciar");
        assert_eq!(last.origin, agent("Cursor"));
        assert_eq!(last.outcome, Outcome::Denied);

        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        app.press(Key::Escape, Modifiers::NONE);
        assert_eq!(reply.result(), denied(), "Escape rejects");
        assert_eq!(app.app().clicks, 2);

        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        app.click(By::test_id("kai.confirmar.una_vez"));
        assert_eq!(reply.result(), DONE);
        assert_eq!(app.app().clicks, 0);
        assert_eq!(app.audit_log().last().unwrap().outcome, Outcome::Done);

        app.invoke(Incrementar).unwrap();
        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(reply.result(), None, "allowed once asks again");
        app.click(By::test_id("kai.confirmar.sesion"));
        assert_eq!(reply.result(), DONE);
        app.invoke(Incrementar).unwrap();
        let reply = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(reply.result(), DONE, "allowed for the session");
        assert_eq!(app.app().clicks, 0);
    }

    #[test]
    fn agent_calls_wait_behind_the_dialog_and_can_be_cancelled() {
        let mut app = TestApp::new(Demo::default());
        app.invoke(Incrementar).unwrap();
        let reset = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        let add = app.agent_call("Cursor", "contador.incrementar", json!({}));
        assert_eq!((reset.result(), add.result()), (None, None));
        assert_eq!(app.app().clicks, 1);

        app.cancel_agent_call(reset.id());
        assert!(reset.result().unwrap().is_err());
        assert!(!app.session().confirmation_open());
        assert_eq!(add.result(), DONE, "the queue moves on");
        assert_eq!(app.app().clicks, 2);
        let outcomes: Vec<_> = app
            .audit_log()
            .entries()
            .map(|entry| (entry.command.as_str(), entry.outcome.clone()))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("contador.reiniciar", Outcome::Cancelled),
                ("contador.incrementar", Outcome::Done),
            ]
        );

        let first = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        let second = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        app.cancel_agent_call(second.id());
        assert!(second.result().unwrap().is_err());
        assert_eq!(first.result(), None);
        app.on_confirmation(Confirm::Deny);
        assert_eq!(first.result(), denied());
        assert_eq!(app.app().clicks, 2);
    }

    #[test]
    fn agent_calls_fail_before_asking_when_they_cannot_run() {
        let mut app = TestApp::new(Demo::default());
        let hidden = app.agent_call("Cursor", "form.alternar_contrasena", json!({}));
        assert_eq!(
            hidden.result(),
            Some(Err(CommandError::new(
                "form.alternar_contrasena no está expuesto a agentes"
            )))
        );
        let reset = app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(
            reset.result(),
            Some(Err(CommandError::new("El contador ya está en cero")))
        );
        let bad = app.agent_call("Cursor", "contador.sumar", json!({"cantidad": "dos"}));
        assert!(bad.result().unwrap().is_err());
        assert!(!app.session().confirmation_open());
        let query = app.agent_call("Cursor", "form.ultimo_envio", json!({}));
        assert_eq!(query.result(), DONE);
        assert_eq!(app.audit_log().len(), 3, "queries are not audited");
        assert!(
            app.audit_log()
                .entries()
                .all(|entry| matches!(entry.outcome, Outcome::Failed(_)))
        );
    }

    #[test]
    fn people_and_scripts_never_see_the_dialog() {
        let mut app = TestApp::new(Demo::default());
        app.invoke(Incrementar).unwrap();
        app.click(By::test_id("contador.reiniciar"));
        assert_eq!(app.app().clicks, 0);
        assert!(!app.session().confirmation_open());
        assert!(app.audit_log().is_empty(), "the person is not audited");

        app.call("contador.sumar", json!({"cantidad": 3})).unwrap();
        app.call("contador.reiniciar", json!({})).unwrap();
        assert!(!app.session().confirmation_open());
        let entries: Vec<_> = app.audit_log().entries().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].origin, Origin::Script);
        assert_eq!(entries[0].input, json!({"cantidad": 3}));
        assert_eq!(entries[1].outcome, Outcome::Done);
    }

    #[test]
    fn the_indicator_shows_the_agent_and_its_last_action() {
        let mut app = TestApp::new(Demo::default());
        assert!(app.screen().query(By::test_id("kai.agente")).is_none());
        app.agent_connected("Cursor");
        app.screen()
            .get(By::role(Role::Status).name("Agente: Cursor"));
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
        app.on_confirmation(Confirm::AllowOnce);
        app.agent_call("Cursor", "contador.incrementar", json!({}));
        app.agent_call("Cursor", "contador.reiniciar", json!({}));
        assert_eq!(app.app().clicks, 0);
        app.screen()
            .get(By::role(Role::Status).name("Agente: Cursor · Reiniciar"));
    }

    #[test]
    fn messages_record_where_they_came_from() {
        let mut app = TestApp::new(Demo::default());
        app.click(By::role(Role::Button).name("Incrementar"));
        app.press(Key::Char('i'), CMD);
        assert!(app.perform("contador.incrementar"));
        app.invoke(Incrementar).unwrap();
        app.call("contador.sumar", json!({"cantidad": 2})).unwrap();
        let button = app.screen().get(By::test_id("contador.incrementar")).id();
        app.accessibility_action(&accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: crate::a11y::node_id(button),
            data: None,
        });
        app.agent_call("kai-test", "contador.reiniciar", json!({}));
        app.click(By::test_id("kai.confirmar.una_vez"));

        assert_eq!(
            app.messages(),
            [
                Msg::Increment,
                Msg::Increment,
                Msg::Increment,
                Msg::Increment,
                Msg::Add(2),
                Msg::Increment,
                Msg::Reset,
            ]
        );
        assert_eq!(
            app.message_sources(),
            [
                MsgSource::Pointer,
                MsgSource::Keyboard,
                MsgSource::Menu,
                MsgSource::Script,
                MsgSource::Script,
                MsgSource::A11y,
                MsgSource::Agent("kai-test".into()),
            ],
            "an agent's call a click allowed still comes from the agent"
        );
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_lists_the_exposed_commands_as_tools() {
        let mut app = TestApp::new(Demo::default());
        let mut client = app.mcp_client();
        assert!(client.initialized()["capabilities"]["tools"].is_object());
        let tools = client.tools();
        let described = crate::commands::describe::<Demo>();
        let exposed: Vec<&serde_json::Value> = described["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|command| command["exposed"] == json!(true))
            .collect();
        assert_eq!(tools.len(), exposed.len());
        for (tool, command) in tools.iter().zip(exposed) {
            assert_eq!(tool["name"], command["id"]);
            assert_eq!(tool["title"], command["title"]);
            assert_eq!(tool["inputSchema"], command["inputSchema"]);
            let read_only = command["kind"] == "query";
            assert_eq!(tool["annotations"]["readOnlyHint"], json!(read_only));
            let destructive = command["safety"] == "destructive";
            assert_eq!(tool["annotations"]["destructiveHint"], json!(destructive));
            match command.get("outputSchema") {
                Some(output) => assert_eq!(
                    tool["outputSchema"]["properties"]["result"], *output,
                    "{} envuelve su salida",
                    command["id"]
                ),
                None => assert!(tool.get("outputSchema").is_none()),
            }
        }
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(!names.contains(&"form.alternar_contrasena"));
        assert!(
            client.close(),
            "el servidor termina cuando el cliente se va"
        );
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_destructive_calls_wait_for_the_person() {
        use crate::testing::MCP_CLIENT;

        let mut app = TestApp::new(Demo::default());
        let mut client = app.mcp_client();
        client
            .app()
            .screen()
            .get(By::role(Role::Status).name(format!("Agente: {MCP_CLIENT}")));
        client
            .call_tool("contador.sumar", json!({"cantidad": 3}))
            .unwrap();
        client
            .app()
            .screen()
            .get(By::role(Role::Status).name(format!("Agente: {MCP_CLIENT} · Sumar")));

        let reset = client.send_call("contador.reiniciar", json!({}));
        let screen = client.app().screen();
        let dialog = screen.get(By::role(Role::Dialog).name("¿Permitir «Reiniciar»?"));
        screen.within(dialog).get(By::text(format!(
            "{MCP_CLIENT} quiere ejecutar contador.reiniciar."
        )));
        client.app().click(By::test_id("kai.confirmar.rechazar"));
        let result = client.finish(reset).unwrap();
        assert_eq!(result["isError"], json!(true));
        assert_eq!(
            result["content"][0]["text"],
            json!("La persona rechazó «Reiniciar»")
        );
        assert_eq!(client.app().app().clicks, 3);
        let last = client.app().audit_log().last().unwrap().clone();
        assert_eq!(last.origin, agent(MCP_CLIENT));
        assert_eq!(last.outcome, Outcome::Denied);

        let reset = client.send_call("contador.reiniciar", json!({}));
        client.cancel(reset);
        assert!(!client.app().confirmation_open());
        assert_eq!(client.app().app().clicks, 3);
        let last = client.app().audit_log().last().unwrap().outcome.clone();
        assert_eq!(last, Outcome::Cancelled);

        client.app().on_confirmation(Confirm::AllowOnce);
        let result = client.call_tool("contador.reiniciar", json!({})).unwrap();
        assert_eq!(result["isError"], json!(false));
        assert_eq!(client.app().app().clicks, 0);
        let last = client.app().audit_log().last().unwrap().clone();
        assert_eq!(
            (last.origin, last.outcome),
            (agent(MCP_CLIENT), Outcome::Done)
        );
        assert!(client.close());
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_tool_calls_run_on_the_app() {
        let mut app = TestApp::new(Demo::default());
        let mut client = app.mcp_client();

        let result = client
            .call_tool("contador.sumar", json!({"cantidad": 2}))
            .unwrap();
        assert_eq!(result["isError"], json!(false));
        assert_eq!(client.app().app().clicks, 2);
        let screen = client.app().screen();
        screen
            .within(screen.get_by_test_id("contador"))
            .get(By::role(Role::Status).name("2"));

        let reset = client.send_call("contador.reiniciar", json!({}));
        assert!(client.app().confirmation_open());
        client.app().click(By::test_id("kai.confirmar.una_vez"));
        assert_eq!(client.finish(reset).unwrap()["isError"], json!(false));
        assert_eq!(client.app().app().clicks, 0);
        let result = client.call_tool("contador.reiniciar", json!({})).unwrap();
        assert_eq!(result["isError"], json!(true));
        assert_eq!(
            result["content"][0]["text"],
            json!("El contador ya está en cero"),
            "an action that cannot run fails without asking"
        );

        let result = client
            .call_tool("contador.sumar", json!({"cantidad": "dos"}))
            .unwrap();
        assert_eq!(result["isError"], json!(true));
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .starts_with("entrada inválida para contador.sumar")
        );

        let error = client
            .call_tool("form.alternar_contrasena", json!({}))
            .unwrap_err();
        assert!(error["message"].as_str().unwrap().contains("desconocida"));
        assert_eq!(client.app().app().clicks, 0);

        client.app().type_text(By::test_id("form.nombre"), "Ana");
        let result = client.call_tool("form.enviar", json!({})).unwrap();
        assert_eq!(result["structuredContent"], json!({"result": "Ana"}));
        let result = client.call_tool("form.ultimo_envio", json!({})).unwrap();
        assert_eq!(result["structuredContent"], json!({"result": "Ana"}));
        assert_eq!(result["content"][0]["text"], json!(r#"{"result":"Ana"}"#));
        assert!(client.close());
    }

    /// Lines that differ between `expected` and `actual`, numbered; `None`
    /// when they are equal.
    fn line_diff(expected: &str, actual: &str) -> Option<String> {
        let expected: Vec<&str> = expected.lines().collect();
        let actual: Vec<&str> = actual.lines().collect();
        let mut diff = String::new();
        for line in 0..expected.len().max(actual.len()) {
            let (old, new) = (expected.get(line), actual.get(line));
            if old != new {
                for (sign, text) in [("-", old), ("+", new)] {
                    if let Some(text) = text {
                        diff.push_str(&format!("{:>4} {sign} {text}\n", line + 1));
                    }
                }
            }
        }
        (!diff.is_empty()).then_some(diff)
    }

    /// `mcp-schema.json` is what the demo publishes to agents; a change
    /// must be deliberate. `KAI_UPDATE_SCHEMA=1 cargo test mcp_schema`
    /// rewrites it.
    #[test]
    fn mcp_schema_matches_snapshot() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("mcp-schema.json");
        let actual = describe_json::<Demo>() + "\n";
        if std::env::var_os("KAI_UPDATE_SCHEMA").is_some() {
            std::fs::write(&path, &actual).expect("write mcp-schema.json");
            return;
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_default();
        if let Some(diff) = line_diff(&expected, &actual) {
            panic!(
                "mcp-schema.json cambió; si es a propósito, KAI_UPDATE_SCHEMA=1 cargo test \
                 mcp_schema\n{diff}"
            );
        }
        let document: serde_json::Value = serde_json::from_str(&actual).unwrap();
        assert_eq!(document["$schema"], DIALECT);
        let ids: Vec<&str> = document["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|command| command["id"].as_str())
            .collect();
        assert!(ids.contains(&"contador.sumar") && ids.contains(&"form.ultimo_envio"));
        assert!(ids.iter().all(|id| !id.starts_with("kai.")), "{ids:?}");
    }

    #[test]
    fn actions_with_input_stay_out_of_menus_and_palette() {
        let mut app = TestApp::new(Demo::default());
        let sumar = app.commands().iter().find(|c| c.id == "contador.sumar");
        assert!(sumar.is_some_and(|c| c.needs_input));
        let menu = menu_model(app.commands());
        assert!(
            menu.iter()
                .flat_map(|g| &g.items)
                .all(|e| e.id != "contador.sumar")
        );
        app.press(Key::Char('p'), CMD_SHIFT);
        app.type_text(search(), "Sumar");
        assert!(app.screen().query(row("contador.sumar")).is_none());
        assert!(app.screen().query(By::text("Sin resultados")).is_some());
    }

    #[test]
    fn query_reads_the_model_without_msg() {
        let mut app = TestApp::new(Demo::default());
        assert_eq!(app.query(Saludo), Ok(None));
        app.type_text(By::test_id("form.nombre"), "Ana");
        app.invoke(Enviar).unwrap();
        let sent = app.messages().len();
        assert_eq!(app.query(Saludo), Ok(Some("Ana".to_owned())));
        assert_eq!(app.messages().len(), sent, "a query sends nothing");
    }

    /// Scroll offsets that bring the card titled `title` to the top of the
    /// content area, measured with real text.
    fn scrolled_to(theme: &Theme, demo: &Demo, viewport: Size, title: &str) -> ScrollOffsets {
        let tree = build_view(demo);
        let mut text = TextStore::new(FontSystem::new());
        let layout = Layout::compute(
            &tree,
            &LayoutInput {
                viewport,
                theme,
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            &mut StyleCache::default(),
            &mut text,
        );
        let content = Screen::new(&layout).get_by_test_id("contenido").id();
        let card = card_header_of(demo, title);
        let top = layout.bounds(card).unwrap().y - layout.bounds(content).unwrap().y;
        ScrollOffsets::from([(content, top - 40.0)])
    }

    /// Where a set of captures goes and with which theme.
    struct Out {
        theme: Theme,
        /// Relative to the crate root.
        dir: &'static str,
        /// Appended to every file name.
        suffix: &'static str,
        /// Also write a PNG next to each BMP (with `--features snapshots`).
        png: bool,
    }

    /// Render the demo offscreen exactly as the runtime does and save it as
    /// a BMP under `out.dir`.
    fn snapshot(
        out: &Out,
        name: &str,
        demo: &Demo,
        logical: Size,
        scale: f32,
        interaction: &Interaction,
        scroll: &ScrollOffsets,
    ) {
        let tree = build_view(demo);
        let image = snapshot::render(&Shot {
            tree: &tree,
            theme: &out.theme,
            logical,
            scale,
            interaction,
            scroll,
        })
        .expect("snapshot render");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(out.dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = format!("{name}{}", out.suffix);
        snapshot::write_bmp(&dir.join(format!("{file}.bmp")), &image).unwrap();
        #[cfg(feature = "snapshots")]
        if out.png {
            snapshot::write_png(&dir.join(format!("{file}.png")), &image).unwrap();
        }
        // PNG output needs `--features snapshots`.
        #[cfg(not(feature = "snapshots"))]
        let _ = out.png;
    }

    /// Writes `target/snapshots/*.bmp` for visual review. Needs a GPU:
    /// `cargo test -- --ignored snapshot_demo`
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshot_demo() {
        render_demo_shots(&Out {
            theme: Theme::kai(Density::Comfortable),
            dir: "target/snapshots",
            suffix: "",
            png: false,
        });
    }

    /// The same captures with the static AppKit theme, light and dark, as
    /// BMP and PNG under `target/snapshots/appkit/` (not part of the
    /// baseline). `cargo test --features snapshots -- --ignored snapshots_appkit`
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_appkit() {
        for (generation, appearance, suffix) in [
            (AppKitGeneration::Aqua11, Appearance::Light, ""),
            (AppKitGeneration::Aqua11, Appearance::Dark, "-dark"),
            (AppKitGeneration::Glass27, Appearance::Light, "-glass27"),
            (AppKitGeneration::Glass27, Appearance::Dark, "-glass27-dark"),
        ] {
            render_demo_shots(&Out {
                theme: Theme::appkit_static(generation, appearance),
                dir: "target/snapshots/appkit",
                suffix,
                png: true,
            });
        }
    }

    /// Mixed-theme candidates side by side, for Felipe to choose:
    /// `target/snapshots/mix/mix-compare-{light,dark}.png` (rows Kai |
    /// strict AppKit | A | B | C; columns buttons card | fields card) plus
    /// each candidate's variants-focus and fields shots.
    /// `cargo test --features snapshots -- --ignored snapshots_mix`
    #[cfg(feature = "snapshots")]
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_mix() {
        use crate::element::{div, text};
        use crate::snapshot::Image;
        use crate::theme::MixStyle;

        const SCALE: f32 = 2.0;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots/mix");
        std::fs::create_dir_all(&dir).unwrap();
        fn render(
            theme: &Theme,
            demo: &Demo,
            logical: Size,
            interaction: &Interaction,
            scroll: &ScrollOffsets,
        ) -> Image {
            let tree = build_view(demo);
            snapshot::render(&Shot {
                tree: &tree,
                theme,
                logical,
                scale: SCALE,
                interaction,
                scroll,
            })
            .expect("snapshot render")
        }
        // Card `title` at the top of an 800 × `height` crop, below the header.
        let section = |theme: &Theme, demo: &Demo, title: &str, height: f32| -> Image {
            let view = Size {
                width: 800.0,
                height: height + 160.0,
            };
            let scroll = scrolled_to(theme, demo, view, title);
            let full = render(theme, demo, view, &Interaction::default(), &scroll);
            let tree = build_view(demo);
            let mut text = TextStore::new(FontSystem::new());
            let layout = Layout::compute(
                &tree,
                &LayoutInput {
                    viewport: view,
                    theme,
                    interaction: &Interaction::default(),
                    scroll: &scroll,
                },
                &mut StyleCache::default(),
                &mut text,
            );
            let content = Screen::new(&layout).get_by_test_id("contenido");
            let top = content.bounds().y + 28.0;
            crop(&full, (top * SCALE) as u32, (height * SCALE) as u32)
        };
        let label = |caption: &str| -> Image {
            let mut tree: crate::element::Element<Msg> = div()
                .class("w-full h-full flex items-center px-4 bg-neutral")
                .child(text(caption).class("text-lg font-semibold text-foreground"));
            tree.assign_ids();
            snapshot::render(&Shot {
                tree: &tree,
                theme: &Theme::kai(Density::Comfortable),
                logical: Size {
                    width: 1600.0,
                    height: 36.0,
                },
                scale: SCALE,
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            })
            .expect("label render")
        };
        let demo = Demo {
            clicks: 3,
            ..Demo::default()
        };
        let filled = Demo {
            password: "kai-2026".to_owned(),
            ..Demo::default()
        };
        for (generation, appearance, name) in [
            (AppKitGeneration::Aqua11, Appearance::Light, "light"),
            (AppKitGeneration::Glass27, Appearance::Dark, "dark"),
        ] {
            let strict = Theme::appkit_static(generation, appearance);
            let rows = [
                ("Kai (referencia)", Theme::kai(Density::Comfortable), "kai"),
                ("AppKit estricto (KAI_MIX=appkit)", strict, "appkit"),
                (
                    "A · cerca de AppKit (KAI_MIX=a)",
                    strict.with_mix(MixStyle::A),
                    "a",
                ),
                (
                    "B · intermedio (KAI_MIX=b)",
                    strict.with_mix(MixStyle::B),
                    "b",
                ),
                (
                    "C · cerca de Kai (KAI_MIX=c, por defecto)",
                    strict.with_mix(MixStyle::C),
                    "c",
                ),
            ];
            let mut sheet: Vec<Image> = Vec::new();
            for (caption, theme, tag) in &rows {
                let buttons = section(theme, &demo, BUTTONS_CARD, 300.0);
                let fields = section(theme, &filled, "Variantes de TextField", 470.0);
                sheet.push(label(&format!("{caption} — {generation:?} {appearance:?}")));
                sheet.push(side_by_side(&buttons, &fields));
                if *tag != "kai" {
                    let wide = Size {
                        width: 1024.0,
                        height: 720.0,
                    };
                    let shots = [
                        (
                            "demo-variants-focus",
                            render(
                                theme,
                                &demo,
                                wide,
                                &Interaction {
                                    focused: Some(find(&demo, By::test_id("botones.secondary"))),
                                    focus_visible: true,
                                    ..Interaction::default()
                                },
                                &scrolled_to(theme, &demo, wide, BUTTONS_CARD),
                            ),
                        ),
                        ("demo-fields", {
                            let view = Size {
                                width: 1024.0,
                                height: 640.0,
                            };
                            let scroll =
                                scrolled_to(theme, &filled, view, "Variantes de TextField");
                            render(theme, &filled, view, &Interaction::default(), &scroll)
                        }),
                    ];
                    for (shot, image) in shots {
                        let path = dir.join(format!("{shot}-{name}-{tag}.png"));
                        snapshot::write_png(&path, &image).unwrap();
                    }
                }
            }
            let path = dir.join(format!("mix-compare-{name}.png"));
            snapshot::write_png(&path, &stack(&sheet)).unwrap();
            println!("{}", path.display());
        }
    }

    /// Renders for Felipe's review under `target/snapshots/renders/`
    /// (`KAI_RENDER_TAG` names the form-row sheet, default `after`):
    /// the form row at wide and narrow widths.
    /// `cargo test --features snapshots -- --ignored snapshots_renders`
    #[cfg(feature = "snapshots")]
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_renders() {
        use crate::components::{TextField, card, card_content};
        use crate::element::{Element, div, text};
        use crate::theme::{FieldLabels, MixStyle};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots/renders");
        std::fs::create_dir_all(&dir).unwrap();
        let tag = std::env::var("KAI_RENDER_TAG").unwrap_or_else(|_| "after".to_owned());
        let demo = Demo {
            name: "Ana".to_owned(),
            ..Demo::default()
        };
        let appkit = |appearance| {
            Theme::appkit_static(AppKitGeneration::Aqua11, appearance).with_mix(MixStyle::C)
        };
        // Form row: Kai and mix C, wide | narrow.
        let mut rows = Vec::new();
        for theme in [Theme::kai(Density::Comfortable), appkit(Appearance::Light)] {
            let shot = |width: f32| {
                let view = Size {
                    width,
                    height: 420.0,
                };
                let scroll = scrolled_to(&theme, &demo, view, "Formulario controlado");
                render_image(&theme, &demo, view, &Interaction::default(), &scroll)
            };
            rows.push(side_by_side(&shot(1024.0), &shot(420.0)));
        }
        let path = dir.join(format!("form-row-{tag}.png"));
        snapshot::write_png(&path, &stack(&rows)).unwrap();
        println!("{}", path.display());

        // Whole demo, mix C light (white window), tall enough to show
        // every card.
        let whole = Size {
            width: 1024.0,
            height: 3000.0,
        };
        let image = render_image(
            &appkit(Appearance::Light),
            &demo,
            whole,
            &Interaction::default(),
            &ScrollOffsets::new(),
        );
        let path = dir.join("demo-white-light.png");
        snapshot::write_png(&path, &image).unwrap();
        println!("{}", path.display());

        // Control sizes card: Kai, mix C light and dark.
        let clicks = Demo {
            clicks: 3,
            ..Demo::default()
        };
        for (name, theme) in [
            ("kai", Theme::kai(Density::Comfortable)),
            ("appkit-light", appkit(Appearance::Light)),
            ("appkit-dark", appkit(Appearance::Dark)),
        ] {
            let image = card_shot(&theme, &clicks, "Tamaños de control", 900.0, 700.0);
            let path = dir.join(format!("sizes-{name}.png"));
            snapshot::write_png(&path, &image).unwrap();
            println!("{}", path.display());
        }

        // Mix C field labels, appkit | top | near (default) | tab, on a group box
        // whose fill differs from the fields': empty, filled, focused
        // (empty) and required filled.
        for (name, appearance) in [("light", Appearance::Light), ("dark", Appearance::Dark)] {
            let theme = appkit(appearance);
            let columns = [
                ("KAI_LABEL=appkit", FieldLabels::AppKit),
                ("KAI_LABEL=top", FieldLabels::Top),
                ("near (default)", FieldLabels::Near),
                ("KAI_LABEL=tab", FieldLabels::Tab),
            ]
            .map(|(title, labels)| {
                let mut tree: Element<Msg> = div()
                    .class("w-full h-full flex flex-col gap-3 p-4 bg-window")
                    .children([
                        text(title).class("text-headline text-label"),
                        card()
                            .class("w-full")
                            .child(card_content().class("gap-4").children([
                                TextField::new("Nombre", "").build(),
                                TextField::new("Correo", "ana@kai.cl").build(),
                                TextField::new("Ciudad", "").build(),
                                TextField::new("Comuna", "Ñuñoa").required(true).build(),
                            ])),
                    ]);
                tree.assign_ids();
                let mut focused = None;
                tree.walk(&mut |e| {
                    if matches!(e.kind, Kind::Input { .. }) && e.label.as_deref() == Some("Ciudad")
                    {
                        focused = Some(e.id);
                    }
                });
                snapshot::render(&Shot {
                    tree: &tree,
                    theme: &theme.with_field_labels(labels),
                    logical: Size {
                        width: 300.0,
                        height: 400.0,
                    },
                    scale: 2.0,
                    interaction: &Interaction {
                        focused,
                        ..Interaction::default()
                    },
                    scroll: &ScrollOffsets::new(),
                })
                .expect("labels render")
            });
            let sheet = columns[1..]
                .iter()
                .fold(columns[0].clone(), |sheet, column| {
                    side_by_side(&sheet, column)
                });
            let path = dir.join(format!("labels-compare-{name}.png"));
            snapshot::write_png(&path, &sheet).unwrap();
            println!("{}", path.display());
        }

        // Mix C field (near label) next to every button size, in an
        // `items-end` row and an `items-center` row, regular and compact.
        for (name, appearance) in [
            ("field-button-align", Appearance::Light),
            ("field-button-align-dark", Appearance::Dark),
        ] {
            use crate::components::{ButtonVariant, FieldDensity, button};
            use crate::element::ControlSize;
            let theme = appkit(appearance);
            let row = |items: &str, density: FieldDensity, variant: ButtonVariant| {
                div()
                    .class(&format!("flex gap-2 {items}"))
                    .child(
                        div()
                            .class("w-[220px]")
                            .child(TextField::new("Campo", "").density(density).build()),
                    )
                    .children(
                        ControlSize::ALL
                            .map(|size| button(format!("{size:?}"), variant).control_size(size)),
                    )
            };
            let mut tree: Element<Msg> = div()
                .class("w-full h-full flex flex-col gap-4 p-4 bg-window")
                .children(["items-end", "items-center"].map(|items| {
                    div().class("flex flex-col gap-3").children([
                        text(format!("{items} (campo normal y compacto)"))
                            .class("text-headline text-label"),
                        row(items, FieldDensity::Default, ButtonVariant::Primary),
                        row(items, FieldDensity::Default, ButtonVariant::Outlined),
                        row(items, FieldDensity::Compact, ButtonVariant::Primary),
                        row(items, FieldDensity::Compact, ButtonVariant::Outlined),
                    ])
                }));
            tree.assign_ids();
            let image = snapshot::render(&Shot {
                tree: &tree,
                theme: &theme,
                logical: Size {
                    width: 760.0,
                    height: 640.0,
                },
                scale: 2.0,
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            })
            .expect("field-button render");
            let path = dir.join(format!("{name}.png"));
            snapshot::write_png(&path, &image).unwrap();
            println!("{}", path.display());
        }
    }

    /// Background effects under the demo, as the runtime draws them, under
    /// `target/snapshots/renders/`: `bg-{none,mesh,aurora,glow}-{kai,
    /// appkit-light,appkit-dark}.png` (static frames, top of the page with
    /// the panel), `bg-sheet-*.png` side by side, and `bg-{kind}-t{s}.png`
    /// frames of the animated effects at a few times.
    /// `cargo test --features snapshots -- --ignored snapshots_backgrounds`
    #[cfg(feature = "snapshots")]
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshots_backgrounds() {
        use crate::background::{BackgroundKind, BackgroundMotion};
        use crate::theme::MixStyle;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots/renders");
        std::fs::create_dir_all(&dir).unwrap();
        let view = Size {
            width: 1024.0,
            height: 720.0,
        };
        let shot = |theme: &Theme, demo: &Demo, seconds: f32| {
            let tree = build_view(demo);
            snapshot::render_with_background(
                &Shot {
                    tree: &tree,
                    theme,
                    logical: view,
                    scale: 2.0,
                    interaction: &Interaction::default(),
                    scroll: &ScrollOffsets::new(),
                },
                demo.background(),
                seconds,
            )
            .expect("background render")
        };
        let mix = |appearance| {
            Theme::appkit_static(AppKitGeneration::Aqua11, appearance).with_mix(MixStyle::C)
        };
        let themes = [
            ("kai", Theme::kai(Density::Comfortable)),
            ("appkit-light", mix(Appearance::Light)),
            ("appkit-dark", mix(Appearance::Dark)),
        ];
        for (name, theme) in &themes {
            let mut row: Option<crate::snapshot::Image> = None;
            for kind in BackgroundKind::ALL {
                let demo = Demo {
                    background: kind,
                    ..Demo::default()
                };
                let image = shot(theme, &demo, 0.0);
                let path = dir.join(format!("bg-{}-{name}.png", kind.name()));
                snapshot::write_png(&path, &image).unwrap();
                println!("{}", path.display());
                row = Some(match row {
                    None => image,
                    Some(row) => side_by_side(&row, &image),
                });
            }
            let path = dir.join(format!("bg-sheet-{name}.png"));
            snapshot::write_png(&path, &row.unwrap()).unwrap();
            println!("{}", path.display());
        }
        // Motion: the animated effects at 0, 4 and 8 s of a 16 s cycle.
        let theme = mix(Appearance::Light);
        for kind in [
            BackgroundKind::Mesh,
            BackgroundKind::Aurora,
            BackgroundKind::Glow,
        ] {
            let demo = Demo {
                background: kind,
                background_motion: BackgroundMotion::Animated,
                ..Demo::default()
            };
            assert!(demo.background().is_animated());
            let frames: Vec<_> = [0.0, 4.0, 8.0]
                .into_iter()
                .map(|seconds| {
                    let image = shot(&theme, &demo, seconds);
                    let path = dir.join(format!("bg-{}-t{seconds}.png", kind.name()));
                    snapshot::write_png(&path, &image).unwrap();
                    println!("{}", path.display());
                    image
                })
                .collect();
            assert_ne!(frames[0], frames[1], "{kind:?} moves");
            let sheet = side_by_side(&side_by_side(&frames[0], &frames[1]), &frames[2]);
            let path = dir.join(format!("bg-{}-motion.png", kind.name()));
            snapshot::write_png(&path, &sheet).unwrap();
            println!("{}", path.display());
        }
        // Static effects ignore the time.
        let still = Demo {
            background: BackgroundKind::Aurora,
            ..Demo::default()
        };
        assert_eq!(shot(&theme, &still, 0.0), shot(&theme, &still, 5.0));
    }

    /// The card titled `title`, `width` × `height` points from just above
    /// its top (below the header), at 2x.
    #[cfg(feature = "snapshots")]
    fn card_shot(
        theme: &Theme,
        demo: &Demo,
        title: &str,
        width: f32,
        height: f32,
    ) -> crate::snapshot::Image {
        let view = Size {
            width,
            height: height + 160.0,
        };
        let scroll = scrolled_to(theme, demo, view, title);
        let full = render_image(theme, demo, view, &Interaction::default(), &scroll);
        let tree = build_view(demo);
        let mut text = TextStore::new(FontSystem::new());
        let layout = Layout::compute(
            &tree,
            &LayoutInput {
                viewport: view,
                theme,
                interaction: &Interaction::default(),
                scroll: &scroll,
            },
            &mut StyleCache::default(),
            &mut text,
        );
        // 28 pt above the title: the card's top edge plus a margin.
        let header = Screen::new(&layout).get(By::role(Role::Header)).bounds();
        let title = layout.bounds(card_header_of(demo, title)).unwrap().y - 28.0;
        let top = title.max(header.y + header.height);
        crop(&full, (top * 2.0) as u32, (height * 2.0) as u32)
    }

    /// The demo at `logical` (2x), as the runtime draws it.
    #[cfg(feature = "snapshots")]
    fn render_image(
        theme: &Theme,
        demo: &Demo,
        logical: Size,
        interaction: &Interaction,
        scroll: &ScrollOffsets,
    ) -> crate::snapshot::Image {
        let tree = build_view(demo);
        snapshot::render(&Shot {
            tree: &tree,
            theme,
            logical,
            scale: 2.0,
            interaction,
            scroll,
        })
        .expect("snapshot render")
    }

    /// Rows `top..top + height` of `image` (clamped).
    #[cfg(feature = "snapshots")]
    fn crop(image: &crate::snapshot::Image, top: u32, height: u32) -> crate::snapshot::Image {
        let top = top.min(image.height);
        let height = height.min(image.height - top);
        let row = (image.width * 4) as usize;
        let start = top as usize * row;
        crate::snapshot::Image {
            width: image.width,
            height,
            rgba: image.rgba[start..start + height as usize * row].to_vec(),
        }
    }

    /// `a` and `b` next to each other, top-aligned, on white.
    #[cfg(feature = "snapshots")]
    fn side_by_side(
        a: &crate::snapshot::Image,
        b: &crate::snapshot::Image,
    ) -> crate::snapshot::Image {
        let (width, height) = (a.width + b.width, a.height.max(b.height));
        let mut rgba = vec![255; (width * height * 4) as usize];
        for (image, x0) in [(a, 0), (b, a.width)] {
            for y in 0..image.height {
                let src = (y * image.width * 4) as usize;
                let dst = ((y * width + x0) * 4) as usize;
                rgba[dst..dst + (image.width * 4) as usize]
                    .copy_from_slice(&image.rgba[src..src + (image.width * 4) as usize]);
            }
        }
        crate::snapshot::Image {
            width,
            height,
            rgba,
        }
    }

    /// `images` one below the other, left-aligned, on white.
    #[cfg(feature = "snapshots")]
    fn stack(images: &[crate::snapshot::Image]) -> crate::snapshot::Image {
        let width = images.iter().map(|i| i.width).max().unwrap_or(0);
        let mut rgba = Vec::new();
        let mut height = 0;
        for image in images {
            for y in 0..image.height {
                let src = (y * image.width * 4) as usize;
                rgba.extend_from_slice(&image.rgba[src..src + (image.width * 4) as usize]);
                rgba.resize(rgba.len() + ((width - image.width) * 4) as usize, 255);
            }
            height += image.height;
        }
        crate::snapshot::Image {
            width,
            height,
            rgba,
        }
    }

    fn render_demo_shots(out: &Out) {
        let wide = Size {
            width: 1024.0,
            height: 720.0,
        };
        let idle = Demo::default();
        let none = ScrollOffsets::new();
        snapshot(
            out,
            "demo-idle",
            &idle,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        let mut typed = Demo {
            clicks: 3,
            ..Demo::default()
        };
        typed.update(Msg::NameChanged("Ana".to_owned()));
        let hovered_button = find(&typed, By::role(Role::Button).name("Incrementar"));
        let field = find(&typed, By::label("Nombre"));
        let tree = build_view(&typed);
        snapshot(
            out,
            "demo-interaction",
            &typed,
            wide,
            2.0,
            &Interaction {
                hovered: tree.path_to(hovered_button).unwrap(),
                focused: Some(field),
                ..Interaction::default()
            },
            &none,
        );
        snapshot(
            out,
            "demo-narrow",
            &typed,
            Size {
                width: 420.0,
                height: 560.0,
            },
            2.0,
            &Interaction::default(),
            &none,
        );

        snapshot(
            out,
            "demo-variants-focus",
            &typed,
            wide,
            2.0,
            &Interaction {
                focused: Some(find(&typed, By::test_id("botones.secondary"))),
                // Reached with Tab: the ring is drawn.
                focus_visible: true,
                ..Interaction::default()
            },
            &scrolled_to(&out.theme, &typed, wide, BUTTONS_CARD),
        );

        let fields_view = Size {
            width: 1024.0,
            height: 640.0,
        };
        let title = "Variantes de TextField";
        let masked = Demo {
            password: "kai-2026".to_owned(),
            ..Demo::default()
        };
        snapshot(
            out,
            "demo-fields",
            &masked,
            fields_view,
            2.0,
            &Interaction::default(),
            &scrolled_to(&out.theme, &masked, fields_view, title),
        );
        let mut filled = Demo {
            show_password: true,
            ..Demo::default()
        };
        for (field, value) in [
            (Field::Search, "zapatillas"),
            (Field::Password, "kai-2026"),
            (Field::Amount, "15000"),
            (Field::Code, "A-12"),
            (Field::Quantity, "3"),
        ] {
            filled.update(Msg::Edit(field, value.to_owned()));
        }
        let password = find(&filled, By::test_id("form.password.input"));
        let tree = build_view(&filled);
        snapshot(
            out,
            "demo-fields-focus",
            &filled,
            fields_view,
            2.0,
            &Interaction {
                focused: Some(password),
                hovered: tree.path_to(password).unwrap(),
                ..Interaction::default()
            },
            &scrolled_to(&out.theme, &filled, fields_view, title),
        );
    }

    #[test]
    fn editor_inserts_unicode_and_selects_text() {
        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(15.0, 22.0));
        buffer.set_text("", &Attrs::new(), Shaping::Advanced, None);
        let mut editor = Editor::new(buffer);
        editor.insert_string("Kai 😀", None);
        editor.shape_as_needed(&mut font_system, false);
        editor.action(&mut font_system, Action::Motion(Motion::BufferStart));
        let anchor = editor.cursor();
        editor.set_selection(Selection::Normal(anchor));
        editor.action(&mut font_system, Action::Motion(Motion::BufferEnd));
        assert_eq!(editor.copy_selection().as_deref(), Some("Kai 😀"));
        editor.insert_string("Kai UI", None);
        let value = editor.with_buffer(|buffer| buffer.lines[0].text().to_owned());
        assert_eq!(value, "Kai UI");
    }
}
