//! Color field: a hex text field with a swatch of the color, and a palette
//! of suggested colors (like the color of an area type in DSP).

use std::rc::Rc;

use super::{TextField, themed};
use crate::element::{Element, Role, div, icon, text};
use crate::icons::Icon;
use crate::render::Color;

/// Text field for a color as CSS hex (`#1e73ae`, `#f80`), a swatch of the
/// color in it, and a row of suggested colors below. Picking a suggestion
/// writes its hex; «Sin color» ([`ColorField::allow_empty`]) empties it.
///
/// The app keeps the text, not a [`Color`]: every edit and every pick
/// goes through `on_input`, so a half-typed hex stays in the field. Read
/// the color with [`Color::parse_hex`]; a text that is neither empty nor a
/// color shows an error under the field.
///
/// The palette is a radio group, one Tab stop whose colors the arrow keys
/// walk; each color is named («Azul») and described by its hex.
///
/// Parts, with a `test_id`: `<id>.hex` (the [`TextField`], whose input is
/// `<id>.hex.input`), `<id>.swatch`, `<id>.palette`, `<id>.palette.<i>`,
/// `<id>.clear` and `<id>.error`.
pub struct ColorField<Msg> {
    label: String,
    value: String,
    palette: Vec<(String, Color)>,
    on_input: Rc<dyn Fn(String) -> Msg>,
    allow_empty: bool,
    disabled: bool,
    test_id: Option<String>,
}

/// A [`ColorField`] labeled `label`, showing `value` and sending each edit
/// or pick to `on_input`.
pub fn color_field<Msg>(
    label: impl Into<String>,
    value: impl Into<String>,
    on_input: impl Fn(String) -> Msg + 'static,
) -> ColorField<Msg> {
    ColorField {
        label: label.into(),
        value: value.into(),
        palette: Vec::new(),
        on_input: Rc::new(on_input),
        allow_empty: false,
        disabled: false,
        test_id: None,
    }
}

/// Message under a text that is not a color.
pub const INVALID_COLOR: &str = "Escribe el color como #RGB o #RRGGBB";

impl<Msg: Clone + 'static> ColorField<Msg> {
    /// Suggested colors, with the names that read them out.
    pub fn palette<N: Into<String>>(
        mut self,
        colors: impl IntoIterator<Item = (N, Color)>,
    ) -> Self {
        self.palette = colors
            .into_iter()
            .map(|(name, color)| (name.into(), color))
            .collect();
        self
    }

    /// Offer «Sin color», which empties the field.
    pub fn allow_empty(mut self, allow_empty: bool) -> Self {
        self.allow_empty = allow_empty;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let color = Color::parse_hex(&self.value);
        let empty = self.value.trim().is_empty();
        let id = |part: &str| self.test_id.as_ref().map(|id| format!("{id}.{part}"));

        let mut swatch = div()
            .class(&match color {
                Some(color) => themed(
                    &format!(
                        "size-4 shrink-0 rounded-[4px] border border-[#000000]/15 bg-[{}]",
                        color.to_hex()
                    ),
                    &format!(
                        "size-[14px] shrink-0 rounded-[3px] border-[0.5px] border-separator bg-[{}]",
                        color.to_hex()
                    ),
                ),
                None => themed(
                    "size-4 shrink-0 rounded-[4px] border border-border bg-transparent",
                    "size-[14px] shrink-0 rounded-[3px] border-[0.5px] border-separator \
                     bg-transparent",
                ),
            })
            .role(Role::Image)
            .label(match color {
                Some(color) => format!("Color {}", color.to_hex()),
                None => "Sin color".to_owned(),
            });
        swatch.test_id = id("swatch");

        let on_input = self.on_input.clone();
        let mut field = TextField::new(&self.label, &self.value)
            .placeholder("#rrggbb")
            .start_adornment(swatch)
            .disabled(self.disabled)
            .on_input(move |text| on_input(text));
        if let Some(hex) = id("hex") {
            field = field.test_id(hex);
        }

        let option = |selected: bool| {
            themed(
                &format!(
                    "flex items-center justify-center size-6 rounded-md cursor-pointer \
                     selected:ring-2 selected:ring-primary selected:ring-offset-2 \
                     focus-visible:ring-2 focus-visible:ring-primary/40 \
                     disabled:opacity-50 disabled:cursor-not-allowed{}",
                    if selected { "" } else { " hover:opacity-80" }
                ),
                "flex items-center justify-center size-[18px] rounded-[4px] cursor-default \
                 selected:ring-2 selected:ring-control-accent selected:ring-offset-1 \
                 keyboard-focus:ring-focus window-inactive:ring-0 disabled:opacity-50",
            )
        };
        let mut swatches: Vec<Element<Msg>> = self
            .palette
            .iter()
            .enumerate()
            .map(|(index, (name, suggested))| {
                let selected = color == Some(*suggested);
                let mut option = div()
                    .class(&option(selected))
                    .child(div().class(&themed(
                        &format!(
                            "size-full rounded-md border border-[#000000]/15 bg-[{}]",
                            suggested.to_hex()
                        ),
                        &format!(
                            "size-full rounded-[4px] border-[0.5px] border-separator bg-[{}]",
                            suggested.to_hex()
                        ),
                    )))
                    .role(Role::Radio)
                    .label(name.clone())
                    .description(suggested.to_hex())
                    .key(index.to_string())
                    .pressed(selected)
                    .disabled(self.disabled)
                    .on_click((self.on_input)(suggested.to_hex()));
                option.test_id = id(&format!("palette.{index}"));
                option
            })
            .collect();
        if self.allow_empty {
            let mut clear = div()
                .class(&option(empty))
                .child(
                    div()
                        .class(&themed(
                            "flex items-center justify-center size-full rounded-md \
                             border border-border text-muted-foreground",
                            "flex items-center justify-center size-full rounded-[4px] \
                             border-[0.5px] border-separator text-secondary-label",
                        ))
                        .child(icon(Icon::X).class("size-3")),
                )
                .role(Role::Radio)
                .label("Sin color")
                .key("sin-color")
                .pressed(empty)
                .disabled(self.disabled)
                .on_click((self.on_input)(String::new()));
            clear.test_id = id("clear");
            swatches.push(clear);
        }
        let palette = (!swatches.is_empty()).then(|| {
            let mut palette = div()
                .class("flex flex-wrap items-center gap-2 p-1")
                .role(Role::RadioGroup)
                .focus_group()
                .label(format!("{}: colores sugeridos", self.label))
                .children(swatches);
            palette.test_id = id("palette");
            palette
        });

        let error = (!empty && color.is_none()).then(|| {
            let mut error = text(INVALID_COLOR)
                .class(&themed("text-xs text-error", "text-xs text-system-red"))
                .role(Role::Alert);
            error.test_id = id("error");
            error
        });

        let mut root = div()
            .class("flex flex-col gap-1.5 w-full min-w-[180px]")
            .role(Role::Group)
            .label(self.label.clone())
            .child(field.build())
            .children(error)
            .children(palette);
        root.test_id = self.test_id;
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{INVALID_COLOR, color_field};
    use crate::element::{ControlState, Element, Role, div};
    use crate::render::Color;
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::theme::{AppKitGeneration, Appearance, Theme};
    use crate::ui_engine::{Key, Modifiers};

    const AZUL: Color = Color::hex(0x1e73ae);
    const VERDE: Color = Color::hex(0x2d6a4f);

    #[derive(Default)]
    struct Tipo {
        color: String,
        disabled: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Color(String),
    }

    impl SimpleApp for Tipo {
        type Msg = Msg;
        fn title(&self) -> String {
            "Tipo de área".into()
        }
        fn update(&mut self, Msg::Color(color): Msg) {
            self.color = color;
        }
        fn view(&self) -> Element<Msg> {
            div().class("flex flex-col p-4 w-96").child(
                color_field("Color", &self.color, Msg::Color)
                    .palette([("Azul", AZUL), ("Verde", VERDE)])
                    .allow_empty(true)
                    .disabled(self.disabled)
                    .test_id("tipo.color")
                    .build(),
            )
        }
    }

    fn app(color: &str) -> TestApp<Tipo> {
        TestApp::new(Tipo {
            color: color.into(),
            disabled: false,
        })
    }

    fn pressed(app: &TestApp<Tipo>, by: By) -> bool {
        app.screen().get(by).node().control_state == Some(ControlState::On)
    }

    #[test]
    fn picking_a_suggestion_writes_its_hex_and_marks_it() {
        let mut app = app("");
        let azul = By::role(Role::Radio).name("Azul");
        let clear = By::test_id("tipo.color.clear");
        assert!(!pressed(&app, azul.clone()));
        assert!(pressed(&app, clear.clone()));

        app.click(azul.clone());
        assert_eq!(app.app().color, "#1e73ae");
        assert!(pressed(&app, azul));
        assert!(!pressed(&app, clear));
        assert_eq!(app.value(By::test_id("tipo.color.hex")), "#1e73ae");
        assert_eq!(
            app.screen()
                .get(By::test_id("tipo.color.swatch"))
                .name()
                .as_deref(),
            Some("Color #1e73ae")
        );

        app.click(By::test_id("tipo.color.clear"));
        assert_eq!(app.app().color, "");
    }

    #[test]
    fn typing_keeps_a_partial_hex_and_shows_the_error_until_it_is_a_color() {
        let mut app = app("");
        let error = By::test_id("tipo.color.error");
        app.type_text(By::test_id("tipo.color.hex"), "#2d6a");
        assert_eq!(app.app().color, "#2d6a", "the field keeps what was typed");
        assert_eq!(
            app.screen().get(error.clone()).name().as_deref(),
            Some(INVALID_COLOR)
        );
        assert_eq!(app.screen().get(error.clone()).role(), Role::Alert);

        app.type_text(By::test_id("tipo.color.hex"), "4F");
        assert_eq!(app.app().color, "#2d6a4F");
        assert_eq!(Color::parse_hex(&app.app().color), Some(VERDE));
        assert!(app.screen().query(error).is_none());
        assert!(
            pressed(&app, By::role(Role::Radio).name("Verde")),
            "a typed color that is in the palette is marked"
        );
    }

    #[test]
    fn the_palette_is_one_tab_stop_walked_with_arrows_and_named_with_hex() {
        let mut app = app("#1e73ae");
        let screen = app.screen();
        let palette = screen.get(By::role(Role::RadioGroup).name("Color: colores sugeridos"));
        assert_eq!(palette.test_id(), Some("tipo.color.palette"));
        let azul = screen.get(By::role(Role::Radio).name("Azul"));
        assert_eq!(azul.node().description.as_deref(), Some("#1e73ae"));
        assert_eq!(
            screen
                .all(By::role(Role::Radio))
                .iter()
                .map(|radio| radio.name().unwrap_or_default())
                .collect::<Vec<_>>(),
            ["Azul", "Verde", "Sin color"]
        );

        app.click(By::role(Role::Radio).name("Azul"));
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert!(app.is_focused(By::role(Role::Radio).name("Verde")));
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(app.app().color, "#2d6a4f");
    }

    #[test]
    fn a_disabled_field_takes_no_picks() {
        let mut app = TestApp::new(Tipo {
            color: "#1e73ae".into(),
            disabled: true,
        });
        assert!(!app.is_enabled(By::role(Role::Radio).name("Verde")));
        assert!(!app.is_enabled(By::test_id("tipo.color.hex")));
        app.click(By::role(Role::Radio).name("Verde"));
        assert_eq!(app.app().color, "#1e73ae");
    }

    #[test]
    fn every_class_is_known_in_both_themes() {
        for theme in [
            Theme::kai(crate::theme::Density::Comfortable),
            Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light),
            Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark),
        ] {
            for value in ["", "#1e73ae", "#zz"] {
                let app = TestApp::builder(Tipo {
                    color: value.into(),
                    disabled: false,
                })
                .theme(theme)
                .build();
                assert_eq!(
                    app.session().unknown_classes(),
                    Vec::<&str>::new(),
                    "{value:?}"
                );
            }
        }
    }
}
