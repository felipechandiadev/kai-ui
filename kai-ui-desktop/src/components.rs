//! Kai components as element builders, styled with the same classes and
//! tokens as the web package (`kai-ui-web/src/components`).
//!
//! Components are plain functions returning [`Element`]s, so they compose
//! like JSX: pass children in, chain `.on_click(..)`, `.disabled(..)` or
//! extra `.class(..)` on the result. State lives in the app model; the only
//! framework-owned state is caret/selection inside inputs.

use std::rc::Rc;

use crate::element::{ControlSize, Element, Role, div, icon, input, text};
use crate::icons::Icon;

/// One class string per theme: prefixes every class of `kai` with `kai:` and
/// every class of `appkit` with `appkit:`. Classes that already name a theme
/// are kept as they are. The Kai part of a component is exactly its previous
/// string, so the Kai look does not change.
pub fn themed(kai: &str, appkit: &str) -> String {
    fn prefix(out: &mut String, theme: &str, classes: &str) {
        for class in classes.split_whitespace() {
            if !out.is_empty() {
                out.push(' ');
            }
            if !(class.starts_with("kai:") || class.starts_with("appkit:")) {
                out.push_str(theme);
                out.push(':');
            }
            out.push_str(class);
        }
    }
    let mut out = String::with_capacity(kai.len() + appkit.len() + 64);
    prefix(&mut out, "kai", kai);
    prefix(&mut out, "appkit", appkit);
    out
}

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

impl From<ButtonSize> for ControlSize {
    fn from(size: ButtonSize) -> Self {
        match size {
            ButtonSize::Sm => Self::Small,
            ButtonSize::Md => Self::Regular,
            ButtonSize::Lg => Self::Large,
        }
    }
}

/// `NSButton.BezelStyle` (subset) plus `isBordered`, for the AppKit theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BezelStyle {
    /// `.push`: bezel with a shadow; fixed height per control size.
    #[default]
    Push,
    /// `fill` background without shadow or border (toolbar style) \[C\].
    Bordered,
    /// `isBordered = false`: only the title, in the accent color.
    Borderless,
}

/// AppKit button: size from the control size (`h-control`…), keyboard-only
/// focus ring, colors instead of opacity when disabled, no hover.
const APPKIT_BUTTON_BASE: &str = "inline-flex items-center justify-center gap-1 \
     whitespace-nowrap h-control px-control rounded-control text-control font-normal \
     cursor-default keyboard-focus:ring-focus window-inactive:ring-0 \
     disabled:text-disabled-control-text";
/// 26+ (`glass:`): flat `secondarySystemFill` bezel, no border or shadow
/// (11 §6.3.8).
///
/// Hover (mixed theme, not in AppKit): `bezel-hover`, a quarter of the way
/// to `.pressed` in Aqua11 and one fill level up in glass; pressed glass is
/// `fill-pressed` (α .16).
const APPKIT_PUSH: &str = "bg-control text-control-text border-[0.5px] border-separator \
     shadow-bezel dark:border-0 hover:bg-bezel-hover active:bg-control-pressed \
     glass:bg-fill-2 glass:border-0 glass:shadow-none glass:active:bg-fill-pressed";
/// Default (or accent) push button; plain push look in an inactive window.
/// 26+: flat accent, no shadow. Hover: `accent-hover`, 15 % toward
/// `.pressed` (darker in light, lighter in dark).
const APPKIT_ACCENT: &str = "bg-control-accent text-on-accent border-0 shadow-bezel \
     hover:bg-accent-hover active:bg-accent-pressed window-inactive:bg-control window-inactive:text-control-text \
     window-inactive:border-[0.5px] window-inactive:border-separator \
     disabled:bg-accent-disabled glass:shadow-none glass:window-inactive:bg-fill-2 \
     glass:window-inactive:border-0";
/// Hover: one fill level up (`fill`); pressed `fill-pressed`.
const APPKIT_BORDERED: &str = "bg-fill-2 text-control-text hover:bg-fill active:bg-fill-pressed";
/// Hover: a faint `fill-3` (α .047) background behind the title.
const APPKIT_BORDERLESS: &str = "bg-transparent text-control-accent hover:bg-fill-3 \
     active:text-accent-pressed window-inactive:text-control-text px-1";
const APPKIT_DESTRUCTIVE_TEXT: &str = "text-system-red";

/// AppKit classes of a button (11 §6.2, §6.4, §7; \[C\] values are checked
/// against the oracle in Paso 19).
pub fn appkit_button_classes(bezel: BezelStyle, accent: bool, destructive: bool) -> String {
    let look = match bezel {
        BezelStyle::Push if accent => APPKIT_ACCENT,
        BezelStyle::Push => APPKIT_PUSH,
        BezelStyle::Bordered => APPKIT_BORDERED,
        BezelStyle::Borderless => APPKIT_BORDERLESS,
    };
    let destructive = if destructive && !accent {
        APPKIT_DESTRUCTIVE_TEXT
    } else {
        ""
    };
    format!("{APPKIT_BUTTON_BASE} {look} {destructive}")
}

/// Secondary button of the mixed-theme candidates (`MixStyle`): A tints
/// the push bezel with Kai `secondary` at 25 % (border and shadow kept), B
/// fills it at 60 % without border or shadow, C is Kai's solid secondary
/// with dark text. Hover/pressed: +10/+25 % (A), 75/90 % (B), 80/70 % (C,
/// as Kai).
const MIX_SECONDARY: &str = "mix-a:bg-mix-secondary/25 mix-a:hover:bg-mix-secondary/35 \
     mix-a:active:bg-mix-secondary/50 \
     mix-b:bg-mix-secondary/60 mix-b:border-0 mix-b:shadow-none \
     mix-b:hover:bg-mix-secondary/75 mix-b:active:bg-mix-secondary/90 \
     mix-c:bg-mix-secondary mix-c:text-mix-on-secondary mix-c:border-0 mix-c:shadow-none \
     mix-c:hover:bg-mix-secondary/80 mix-c:active:bg-mix-secondary/70";
/// Outlined button of the candidates: transparent with a 1 pt border of
/// `separator` (A, α .098), `tertiary-label` (B, α .26) or `control-text`
/// (C, as Kai's `border-foreground`; hover fills with the accent like
/// Kai).
const MIX_OUTLINED: &str = "mix-a:bg-transparent mix-a:border mix-a:border-separator \
     mix-a:shadow-none mix-a:hover:bg-fill-3 mix-a:active:bg-fill-2 \
     mix-b:bg-transparent mix-b:border mix-b:border-tertiary-label mix-b:shadow-none \
     mix-b:hover:bg-fill-2 mix-b:active:bg-fill \
     mix-c:bg-transparent mix-c:border mix-c:border-control-text mix-c:shadow-none \
     mix-c:hover:bg-control-accent mix-c:hover:border-control-accent \
     mix-c:hover:text-on-accent mix-c:active:bg-accent-pressed";

/// AppKit model of a Kai variant: Primary is an accent push button, Text is
/// borderless, Danger a push button with a red title. Secondary and
/// Outlined are push buttons in strict AppKit and change per candidate.
fn appkit_variant(variant: ButtonVariant) -> String {
    let push = || appkit_button_classes(BezelStyle::Push, false, false);
    match variant {
        ButtonVariant::Primary => appkit_button_classes(BezelStyle::Push, true, false),
        ButtonVariant::Secondary => format!("{} {MIX_SECONDARY}", push()),
        ButtonVariant::Outlined => format!("{} {MIX_OUTLINED}", push()),
        ButtonVariant::Text => appkit_button_classes(BezelStyle::Borderless, false, false),
        ButtonVariant::Danger => appkit_button_classes(BezelStyle::Push, false, true),
    }
}

/// `.fs-button`: medium weight, `text-sm`, `leading-5`, `rounded-md`, 1px
/// transparent border. Focus shows a ring (the web relies on the browser
/// outline), only when `:focus-visible`: not after a mouse click; disabled
/// fades (the web only changes the cursor).
const BUTTON_BASE: &str = "inline-flex items-center justify-center gap-2 rounded-md border \
     border-transparent font-medium whitespace-nowrap cursor-pointer focus-visible:ring-2 \
     focus-visible:ring-primary/40 focus-visible:ring-offset-2 disabled:opacity-50 \
     disabled:cursor-not-allowed";

pub fn button_classes(variant: ButtonVariant, size: ButtonSize) -> String {
    themed(&kai_button_classes(variant, size), &appkit_variant(variant))
}

/// The Kai (web) classes of a button, unchanged.
fn kai_button_classes(variant: ButtonVariant, size: ButtonSize) -> String {
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

/// Sized button; `Sm`/`Lg` also set the control size (Small/Large), which
/// only AppKit classes read.
pub fn button_sized<Msg>(
    label: impl Into<String>,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Element<Msg> {
    let button = div()
        .class(&button_classes(variant, size))
        .role(Role::Button)
        .child(text(label));
    match size {
        ButtonSize::Md => button,
        _ => button.control_size(size.into()),
    }
}

/// Button with the AppKit model (`NSButton`): bezel, accent, default and
/// cancel roles, control size. With the Kai theme it maps back to the Kai
/// variants: default/accent → Primary, borderless → Text, destructive →
/// Danger, otherwise Secondary.
pub struct Button<Msg> {
    label: String,
    bezel: BezelStyle,
    accent: bool,
    destructive: bool,
    size: Option<ControlSize>,
    is_default: bool,
    cancel: bool,
    disabled: bool,
    on_click: Option<Msg>,
}

impl<Msg> Button<Msg> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            bezel: BezelStyle::Push,
            accent: false,
            destructive: false,
            size: None,
            is_default: false,
            cancel: false,
            disabled: false,
            on_click: None,
        }
    }

    pub fn bezel(mut self, bezel: BezelStyle) -> Self {
        self.bezel = bezel;
        self
    }

    /// Accent fill without the Return key equivalent.
    pub fn accent(mut self, on: bool) -> Self {
        self.accent = on;
        self
    }

    /// Accent fill and Return (`ControlProps::is_default`).
    pub fn default_button(mut self, on: bool) -> Self {
        self.is_default = on;
        self
    }

    /// Escape and ⌘. activate it.
    pub fn cancel_button(mut self) -> Self {
        self.cancel = true;
        self
    }

    pub fn destructive(mut self, on: bool) -> Self {
        self.destructive = on;
        self
    }

    pub fn control_size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }

    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }

    /// The Kai variant this button looks like with the Kai theme.
    fn kai_variant(&self) -> ButtonVariant {
        if self.bezel == BezelStyle::Borderless {
            ButtonVariant::Text
        } else if self.destructive {
            ButtonVariant::Danger
        } else if self.accent || self.is_default {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Secondary
        }
    }

    pub fn build(self) -> Element<Msg> {
        let kai_size = match self.size {
            Some(ControlSize::Mini | ControlSize::Small) => ButtonSize::Sm,
            Some(ControlSize::Large | ControlSize::ExtraLarge) => ButtonSize::Lg,
            _ => ButtonSize::Md,
        };
        let classes = themed(
            &kai_button_classes(self.kai_variant(), kai_size),
            &appkit_button_classes(self.bezel, self.accent || self.is_default, self.destructive),
        );
        let mut button = div()
            .class(&classes)
            .role(Role::Button)
            .child(text(self.label))
            .default_button(self.is_default)
            .disabled(self.disabled);
        if let Some(size) = self.size {
            button = button.control_size(size);
        }
        if self.cancel {
            button = button.cancel_button();
        }
        if let Some(msg) = self.on_click {
            button = button.on_click(msg);
        }
        button
    }
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

/// Shape of the field with the AppKit theme (`NSTextField.BezelStyle`).
/// The Kai theme ignores it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldBezel {
    /// `.squareBezel`, as `NSTextField(string:)` (?): to check with the
    /// oracle (Paso 19).
    #[default]
    Square,
    /// `.roundedBezel`.
    Rounded,
}

/// AppKit field outline: 3 pt focus ring outside the border with any focus
/// (11 §6.4), off in an inactive window.
const APPKIT_FIELD_RING: &str = "focus-within:ring-focus window-inactive:ring-0";

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
    /// Boxed: an `Element` is much larger than the other variants.
    Element(Box<Element<Msg>>),
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
    bezel: FieldBezel,
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
            bezel: FieldBezel::Square,
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

    /// AppKit shape (square or rounded bezel).
    pub fn bezel(mut self, bezel: FieldBezel) -> Self {
        self.bezel = bezel;
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
        self.start = Some(Adornment::Element(Box::new(element)));
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
        self.end = Some(Adornment::Element(Box::new(element)));
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
        if compact {
            // AppKit: a compact field is a small control.
            root = root.control_size(ControlSize::Small);
        }
        if inline {
            let fade_shell = self.disabled && self.inline_leading.is_none();
            let shell = div()
                .class(&themed(
                    &format!(
                        "flex items-stretch w-full min-w-0 h-control-compact rounded-md border \
                         border-border bg-background focus-within:border-primary {cursor}{}",
                        if fade_shell { " opacity-50" } else { "" }
                    ),
                    &format!(
                        "flex items-stretch w-full min-w-0 {} {APPKIT_FIELD_RING} {cursor}",
                        self.appkit_shell()
                    ),
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
            let appkit_shell = if self.variant == FieldVariant::Autocomplete {
                "border-0 bg-transparent px-1".to_owned()
            } else {
                format!("{} {APPKIT_FIELD_RING}", self.appkit_shell())
            };
            let mut shell = div()
                .class(&themed(
                    &format!(
                        "relative flex items-center gap-2 w-full min-w-0 px-3 rounded-md {height} \
                         {border} {surface} {cursor}"
                    ),
                    &format!(
                        "relative flex items-center gap-1 w-full min-w-0 {appkit_shell} {cursor}"
                    ),
                ))
                .children(row);
            if compact {
                // AppKit reuses the static label above the field.
                root = root
                    .class("gap-1")
                    .child(div().class("flex").children(label_texts(
                        &self.label,
                        self.required,
                        &themed(
                            "text-[11px] font-medium leading-tight text-foreground",
                            "text-control text-label",
                        ),
                        &themed(
                            "ml-1 text-[11px] font-medium leading-tight text-error",
                            "ml-0.5 text-control text-system-red",
                        ),
                    )));
            } else {
                shell = shell.child(self.floating_label(muted));
                // The floating label is Material, not AppKit (11 §11.4):
                // AppKit shows a static label above instead, unless a
                // field-label experiment ([`Theme::field_labels`]) moves it:
                // `label-top:`/`label-tab:` bring Kai's border label back,
                // `label-near:` keeps it above in that type, 2 pt away.
                let experiments = self.variant == FieldVariant::Normal;
                let (spacing, static_label) = if experiments {
                    (
                        "gap-1 label-top:gap-0 label-top:mt-1.5 label-tab:gap-0 label-tab:mt-4 \
                         label-near:gap-0.5",
                        "flex label-top:hidden label-tab:hidden",
                    )
                } else {
                    ("gap-1", "flex")
                };
                let near = if experiments {
                    " label-near:text-xs label-near:leading-4 label-near:font-medium"
                } else {
                    ""
                };
                root = root.class(&themed("mt-1.5", spacing)).child(
                    div()
                        .class(&themed("hidden", static_label))
                        .children(label_texts(
                            &self.label,
                            self.required,
                            &format!("text-control text-label whitespace-nowrap{near}"),
                            &format!("ml-0.5 text-control text-system-red{near}"),
                        )),
                );
            }
            root = root.child(shell);
        }
        if let Some(helper) = &self.helper_text {
            root = root.child(text(helper).class("mt-1 text-xs text-muted-foreground"));
        }
        root
    }

    /// AppKit outline by bezel; disabled fields use the faint fill.
    /// Mixed theme: the outline darkens to `tertiary-label` (α .259 vs the
    /// separator's .098) under the pointer; no ring.
    fn appkit_shell(&self) -> String {
        let surface = if self.disabled {
            "bg-fill-5"
        } else {
            "bg-field hover:border-tertiary-label mix-c:hover:border-secondary-label"
        };
        let shape = match self.bezel {
            FieldBezel::Square => {
                "h-field-square rounded-field-square border-[0.5px] border-separator px-1"
            }
            FieldBezel::Rounded => "h-field rounded-field border-[0.5px] border-separator px-1.5",
        };
        // Candidates (heights come from `Theme::with_mix`): B pads like a
        // small Kai field; C takes Kai's padding, 1 pt border and radius.
        format!(
            "{shape} {surface} mix-b:px-2 mix-c:px-3 mix-c:rounded-md mix-c:border \
             mix-c:border-tertiary-label"
        )
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
        let resting = self.value.is_empty() && !self.always_show_label;
        let visibility = if resting {
            "hidden group-focus-within:flex"
        } else {
            "flex"
        };
        // Experiments ([`Theme::field_labels`]), in AppKit tones:
        // `label-top:` is the same label on the top border, only its lower
        // half masked with the field color (the upper half sits on whatever
        // is behind the field). `label-tab:` puts it in a tab that extends
        // the field: field fill and the field's border, rounded; `filler`
        // opens its bottom (covers the bottom border and corners, keeping
        // straight sides) and the field's top border below the label.
        let appkit = if self.variant == FieldVariant::Normal {
            let shown = if resting {
                "group-focus-within:flex"
            } else {
                "flex"
            };
            let surface = if muted { "bg-fill-5" } else { "bg-field" };
            format!(
                "hidden {} {}",
                prefixed(
                    "label-top",
                    &format!("absolute left-2 -top-2 z-20 px-1 pointer-events-none {shown}"),
                ),
                prefixed(
                    "label-tab",
                    &format!(
                        "absolute left-2 -top-4 z-20 px-1.5 rounded-md pointer-events-none \
                         {surface} {shown} {TAB_BORDER}"
                    ),
                ),
            )
        } else {
            "hidden".to_owned()
        };
        let mask = div().class(&themed(
            "hidden",
            "hidden label-top:flex absolute left-0 right-0 bottom-0 h-[9px] bg-field",
        ));
        let filler = div().class(&themed(
            "hidden",
            &format!(
                "hidden {}",
                prefixed(
                    "label-tab",
                    &format!(
                        "flex absolute -left-[0.5px] -right-[0.5px] -bottom-[0.5px] h-[7px] \
                         mix-c:-left-px mix-c:-right-px mix-c:-bottom-px {} {TAB_BORDER} \
                         border-y-0",
                        if muted { "bg-fill-5" } else { "bg-field" }
                    ),
                )
            ),
        ));
        let mut texts = vec![mask, filler];
        texts.extend(label_texts(
            &self.label,
            self.required,
            &themed(
                &format!("text-xs leading-4 font-medium {color}"),
                "text-xs leading-4 font-medium text-label",
            ),
            &themed(
                "ml-1 text-xs leading-4 font-medium text-error",
                "ml-0.5 text-xs leading-4 font-medium text-system-red",
            ),
        ));
        div()
            .class(&themed(
                &format!(
                    "absolute left-3 -top-2 z-20 px-1 rounded-md pointer-events-none {surface} \
                     {visibility}"
                ),
                &appkit,
            ))
            .children(texts)
    }

    /// The input plus, for required fields, the two-color placeholder
    /// overlay (a native placeholder has a single color).
    fn input_body(&mut self, compact: bool, inline: bool, muted: bool) -> Element<Msg> {
        // AppKit does not repeat the label as placeholder.
        let explicit_placeholder = self.placeholder.is_some();
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
        let mut appkit = String::from(
            "w-full self-stretch bg-transparent text-control text-label placeholder-placeholder",
        );
        if let Some(token) = &self.placeholder_color {
            appkit.push_str(&format!(" placeholder-{token}"));
        }
        if !explicit_placeholder {
            appkit.push_str(" placeholder-transparent");
        }
        if self.variant == FieldVariant::Normal && !compact {
            // `label-top:` experiment: Kai's placeholder (the label) that
            // hides on focus, when the label moves to the border.
            for prefix in ["label-top", "label-tab"] {
                if !explicit_placeholder {
                    appkit.push_str(&format!(" {prefix}:placeholder-placeholder"));
                }
                appkit.push_str(&format!(" {prefix}:focus:placeholder-transparent"));
            }
        }
        appkit.push_str(if self.disabled {
            " text-disabled-control-text cursor-default"
        } else {
            " cursor-text"
        });
        let classes = themed(&classes, &appkit);
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
                    .class(&themed(
                        "absolute inset-y-0 left-0 flex items-center pointer-events-none \
                         group-focus-within:hidden",
                        &if self.variant == FieldVariant::Normal {
                            let overlay = "absolute inset-y-0 left-0 flex items-center \
                                 pointer-events-none group-focus-within:hidden";
                            format!(
                                "hidden {} {}",
                                prefixed("label-top", overlay),
                                prefixed("label-tab", overlay)
                            )
                        } else {
                            "hidden".to_owned()
                        },
                    ))
                    .children(label_texts(
                        &placeholder,
                        true,
                        &themed(
                            "text-sm leading-5 text-muted-foreground whitespace-nowrap",
                            "text-control text-placeholder whitespace-nowrap",
                        ),
                        &themed(
                            "ml-1 text-sm leading-5 text-error",
                            "ml-0.5 text-control text-system-red",
                        ),
                    )),
            );
        }
        body
    }
}

/// Border of the `label-tab:` label tab: the field's outline (strict
/// AppKit 0.5 pt `separator`, mix C 1 pt `tertiary-label`), following its
/// hover; while focused (the field shows the focus ring) the tab's outline
/// turns to the focus color, except in an inactive window.
const TAB_BORDER: &str = "border-[0.5px] border-separator group-hover:border-tertiary-label \
     group-focus-within:border-focus window-inactive:border-separator \
     mix-c:border mix-c:border-tertiary-label mix-c:group-hover:border-secondary-label \
     mix-c:group-focus-within:border-focus mix-c:window-inactive:border-tertiary-label";

/// `classes` behind the `prefix:` variant (`prefixed("x", "a b")` is
/// `"x:a x:b"`).
fn prefixed(prefix: &str, classes: &str) -> String {
    classes
        .split_whitespace()
        .map(|class| format!("{prefix}:{class}"))
        .collect::<Vec<_>>()
        .join(" ")
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

/// Symbols and icons use the secondary color (`.fs-text-field__icon`;
/// `secondary-label` in AppKit, where `secondary` is the control face); a
/// trailing element keeps its own colors (`.fs-text-field__end-adornment`).
fn adornment<Msg>(
    adornment: Adornment<Msg>,
    compact: bool,
    muted: bool,
    trailing: bool,
) -> Element<Msg> {
    let tone = if muted {
        themed(
            "text-muted-foreground opacity-50",
            "text-disabled-control-text",
        )
    } else {
        themed("text-secondary", "text-secondary-label")
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
            .child(*element),
        Adornment::Element(element) => div()
            .class(&format!("flex shrink-0 items-center {tone}"))
            .child(*element),
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
    // AppKit: borderless, accent while revealed; the mixed theme adds a
    // faint `fill-3` behind the eye on hover.
    let appkit_state = match (disabled, revealed) {
        (true, _) => "text-disabled-control-text",
        (false, true) => "text-control-accent window-inactive:text-control-text hover:bg-fill-3",
        (false, false) => "text-secondary-label hover:bg-fill-3",
    };
    div()
        .class(&themed(
            &format!(
                "inline-flex size-8 shrink-0 -mr-2 items-center justify-center rounded-full \
                 focus-visible:ring-2 focus-visible:ring-primary/40 {state} {interaction}"
            ),
            &format!(
                "inline-flex size-[20px] shrink-0 items-center justify-center rounded-control \
                 cursor-default keyboard-focus:ring-focus window-inactive:ring-0 {appkit_state}"
            ),
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
const KAI_CARD: &str = "flex flex-col bg-background border border-border/70 rounded-lg \
     shadow-sm overflow-hidden";
/// Primary `NSBox` (group box): subtle fill, separator border, small radius,
/// no shadow. Fill: 11 §4.2 gives group boxes `quaternarySystemFill`
/// (`fill-4`) but 11 §11.4 says `quinarySystemFill`; open until the oracle
/// (Paso 19) decides. Radius 5 pt [C].
const APPKIT_BOX: &str = "flex flex-col bg-fill-4 border-[0.5px] border-separator \
     rounded-[5px] shadow-none overflow-hidden";
/// `NSBox.titleFont` (11 pt?).
const APPKIT_BOX_TITLE: &str = "text-subheadline font-normal text-label";

/// `.fs-card`; a group box with the AppKit theme.
pub fn card<Msg>() -> Element<Msg> {
    div().class(&themed(KAI_CARD, APPKIT_BOX)).role(Role::Group)
}

/// `.fs-card__header` with `.fs-card__title` and `.fs-card__subtitle`.
pub fn card_header<Msg>(title: &str, subtitle: Option<&str>) -> Element<Msg> {
    let mut header = div()
        .class(&themed(
            "flex flex-col gap-0.5 px-4 pt-4 pb-3",
            "flex flex-col gap-0.5 px-3 pt-3",
        ))
        .child(
            text(title)
                .class(&themed(
                    "text-lg leading-[1.4] font-semibold text-foreground",
                    "text-headline text-label",
                ))
                .role(Role::Heading),
        );
    if let Some(subtitle) = subtitle {
        header = header.child(text(subtitle).class(&themed(
            "text-sm leading-5 text-muted-foreground",
            "text-subheadline text-secondary-label",
        )));
    }
    header
}

/// `.fs-card__content`.
pub fn card_content<Msg>() -> Element<Msg> {
    div().class(&themed(
        "flex flex-col gap-3 p-4",
        "flex flex-col gap-control p-3",
    ))
}

/// `.fs-card__actions`.
pub fn card_actions<Msg>() -> Element<Msg> {
    div().class(&themed(
        "flex flex-wrap items-center justify-end gap-2 border-t border-border/70 px-4 pt-3 pb-4",
        "flex flex-wrap items-center justify-end gap-control px-3 pb-3 border-0",
    ))
}

/// `NSBox` with its title: in AppKit the title sits above the box, outside
/// it; with the Kai theme this is a card with a `card_header`. `content` goes
/// inside the box, in a `card_content`.
pub fn group_box<Msg>(
    title: Option<&str>,
    content: impl IntoIterator<Item = Element<Msg>>,
) -> Element<Msg> {
    let mut boxed = card();
    let mut root = div().class(&themed("flex flex-col", "flex flex-col gap-1"));
    if let Some(title) = title {
        root = root.child(
            text(title)
                .class(&themed("hidden", &format!("block px-1 {APPKIT_BOX_TITLE}")))
                .role(Role::Heading),
        );
        boxed = boxed
            .label(title)
            .child(card_header(title, None).class("appkit:hidden"));
    }
    root.child(boxed.child(card_content().children(content)))
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
    let mix = mix_icon_button(variant, size);
    // AppKit: a borderless button (`NSButton`, `isBordered = false`).
    let appkit_tone = match variant {
        IconButtonVariant::Primary | IconButtonVariant::Text => {
            "text-control-accent window-inactive:text-control-text"
        }
        IconButtonVariant::Action => "text-secondary-label",
        _ => "text-control-text",
    };
    div()
        .class(&themed(
            &format!(
                "inline-flex items-center justify-center shrink-0 cursor-pointer \
                 focus-visible:ring-2 focus-visible:ring-primary/40 disabled:opacity-50 \
                 disabled:cursor-not-allowed {size_class} {variant_classes}"
            ),
            &format!(
                "inline-flex items-center justify-center shrink-0 cursor-default {size_class} \
                 rounded-control bg-transparent {appkit_tone} hover:bg-fill-3 \
                 active:text-accent-pressed \
                 keyboard-focus:ring-focus window-inactive:ring-0 \
                 disabled:text-disabled-control-text {mix}"
            ),
        ))
        .role(Role::Button)
        .label(label)
        .child(icon(icon_kind).class(&format!(
            "{icon_class} {}",
            match size {
                IconButtonSize::Sm => "mix-a:size-3.5",
                IconButtonSize::Md => "mix-a:size-4",
                IconButtonSize::Lg => "mix-a:size-5",
            }
        )))
}

/// Icon button of the mixed-theme candidates. A: Kai radius at AppKit size
/// (20 / 24 / 28 pt, icons 14 / 16 / 20), AppKit tones, outline in
/// `separator`. B: Kai size and radius (28 / 40 / 48 pt), filled accent for
/// Primary, Kai secondary at 60 % for Secondary, `tertiary-label` outline,
/// `fill-2` hover. C: like B with solid Kai secondary and Kai's
/// `primary/10` hover (`control-accent/10`) on Action and Text.
fn mix_icon_button(variant: IconButtonVariant, size: IconButtonSize) -> String {
    let (radius, appkit_size) = match size {
        IconButtonSize::Sm => ("rounded", "size-5"),
        IconButtonSize::Md => ("rounded-md", "size-6"),
        IconButtonSize::Lg => ("rounded-lg", "size-7"),
    };
    let shape = format!("mix-a:{appkit_size} mix-a:{radius} mix-b:{radius} mix-c:{radius}");
    let accent = |m: &str| {
        format!(
            "{m}:bg-control-accent {m}:text-on-accent {m}:hover:bg-accent-hover \
             {m}:active:bg-accent-pressed {m}:window-inactive:bg-fill-2 \
             {m}:window-inactive:text-control-text"
        )
    };
    let tones = match variant {
        IconButtonVariant::Primary => format!("{} {}", accent("mix-b"), accent("mix-c")),
        IconButtonVariant::Secondary => {
            "mix-b:bg-mix-secondary/60 mix-b:hover:bg-mix-secondary/75 \
             mix-b:active:bg-mix-secondary/90 mix-c:bg-mix-secondary mix-c:text-mix-on-secondary \
             mix-c:hover:bg-mix-secondary/80 mix-c:active:bg-mix-secondary/70"
                .to_owned()
        }
        IconButtonVariant::Outlined => "mix-a:border mix-a:border-separator mix-b:border \
             mix-b:border-tertiary-label mix-b:hover:bg-fill-2 mix-c:border \
             mix-c:border-tertiary-label mix-c:hover:bg-fill-3"
            .to_owned(),
        IconButtonVariant::Action | IconButtonVariant::Text => "mix-b:hover:bg-fill-2 \
             mix-b:active:bg-fill mix-c:hover:bg-control-accent/10 \
             mix-c:active:bg-control-accent/20"
            .to_owned(),
        IconButtonVariant::Neutral => {
            "mix-b:hover:bg-fill-2 mix-b:active:bg-fill mix-c:hover:bg-fill-2 \
             mix-c:active:bg-fill"
                .to_owned()
        }
    };
    format!("{shape} {tones}")
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
    div().class("inline-flex items-center gap-2").children([
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
    // AppKit: accent track, gray in an inactive window (11 §6.2). Mixed
    // theme hover: `accent-hover` when on, `fill-pressed` (α .16) when off.
    // Sizes stay Kai until Paso 20.
    let appkit_track = if checked {
        "bg-control-accent hover:bg-accent-hover window-inactive:bg-switch-on-inactive"
    } else {
        "bg-fill hover:bg-fill-pressed"
    };
    let appkit_interaction = if disabled {
        "opacity-50 cursor-default"
    } else {
        "cursor-default"
    };
    div()
        .class(&themed(
            &format!(
                "relative inline-flex items-center shrink-0 rounded-full focus-visible:ring-2 \
                 focus-visible:ring-primary/40 {track_size} {pad} {track_bg} {justify} \
                 {interaction}"
            ),
            &format!(
                "relative inline-flex items-center shrink-0 rounded-full \
                 keyboard-focus:ring-focus window-inactive:ring-0 {track_size} {pad} \
                 {appkit_track} {justify} {appkit_interaction}"
            ),
        ))
        .role(Role::Button)
        .label(if checked { "Activado" } else { "Desactivado" })
        .focus_on_press(false)
        .disabled(disabled)
        .pressed(checked)
        .on_click(on_toggle)
        .child(div().class(&format!("rounded-full bg-white shadow-sm {thumb_size}")))
}

#[cfg(test)]
mod tests {
    use super::{
        AlertVariant, BadgeVariant, BezelStyle, Button, ButtonSize, ButtonVariant, FieldBezel,
        FieldDensity, FieldVariant, IconButtonSize, IconButtonVariant, LabelLayout, SwitchDensity,
        TextField, alert, badge, badge_classes, button, button_classes, card, card_actions,
        card_content, card_header, group_box, icon_button_sized, stat, switch_labeled,
        switch_with_options,
    };
    use crate::element::{ControlSize, Element, FocusOrder, Kind, Role, div, text};
    use crate::geometry::{Rect, Size};
    use crate::icons::Icon;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{
        Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache, WindowActivity,
    };
    use crate::render::Color;
    use crate::render::DrawOp;
    use crate::render::paint::{PaintInput, paint};
    use crate::style::Display;
    use crate::theme::{AppKitGeneration, Appearance, Density, Theme};
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

    fn appkit() -> Theme {
        Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light)
    }

    fn appkit_dark() -> Theme {
        Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Dark)
    }

    /// Every class is known with every theme.
    fn assert_known(classes: &str) {
        for theme in [theme(), appkit(), appkit_dark()] {
            let list = ClassList::parse(classes, &theme);
            assert!(list.unknown.is_empty(), "{classes}: {:?}", list.unknown);
        }
    }

    fn appkit_style(classes: &str, state: State) -> crate::style::Style {
        ClassList::parse(classes, &appkit()).resolve(state)
    }

    fn sized(size: ControlSize) -> State {
        State {
            control_size: size,
            ..State::default()
        }
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
        assert_tree_known(&group_box(Some("Opciones"), [text("Contenido")]));
        assert_tree_known(&group_box(None, [text("Contenido")]));

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
            for size in [IconButtonSize::Sm, IconButtonSize::Md, IconButtonSize::Lg] {
                assert_tree_known(&icon_button_sized(Icon::Search, "Buscar", variant, size));
            }
        }

        for density in [SwitchDensity::Default, SwitchDensity::Compact] {
            for checked in [false, true] {
                for disabled in [false, true] {
                    assert_tree_known(&switch_with_options(
                        checked,
                        Msg::Toggle,
                        disabled,
                        density,
                    ));
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
        let pointer_focus = list.resolve(State {
            focus: true,
            ..State::default()
        });
        assert_eq!(pointer_focus.ring.width, 0.0, "no ring after a click");
        let focus = list.resolve(State {
            focus: true,
            focus_visible: true,
            ..State::default()
        });
        assert_eq!((focus.ring.width, focus.ring.offset), (2.0, 2.0));
    }

    #[test]
    fn appkit_push_button_metrics() {
        let classes = button_classes(ButtonVariant::Secondary, ButtonSize::Md);
        let theme = appkit();
        let idle = appkit_style(&classes, State::default());
        assert_eq!(idle.height, crate::style::Length::Px(20.0));
        assert_eq!(idle.radius, 5.0);
        assert_eq!(idle.text.font_size, Some(13.0));
        assert_eq!(idle.text.weight, Some(theme.typography.weight_regular));
        assert_eq!(idle.background, Some(theme.semantic.control));
        assert_eq!(idle.border_width.top, 0.5);
        assert!(idle.shadow.is_some());
        assert_eq!(idle.cursor, Some(crate::style::Cursor::Default));
        let pointer = appkit_style(
            &classes,
            State {
                focus: true,
                focus_visible: false,
                ..State::default()
            },
        );
        assert_eq!(pointer.ring.width, 0.0, "no ring for pointer focus");
        let keyboard = appkit_style(
            &classes,
            State {
                focus: true,
                focus_visible: true,
                keyboard_focus: true,
                ..State::default()
            },
        );
        assert_eq!((keyboard.ring.width, keyboard.ring.offset), (3.0, 0.0));
        assert_eq!(keyboard.ring.color, Some(theme.semantic.focus_ring));
        let disabled = appkit_style(
            &classes,
            State {
                disabled: true,
                ..State::default()
            },
        );
        assert_eq!(disabled.opacity, 1.0, "AppKit dims with colors");
        assert_eq!(
            disabled.text.color,
            Some(theme.semantic.disabled_control_text)
        );
    }

    #[test]
    fn appkit_button_sizes() {
        let classes = button_classes(ButtonVariant::Primary, ButtonSize::Md);
        let heights: Vec<f32> = [
            ControlSize::Mini,
            ControlSize::Small,
            ControlSize::Regular,
            ControlSize::Large,
        ]
        .into_iter()
        .map(|size| match appkit_style(&classes, sized(size)).height {
            crate::style::Length::Px(px) => px,
            other => panic!("{other:?}"),
        })
        .collect();
        assert_eq!(heights, [13.0, 16.0, 20.0, 28.0]);
        let small: Element<Msg> = super::button_sized("Ok", ButtonVariant::Primary, ButtonSize::Sm);
        assert_eq!(small.control.size, Some(ControlSize::Small));
        let medium: Element<Msg> = button("Ok", ButtonVariant::Primary);
        assert_eq!(medium.control.size, None, "Md inherits");
    }

    #[test]
    fn appkit_default_button_loses_accent_when_inactive() {
        let theme = appkit();
        let default: Element<Msg> = Button::new("Guardar")
            .default_button(true)
            .on_click(Msg::Submit)
            .build();
        assert!(default.control.is_default);
        let active = appkit_style(&default.class, State::default());
        assert_eq!(active.background, Some(theme.semantic.control_accent));
        assert_eq!(active.text.color, Some(theme.semantic.on_accent));
        let inactive = appkit_style(
            &default.class,
            State {
                window_inactive: true,
                ..State::default()
            },
        );
        assert_eq!(inactive.background, Some(theme.semantic.control));
        assert_eq!(inactive.text.color, Some(theme.semantic.control_text));
        assert_eq!(inactive.border_width.top, 0.5);
    }

    fn appkit_themes() -> [Theme; 4] {
        [
            appkit(),
            appkit_dark(),
            Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light),
            Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark),
        ]
    }

    fn hovered() -> State {
        State {
            hover: true,
            ..State::default()
        }
    }

    #[test]
    fn appkit_buttons_hover_by_default() {
        let style = |theme: &Theme, classes: &str, state: State| {
            ClassList::parse(classes, theme).resolve(state).background
        };
        for theme in appkit_themes() {
            let s = theme.semantic;
            assert!(theme.hover_effects, "mixed theme");
            for (bezel, accent, rest, hover) in [
                (BezelStyle::Push, true, s.control_accent, s.accent_hover),
                (BezelStyle::Bordered, false, s.fill[1], s.fill[0]),
                (
                    BezelStyle::Borderless,
                    false,
                    Color::rgba(0, 0, 0, 0),
                    s.fill[2],
                ),
            ] {
                let classes = super::appkit_button_classes(bezel, accent, false);
                assert_eq!(style(&theme, &classes, State::default()), Some(rest));
                assert_eq!(style(&theme, &classes, hovered()), Some(hover), "{bezel:?}");
                let disabled = State {
                    disabled: true,
                    ..State::default()
                };
                let disabled_hover = State {
                    disabled: true,
                    ..hovered()
                };
                assert_eq!(
                    style(&theme, &classes, disabled),
                    style(&theme, &classes, disabled_hover),
                    "no hover when disabled"
                );
            }
            let push = super::appkit_button_classes(BezelStyle::Push, false, false);
            assert_eq!(style(&theme, &push, hovered()), Some(s.bezel_hover));
            assert_ne!(Some(s.bezel_hover), style(&theme, &push, State::default()));
            for variant in [
                ButtonVariant::Primary,
                ButtonVariant::Secondary,
                ButtonVariant::Outlined,
                ButtonVariant::Text,
                ButtonVariant::Danger,
            ] {
                let classes = button_classes(variant, ButtonSize::Md);
                assert_ne!(
                    style(&theme, &classes, State::default()),
                    style(&theme, &classes, hovered()),
                    "{variant:?}"
                );
            }
        }
        // The hover steps sit between rest and pressed.
        let light = appkit().semantic;
        assert_eq!(light.bezel_hover, Color::hex(0xf5f5f5));
        assert_eq!(light.accent_hover, Color::hex(0x0076f9));
        let glass = Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light).semantic;
        assert!(
            glass.fill[1].a < glass.bezel_hover.a && glass.bezel_hover.a < glass.fill_pressed.a
        );
    }

    #[test]
    fn button_builder_maps_to_kai_variants() {
        let kai = |button: Button<Msg>| button.build().class;
        let kai_part = |variant, size| super::kai_button_classes(variant, size);
        for (built, variant) in [
            (
                kai(Button::new("a").default_button(true)),
                ButtonVariant::Primary,
            ),
            (kai(Button::new("a").accent(true)), ButtonVariant::Primary),
            (
                kai(Button::new("a").bezel(BezelStyle::Borderless)),
                ButtonVariant::Text,
            ),
            (
                kai(Button::new("a").destructive(true)),
                ButtonVariant::Danger,
            ),
            (kai(Button::new("a")), ButtonVariant::Secondary),
        ] {
            let list = ClassList::parse(&built, &theme());
            let expected = ClassList::parse(&kai_part(variant, ButtonSize::Md), &theme());
            assert_eq!(list, expected, "{variant:?}");
        }
        let cancel: Element<Msg> = Button::new("Cancelar")
            .cancel_button()
            .control_size(ControlSize::Small)
            .bezel(BezelStyle::Bordered)
            .disabled(true)
            .build();
        assert_eq!(cancel.control.size, Some(ControlSize::Small));
        assert!(cancel.disabled);
        assert!(cancel.control.key_equivalent.is_some());
        assert_known(&cancel.class);
        let bordered = appkit_style(&cancel.class, sized(ControlSize::Small));
        assert_eq!(bordered.background, Some(appkit().semantic.fill[1]));
        assert_eq!(bordered.height, crate::style::Length::Px(16.0));
    }

    fn laid_out(theme: &Theme, root: &Element<Msg>, interaction: &Interaction) -> Layout {
        Layout::compute(
            root,
            &LayoutInput {
                viewport: Size {
                    width: 400.0,
                    height: 300.0,
                },
                theme,
                interaction,
                scroll: &ScrollOffsets::new(),
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        )
    }

    /// Focus ring quads (3 pt wide) painted for `field` focused.
    fn appkit_rings(window: WindowActivity) -> (Rect, Vec<crate::render::Quad>) {
        let mut field: Element<Msg> = TextField::new("Nombre", "").on_input(Msg::Edited).build();
        field.assign_ids();
        let input = field_input(&field).id;
        let shell = field.children[1].id;
        let theme = appkit();
        let layout = laid_out(
            &theme,
            &field,
            &Interaction {
                focused: Some(input),
                window,
                ..Interaction::default()
            },
        );
        let list = paint(&PaintInput {
            layout: &layout,
            theme: &theme,
            focused: Some(input),
            window,
            text_fields: &[],
            background: Default::default(),
        });
        let rings = list
            .ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Quad(quad) if quad.border_width == 3.0 => Some(*quad),
                _ => None,
            })
            .collect();
        (layout.node(shell).unwrap().bounds, rings)
    }

    #[test]
    fn appkit_text_field_has_no_floating_label() {
        let field: Element<Msg> = TextField::new("Nombre", "Ana").required(true).build();
        let floating = field.children[1].children.last().unwrap();
        let static_label = &field.children[0];
        let kai_display = |classes: &str| display(classes, group_focused());
        let appkit_display = |classes: &str| {
            ClassList::parse(classes, &appkit())
                .resolve(group_focused())
                .display
        };
        assert_eq!(kai_display(&floating.class), Display::Flex);
        assert_eq!(appkit_display(&floating.class), Display::None);
        assert_eq!(kai_display(&static_label.class), Display::None);
        assert_eq!(appkit_display(&static_label.class), Display::Flex);
        assert!(text_node(static_label, "Nombre").is_some());

        // AppKit does not repeat the label as placeholder.
        let empty: Element<Msg> = TextField::new("Nombre", "").build();
        let input = appkit_style(&field_input(&empty).class, State::default());
        assert_eq!(input.placeholder_color.map(|c| c.a), Some(0));
        let explicit: Element<Msg> = TextField::new("Nombre", "").placeholder("Ana").build();
        let input = appkit_style(&field_input(&explicit).class, State::default());
        assert_eq!(input.placeholder_color, Some(appkit().semantic.placeholder));
    }

    #[test]
    fn appkit_focus_ring_is_3pt_outside() {
        let (shell, rings) = appkit_rings(WindowActivity::ACTIVE);
        assert_eq!(rings.len(), 1, "{rings:?}");
        let ring = rings[0];
        assert_eq!(
            ring.rect,
            Rect {
                x: shell.x - 3.0,
                y: shell.y - 3.0,
                width: shell.width + 6.0,
                height: shell.height + 6.0,
            }
        );
        assert_eq!(ring.border, appkit().semantic.focus_ring);
        assert_eq!(ring.radius, 0.0, "square bezel, square ring");
    }

    #[test]
    fn appkit_ring_hidden_in_inactive_window() {
        let (_, rings) = appkit_rings(WindowActivity::INACTIVE);
        assert!(rings.is_empty(), "{rings:?}");
    }

    #[test]
    fn appkit_field_heights_by_size() {
        let theme = appkit();
        for (bezel, expected) in [
            (FieldBezel::Square, [15.0, 19.0, 21.0, 21.0]),
            (FieldBezel::Rounded, [17.0, 19.0, 22.0, 30.0]),
        ] {
            let heights: Vec<f32> = [
                ControlSize::Mini,
                ControlSize::Small,
                ControlSize::Regular,
                ControlSize::Large,
            ]
            .into_iter()
            .map(|size| {
                let mut root: Element<Msg> = div()
                    .control_size(size)
                    .child(TextField::new("Campo", "").bezel(bezel).build());
                root.assign_ids();
                let shell = root.children[0].children[1].id;
                let layout = laid_out(&theme, &root, &Interaction::default());
                layout.node(shell).unwrap().bounds.height
            })
            .collect();
            assert_eq!(heights, expected, "{bezel:?}");
        }
        // Compact density is a small control in AppKit.
        let compact: Element<Msg> = TextField::new("Campo", "")
            .density(FieldDensity::Compact)
            .build();
        assert_eq!(compact.control.size, Some(ControlSize::Small));
    }

    #[test]
    fn appkit_card_has_no_shadow() {
        let theme = appkit();
        let card: Element<Msg> = card();
        let kai = ClassList::parse(&card.class, &self::theme()).resolve(State::default());
        assert!(kai.shadow.is_some(), "Kai keeps shadow-sm");
        let boxed = appkit_style(&card.class, State::default());
        assert_eq!(boxed.shadow, None);
        assert_eq!(boxed.background, Some(theme.semantic.fill[3]), "fill-4");
        assert_eq!(boxed.border_width.top, 0.5);
        assert_eq!(boxed.radius, 5.0);
    }

    #[test]
    fn group_box_title_outside_in_appkit() {
        let group: Element<Msg> = group_box(Some("Opciones"), [text("Contenido")]);
        let outside = &group.children[0];
        let boxed = &group.children[1];
        assert_eq!(boxed.role, Some(Role::Group));
        assert_eq!(boxed.label.as_deref(), Some("Opciones"));
        let inside = &boxed.children[0];
        assert!(text_node(inside, "Opciones").is_some());
        assert!(text_node(&boxed.children[1], "Contenido").is_some());
        let kai_display = |classes: &str| display(classes, State::default());
        let appkit_display = |classes: &str| appkit_style(classes, State::default()).display;
        assert_eq!(kai_display(&outside.class), Display::None);
        assert_eq!(appkit_display(&outside.class), Display::Block);
        assert_eq!(kai_display(&inside.class), Display::Flex);
        assert_eq!(appkit_display(&inside.class), Display::None);
    }

    /// One of every public component, in its interactive states.
    fn component_gallery() -> Vec<Element<Msg>> {
        let mut all: Vec<Element<Msg>> = Vec::new();
        for variant in [
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Outlined,
            ButtonVariant::Text,
            ButtonVariant::Danger,
        ] {
            for size in [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg] {
                all.push(super::button_sized("Ok", variant, size));
            }
        }
        for bezel in [
            BezelStyle::Push,
            BezelStyle::Bordered,
            BezelStyle::Borderless,
        ] {
            all.push(Button::new("Ok").bezel(bezel).build());
            all.push(Button::new("Ok").bezel(bezel).default_button(true).build());
        }
        for variant in [
            FieldVariant::Normal,
            FieldVariant::Contrast,
            FieldVariant::Autocomplete,
        ] {
            for density in [FieldDensity::Default, FieldDensity::Compact] {
                for layout in [LabelLayout::Stack, LabelLayout::Inline] {
                    for revealed in [false, true] {
                        all.push(
                            TextField::new("Campo", "x")
                                .variant(variant)
                                .density(density)
                                .label_layout(layout)
                                .password(revealed, Msg::Toggle)
                                .build(),
                        );
                    }
                }
            }
        }
        for variant in [
            IconButtonVariant::Action,
            IconButtonVariant::Primary,
            IconButtonVariant::Secondary,
            IconButtonVariant::Neutral,
            IconButtonVariant::Text,
            IconButtonVariant::Outlined,
        ] {
            all.push(icon_button_sized(
                Icon::Search,
                "Buscar",
                variant,
                IconButtonSize::Md,
            ));
        }
        for checked in [false, true] {
            all.push(switch_labeled(
                "Avisos",
                checked,
                Msg::Toggle,
                false,
                SwitchDensity::Default,
            ));
        }
        all.push(card().children([card_header("T", Some("S")), card_content(), card_actions()]));
        all.push(group_box(Some("Opciones"), [text("x")]));
        all.push(stat("Clics", "3", Some("hoy")));
        all.push(badge("Nuevo", BadgeVariant::Primary));
        all.push(alert(AlertVariant::Info, "Mensaje"));
        all
    }

    #[test]
    fn mix_candidates_restyle_secondary_outlined_icon_buttons_and_fields() {
        use crate::style::Length;
        use crate::theme::MixStyle;
        for strict in appkit_themes() {
            let s = strict.semantic;
            let reference =
                |classes: &str| ClassList::parse(classes, &strict).resolve(State::default());
            for mix in [MixStyle::AppKit, MixStyle::A, MixStyle::B, MixStyle::C] {
                let theme = strict.with_mix(mix);
                let style =
                    |classes: &str| ClassList::parse(classes, &theme).resolve(State::default());
                let secondary = style(&button_classes(ButtonVariant::Secondary, ButtonSize::Md));
                let outlined = style(&button_classes(ButtonVariant::Outlined, ButtonSize::Md));
                let push = style(&super::appkit_button_classes(
                    BezelStyle::Push,
                    false,
                    false,
                ));
                let icon = |variant| {
                    let button =
                        icon_button_sized::<Msg>(Icon::Search, "x", variant, IconButtonSize::Md);
                    style(&button.class)
                };
                let mut shell = None;
                TextField::<Msg>::new("Campo", "x")
                    .build()
                    .walk(&mut |element| {
                        if element.class.contains("h-field") {
                            shell = Some(style(&element.class));
                        }
                    });
                let shell = shell.expect("field shell");
                assert_eq!(
                    shell.height,
                    Length::Px(theme.controls.field_square_height[2])
                );
                let transparent = Some(Color::rgba(0, 0, 0, 0));
                let primary_icon = icon(IconButtonVariant::Primary);
                if mix == MixStyle::AppKit {
                    assert_eq!(secondary, push);
                    assert_eq!(outlined, push);
                    assert_eq!(primary_icon.background, transparent);
                    assert_eq!(shell.border_width.top, 0.5);
                    continue;
                }
                assert_ne!(secondary.background, outlined.background, "{mix:?}");
                assert_eq!(outlined.background, transparent, "{mix:?}");
                assert_eq!(outlined.border_width.top, 1.0, "{mix:?}");
                let (tint, border, icon_width) = match mix {
                    MixStyle::A => ("bg-mix-secondary/25", s.separator, 24.0),
                    MixStyle::B => ("bg-mix-secondary/60", s.tertiary_label, 40.0),
                    _ => ("bg-mix-secondary", s.control_text, 40.0),
                };
                assert_eq!(secondary.background, reference(tint).background, "{mix:?}");
                assert_eq!(outlined.border_color, Some(border), "{mix:?}");
                assert_eq!(primary_icon.width, Length::Px(icon_width), "{mix:?}");
                assert_eq!(primary_icon.radius, 6.0, "Kai rounded-md");
                let filled = if mix == MixStyle::A {
                    transparent
                } else {
                    Some(s.control_accent)
                };
                assert_eq!(primary_icon.background, filled, "{mix:?}");
                if mix == MixStyle::C {
                    assert_eq!(secondary.text.color, Some(s.mix_on_secondary));
                    assert_eq!((shell.radius, shell.border_width.top), (6.0, 1.0));
                    assert_eq!(shell.border_color, Some(s.tertiary_label));
                }
            }
        }
    }

    #[test]
    fn mix_candidates_keep_hover_on_every_button() {
        use crate::theme::MixStyle;
        for strict in appkit_themes() {
            for mix in [MixStyle::A, MixStyle::B, MixStyle::C] {
                let theme = strict.with_mix(mix);
                let resolve =
                    |classes: &str, state| ClassList::parse(classes, &theme).resolve(state);
                for component in component_gallery() {
                    component.walk(&mut |element| {
                        if element.role == Some(Role::Button) {
                            assert_ne!(
                                resolve(&element.class, State::default()),
                                resolve(&element.class, hovered()),
                                "{mix:?} {}",
                                element.class
                            );
                        }
                    });
                }
            }
        }
    }

    #[test]
    fn appkit_hover_reaches_controls_but_not_surfaces() {
        for theme in appkit_themes() {
            let resolve = |classes: &str, state| ClassList::parse(classes, &theme).resolve(state);
            let changes =
                |classes: &str| resolve(classes, State::default()) != resolve(classes, hovered());
            let (mut buttons, mut shells) = (0, 0);
            for component in component_gallery() {
                component.walk(&mut |element| {
                    if element.role == Some(Role::Button) {
                        buttons += 1;
                        assert!(changes(&element.class), "{}", element.class);
                    }
                    if element.class.contains("h-field") {
                        shells += 1;
                        let border = resolve(&element.class, hovered()).border_color;
                        assert_eq!(border, Some(theme.semantic.tertiary_label));
                    }
                });
            }
            assert!(buttons > 10 && shells > 10, "{buttons} {shells}");
            // Cards, group boxes, stats, badges and alerts never react.
            let surfaces: [Element<Msg>; 5] = [
                card().children([card_header("T", Some("S")), card_content(), card_actions()]),
                group_box(Some("Opciones"), [text("x")]),
                stat("Clics", "3", Some("hoy")),
                badge("Nuevo", BadgeVariant::Primary),
                alert(AlertVariant::Info, "Mensaje"),
            ];
            for surface in surfaces {
                surface.walk(&mut |element| {
                    assert!(!changes(&element.class), "{}", element.class);
                });
            }
        }
    }

    #[test]
    fn appkit_accent_controls_dim_when_inactive() {
        let theme = appkit();
        let inactive = State {
            window_inactive: true,
            ..State::default()
        };
        let text_color = |classes: &str, state| appkit_style(classes, state).text.color;
        let borderless = button_classes(ButtonVariant::Text, ButtonSize::Md);
        assert_eq!(
            text_color(&borderless, State::default()),
            Some(theme.semantic.control_accent)
        );
        assert_eq!(
            text_color(&borderless, inactive),
            Some(theme.semantic.control_text)
        );
        let on: Element<Msg> =
            switch_with_options(true, Msg::Toggle, false, SwitchDensity::Default);
        let track = |state| appkit_style(&on.class, state).background;
        assert_eq!(track(State::default()), Some(theme.semantic.control_accent));
        assert_eq!(track(inactive), Some(theme.semantic.switch_on_inactive));
        let off: Element<Msg> =
            switch_with_options(false, Msg::Toggle, false, SwitchDensity::Default);
        assert_eq!(
            appkit_style(&off.class, inactive).background,
            Some(theme.semantic.fill[0])
        );
        // The focus ring of a field goes away too (Paso 10) and the default
        // button loses its accent (Paso 9).
        let (_, rings) = appkit_rings(WindowActivity::INACTIVE);
        assert!(rings.is_empty());
    }

    #[test]
    fn kai_components_unchanged() {
        // The Kai half of every themed string is the previous string: with
        // the Kai theme `themed(kai, appkit)` parses to the same
        // declarations as `kai`. The capture baseline covers the rest.
        let kai = theme();
        for variant in [
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Outlined,
            ButtonVariant::Text,
            ButtonVariant::Danger,
        ] {
            for size in [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Lg] {
                assert_eq!(
                    ClassList::parse(&button_classes(variant, size), &kai),
                    ClassList::parse(&super::kai_button_classes(variant, size), &kai),
                );
            }
        }
        let sample = "group flex hover:bg-accent focus-within:border-primary";
        assert_eq!(
            ClassList::parse(&super::themed(sample, "bg-fill p-1"), &kai),
            ClassList::parse(sample, &kai)
        );
    }

    // Default-density fields: `children[0]` is the AppKit static label
    // (hidden with the Kai theme), `children[1]` the outline.
    #[test]
    fn text_field_floats_its_label_only_with_value_or_focus() {
        let empty: Element<Msg> = TextField::new("Nombre", "")
            .on_input(Msg::Edited)
            .on_submit(Msg::Submit)
            .build();
        assert!(empty.delegates_focus, "clicks anywhere focus the input");
        let shell = &empty.children[1];
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
        let label = filled.children[1].children.last().unwrap();
        assert_eq!(display(&label.class, State::default()), Display::Flex);
        assert!(matches!(
            &field_input(&filled).kind,
            Kind::Input { placeholder, .. } if placeholder == "Tu nombre"
        ));
    }

    /// Field-label modes ([`FieldLabels`], default `near`): only AppKit
    /// themes change; Kai keeps its labels.
    #[test]
    fn label_experiments_place_appkit_field_labels() {
        use crate::theme::{FieldLabels, MixStyle};
        let field: Element<Msg> = TextField::new("Correo", "a@b.cl").build();
        let floating = find(&field, |e| e.class.contains("label-top:-top-2")).unwrap();
        let fixed = find(&field, |e| e.class.contains("label-top:hidden")).unwrap();
        let mask = find(&field, |e| e.class.contains("h-[9px]")).unwrap();
        let filler = find(&field, |e| e.class.contains("h-[7px]")).unwrap();
        fn style(element: &Element<Msg>, theme: &Theme, state: State) -> crate::style::Style {
            ClassList::parse(&element.class, theme).resolve(state)
        }
        fn shown(element: &Element<Msg>, theme: &Theme) -> bool {
            style(element, theme, State::default()).display != Display::None
        }
        let mix =
            Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light).with_mix(MixStyle::C);
        // Default `near`; `appkit` is the original static label.
        assert_eq!(mix.field_labels, FieldLabels::Near);
        let appkit = mix.with_field_labels(FieldLabels::AppKit);
        let with = |labels| appkit.with_field_labels(labels);
        // (border label, static label, half mask, tab filler)
        let seen = |theme: &Theme| {
            (
                shown(floating, theme),
                shown(fixed, theme),
                shown(mask, theme),
                shown(filler, theme),
            )
        };
        assert_eq!(seen(&appkit), (false, true, false, false));
        assert_eq!(seen(&with(FieldLabels::Top)), (true, false, true, false));
        assert_eq!(seen(&with(FieldLabels::Near)), (false, true, false, false));
        assert_eq!(seen(&with(FieldLabels::Tab)), (true, false, false, true));
        let kai = Theme::kai(Density::Comfortable);
        for labels in [FieldLabels::AppKit, FieldLabels::Top, FieldLabels::Tab] {
            assert_eq!(
                seen(&kai.with_field_labels(labels)),
                (true, false, false, false)
            );
        }

        // Near: static label in the border label's type, 2 pt above.
        let near = with(FieldLabels::Near);
        let label = text_node(fixed, "Correo").unwrap();
        let size = |theme| style(label, theme, State::default()).text.font_size;
        let floating_text = text_node(floating, "Correo").unwrap();
        assert_eq!(
            size(&near),
            style(floating_text, &near, State::default()).text.font_size
        );
        assert_ne!(size(&near), size(&appkit));
        let gap = |theme| style(&field, theme, State::default()).gap_y;
        assert_eq!((gap(&appkit), gap(&near)), (4.0, 2.0));

        // Tab: field fill and the field's 1 pt outline; the filler opens
        // its bottom edge into the field. The focus color while focused.
        let tab = with(FieldLabels::Tab);
        let rest = style(floating, &tab, State::default());
        assert_eq!(rest.background, tab.color("field"));
        assert_eq!(rest.border_color, tab.color("tertiary-label"));
        assert_eq!(
            (rest.border_width.top, rest.border_width.left, rest.radius),
            (1.0, 1.0, 6.0)
        );
        let focused = State {
            group_focus_within: true,
            ..State::default()
        };
        assert_eq!(
            style(floating, &tab, focused).border_color,
            tab.color("focus")
        );
        let inactive = State {
            window_inactive: true,
            ..focused
        };
        assert_eq!(
            style(floating, &tab, inactive).border_color,
            tab.color("tertiary-label")
        );
        let filler = style(filler, &tab, State::default());
        assert_eq!(filler.background, tab.color("field"));
        assert_eq!(
            (filler.border_width.top, filler.border_width.bottom),
            (0.0, 0.0)
        );
        assert_eq!(filler.border_width.left, 1.0);
        assert_eq!(filler.inset.bottom, crate::style::Length::Px(-1.0));

        // Contrast fields keep the AppKit label in every experiment.
        let contrast: Element<Msg> = TextField::new("Buscar", "x")
            .variant(FieldVariant::Contrast)
            .build();
        let mut experimental = false;
        contrast.walk(&mut |e| {
            experimental |= e.class.contains("label-top:") || e.class.contains("label-tab:");
            experimental |= e.class.contains("label-near:");
        });
        // Only the hidden mask and filler (inside the hidden border label).
        let label = find(&contrast, |e| e.class.contains("-top-2")).unwrap();
        for labels in [FieldLabels::Top, FieldLabels::Near, FieldLabels::Tab] {
            assert!(!shown(label, &with(labels)));
        }
        assert!(experimental);
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
        // Floating label and placeholder overlay (Kai), static label (AppKit).
        assert_eq!(asterisks.len(), 3);
        let kai = asterisks
            .iter()
            .filter(|class| class.contains("text-error"));
        assert_eq!(kai.count(), 2);
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
        let shell = &hidden.children[1];
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
        let shell = &read_only.children[1];
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
        assert!(disabled.children[1].class.contains("cursor-not-allowed"));
        assert_eq!(FocusOrder::from_tree(&disabled).ids().count(), 0);

        let contrast: Element<Msg> = TextField::new("Buscar", "x")
            .variant(FieldVariant::Contrast)
            .build();
        let label = contrast.children[1].children.last().unwrap();
        assert!(label.class.contains("bg-primary"));
        let autocomplete: Element<Msg> = TextField::new("Buscar", "")
            .variant(FieldVariant::Autocomplete)
            .read_only(true)
            .build();
        let shell =
            ClassList::parse(&autocomplete.children[1].class, &theme).resolve(State::default());
        assert_eq!(shell.border_width.top, 0.0);
        assert_eq!(shell.background, theme.color("transparent"));
    }
}
