//! Kai components as element builders, styled with the same classes and
//! tokens as the web package (`kai-ui-web/src/components`).
//!
//! Components are plain functions returning [`Element`]s, so they compose
//! like JSX: pass children in, chain `.on_click(..)`, `.disabled(..)` or
//! extra `.class(..)` on the result. State lives in the app model; the only
//! framework-owned state is caret/selection inside inputs.

use std::rc::Rc;

use crate::element::{Element, Role, div, icon, input, text};
use crate::icons::Icon;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    /// `.fs-button--contained-primary`.
    Primary,
    Secondary,
    /// `.fs-button--outlined`.
    Outlined,
    /// `.fs-button--text`.
    Text,
    /// `.fs-button--contained-danger`.
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

/// `.fs-button`: medium weight, `text-sm`, `leading-5`, `rounded-md`, 1px
/// transparent border. Focus shows a ring (the web relies on the browser
/// outline); disabled fades (the web only changes the cursor).
const BUTTON_BASE: &str = "inline-flex items-center justify-center gap-2 rounded-md border \
     border-transparent font-medium whitespace-nowrap cursor-pointer focus:ring-2 \
     focus:ring-primary/40 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed";

pub fn button_classes(variant: ButtonVariant, size: ButtonSize) -> String {
    let size = match size {
        ButtonSize::Sm => "px-3 py-1.5 text-sm leading-5",
        ButtonSize::Md => "px-4 h-control text-sm leading-5",
        ButtonSize::Lg => "px-6 py-3 text-lg",
    };
    let variant = match variant {
        ButtonVariant::Primary => "bg-primary text-white hover:bg-accent active:bg-active/90",
        ButtonVariant::Secondary => {
            "bg-secondary text-foreground hover:bg-secondary/80 active:bg-secondary/70"
        }
        ButtonVariant::Outlined => {
            "bg-transparent border-foreground text-foreground hover:bg-accent \
             hover:border-accent hover:text-white active:bg-active"
        }
        ButtonVariant::Text => {
            "bg-transparent text-primary hover:bg-primary/10 active:bg-primary/20"
        }
        ButtonVariant::Danger => "bg-error text-white hover:bg-error/80 active:bg-error/70",
    };
    format!("{BUTTON_BASE} {size} {variant}")
}

/// Medium button; attach behavior with `.on_click(msg)`.
pub fn button<Msg>(label: impl Into<String>, variant: ButtonVariant) -> Element<Msg> {
    button_sized(label, variant, ButtonSize::Md)
}

pub fn button_sized<Msg>(
    label: impl Into<String>,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Element<Msg> {
    div()
        .class(&button_classes(variant, size))
        .role(Role::Button)
        .child(text(label))
}

/// `variante` of the web `TextField`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldVariant {
    #[default]
    Normal,
    /// For fields on a primary bar: light border and text, label on primary.
    Contrast,
    /// Borderless and transparent; the frame belongs to a combobox shell.
    Autocomplete,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldDensity {
    /// Control height with a floating label on the top border.
    #[default]
    Default,
    /// Compact height with a static label above (or inline, see
    /// [`LabelLayout::Inline`]).
    Compact,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelLayout {
    #[default]
    Stack,
    /// Compact only: the label sits inside the border, before the input.
    Inline,
}

/// Leading or trailing decoration inside the field outline.
pub enum Adornment<Msg> {
    /// Short text such as `$` or `%`.
    Symbol(String),
    Icon(Icon),
    Element(Element<Msg>),
}

/// Kai `TextField` (`kai-ui-web/src/components/TextField`), built like the
/// web props and turned into elements with [`TextField::build`].
///
/// The outline is a flex row `[start][input][end][password toggle]`, so
/// adornments take their own room instead of being measured and overlaid.
/// A click anywhere on the field (label, icons, padding) focuses the input.
pub struct TextField<Msg> {
    label: String,
    value: String,
    placeholder: Option<String>,
    variant: FieldVariant,
    density: FieldDensity,
    label_layout: LabelLayout,
    inline_leading: Option<Element<Msg>>,
    start: Option<Adornment<Msg>>,
    end: Option<Adornment<Msg>>,
    password: Option<(bool, Msg)>,
    required: bool,
    helper_text: Option<String>,
    read_only: bool,
    disabled: bool,
    select_on_focus: bool,
    always_show_label: bool,
    placeholder_color: Option<String>,
    on_input: Option<Rc<dyn Fn(String) -> Msg>>,
    on_submit: Option<Msg>,
}

impl<Msg> TextField<Msg> {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            placeholder: None,
            variant: FieldVariant::Normal,
            density: FieldDensity::Default,
            label_layout: LabelLayout::Stack,
            inline_leading: None,
            start: None,
            end: None,
            password: None,
            required: false,
            helper_text: None,
            read_only: false,
            disabled: false,
            select_on_focus: false,
            always_show_label: false,
            placeholder_color: None,
            on_input: None,
            on_submit: None,
        }
    }

    /// Shown while the field is empty and unfocused; defaults to the label.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn variant(mut self, variant: FieldVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn density(mut self, density: FieldDensity) -> Self {
        self.density = density;
        self
    }

    pub fn label_layout(mut self, layout: LabelLayout) -> Self {
        self.label_layout = layout;
        self
    }

    /// Inline layout only: element before the label, inside the border.
    pub fn inline_leading(mut self, element: Element<Msg>) -> Self {
        self.inline_leading = Some(element);
        self
    }

    pub fn start_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.start = Some(Adornment::Symbol(symbol.into()));
        self
    }

    pub fn start_icon(mut self, icon: Icon) -> Self {
        self.start = Some(Adornment::Icon(icon));
        self
    }

    pub fn start_adornment(mut self, element: Element<Msg>) -> Self {
        self.start = Some(Adornment::Element(element));
        self
    }

    pub fn end_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.end = Some(Adornment::Symbol(symbol.into()));
        self
    }

    pub fn end_icon(mut self, icon: Icon) -> Self {
        self.end = Some(Adornment::Icon(icon));
        self
    }

    pub fn end_adornment(mut self, element: Element<Msg>) -> Self {
        self.end = Some(Adornment::Element(element));
        self
    }

    /// Password entry: bullets unless `revealed`, plus an eye button that
    /// sends `on_toggle` without taking focus from the input.
    pub fn password(mut self, revealed: bool, on_toggle: Msg) -> Self {
        self.password = Some((revealed, on_toggle));
        self
    }

    /// Red asterisk next to the label and the placeholder.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Hint below the field, also its accessible description.
    pub fn helper_text(mut self, text: impl Into<String>) -> Self {
        self.helper_text = Some(text.into());
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn select_on_focus(mut self, select_on_focus: bool) -> Self {
        self.select_on_focus = select_on_focus;
        self
    }

    pub fn always_show_label(mut self, always_show_label: bool) -> Self {
        self.always_show_label = always_show_label;
        self
    }

    /// Theme color token for the placeholder (`primary`, `error/60`…).
    pub fn placeholder_color(mut self, token: impl Into<String>) -> Self {
        self.placeholder_color = Some(token.into());
        self
    }

    pub fn on_input(mut self, handler: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_input = Some(Rc::new(handler));
        self
    }

    pub fn on_submit(mut self, msg: Msg) -> Self {
        self.on_submit = Some(msg);
        self
    }

    pub fn build(mut self) -> Element<Msg> {
        let compact = self.density == FieldDensity::Compact;
        let inline = compact && self.label_layout == LabelLayout::Inline;
        let muted = self.disabled
            || (self.read_only && self.variant != FieldVariant::Autocomplete && !inline);

        let mut row = Vec::new();
        if let Some(start) = self.start.take() {
            row.push(adornment(start, compact, muted, false));
        }
        row.push(self.input_body(compact, inline, muted));
        if let Some(end) = self.end.take() {
            row.push(adornment(end, compact, muted, true));
        }
        if let Some((revealed, on_toggle)) = self.password.take() {
            row.push(password_toggle(revealed, on_toggle, self.disabled));
        }

        let cursor = if self.disabled {
            "cursor-not-allowed"
        } else {
            "cursor-text"
        };
        let mut root = div()
            .class("group flex flex-col w-full min-w-[180px]")
            .delegates_focus();
        if inline {
            let fade_shell = self.disabled && self.inline_leading.is_none();
            let shell = div()
                .class(&format!(
                    "flex items-stretch w-full min-w-0 h-control-compact rounded-md border \
                     border-border bg-background focus-within:border-primary {cursor}{}",
                    if fade_shell { " opacity-50" } else { "" }
                ))
                .children(self.inline_leading.take().map(|leading| {
                    div()
                        .class("flex shrink-0 items-center pl-2 pr-1.5")
                        .child(leading)
                }))
                .child(
                    div()
                        .class(&format!(
                            "flex flex-1 min-w-0 items-stretch{}",
                            if self.disabled && !fade_shell {
                                " opacity-50"
                            } else {
                                ""
                            }
                        ))
                        .children([
                            div()
                                .class("flex shrink-0 self-center px-2 border-r border-border")
                                .children(label_texts(
                                    &self.label,
                                    self.required,
                                    "text-[13px] font-semibold leading-5 text-muted-foreground \
                                     whitespace-nowrap",
                                    "ml-0.5 text-[13px] leading-5 text-error",
                                )),
                            div()
                                .class(&format!(
                                    "flex flex-1 items-center gap-2 px-3{}",
                                    if self.disabled && !fade_shell {
                                        " opacity-50"
                                    } else {
                                        ""
                                    }
                                ))
                                .children(row),
                        ]),
                );
            root = root.child(shell);
        } else {
            let (border, surface) = match self.variant {
                FieldVariant::Normal if muted => ("border border-border/50", "bg-field-muted"),
                FieldVariant::Normal => (
                    "border border-border hover:border-muted-foreground \
                     focus-within:border-primary",
                    "bg-background",
                ),
                FieldVariant::Contrast => (
                    "border border-background focus-within:border-primary",
                    if muted {
                        "bg-field-muted"
                    } else {
                        "bg-transparent"
                    },
                ),
                FieldVariant::Autocomplete => ("border-0", "bg-transparent"),
            };
            let height = if compact {
                "h-control-compact"
            } else {
                "h-control"
            };
            let mut shell = div()
                .class(&format!(
                    "relative flex items-center gap-2 w-full min-w-0 px-3 rounded-md {height} \
                     {border} {surface} {cursor}"
                ))
                .children(row);
            if compact {
                root = root
                    .class("gap-1")
                    .child(div().class("flex").children(label_texts(
                        &self.label,
                        self.required,
                        "text-[11px] font-medium leading-tight text-foreground",
                        "ml-1 text-[11px] font-medium leading-tight text-error",
                    )));
            } else {
                shell = shell.child(self.floating_label(muted));
                root = root.class("mt-1.5");
            }
            root = root.child(shell);
        }
        if let Some(helper) = &self.helper_text {
            root = root.child(text(helper).class("mt-1 text-xs text-muted-foreground"));
        }
        root
    }

    /// The label resting on the top border while focused or filled
    /// (`.fs-text-field__floating-label`), masked by the field surface.
    fn floating_label(&self, muted: bool) -> Element<Msg> {
        let contrast = self.variant == FieldVariant::Contrast;
        let surface = if muted {
            "bg-field-muted"
        } else if contrast {
            "bg-primary"
        } else {
            "bg-background"
        };
        let color = if contrast {
            "text-background"
        } else {
            "text-foreground"
        };
        let visibility = if self.value.is_empty() && !self.always_show_label {
            "hidden group-focus-within:flex"
        } else {
            "flex"
        };
        div()
            .class(&format!(
                "absolute left-3 -top-2 z-20 px-1 rounded-md pointer-events-none {surface} \
                 {visibility}"
            ))
            .children(label_texts(
                &self.label,
                self.required,
                &format!("text-xs leading-4 font-medium {color}"),
                "ml-1 text-xs leading-4 font-medium text-error",
            ))
    }

    /// The input plus, for required fields, the two-color placeholder
    /// overlay (a native placeholder has a single color).
    fn input_body(&mut self, compact: bool, inline: bool, muted: bool) -> Element<Msg> {
        let placeholder = self
            .placeholder
            .take()
            .unwrap_or_else(|| self.label.clone());
        let overlay = self.required && !compact && self.value.is_empty();
        let color = if self.variant == FieldVariant::Contrast {
            "text-background"
        } else {
            "text-foreground"
        };
        let mut classes = format!(
            "w-full self-stretch bg-transparent leading-5 {color} placeholder-muted-foreground \
             focus:placeholder-transparent {}",
            if inline { "text-[13px]" } else { "text-sm" }
        );
        if let Some(token) = &self.placeholder_color {
            classes.push_str(&format!(" placeholder-{token}"));
        }
        classes.push_str(if self.disabled {
            " cursor-not-allowed"
        } else {
            " cursor-text"
        });
        if muted {
            classes.push_str(" opacity-50");
        }
        let masked = self
            .password
            .as_ref()
            .is_some_and(|(revealed, _)| !revealed);
        let mut field = input(
            self.value.clone(),
            if self.required {
                String::new()
            } else {
                placeholder.clone()
            },
        )
        .class(&classes)
        .role(Role::TextInput)
        .label(self.label.clone())
        .masked(masked)
        .read_only(self.read_only)
        .select_on_focus(
            self.select_on_focus && !self.disabled && self.variant != FieldVariant::Autocomplete,
        )
        .required(self.required)
        .disabled(self.disabled);
        if let Some(helper) = &self.helper_text {
            field = field.description(helper.clone());
        }
        field.handlers.on_input = self.on_input.take();
        field.handlers.on_submit = self.on_submit.take();

        let mut body = div()
            .class("relative flex flex-1 min-w-0 self-stretch items-center")
            .child(field);
        if overlay {
            body = body.child(
                div()
                    .class(
                        "absolute inset-y-0 left-0 flex items-center pointer-events-none \
                         group-focus-within:hidden",
                    )
                    .children(label_texts(
                        &placeholder,
                        true,
                        "text-sm leading-5 text-muted-foreground whitespace-nowrap",
                        "ml-1 text-sm leading-5 text-error",
                    )),
            );
        }
        body
    }
}

/// Label text followed by the required asterisk, as sibling texts.
fn label_texts<Msg>(
    label: &str,
    required: bool,
    classes: &str,
    asterisk: &str,
) -> Vec<Element<Msg>> {
    let mut texts = vec![text(label).class(classes)];
    if required {
        texts.push(text("*").class(asterisk));
    }
    texts
}

/// Symbols and icons use the secondary color (`.fs-text-field__icon`); a
/// trailing element keeps its own colors (`.fs-text-field__end-adornment`).
fn adornment<Msg>(
    adornment: Adornment<Msg>,
    compact: bool,
    muted: bool,
    trailing: bool,
) -> Element<Msg> {
    let tone = if muted {
        "text-muted-foreground opacity-50"
    } else {
        "text-secondary"
    };
    match adornment {
        Adornment::Symbol(symbol) => text(symbol).class(&format!(
            "shrink-0 whitespace-nowrap leading-none {} {tone}",
            if compact {
                "text-[15px]"
            } else {
                "text-[20px]"
            }
        )),
        Adornment::Icon(glyph) => icon(glyph).class(&format!(
            "shrink-0 {} {tone}",
            if compact { "size-4" } else { "size-5" }
        )),
        Adornment::Element(element) if trailing => div()
            .class(&format!(
                "flex shrink-0 items-center gap-0.5 -mr-2{}",
                if muted { " opacity-50" } else { "" }
            ))
            .child(element),
        Adornment::Element(element) => div()
            .class(&format!("flex shrink-0 items-center {tone}"))
            .child(element),
    }
}

/// `.fs-text-field__password-toggle`: round eye button that keeps focus in
/// the input when pressed.
fn password_toggle<Msg>(revealed: bool, on_toggle: Msg, disabled: bool) -> Element<Msg> {
    let state = if revealed {
        "text-primary"
    } else {
        "text-secondary group-focus-within:text-primary"
    };
    let interaction = if disabled {
        "opacity-50 cursor-not-allowed"
    } else {
        "cursor-pointer hover:bg-primary/10"
    };
    div()
        .class(&format!(
            "inline-flex size-8 shrink-0 -mr-2 items-center justify-center rounded-full \
             focus:ring-2 focus:ring-primary/40 {state} {interaction}"
        ))
        .role(Role::Button)
        .label(if revealed {
            "Ocultar contraseña"
        } else {
            "Mostrar contraseña"
        })
        .focus_on_press(false)
        .disabled(disabled)
        .pressed(revealed)
        .on_click(on_toggle)
        .child(icon(if revealed { Icon::EyeOff } else { Icon::Eye }).class("size-5"))
}
/// `.fs-card`.
pub fn card<Msg>() -> Element<Msg> {
    div()
        .class(
            "flex flex-col bg-background border border-border/70 rounded-lg shadow-sm \
             overflow-hidden",
        )
        .role(Role::Group)
}

/// `.fs-card__header` with `.fs-card__title` and `.fs-card__subtitle`.
pub fn card_header<Msg>(title: &str, subtitle: Option<&str>) -> Element<Msg> {
    let mut header = div().class("flex flex-col gap-0.5 px-4 pt-4 pb-3").child(
        text(title)
            .class("text-lg leading-[1.4] font-semibold text-foreground")
            .role(Role::Heading),
    );
    if let Some(subtitle) = subtitle {
        header = header.child(text(subtitle).class("text-sm leading-5 text-muted-foreground"));
    }
    header
}

/// `.fs-card__content`.
pub fn card_content<Msg>() -> Element<Msg> {
    div().class("flex flex-col gap-3 p-4")
}

/// `.fs-card__actions`.
pub fn card_actions<Msg>() -> Element<Msg> {
    div().class(
        "flex flex-wrap items-center justify-end gap-2 border-t border-border/70 px-4 pt-3 pb-4",
    )
}

/// Compact statistics tile (`StatisticsCard`).
pub fn stat<Msg>(label: &str, value: impl Into<String>, hint: Option<&str>) -> Element<Msg> {
    let mut tile = card().class("p-4 gap-1").children([
        text(label).class("text-sm leading-5 font-medium text-muted-foreground"),
        text(value).class("text-2xl font-semibold text-foreground"),
    ]);
    if let Some(hint) = hint {
        tile = tile.child(text(hint).class("text-xs text-muted-foreground"));
    }
    tile
}

pub fn heading<Msg>(content: impl Into<String>) -> Element<Msg> {
    text(content)
        .class("text-xl leading-tight font-semibold text-foreground")
        .role(Role::Heading)
}

pub fn paragraph<Msg>(content: impl Into<String>) -> Element<Msg> {
    text(content)
        .class("text-base text-foreground")
        .role(Role::Paragraph)
}

pub fn caption<Msg>(content: impl Into<String>) -> Element<Msg> {
    text(content).class("text-sm text-muted-foreground")
}

/// `.fs-badge` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeVariant {
    Primary,
    Secondary,
    Success,
    Error,
    Warning,
    Info,
    PrimaryOutlined,
    SecondaryOutlined,
    SuccessOutlined,
    ErrorOutlined,
    WarningOutlined,
    InfoOutlined,
}

pub fn badge_classes(variant: BadgeVariant) -> String {
    let variant_classes = match variant {
        BadgeVariant::Primary => "bg-primary text-white",
        BadgeVariant::Secondary => "bg-secondary text-white",
        BadgeVariant::Success => "bg-success text-white",
        BadgeVariant::Error => "bg-error text-white",
        BadgeVariant::Warning => "bg-warning text-foreground",
        BadgeVariant::Info => "bg-info text-white",
        BadgeVariant::PrimaryOutlined => "border border-primary text-primary bg-transparent",
        BadgeVariant::SecondaryOutlined => "border border-secondary text-secondary bg-transparent",
        BadgeVariant::SuccessOutlined => "border border-success text-success bg-transparent",
        BadgeVariant::ErrorOutlined => "border border-error text-error bg-transparent",
        BadgeVariant::WarningOutlined => "border border-warning text-warning bg-transparent",
        BadgeVariant::InfoOutlined => "border border-info text-info bg-transparent",
    };
    format!(
        "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs leading-4 font-medium {variant_classes}"
    )
}

/// `.fs-badge` pill indicator.
pub fn badge<Msg>(label: impl Into<String>, variant: BadgeVariant) -> Element<Msg> {
    div()
        .class(&badge_classes(variant))
        .role(Role::Status)
        .child(text(label))
}

/// `.fs-alert` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertVariant {
    Success,
    Info,
    Warning,
    Error,
}

/// `.fs-alert` message box with Lucide icon and semantic border/background.
pub fn alert<Msg>(variant: AlertVariant, message: impl Into<String>) -> Element<Msg> {
    let (icon_kind, border_and_bg, icon_color) = match variant {
        AlertVariant::Success => (
            Icon::CircleCheckBig,
            "border border-success bg-success/20",
            "text-success",
        ),
        AlertVariant::Info => (Icon::Info, "border border-info bg-info/20", "text-info"),
        AlertVariant::Warning => (
            Icon::TriangleAlert,
            "border border-warning bg-warning/20",
            "text-warning",
        ),
        AlertVariant::Error => (
            Icon::CircleAlert,
            "border border-error bg-error/20",
            "text-error",
        ),
    };
    div()
        .class(&format!(
            "relative flex items-center gap-3 w-full p-4 rounded-lg text-sm leading-5 {border_and_bg}"
        ))
        .role(Role::Status)
        .children([
            icon(icon_kind).class(&format!("size-5 shrink-0 {icon_color}")),
            text(message).class("flex-1 text-sm font-normal text-foreground"),
        ])
}

/// `.fs-icon-button` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconButtonVariant {
    Action,
    Primary,
    Secondary,
    Neutral,
    Text,
    Outlined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconButtonSize {
    Sm,
    Md,
    Lg,
}

/// Icon button sized to medium (h-10).
pub fn icon_button<Msg>(
    icon_kind: Icon,
    label: impl Into<String>,
    variant: IconButtonVariant,
) -> Element<Msg> {
    icon_button_sized(icon_kind, label, variant, IconButtonSize::Md)
}

/// Sized icon button with accessible label and interaction classes.
pub fn icon_button_sized<Msg>(
    icon_kind: Icon,
    label: impl Into<String>,
    variant: IconButtonVariant,
    size: IconButtonSize,
) -> Element<Msg> {
    let (size_class, icon_class) = match size {
        IconButtonSize::Sm => ("size-7 rounded", "size-4"),
        IconButtonSize::Md => ("size-10 rounded-md", "size-5"),
        IconButtonSize::Lg => ("size-12 rounded-lg", "size-6"),
    };
    let variant_classes = match variant {
        IconButtonVariant::Action => "text-secondary hover:bg-primary/10 active:bg-primary/20",
        IconButtonVariant::Primary => "bg-primary text-white hover:bg-accent active:bg-active",
        IconButtonVariant::Secondary => {
            "bg-secondary text-foreground hover:bg-secondary/80 active:bg-secondary/70"
        }
        IconButtonVariant::Neutral => "text-foreground hover:bg-neutral active:bg-border",
        IconButtonVariant::Text => "text-primary hover:bg-primary/10 active:bg-primary/20",
        IconButtonVariant::Outlined => {
            "border border-border text-foreground hover:bg-neutral active:bg-border"
        }
    };
    div()
        .class(&format!(
            "inline-flex items-center justify-center shrink-0 cursor-pointer focus:ring-2 \
             focus:ring-primary/40 disabled:opacity-50 disabled:cursor-not-allowed {size_class} \
             {variant_classes}"
        ))
        .role(Role::Button)
        .label(label)
        .child(icon(icon_kind).class(icon_class))
}

/// `.fs-switch` density.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwitchDensity {
    #[default]
    Default,
    Compact,
}

/// Standard toggle switch.
pub fn switch<Msg>(checked: bool, on_toggle: Msg) -> Element<Msg> {
    switch_with_options(checked, on_toggle, false, SwitchDensity::Default)
}

/// Switch with optional label and density configuration.
pub fn switch_labeled<Msg>(
    label: &str,
    checked: bool,
    on_toggle: Msg,
    disabled: bool,
    density: SwitchDensity,
) -> Element<Msg> {
    let compact = density == SwitchDensity::Compact;
    let label_class = if compact {
        "text-[13px] font-medium leading-tight text-foreground"
    } else {
        "text-sm font-medium leading-normal text-foreground"
    };
    div()
        .class("inline-flex items-center gap-2")
        .children([
            switch_with_options(checked, on_toggle, disabled, density),
            text(label).class(label_class),
        ])
}

/// Configurable switch track and thumb.
pub fn switch_with_options<Msg>(
    checked: bool,
    on_toggle: Msg,
    disabled: bool,
    density: SwitchDensity,
) -> Element<Msg> {
    let compact = density == SwitchDensity::Compact;
    let (track_size, thumb_size, pad) = if compact {
        ("w-7 h-4", "size-3", "p-0.5")
    } else {
        ("w-10 h-6", "size-4", "p-1")
    };
    let track_bg = if checked {
        "bg-primary"
    } else {
        "bg-border hover:bg-muted-foreground/40"
    };
    let justify = if checked {
        "justify-end"
    } else {
        "justify-start"
    };
    let interaction = if disabled {
        "opacity-50 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };
    div()
        .class(&format!(
            "relative inline-flex items-center shrink-0 rounded-full focus:ring-2 \
             focus:ring-primary/40 {track_size} {pad} {track_bg} {justify} {interaction}"
        ))
        .role(Role::Button)
        .label(if checked {
            "Activado"
        } else {
            "Desactivado"
        })
        .focus_on_press(false)
        .disabled(disabled)
        .pressed(checked)
        .on_click(on_toggle)
        .child(div().class(&format!("rounded-full bg-white shadow-sm {thumb_size}")))
}

#[cfg(test)]
mod tests {
    use super::{
        AlertVariant, BadgeVariant, ButtonSize, ButtonVariant, FieldDensity, FieldVariant,
        IconButtonSize, IconButtonVariant, LabelLayout, SwitchDensity, TextField, alert, badge,
        badge_classes, button, button_classes, card, card_actions, card_content, card_header,
        icon_button_sized, stat, switch_labeled, switch_with_options,
    };
    use crate::element::{Element, FocusOrder, Kind, Role, text};
    use crate::icons::Icon;
    use crate::style::Display;
    use crate::theme::{Density, Theme};
    use crate::tw::{ClassList, State};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggle,
        Edited(String),
        Submit,
    }

    fn theme() -> Theme {
        Theme::kai(Density::Comfortable)
    }

    fn assert_known(classes: &str) {
        let list = ClassList::parse(classes, &theme());
        assert!(list.unknown.is_empty(), "{classes}: {:?}", list.unknown);
    }

    fn assert_tree_known(element: &Element<Msg>) {
        element.walk(&mut |element| assert_known(&element.class));
    }

    fn find(
        element: &Element<Msg>,
        predicate: impl Fn(&Element<Msg>) -> bool,
    ) -> Option<&Element<Msg>> {
        let mut found = None;
        element.walk(&mut |element| {
            if found.is_none() && predicate(element) {
                found = Some(element);
            }
        });
        found
    }

    fn field_input(element: &Element<Msg>) -> &Element<Msg> {
        find(element, |e| matches!(e.kind, Kind::Input { .. })).expect("input")
    }

    fn text_node<'a>(element: &'a Element<Msg>, content: &str) -> Option<&'a Element<Msg>> {
        find(
            element,
            |e| matches!(&e.kind, Kind::Text(t) if t == content),
        )
    }

    fn display(classes: &str, state: State) -> Display {
        ClassList::parse(classes, &theme()).resolve(state).display
    }

    fn group_focused() -> State {
        State {
            group_focus_within: true,
            ..State::default()
        }
    }

    #[test]
    fn every_component_class_is_supported() {
        for variant in [
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Outlined,
            ButtonVariant::Text,
            ButtonVariant::Danger,
        ] {
            for size in [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg] {
                assert_known(&button_classes(variant, size));
            }
        }
        assert_tree_known(&card().children([
            card_header("Título", Some("Subtítulo")),
            card_content().child(TextField::new("Nombre", "").build()),
            card_actions().child(button("Ok", ButtonVariant::Primary)),
            stat("Clics", "3", Some("hoy")),
        ]));

        for variant in [
            FieldVariant::Normal,
            FieldVariant::Contrast,
            FieldVariant::Autocomplete,
        ] {
            for density in [FieldDensity::Default, FieldDensity::Compact] {
                for layout in [LabelLayout::Stack, LabelLayout::Inline] {
                    for (flag, value) in [(0, ""), (1, "x"), (2, "x"), (3, "")] {
                        let field = TextField::new("Campo", value)
                            .variant(variant)
                            .density(density)
                            .label_layout(layout)
                            .inline_leading(text("#"))
                            .start_icon(Icon::Search)
                            .end_adornment(text("ok"))
                            .password(flag == 1, Msg::Toggle)
                            .required(flag == 3)
                            .read_only(flag == 1)
                            .disabled(flag == 2)
                            .placeholder_color("primary/60")
                            .helper_text("Ayuda");
                        assert_tree_known(&field.build());
                    }
                }
            }
        }
        for variant in [
            BadgeVariant::Primary,
            BadgeVariant::Secondary,
            BadgeVariant::Success,
            BadgeVariant::Error,
            BadgeVariant::Warning,
            BadgeVariant::Info,
            BadgeVariant::PrimaryOutlined,
            BadgeVariant::SecondaryOutlined,
            BadgeVariant::SuccessOutlined,
            BadgeVariant::ErrorOutlined,
            BadgeVariant::WarningOutlined,
            BadgeVariant::InfoOutlined,
        ] {
            assert_known(&badge_classes(variant));
            assert_tree_known(&badge("Badge", variant));
        }

        for variant in [
            AlertVariant::Success,
            AlertVariant::Info,
            AlertVariant::Warning,
            AlertVariant::Error,
        ] {
            assert_tree_known(&alert(variant, "Mensaje de alerta"));
        }

        for variant in [
            IconButtonVariant::Action,
            IconButtonVariant::Primary,
            IconButtonVariant::Secondary,
            IconButtonVariant::Neutral,
            IconButtonVariant::Text,
            IconButtonVariant::Outlined,
        ] {
            for size in [
                IconButtonSize::Sm,
                IconButtonSize::Md,
                IconButtonSize::Lg,
            ] {
                assert_tree_known(&icon_button_sized(Icon::Search, "Buscar", variant, size));
            }
        }

        for density in [SwitchDensity::Default, SwitchDensity::Compact] {
            for checked in [false, true] {
                for disabled in [false, true] {
                    assert_tree_known(&switch_with_options(checked, Msg::Toggle, disabled, density));
                    assert_tree_known(&switch_labeled(
                        "Notificaciones",
                        checked,
                        Msg::Toggle,
                        disabled,
                        density,
                    ));
                }
            }
        }

        assert_tree_known(
            &TextField::new("Monto", "")
                .start_symbol("$")
                .end_symbol("%")
                .start_adornment(text("a"))
                .end_icon(Icon::Info)
                .build(),
        );
    }

    #[test]
    fn primary_button_matches_web_tokens_in_each_state() {
        let theme = theme();
        let list = ClassList::parse(
            &button_classes(ButtonVariant::Primary, ButtonSize::Md),
            &theme,
        );
        let idle = list.resolve(State::default());
        assert_eq!(idle.background, Some(theme.colors.primary));
        assert_eq!(idle.radius, 6.0);
        assert_eq!(idle.text.font_size, Some(theme.typography.font_size_sm));
        assert_eq!(idle.border_width.top, 1.0);
        let hover = list.resolve(State {
            hover: true,
            ..State::default()
        });
        assert_eq!(hover.background, Some(theme.colors.accent));
        let focus = list.resolve(State {
            focus: true,
            ..State::default()
        });
        assert_eq!((focus.ring.width, focus.ring.offset), (2.0, 2.0));
    }

    #[test]
    fn text_field_floats_its_label_only_with_value_or_focus() {
        let empty: Element<Msg> = TextField::new("Nombre", "")
            .on_input(Msg::Edited)
            .on_submit(Msg::Submit)
            .build();
        assert!(empty.delegates_focus, "clicks anywhere focus the input");
        let shell = &empty.children[0];
        let label = shell.children.last().unwrap();
        assert_eq!(display(&label.class, State::default()), Display::None);
        assert_eq!(display(&label.class, group_focused()), Display::Flex);
        let input = field_input(&empty);
        assert!(matches!(
            &input.kind,
            Kind::Input { placeholder, .. } if placeholder == "Nombre"
        ));
        assert_eq!(input.label.as_deref(), Some("Nombre"));
        assert_eq!(input.role, Some(Role::TextInput));
        let on_input = input.handlers.on_input.as_ref().unwrap();
        assert_eq!(on_input("a".to_owned()), Msg::Edited("a".to_owned()));
        assert_eq!(input.handlers.on_submit, Some(Msg::Submit));

        let shell_style = ClassList::parse(&shell.class, &theme()).resolve(State {
            focus_within: true,
            ..State::default()
        });
        assert_eq!(shell_style.border_color, Some(theme().colors.primary));

        let filled: Element<Msg> = TextField::new("Nombre", "Ana")
            .placeholder("Tu nombre")
            .build();
        let label = filled.children[0].children.last().unwrap();
        assert_eq!(display(&label.class, State::default()), Display::Flex);
        assert!(matches!(
            &field_input(&filled).kind,
            Kind::Input { placeholder, .. } if placeholder == "Tu nombre"
        ));
    }

    #[test]
    fn required_fields_mark_label_and_placeholder_with_an_asterisk() {
        let field: Element<Msg> = TextField::new("Correo", "")
            .required(true)
            .helper_text("Usaremos este correo para avisarte")
            .build();
        let input = field_input(&field);
        assert!(input.required);
        assert_eq!(
            input.description.as_deref(),
            Some("Usaremos este correo para avisarte")
        );
        assert!(matches!(
            &input.kind,
            Kind::Input { placeholder, .. } if placeholder.is_empty()
        ));
        let mut asterisks = Vec::new();
        field.walk(&mut |e| {
            if matches!(&e.kind, Kind::Text(t) if t == "*") {
                asterisks.push(e.class.clone());
            }
        });
        assert_eq!(asterisks.len(), 2, "floating label and placeholder overlay");
        assert!(asterisks.iter().all(|class| class.contains("text-error")));
        let overlay = find(&field, |e| e.class.contains("group-focus-within:hidden")).unwrap();
        assert_eq!(display(&overlay.class, group_focused()), Display::None);
        let helper = text_node(&field, "Usaremos este correo para avisarte").unwrap();
        assert!(helper.class.contains("text-muted-foreground"));

        let filled: Element<Msg> = TextField::new("Correo", "a@b.cl").required(true).build();
        assert!(find(&filled, |e| e.class.contains("group-focus-within:hidden")).is_none());
    }

    #[test]
    fn password_fields_mask_the_value_and_toggle_without_stealing_focus() {
        let hidden: Element<Msg> = TextField::new("Contraseña", "secreto")
            .password(false, Msg::Toggle)
            .start_icon(Icon::User)
            .build();
        assert!(matches!(
            field_input(&hidden).kind,
            Kind::Input { masked: true, .. }
        ));
        let toggle = find(&hidden, |e| e.role == Some(Role::Button)).unwrap();
        assert_eq!(toggle.label.as_deref(), Some("Mostrar contraseña"));
        assert!(!toggle.focus_on_press);
        assert_eq!(toggle.handlers.on_click, Some(Msg::Toggle));
        assert_eq!(toggle.children[0].kind, Kind::Icon(Icon::Eye));
        let shell = &hidden.children[0];
        assert_eq!(shell.children[0].kind, Kind::Icon(Icon::User));
        assert!(shell.children[0].class.contains("size-5"));

        let mut shown: Element<Msg> = TextField::new("Contraseña", "secreto")
            .password(true, Msg::Toggle)
            .build();
        assert!(matches!(
            field_input(&shown).kind,
            Kind::Input { masked: false, .. }
        ));
        let toggle = find(&shown, |e| e.role == Some(Role::Button)).unwrap();
        assert_eq!(toggle.label.as_deref(), Some("Ocultar contraseña"));
        assert_eq!(toggle.children[0].kind, Kind::Icon(Icon::EyeOff));
        shown.assign_ids();
        let order = FocusOrder::from_tree(&shown);
        assert_eq!(order.ids().count(), 2, "input and toggle are tabbable");
        let toggle = find(&shown, |e| e.role == Some(Role::Button)).unwrap();
        assert!(!order.focus_on_press(toggle.id));
    }

    #[test]
    fn compact_and_inline_layouts_move_the_label() {
        let compact: Element<Msg> = TextField::new("Código", "")
            .density(FieldDensity::Compact)
            .required(true)
            .start_symbol("$")
            .build();
        let label = text_node(&compact, "Código").unwrap();
        assert!(label.class.contains("text-[11px]"));
        let shell = &compact.children[1];
        assert!(shell.class.contains("h-control-compact"));
        assert!(
            find(shell, |e| e.class.contains("-top-2")).is_none(),
            "no floating label"
        );
        assert!(shell.children[0].class.contains("text-[15px]"));

        let inline: Element<Msg> = TextField::new("RUT", "")
            .density(FieldDensity::Compact)
            .label_layout(LabelLayout::Inline)
            .inline_leading(text("#"))
            .build();
        let shell = &inline.children[0];
        assert!(text_node(&shell.children[0], "#").is_some());
        let body = &shell.children[1];
        assert!(body.children[0].class.contains("border-r"));
        assert!(text_node(&body.children[0], "RUT").is_some());
        assert!(find(&body.children[1], |e| matches!(e.kind, Kind::Input { .. })).is_some());

        let ignored: Element<Msg> = TextField::new("Nombre", "")
            .label_layout(LabelLayout::Inline)
            .build();
        assert!(
            find(&ignored, |e| e.class.contains("border-r")).is_none(),
            "inline needs compact density"
        );
    }

    #[test]
    fn read_only_and_disabled_fields_use_the_muted_surface() {
        let theme = theme();
        let read_only: Element<Msg> = TextField::new("Folio", "123")
            .read_only(true)
            .select_on_focus(true)
            .build();
        let input = field_input(&read_only);
        assert!(matches!(
            input.kind,
            Kind::Input {
                read_only: true,
                select_on_focus: true,
                ..
            }
        ));
        assert!(!input.disabled);
        let shell = &read_only.children[0];
        let surface = ClassList::parse(&shell.class, &theme).resolve(State::default());
        assert_eq!(surface.background, theme.color("field-muted"));
        let label = shell.children.last().unwrap();
        assert!(label.class.contains("bg-field-muted"));

        let disabled: Element<Msg> = TextField::new("Folio", "123")
            .disabled(true)
            .select_on_focus(true)
            .password(false, Msg::Toggle)
            .build();
        let input = field_input(&disabled);
        assert!(input.disabled);
        assert!(matches!(
            input.kind,
            Kind::Input {
                select_on_focus: false,
                ..
            }
        ));
        assert!(
            find(&disabled, |e| e.role == Some(Role::Button))
                .unwrap()
                .disabled
        );
        assert!(disabled.children[0].class.contains("cursor-not-allowed"));
        assert_eq!(FocusOrder::from_tree(&disabled).ids().count(), 0);

        let contrast: Element<Msg> = TextField::new("Buscar", "x")
            .variant(FieldVariant::Contrast)
            .build();
        let label = contrast.children[0].children.last().unwrap();
        assert!(label.class.contains("bg-primary"));
        let autocomplete: Element<Msg> = TextField::new("Buscar", "")
            .variant(FieldVariant::Autocomplete)
            .read_only(true)
            .build();
        let shell =
            ClassList::parse(&autocomplete.children[0].class, &theme).resolve(State::default());
        assert_eq!(shell.border_width.top, 0.0);
        assert_eq!(shell.background, theme.color("transparent"));
    }
}
