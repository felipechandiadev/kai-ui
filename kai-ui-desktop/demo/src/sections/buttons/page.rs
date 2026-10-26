//! /buttons: variants, sizes and states of Button and IconButton, and
//! buttons that wait for a task.

use std::time::Duration;

use kai::commands::{CommandRegistry, PageAction};
use kai::components::{
    BezelStyle, Button, ButtonSize, ButtonVariant, DotProgress, DotSize, IconButtonSize,
    IconButtonVariant, button, button_sized, caption, card, card_content, card_header,
    dot_progress, icon_button, icon_button_sized, page, page_body,
};
use kai::element::{ControlSize, Element, Role, div};
use kai::icons::Icon;
use kai::router::{Context, HasPage, Page};
use kai::runtime::{Effects, current_invocation};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::State;

/// How long the waiting buttons wait.
const WAIT: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct Screen {
    /// Clicks on the buttons that do something, shown in the header.
    clicks: u32,
    /// The last wait that ended.
    waited: Option<&'static str>,
    /// Background syncs still running.
    syncing: u32,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Click,
    Reset,
    Wait(&'static str),
    WaitDone(&'static str),
    SaveAndReload,
    Reload,
    Sync,
    Synced,
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
            Msg::Click => self.clicks += 1,
            Msg::Reset => self.clicks = 0,
            Msg::Wait(name) => {
                let mut effect =
                    Effects::perform(|| std::thread::sleep(WAIT), move |()| Msg::WaitDone(name));
                // A script or an agent hears back when the wait ends.
                if let Some(invocation) = current_invocation() {
                    effect = effect.completes(invocation);
                }
                return effect;
            }
            Msg::WaitDone(name) => self.waited = Some(name),
            Msg::SaveAndReload => {
                return Effects::perform(|| std::thread::sleep(WAIT / 2), |()| Msg::Reload);
            }
            Msg::Reload => {
                return Effects::perform(
                    || std::thread::sleep(WAIT / 2),
                    |()| Msg::WaitDone("guardar y recargar"),
                );
            }
            Msg::Sync => {
                self.syncing += 1;
                return Effects::perform(|| std::thread::sleep(WAIT), |()| Msg::Synced);
            }
            Msg::Synced => self.syncing = self.syncing.saturating_sub(1),
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().children([
            caption(format!("Clics: {}", self.clicks))
                .role(Role::Status)
                .test_id("buttons.clicks"),
            variants(),
            states(),
            self.waiting(),
            appkit(),
            control_sizes(),
        ]))
    }

    fn title(&self, _: &State) -> String {
        "Botones".into()
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        registry
            .page_action::<SimulateWait>()
            .menu("Botones")
            .expose();
    }
}

/// `buttons.simulate_wait`: the wait of the buttons, from the menu, the
/// palette, a script or an agent.
#[derive(Clone, Deserialize, JsonSchema)]
pub struct SimulateWait;

impl PageAction for SimulateWait {
    type Page = Screen;
    const ID: &'static str = "buttons.simulate_wait";
    const TITLE: &'static str = "Simular espera";
    fn description() -> &'static str {
        "Espera 1,5 s, como los botones de «Acciones con espera»"
    }
    fn into_msg(self) -> Msg {
        Msg::Wait("comando")
    }
}

fn variants() -> Element<Msg> {
    let icon = |glyph, label, variant| icon_button_sized(glyph, label, variant, IconButtonSize::Md);
    card().class("w-full max-w-3xl").children([
        card_header(
            "Botones, Tamaños e IconButtons",
            Some("Variantes visuales, tamaños y botones de iconos"),
        ),
        card_content().class("gap-4").children([
            div().class("flex flex-wrap items-center gap-3").children([
                button("Primary", ButtonVariant::Primary).on_click(Msg::Click),
                button("Secondary", ButtonVariant::Secondary)
                    .test_id("buttons.secondary")
                    .on_click(Msg::Click),
                button("Outlined", ButtonVariant::Outlined).on_click(Msg::Click),
                button("Text", ButtonVariant::Text).on_click(Msg::Click),
                button("Danger", ButtonVariant::Danger)
                    .on_click(Msg::Reset)
                    .test_id("buttons.danger"),
                button("Deshabilitado", ButtonVariant::Primary)
                    .on_click(Msg::Click)
                    .disabled(true)
                    .test_id("buttons.disabled"),
            ]),
            div().class("flex items-center gap-3 pt-2").children([
                caption("Tamaños:"),
                button_sized("Pequeño", ButtonVariant::Primary, ButtonSize::Sm),
                button_sized("Mediano", ButtonVariant::Primary, ButtonSize::Md),
                button_sized("Grande", ButtonVariant::Primary, ButtonSize::Lg),
            ]),
            div().class("flex items-center gap-3 pt-2").children([
                caption("IconButtons:"),
                icon(Icon::Search, "Buscar", IconButtonVariant::Primary),
                icon(Icon::User, "Usuario", IconButtonVariant::Secondary),
                icon(Icon::Info, "Info", IconButtonVariant::Action),
                icon(Icon::Trash2, "Eliminar", IconButtonVariant::Outlined),
            ]),
        ]),
    ])
}

fn states() -> Element<Msg> {
    let row = |label: &str, children: Vec<Element<Msg>>| {
        div().class("flex flex-col gap-2").children([
            caption(label),
            div()
                .class("flex flex-wrap items-center gap-3")
                .children(children),
        ])
    };
    let variants = [
        ("Primary", ButtonVariant::Primary),
        ("Secondary", ButtonVariant::Secondary),
        ("Outlined", ButtonVariant::Outlined),
        ("Danger", ButtonVariant::Danger),
    ];
    card()
        .class("w-full max-w-3xl")
        .test_id("buttons.states")
        .children([
            card_header(
                "Estados de Button",
                Some("Primary / Secondary / Outlined / Danger, disabled y loading"),
            ),
            card_content().class("gap-4").children([
                row(
                    "Variantes",
                    variants
                        .map(|(name, variant)| button(name, variant))
                        .into(),
                ),
                row(
                    "Disabled",
                    variants
                        .map(|(name, variant)| button(name, variant).disabled(true))
                        .into(),
                ),
                row(
                    "Loading",
                    vec![
                        Button::new("Primary")
                            .accent(true)
                            .loading(true)
                            .on_click(Msg::Click)
                            .build(),
                        Button::new("Secondary")
                            .loading(true)
                            .on_click(Msg::Click)
                            .build(),
                        button("Outlined", ButtonVariant::Outlined)
                            .on_click(Msg::Click)
                            .busy(true),
                        Button::new("Danger")
                            .destructive(true)
                            .loading(true)
                            .on_click(Msg::Click)
                            .build(),
                    ],
                ),
            ]),
        ])
}

impl Screen {
    /// Buttons that start a task: busy, with the dots over their label,
    /// until it ends. A chained save and reload, one that opts out, one
    /// the app keeps loading, and `DotProgress` alone in its three sizes.
    fn waiting(&self) -> Element<Msg> {
        let variants = [
            ("Primary", ButtonVariant::Primary),
            ("Secondary", ButtonVariant::Secondary),
            ("Outlined", ButtonVariant::Outlined),
            ("Text", ButtonVariant::Text),
            ("Danger", ButtonVariant::Danger),
        ];
        let status = match (self.waited, self.syncing) {
            (_, 1..) => format!("Sincronizando ({})…", self.syncing),
            (Some(name), _) => format!("Terminó: {name}"),
            (None, _) => "Pulsa un botón: espera 1,5 s".to_owned(),
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("buttons.wait")
            .children([
                card_header(
                    "Acciones con espera",
                    Some(
                        "El botón que lanza una tarea muestra los puntos y no responde \
                         hasta que termina",
                    ),
                ),
                card_content().class("gap-4").children([
                    div()
                        .class("flex flex-wrap items-center gap-3")
                        .children(variants.map(|(name, variant)| {
                            button(name, variant)
                                .on_click(Msg::Wait(name))
                                .test_id(format!("wait.{}", name.to_lowercase()))
                        })),
                    div().class("flex flex-wrap items-center gap-3").children([
                        Button::new("Guardar y recargar")
                            .accent(true)
                            .on_click(Msg::SaveAndReload)
                            .build()
                            .test_id("wait.reload"),
                        Button::new("Sincronizar en segundo plano")
                            .busy_feedback(false)
                            .on_click(Msg::Sync)
                            .build()
                            .test_id("wait.sync"),
                        button("Simular espera (comando)", ButtonVariant::Outlined)
                            .on_page_command(SimulateWait),
                        Button::new("Cargando")
                            .loading(true)
                            .build()
                            .test_id("wait.loading"),
                    ]),
                    div()
                        .class("flex items-center gap-6 pt-2 text-primary")
                        .children([
                            caption("DotProgress:"),
                            DotProgress::new()
                                .size(DotSize::Sm)
                                .test_id("wait.dots.sm")
                                .build(),
                            dot_progress().test_id("wait.dots.md"),
                            DotProgress::new()
                                .size(DotSize::Lg)
                                .test_id("wait.dots.lg")
                                .build(),
                        ]),
                    caption(status).role(Role::Status).test_id("wait.state"),
                ]),
            ])
    }
}

/// The AppKit API (`Button::new`): the three bezels, accent, destructive,
/// cancel (Escape) and disabled, and an `icon_button`.
fn appkit() -> Element<Msg> {
    let built = |button: Button<Msg>, id: &str| button.build().test_id(id);
    card()
        .class("w-full max-w-3xl")
        .test_id("buttons.appkit")
        .children([
            card_header(
                "Button::new",
                Some("Biseles push, bordered y borderless; acento, destructivo y Esc"),
            ),
            card_content().class("gap-4").children([
                div().class("flex flex-wrap items-center gap-3").children([
                    built(
                        Button::new("Push").bezel(BezelStyle::Push).on_click(Msg::Click),
                        "buttons.push",
                    ),
                    built(
                        Button::new("Bordered")
                            .bezel(BezelStyle::Bordered)
                            .on_click(Msg::Click),
                        "buttons.bordered",
                    ),
                    built(
                        Button::new("Borderless")
                            .bezel(BezelStyle::Borderless)
                            .on_click(Msg::Click),
                        "buttons.borderless",
                    ),
                ]),
                div().class("flex flex-wrap items-center gap-3").children([
                    built(
                        Button::new("Acento").accent(true).on_click(Msg::Click),
                        "buttons.accent",
                    ),
                    built(
                        Button::new("Destructivo")
                            .destructive(true)
                            .on_click(Msg::Reset),
                        "buttons.destructive",
                    ),
                    built(
                        Button::new("Cancelar (Esc)")
                            .cancel_button()
                            .on_click(Msg::Reset),
                        "buttons.cancel",
                    ),
                    built(
                        Button::new("Pequeño")
                            .control_size(ControlSize::Small)
                            .disabled(true),
                        "buttons.small",
                    ),
                    icon_button(Icon::Settings, "Ajustes", IconButtonVariant::Secondary)
                        .on_click(Msg::Click)
                        .test_id("buttons.icon"),
                ]),
            ]),
        ])
}

fn control_sizes() -> Element<Msg> {
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
            .children(
                sizes.map(|(size, label)| button(label, variant).control_size(size).on_click(Msg::Click)),
            )
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
                .map(|(size, glyph)| icon_button_sized(glyph, name, variant, size)),
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
