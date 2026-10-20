//! Control gallery (`docs/dev/12-modelo-appkit.md`, Paso 17): every
//! current control in every control size, with the Kai and AppKit themes,
//! light/dark and key/inactive window. Visual review and source of the
//! golden captures (Paso 18). `cargo run --example gallery`.

use crate::components::{
    BezelStyle, Button, SidebarGroup, SidebarItem, SwitchDensity, TextField, card, card_actions,
    card_content, card_header, checkbox_with_options, group_box, paragraph, segmented,
    segmented_with_options, select, select_with_options, sidebar, switch_with_options,
};
use crate::element::{ControlSize, ControlState, Element, Role, div, text};
use crate::icons::Icon;
use crate::layout::WindowActivity;
use crate::runtime::SimpleApp;
use crate::theme::{Appearance, Density, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GalleryTheme {
    Kai,
    AppKit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    All,
    Buttons,
    Fields,
    Toggles,
    Selection,
    Navigation,
    Boxes,
    Text,
    Colors,
}

impl Section {
    /// Sections with content, in display order (without `All`).
    pub const CONTENT: [Section; 8] = [
        Self::Buttons,
        Self::Fields,
        Self::Toggles,
        Self::Selection,
        Self::Navigation,
        Self::Boxes,
        Self::Text,
        Self::Colors,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::All => "Todo",
            Self::Buttons => "Buttons",
            Self::Fields => "Fields",
            Self::Toggles => "Toggles",
            Self::Selection => "Selection",
            Self::Navigation => "Navigation",
            Self::Boxes => "Boxes",
            Self::Text => "Text",
            Self::Colors => "Colors",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        [Self::All]
            .into_iter()
            .chain(Self::CONTENT)
            .find(|section| section.name().eq_ignore_ascii_case(value))
            .or_else(|| (value == "todo" || value == "all").then_some(Self::All))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gallery {
    pub theme: GalleryTheme,
    pub appearance: Appearance,
    pub force_inactive: bool,
    pub section: Section,
    pub text: String,
    pub switches: [bool; 2],
    /// Checkboxes of the Selection section: off, on and mixed rows.
    pub checks: [ControlState; 3],
    /// Selected segment of the Selection section.
    pub segment: usize,
    /// Choices of the Select and Select vacío rows.
    pub choices: [Option<usize>; 2],
    /// Selected row of the Navigation section's sidebar.
    pub nav: usize,
    pub last_action: Option<&'static str>,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            theme: GalleryTheme::AppKit,
            appearance: Appearance::Light,
            force_inactive: false,
            section: Section::All,
            text: "Texto".to_owned(),
            switches: [false, true],
            checks: [ControlState::Off, ControlState::On, ControlState::Mixed],
            segment: 0,
            choices: [Some(0), None],
            nav: 0,
            last_action: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    SetTheme(GalleryTheme),
    SetAppearance(Appearance),
    ToggleInactive,
    Show(Section),
    Edit(String),
    Switch(usize),
    Check(usize),
    Segment(usize),
    /// Row of the Select section and option index.
    Choose(usize, usize),
    /// Row of the Navigation sidebar.
    Navigate(usize),
    Clicked(&'static str),
}

/// Rows of the Buttons section: name and builder for one size.
const BUTTON_ROWS: [&str; 7] = [
    "Push",
    "Default",
    "Accent",
    "Bordered",
    "Borderless",
    "Destructive",
    "Disabled",
];

/// Rows of the Fields section.
const FIELD_ROWS: [&str; 5] = [
    "Placeholder",
    "Con valor",
    "Read-only",
    "Disabled",
    "Password",
];

/// Rows of the Toggles section.
const TOGGLE_ROWS: [&str; 3] = ["Apagado", "Encendido", "Disabled"];

/// Rows of the Selection section.
const SELECTION_ROWS: [&str; 9] = [
    "Checkbox off",
    "Checkbox on",
    "Checkbox mixto",
    "Checkbox disabled",
    "Segmented",
    "Segmented disabled",
    "Select",
    "Select vacío",
    "Select disabled",
];

const SEGMENTS: [&str; 3] = ["Día", "Semana", "Mes"];

const FRUITS: [&str; 4] = ["Manzana", "Pera", "Uva", "Frutilla"];

/// Rows of the Navigation sidebar, in order across its groups.
const NAV_ROWS: [&str; 5] = ["Inicio", "Etiquetas", "Campos", "Botones", "Usuarios"];

fn nav_groups() -> [SidebarGroup; 2] {
    [
        SidebarGroup::titled(
            "Demo",
            [
                SidebarItem::new(NAV_ROWS[0]).icon(Icon::House),
                SidebarItem::new(NAV_ROWS[1]).icon(Icon::Tag).badge("12"),
            ],
        ),
        SidebarGroup::titled(
            "Componentes",
            [
                SidebarItem::new(NAV_ROWS[2]).icon(Icon::TextCursorInput),
                SidebarItem::new(NAV_ROWS[3]).icon(Icon::MousePointerClick),
                SidebarItem::new(NAV_ROWS[4]).icon(Icon::User),
            ],
        ),
    ]
}

/// Text styles of the Text section (`docs/dev/11-tema-appkit.md`).
const TEXT_STYLES: [&str; 11] = [
    "text-large-title",
    "text-title1",
    "text-title2",
    "text-title3",
    "text-headline",
    "text-body",
    "text-callout",
    "text-subheadline",
    "text-footnote",
    "text-caption1",
    "text-caption2",
];

/// Semantic colors of the Colors section (`Theme::color` names).
const COLORS: [&str; 28] = [
    "label",
    "secondary-label",
    "tertiary-label",
    "quaternary-label",
    "placeholder",
    "control-text",
    "disabled-control-text",
    "on-accent",
    "link",
    "window",
    "under-page",
    "control-background",
    "field",
    "control",
    "control-pressed",
    "control-accent",
    "accent-pressed",
    "accent-disabled",
    "selection",
    "selection-inactive",
    "text-selection",
    "focus",
    "separator",
    "grid",
    "fill",
    "fill-2",
    "fill-3",
    "bezel-hover",
];

fn size_name(size: ControlSize) -> &'static str {
    match size {
        ControlSize::Mini => "Mini",
        ControlSize::Small => "Small",
        ControlSize::Regular => "Regular",
        ControlSize::Large => "Large",
        ControlSize::ExtraLarge => "XL",
    }
}

impl Gallery {
    /// Model from `--theme kai|appkit`, `--appearance light|dark`,
    /// `--inactive` and `--section <name>`; unknown arguments are reported
    /// and ignored.
    pub fn from_args(args: impl Iterator<Item = String>) -> Self {
        let mut gallery = Self::default();
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--inactive" => gallery.force_inactive = true,
                "--theme" => match args.next().as_deref() {
                    Some("kai") => gallery.theme = GalleryTheme::Kai,
                    Some("appkit") => gallery.theme = GalleryTheme::AppKit,
                    other => eprintln!("--theme {other:?} desconocido (kai, appkit)"),
                },
                "--appearance" => match args.next().as_deref() {
                    Some("light") => gallery.appearance = Appearance::Light,
                    Some("dark") => gallery.appearance = Appearance::Dark,
                    other => eprintln!("--appearance {other:?} desconocido (light, dark)"),
                },
                "--section" => match args.next().as_deref().and_then(Section::parse) {
                    Some(section) => gallery.section = section,
                    None => eprintln!(
                        "--section desconocida (todo, buttons, fields, toggles, selection, \
                         navigation, boxes, text, colors)"
                    ),
                },
                other => eprintln!("argumento desconocido: {other}"),
            }
        }
        gallery
    }

    /// Theme for the current model. Kai has no dark appearance.
    pub fn current_theme(&self) -> Theme {
        match self.theme {
            GalleryTheme::Kai => Theme::kai(Density::Comfortable),
            GalleryTheme::AppKit => Theme::appkit_current(self.appearance),
        }
    }

    /// One section without the header, for tests and golden captures.
    pub fn section_view(&self, section: Section) -> Element<Msg> {
        match section {
            Section::All => div()
                .class("flex flex-col gap-8")
                .children(Section::CONTENT.map(|section| self.titled(section))),
            Section::Buttons => sized_grid(section, &BUTTON_ROWS, button_cell),
            Section::Fields => {
                sized_grid(section, &FIELD_ROWS, |row, size| self.field_cell(row, size))
            }
            Section::Toggles => sized_grid(section, &TOGGLE_ROWS, |row, size| {
                self.toggle_cell(row, size)
            }),
            Section::Selection => sized_grid(section, &SELECTION_ROWS, |row, size| {
                self.selection_cell(row, size)
            }),
            Section::Navigation => self.navigation(),
            Section::Boxes => boxes(),
            Section::Text => text_styles(),
            Section::Colors => self.colors(),
        }
    }

    /// A window-like box: the sidebar and a pane naming the selected row.
    fn navigation(&self) -> Element<Msg> {
        div()
            .class(
                "flex h-[300px] max-w-2xl rounded-lg border border-border overflow-hidden \
                 bg-background appkit:border-separator appkit:bg-window",
            )
            .children([
                sidebar(&nav_groups(), self.nav, Msg::Navigate)
                    .class(
                        "w-56 shrink-0 border-r border-border appkit:w-[200px] appkit:bg-fill-4 \
                         appkit:border-separator",
                    )
                    .label("Secciones")
                    .test_id("gallery.navigation.sidebar"),
                div().class("flex-1 p-6").child(
                    text(NAV_ROWS[self.nav])
                        .class("text-lg font-semibold appkit:text-title3")
                        .role(Role::Heading),
                ),
            ])
    }

    fn titled(&self, section: Section) -> Element<Msg> {
        div().class("flex flex-col gap-3").children([
            text(section.name())
                .class("text-lg font-semibold appkit:text-title3")
                .role(Role::Heading),
            self.section_view(section),
        ])
    }

    fn field_cell(&self, row: &str, size: ControlSize) -> Element<Msg> {
        let field = match row {
            "Placeholder" => TextField::new("Campo", "")
                .placeholder("Placeholder")
                .on_input(Msg::Edit),
            "Con valor" => TextField::new("Campo", self.text.clone()).on_input(Msg::Edit),
            "Read-only" => TextField::new("Campo", "Solo lectura").read_only(true),
            "Disabled" => TextField::new("Campo", "Deshabilitado").disabled(true),
            _ => TextField::new("Clave", "secreto").password(false, Msg::Clicked("password")),
        };
        field.build().control_size(size)
    }

    fn toggle_cell(&self, row: &str, size: ControlSize) -> Element<Msg> {
        let density = if matches!(size, ControlSize::Mini | ControlSize::Small) {
            SwitchDensity::Compact
        } else {
            SwitchDensity::Default
        };
        let switch = match row {
            "Apagado" => switch_with_options(self.switches[0], Msg::Switch(0), false, density),
            "Encendido" => switch_with_options(self.switches[1], Msg::Switch(1), false, density),
            _ => switch_with_options(false, Msg::Clicked("disabled"), true, density),
        };
        div().control_size(size).child(switch)
    }

    fn selection_cell(&self, row: &str, size: ControlSize) -> Element<Msg> {
        let control = match row {
            "Checkbox off" => checkbox_with_options("Opción", self.checks[0], Msg::Check(0), false),
            "Checkbox on" => checkbox_with_options("Opción", self.checks[1], Msg::Check(1), false),
            "Checkbox mixto" => {
                checkbox_with_options("Opción", self.checks[2], Msg::Check(2), false)
            }
            "Segmented" => segmented(&SEGMENTS, self.segment, Msg::Segment).label("Periodo"),
            "Segmented disabled" => {
                segmented_with_options(&SEGMENTS, 1, |_| Msg::Clicked("segmented"), true)
                    .label("Periodo")
            }
            "Select" => select(&FRUITS, self.choices[0], "Elegir", |index| {
                Msg::Choose(0, index)
            }),
            "Select vacío" => select(&FRUITS, self.choices[1], "Elegir", |index| {
                Msg::Choose(1, index)
            }),
            "Select disabled" => {
                select_with_options(&FRUITS, Some(1), "Elegir", |_| Msg::Clicked("select"), true)
            }
            _ => checkbox_with_options("Opción", ControlState::On, Msg::Clicked("checkbox"), true),
        };
        control.control_size(size)
    }

    fn colors(&self) -> Element<Msg> {
        let theme = self.current_theme();
        div()
            .class("grid grid-cols-4 gap-3")
            .children(COLORS.map(|name| {
                let hex = theme.color(name).map_or_else(
                    || "?".to_owned(),
                    |c| format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a),
                );
                div().class("flex items-center gap-2").children([
                    div().class(&format!(
                        "size-6 rounded-md border border-separator bg-{name}"
                    )),
                    div().class("flex flex-col").children([
                        text(name).class("text-sm"),
                        text(hex).class("text-xs text-secondary-label"),
                    ]),
                ])
            }))
    }

    fn header(&self) -> Element<Msg> {
        let toggle = |label: &'static str, on: bool, msg: Msg| {
            Button::new(label)
                .bezel(BezelStyle::Bordered)
                .accent(on)
                .control_size(ControlSize::Small)
                .on_click(msg)
                .build()
        };
        let mut sections = vec![toggle(
            Section::All.name(),
            self.section == Section::All,
            Msg::Show(Section::All),
        )];
        sections
            .extend(Section::CONTENT.map(|section| {
                toggle(section.name(), self.section == section, Msg::Show(section))
            }));
        div()
            .class(
                "flex flex-wrap items-center gap-2 px-6 py-3 border-b border-separator \
                 bg-background appkit:bg-window z-10",
            )
            .role(Role::Header)
            .children([
                toggle(
                    "Kai",
                    self.theme == GalleryTheme::Kai,
                    Msg::SetTheme(GalleryTheme::Kai),
                ),
                toggle(
                    "AppKit",
                    self.theme == GalleryTheme::AppKit,
                    Msg::SetTheme(GalleryTheme::AppKit),
                ),
                toggle(
                    "Claro",
                    self.appearance == Appearance::Light,
                    Msg::SetAppearance(Appearance::Light),
                ),
                toggle(
                    "Oscuro",
                    self.appearance == Appearance::Dark,
                    Msg::SetAppearance(Appearance::Dark),
                ),
                toggle("Simular inactiva", self.force_inactive, Msg::ToggleInactive),
                div().class("w-px h-5 bg-separator"),
            ])
            .children(sections)
            .child(
                text(
                    self.last_action
                        .map_or(String::new(), |a| format!("Última acción: {a}")),
                )
                .class("text-sm text-secondary-label"),
            )
    }
}

/// `gallery.<section>.<row>.<size>`, e.g. `gallery.fields.con-valor.small`.
pub fn cell_test_id(section: Section, row: &str, size: ControlSize) -> String {
    format!(
        "gallery.{}.{}.{}",
        slug(section.name()),
        slug(row),
        slug(size_name(size))
    )
}

fn slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Gives `test_id` to the first control of the subtree (document order).
fn tag_first_control<Msg>(element: &mut Element<Msg>, test_id: &str) -> bool {
    if element.is_control() {
        element.test_id = Some(test_id.to_owned());
        return true;
    }
    element
        .children
        .iter_mut()
        .any(|child| tag_first_control(child, test_id))
}

/// A row per entry of `rows`, a column per control size with its name on
/// top. Cells are built by `cell(row, size)`; their first control gets
/// [`cell_test_id`].
fn sized_grid<Msg>(
    section: Section,
    rows: &[&'static str],
    cell: impl Fn(&'static str, ControlSize) -> Element<Msg>,
) -> Element<Msg> {
    let column = "w-44 flex items-center";
    let header = div()
        .class("flex items-center gap-4")
        .child(div().class("w-28"))
        .children(ControlSize::ALL.map(|size| {
            text(size_name(size)).class(&format!("{column} text-xs text-secondary-label"))
        }));
    div()
        .class("flex flex-col gap-3")
        .child(header)
        .children(rows.iter().map(|&row| {
            div()
                .class("flex items-center gap-4")
                .role(Role::Group)
                .label(row)
                .child(text(row).class("w-28 text-sm text-secondary-label"))
                .children(ControlSize::ALL.map(|size| {
                    let mut control = cell(row, size);
                    tag_first_control(&mut control, &cell_test_id(section, row, size));
                    div().class(column).child(control)
                }))
        }))
}

fn button_cell(row: &'static str, size: ControlSize) -> Element<Msg> {
    let button = Button::new("Button").control_size(size);
    let button = match row {
        // Only one default button per window: Return activates Regular.
        "Default" => button
            .default_button(size == ControlSize::Regular)
            .accent(size != ControlSize::Regular),
        "Accent" => button.accent(true),
        "Bordered" => button.bezel(BezelStyle::Bordered),
        "Borderless" => button.bezel(BezelStyle::Borderless),
        "Destructive" => button.destructive(true),
        "Disabled" => button.disabled(true),
        _ => button,
    };
    button.on_click(Msg::Clicked(row)).build()
}

fn boxes() -> Element<Msg> {
    div().class("flex items-start gap-6").children([
        card().class("w-80").children([
            card_header("Card", Some("Header, contenido y acciones")),
            card_content().child(paragraph("Contenido de la tarjeta.")),
            card_actions().child(
                Button::new("Aceptar")
                    .on_click(Msg::Clicked("card"))
                    .build(),
            ),
        ]),
        group_box(Some("Título"), [paragraph("Contenido del group box.")]).class("w-80"),
    ])
}

fn text_styles() -> Element<Msg> {
    div()
        .class("flex flex-col gap-2")
        .children(TEXT_STYLES.map(|style| {
            div().class("flex items-baseline gap-4").children([
                text(style).class("w-40 text-xs text-secondary-label"),
                text("Kai UI 0123456789").class(style),
            ])
        }))
}

impl SimpleApp for Gallery {
    type Msg = Msg;

    fn title(&self) -> String {
        "Kai UI — Galería".to_owned()
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::SetTheme(theme) => self.theme = theme,
            Msg::SetAppearance(appearance) => self.appearance = appearance,
            Msg::ToggleInactive => self.force_inactive = !self.force_inactive,
            Msg::Show(section) => self.section = section,
            Msg::Edit(value) => self.text = value,
            Msg::Switch(index) => self.switches[index] = !self.switches[index],
            // A mixed checkbox turns on, as `NSButton` without
            // `allowsMixedState`.
            Msg::Check(index) => {
                self.checks[index] = match self.checks[index] {
                    ControlState::On => ControlState::Off,
                    ControlState::Off | ControlState::Mixed => ControlState::On,
                }
            }
            Msg::Segment(index) => self.segment = index,
            Msg::Choose(row, index) => self.choices[row] = Some(index),
            Msg::Navigate(index) => self.nav = index,
            Msg::Clicked(name) => self.last_action = Some(name),
        }
    }

    fn view(&self) -> Element<Msg> {
        div()
            .class("w-full h-full flex flex-col bg-neutral appkit:bg-window")
            .children([
                self.header(),
                div()
                    .class("flex-1 overflow-y-auto")
                    .role(Role::Main)
                    .child(div().class("p-6").child(self.section_view(self.section))),
            ])
    }

    fn theme(&self) -> Option<Theme> {
        Some(self.current_theme())
    }

    fn window_override(&self) -> Option<WindowActivity> {
        self.force_inactive.then_some(WindowActivity::INACTIVE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::FocusOrder;
    use crate::geometry::Size;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::runtime::build_view;
    use crate::testing::{By, TestApp};
    use crate::theme::{AppKitGeneration, Appearance};

    fn themes() -> [Theme; 4] {
        [
            Theme::kai(Density::Comfortable),
            Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light),
            Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Dark),
            Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark),
        ]
    }

    #[test]
    fn uses_only_supported_classes() {
        for theme in themes() {
            let mut styles = StyleCache::default();
            let gallery = Gallery::default();
            Layout::compute(
                &build_view(&gallery),
                &LayoutInput {
                    viewport: Size {
                        width: 1280.0,
                        height: 800.0,
                    },
                    theme: &theme,
                    interaction: &Interaction::default(),
                    scroll: &ScrollOffsets::new(),
                },
                &mut styles,
                &mut FixedMeasure,
            );
            assert!(
                styles.unknown().is_empty(),
                "{:?} {:?}: {:?}",
                theme.kind,
                theme.appearance,
                styles.unknown()
            );
        }
    }

    #[test]
    fn every_size_is_shown() {
        let gallery = Gallery::default();
        for (section, rows) in [
            (Section::Buttons, BUTTON_ROWS.len()),
            (Section::Fields, FIELD_ROWS.len()),
            (Section::Toggles, TOGGLE_ROWS.len()),
            (Section::Selection, SELECTION_ROWS.len()),
        ] {
            let mut tree = gallery.section_view(section);
            tree.assign_ids();
            let mut groups = Vec::new();
            tree.walk(&mut |element| {
                if element.role == Some(Role::Group) {
                    // Label plus one cell per size.
                    groups.push(element.children.len());
                }
            });
            assert_eq!(
                groups,
                vec![1 + ControlSize::ALL.len(); rows],
                "{section:?}"
            );
        }
    }

    #[test]
    fn every_cell_has_a_unique_test_id_on_its_control() {
        let gallery = Gallery::default();
        let tree = build_view(&gallery);
        assert_eq!(tree.validate_ids(), Vec::<String>::new());
        let mut tagged = Vec::new();
        tree.walk(&mut |element| {
            // Parts (`<cell>.label`…) derive from the cell's id; the
            // sidebar's rows are its controls.
            if let Some(test_id) = &element.test_id
                && (element.part.is_none() || element.is_control())
                && element.role != Some(Role::Sidebar)
            {
                assert!(element.is_control(), "{test_id} is not a control");
                tagged.push(test_id.clone());
            }
        });
        for (section, rows) in [
            (Section::Buttons, &BUTTON_ROWS[..]),
            (Section::Fields, &FIELD_ROWS[..]),
            (Section::Toggles, &TOGGLE_ROWS[..]),
            (Section::Selection, &SELECTION_ROWS[..]),
        ] {
            for row in rows {
                for size in ControlSize::ALL {
                    let id = cell_test_id(section, row, size);
                    assert!(tagged.contains(&id), "{id} missing");
                }
            }
        }
        assert!(tagged.contains(&"gallery.fields.con-valor.xl".to_owned()));
        assert!(tagged.contains(&"gallery.navigation.sidebar.4".to_owned()));
        assert_eq!(
            tagged.len(),
            ControlSize::ALL.len()
                * (BUTTON_ROWS.len() + FIELD_ROWS.len() + TOGGLE_ROWS.len() + SELECTION_ROWS.len())
                + NAV_ROWS.len()
        );
    }

    #[test]
    fn focus_order_covers_all_enabled_controls() {
        let gallery = Gallery::default();
        let sizes = ControlSize::ALL.len();
        let disabled_buttons = 1;
        // Placeholder, value, read-only and password fields, plus the
        // password toggle.
        let field_controls = 5;
        let enabled_switches = 2;
        // Three checkboxes, the three segments of one control and two
        // selects.
        let enabled_selection = 3 + SEGMENTS.len() + 2;
        for (section, expected) in [
            (
                Section::Buttons,
                sizes * (BUTTON_ROWS.len() - disabled_buttons),
            ),
            (Section::Fields, sizes * field_controls),
            (Section::Toggles, sizes * enabled_switches),
            (Section::Selection, sizes * enabled_selection),
            (Section::Navigation, NAV_ROWS.len()),
            (Section::Boxes, 1),
        ] {
            let mut tree = gallery.section_view(section);
            tree.assign_ids();
            let order = FocusOrder::from_tree(&tree);
            assert_eq!(order.ids().count(), expected, "{section:?}");
        }
    }

    #[test]
    fn return_activates_the_single_default_button() {
        let mut tree = Gallery::default().section_view(Section::Buttons);
        tree.assign_ids();
        let mut defaults = Vec::new();
        tree.walk(&mut |element| {
            if element.control.is_default {
                defaults.push(element.control.size);
            }
        });
        assert_eq!(defaults, vec![Some(ControlSize::Regular)]);
    }

    #[test]
    fn args_pick_theme_appearance_window_and_section() {
        let args = [
            "--theme",
            "kai",
            "--appearance",
            "dark",
            "--inactive",
            "--section",
            "fields",
        ];
        let gallery = Gallery::from_args(args.into_iter().map(str::to_owned));
        assert_eq!(gallery.theme, GalleryTheme::Kai);
        assert_eq!(gallery.appearance, Appearance::Dark);
        assert_eq!(gallery.section, Section::Fields);
        assert_eq!(gallery.window_override(), Some(WindowActivity::INACTIVE));
        assert_eq!(Gallery::default().window_override(), None);
    }

    /// Every section with Kai, AppKit light/dark and AppKit inactive, as
    /// PNG under `target/snapshots/gallery/` for visual review (golden
    /// captures come in Paso 18). `cargo test -- --ignored snapshot_gallery`
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshot_gallery() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots/gallery");
        std::fs::create_dir_all(&dir).unwrap();
        let cases = [
            (
                "kai",
                Theme::kai(Density::Comfortable),
                WindowActivity::ACTIVE,
            ),
            (
                "appkit",
                Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light),
                WindowActivity::ACTIVE,
            ),
            (
                "appkit-dark",
                Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Dark),
                WindowActivity::ACTIVE,
            ),
            (
                "appkit-inactive",
                Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light),
                WindowActivity::INACTIVE,
            ),
        ];
        let gallery = Gallery::default();
        for (name, theme, window) in cases {
            for section in Section::CONTENT {
                let mut tree: Element<Msg> = div()
                    .class("w-full h-full p-6 bg-neutral appkit:bg-window")
                    .child(gallery.section_view(section));
                tree.assign_ids();
                let image = crate::snapshot::render(&crate::snapshot::Shot {
                    tree: &tree,
                    theme: &theme,
                    logical: Size {
                        width: 1180.0,
                        height: 520.0,
                    },
                    scale: 2.0,
                    interaction: &Interaction {
                        window,
                        ..Interaction::default()
                    },
                    scroll: &ScrollOffsets::new(),
                })
                .expect("render");
                let file = format!("{}-{name}.png", section.name().to_lowercase());
                crate::snapshot::write_png(&dir.join(file), &image).unwrap();
            }
        }
    }

    #[test]
    fn checkboxes_toggle_by_click_on_the_title_and_by_space() {
        use crate::ui_engine::{Key, Modifiers};

        let mut app = TestApp::new(Gallery {
            section: Section::Selection,
            ..Gallery::default()
        });
        let regular =
            |row| By::test_id(cell_test_id(Section::Selection, row, ControlSize::Regular));
        app.click(By::test_id(format!(
            "{}.label",
            cell_test_id(Section::Selection, "Checkbox off", ControlSize::Regular)
        )));
        assert_eq!(app.app().checks[0], ControlState::On);
        assert!(app.is_focused(regular("Checkbox off")));
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(app.app().checks[0], ControlState::Off);
        app.click(regular("Checkbox mixto"));
        assert_eq!(app.app().checks[2], ControlState::On, "mixed turns on");
        app.click(regular("Checkbox disabled"));
        assert_eq!(app.app().last_action, None);
    }

    #[test]
    fn segmented_is_one_tab_stop_whose_arrows_move_focus_and_space_selects() {
        use crate::ui_engine::{Key, Modifiers};

        let mut app = TestApp::new(Gallery {
            section: Section::Selection,
            segment: 1,
            ..Gallery::default()
        });
        let id = |row, size| By::test_id(cell_test_id(Section::Selection, row, size));
        let segment = |name: &str| By::role(Role::Radio).name(name);
        // From the last enabled checkbox (Mixto, XL) Tab enters the Mini
        // segmented control at its selected segment.
        let nth =
            |app: &TestApp<Gallery>, name: &str, n: usize| app.screen().all(segment(name))[n].id();
        let focused_name = |app: &TestApp<Gallery>| {
            let id = app.session().focused().unwrap();
            app.screen().by_id(id).unwrap().name()
        };
        app.click(id("Checkbox mixto", ControlSize::ExtraLarge));
        app.press(Key::Tab, Modifiers::NONE);
        let mini_week = nth(&app, "Semana", 0);
        assert_eq!(app.session().focused(), Some(mini_week));

        app.press(Key::ArrowRight, Modifiers::NONE);
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(app.app().segment, 1, "arrows only move the focus");
        assert_eq!(
            focused_name(&app).as_deref(),
            Some("Mes"),
            "it stops at the end"
        );
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(app.app().segment, 2);
        app.press(Key::ArrowLeft, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(app.app().segment, 1);

        // Tab leaves the group: the next stop is the Small control's
        // selected segment.
        app.press(Key::Tab, Modifiers::NONE);
        assert_eq!(app.session().focused(), Some(nth(&app, "Semana", 1)));
        let shift = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, shift);
        assert_eq!(app.session().focused(), Some(mini_week));
    }

    fn selection_app() -> TestApp<Gallery> {
        TestApp::new(Gallery {
            section: Section::Selection,
            ..Gallery::default()
        })
    }

    fn select_cell(row: &str) -> By {
        By::test_id(cell_test_id(Section::Selection, row, ControlSize::Regular))
    }

    #[test]
    fn select_opens_its_list_under_it_and_a_click_picks() {
        let mut app = selection_app();
        app.click(select_cell("Select vacío"));
        assert!(app.session().popup_open());
        let screen = app.screen();
        let trigger = screen.get(select_cell("Select vacío"));
        let panel = screen.get(By::role(Role::Menu));
        assert_eq!(panel.test_id(), Some("kai.menu"));
        assert!(panel.bounds().y >= trigger.bounds().y + trigger.bounds().height);
        assert_eq!(panel.bounds().x, trigger.bounds().x);
        assert_eq!(screen.all(By::role(Role::MenuItem)).len(), FRUITS.len());

        app.click(By::test_id("kai.menu.2"));
        assert!(!app.session().popup_open());
        assert_eq!(app.app().choices[1], Some(2));
        assert_eq!(app.messages().last(), Some(&Msg::Choose(1, 2)));
        assert!(app.is_focused(select_cell("Select vacío")));
        assert_eq!(
            app.screen()
                .get(select_cell("Select vacío"))
                .name()
                .as_deref(),
            Some("Uva")
        );
    }

    #[test]
    fn select_list_follows_the_keyboard() {
        use crate::ui_engine::{Key, Modifiers};

        let mut app = selection_app();
        let none = Modifiers::NONE;
        app.click(select_cell("Select"));
        app.press(Key::Escape, none);
        assert!(!app.session().popup_open());
        assert!(app.is_focused(select_cell("Select")));
        assert!(app.messages().is_empty(), "Escape picks nothing");

        // ↓ opens at the current choice (Manzana) and moves through it.
        app.press(Key::ArrowDown, none);
        assert!(app.session().popup_open());
        assert!(app.is_focused(By::test_id("kai.menu.0")));
        app.press(Key::ArrowDown, none);
        assert!(app.is_focused(By::test_id("kai.menu.1")));
        app.press(Key::Char('x'), none);
        assert!(app.session().popup_open(), "other keys do nothing");
        app.press(Key::Enter, none);
        assert_eq!(app.app().choices[0], Some(1));
        assert!(app.is_focused(select_cell("Select")));

        app.press(Key::Space, none);
        assert!(app.is_focused(By::test_id("kai.menu.1")), "opens at Pera");
        app.press(Key::End, none);
        app.press(Key::Space, none);
        assert_eq!(app.app().choices[0], Some(FRUITS.len() - 1));

        app.press(Key::Enter, none);
        assert!(app.session().popup_open());
        app.press(Key::Tab, none);
        assert!(!app.session().popup_open());
        assert!(app.is_focused(select_cell("Select")));
    }

    #[test]
    fn a_click_outside_the_list_only_closes_it() {
        let mut app = selection_app();
        app.click(select_cell("Select"));
        let checkbox = By::test_id(cell_test_id(
            Section::Selection,
            "Checkbox off",
            ControlSize::Regular,
        ));
        let screen = app.screen();
        let center = {
            let bounds = screen.get(checkbox.clone()).bounds();
            crate::geometry::Point {
                x: bounds.x + bounds.width / 2.0,
                y: bounds.y + bounds.height / 2.0,
            }
        };
        app.click_at(center);
        assert!(!app.session().popup_open());
        assert_eq!(app.app().checks[0], ControlState::Off, "the click is spent");
        assert!(app.messages().is_empty());

        // Clicking the button again closes the list too.
        app.click(select_cell("Select"));
        assert!(app.session().popup_open());
        app.click_at(center);
        app.click(select_cell("Select disabled"));
        assert!(
            !app.session().popup_open(),
            "a disabled select does not open"
        );
    }

    #[test]
    fn select_list_is_an_expanded_pop_up_with_options_for_assistive_technology() {
        let mut app = selection_app();
        let trigger = app.screen().get(select_cell("Select")).id();
        let tree = app.session().a11y_tree(1.0);
        let node = |tree: &accesskit::TreeUpdate, id: crate::element::WidgetId| {
            tree.nodes
                .iter()
                .find(|(node, _)| *node == crate::a11y::node_id(id))
                .map(|(_, node)| node.clone())
                .unwrap()
        };
        let closed = node(&tree, trigger);
        assert_eq!(closed.role(), accesskit::Role::ComboBox);
        assert_eq!(closed.is_expanded(), Some(false));
        assert_eq!(closed.value(), Some("Manzana"));

        app.accessibility_action(&accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: crate::a11y::node_id(trigger),
            data: None,
        });
        assert!(app.session().popup_open());
        let tree = app.session().a11y_tree(1.0);
        assert_eq!(node(&tree, trigger).is_expanded(), Some(true));
        let screen = app.screen();
        let first = node(&tree, screen.get(By::test_id("kai.menu.0")).id());
        assert_eq!(first.role(), accesskit::Role::MenuListOption);
        assert_eq!(first.label(), Some("Manzana"));
        assert_eq!(first.is_selected(), Some(true));
        let second = node(&tree, screen.get(By::test_id("kai.menu.1")).id());
        assert_eq!(second.is_selected(), Some(false));
        assert_eq!(
            node(&tree, screen.get(By::role(Role::Menu)).id()).role(),
            accesskit::Role::MenuListPopup
        );
    }

    #[test]
    fn opening_the_palette_closes_the_list() {
        let mut app = selection_app();
        app.click(select_cell("Select"));
        assert!(app.perform(crate::commands::PALETTE));
        assert!(!app.session().popup_open());
        assert!(app.session().palette_open());
    }

    fn nav_row(index: usize) -> By {
        By::test_id(format!("gallery.navigation.sidebar.{index}"))
    }

    #[test]
    fn sidebar_rows_are_chosen_by_click_and_by_up_and_down() {
        use crate::ui_engine::{Key, Modifiers};

        let none = Modifiers::NONE;
        let mut app = TestApp::new(Gallery {
            section: Section::Navigation,
            nav: 1,
            ..Gallery::default()
        });
        app.click(nav_row(2));
        assert_eq!(app.app().nav, 2);
        app.screen().get(By::role(Role::Heading).name("Campos"));
        assert!(app.is_focused(nav_row(2)));

        // From the header, Tab enters at the selected row.
        app.click(By::role(Role::Button).name("Navigation"));
        let rows: Vec<_> = (0..NAV_ROWS.len())
            .map(|index| app.screen().get(nav_row(index)).id())
            .collect();
        for _ in 0..20 {
            app.press(Key::Tab, none);
            if rows.contains(&app.session().focused().unwrap()) {
                break;
            }
        }
        assert!(app.is_focused(nav_row(2)));
        app.press(Key::ArrowDown, none);
        assert!(app.is_focused(nav_row(3)));
        assert_eq!(app.app().nav, 3);
        app.press(Key::ArrowUp, none);
        app.press(Key::ArrowUp, none);
        app.press(Key::ArrowUp, none);
        assert_eq!(app.app().nav, 0);
        app.press(Key::ArrowUp, none);
        assert_eq!(
            app.messages()
                .iter()
                .filter(|msg| matches!(msg, Msg::Navigate(_)))
                .count(),
            5,
            "the first row is not chosen again"
        );
        app.press(Key::ArrowRight, none);
        assert_eq!(app.app().nav, 0, "← and → do nothing");

        app.press(Key::Tab, none);
        let focused = app.session().focused().unwrap();
        assert!(!rows.contains(&focused), "Tab leaves the sidebar");
    }

    #[test]
    fn the_focused_row_scrolls_into_view_in_a_short_window() {
        use crate::ui_engine::{Key, Modifiers};

        let mut app = TestApp::builder(Gallery {
            section: Section::Navigation,
            ..Gallery::default()
        })
        .viewport(800.0, 200.0)
        .build();
        app.click(nav_row(0));
        for _ in 1..NAV_ROWS.len() {
            app.press(Key::ArrowDown, Modifiers::NONE);
        }
        assert_eq!(app.app().nav, NAV_ROWS.len() - 1);
        let row = app.screen().get(nav_row(NAV_ROWS.len() - 1)).bounds();
        assert!(row.y >= 0.0 && row.y + row.height <= 200.0, "{row:?}");
    }

    #[test]
    fn theme_follows_the_model() {
        let mut gallery = Gallery::default();
        assert!(gallery.theme().unwrap().is_appkit());
        gallery.update(Msg::SetTheme(GalleryTheme::Kai));
        assert!(!gallery.theme().unwrap().is_appkit());
    }

    #[test]
    fn theme_buttons_switch_the_theme_in_use() {
        let mut app = TestApp::new(Gallery::default());
        assert!(app.theme().is_appkit());
        app.click(By::role(Role::Button).name("Kai"));
        assert_eq!(app.messages(), [Msg::SetTheme(GalleryTheme::Kai)]);
        assert!(!app.theme().is_appkit());
        app.click(By::role(Role::Button).name("Oscuro"));
        assert_eq!(
            app.theme().appearance,
            Appearance::Light,
            "Kai has a single appearance"
        );
        app.click(By::role(Role::Button).name("AppKit"));
        assert!(app.theme().is_appkit());
        assert_eq!(app.theme().appearance, Appearance::Dark);
    }
}
