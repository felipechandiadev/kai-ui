//! /catalog: the live catalog. Every control in one place, under the theme,
//! density and control size chosen at the top, which apply to the whole
//! window while the section is open.

use kai::components::{
    AlertVariant, BadgeVariant, Button, ButtonVariant, DotProgress, DotSize, EmptyState,
    FieldDensity, FieldKind, SwitchDensity, Tabs, TextField, alert, badge, button, caption, card,
    card_content, card_header, checkbox_with_options, page, page_body, page_grid, page_grid_item,
    paragraph, segmented_with_options, select_with_options, switch_labeled,
};
use kai::element::{ControlSize, ControlState, Element, div, text};
use kai::router::{Context, Page};
use kai::runtime::Effects;
use kai::theme::{AppKitGeneration, Appearance, Density, Theme};

use crate::state::State;

/// Theme preset of the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CatalogPreset {
    AppKitLight,
    AppKitDark,
    GlassLight,
    GlassDark,
    Kai,
}

impl CatalogPreset {
    const ALL: [CatalogPreset; 5] = [
        Self::AppKitLight,
        Self::AppKitDark,
        Self::GlassLight,
        Self::GlassDark,
        Self::Kai,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::AppKitLight => "AppKit",
            Self::AppKitDark => "AppKit Dark",
            Self::GlassLight => "Glass",
            Self::GlassDark => "Glass Dark",
            Self::Kai => "Kai",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }
}

/// Sizes, in the order of their segmented control.
const CATALOG_SIZES: [ControlSize; 5] = [
    ControlSize::Mini,
    ControlSize::Small,
    ControlSize::Regular,
    ControlSize::Large,
    ControlSize::ExtraLarge,
];

pub struct Screen {
    preset: CatalogPreset,
    density: Density,
    size: ControlSize,
    sample: String,
    check: bool,
    switch: bool,
    select: Option<usize>,
    tab: usize,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Preset(usize),
    Density(usize),
    Size(usize),
    Sample(String),
    ClearFilter,
    ToggleCheck,
    ToggleSwitch,
    Select(usize),
    Tab(usize),
    /// A button that only shows itself.
    Pressed,
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        let screen = Self {
            preset: CatalogPreset::GlassLight,
            density: Density::Comfortable,
            size: ControlSize::Large,
            sample: String::new(),
            check: true,
            switch: true,
            select: Some(0),
            tab: 0,
        };
        (screen, Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Preset(index) => {
                self.preset = CatalogPreset::ALL
                    .get(index)
                    .copied()
                    .unwrap_or(CatalogPreset::AppKitLight);
            }
            Msg::Density(index) => {
                self.density = if index == 0 {
                    Density::Comfortable
                } else {
                    Density::Compact
                };
            }
            Msg::Size(index) => {
                self.size = CATALOG_SIZES
                    .get(index)
                    .copied()
                    .unwrap_or(ControlSize::Regular);
            }
            Msg::Sample(value) => self.sample = value,
            Msg::ClearFilter => self.sample.clear(),
            Msg::ToggleCheck => self.check = !self.check,
            Msg::ToggleSwitch => self.switch = !self.switch,
            Msg::Select(index) => self.select = Some(index),
            Msg::Tab(index) => self.tab = index,
            Msg::Pressed => {}
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().child(self.catalog()))
    }

    fn title(&self, _: &State) -> String {
        "Catálogo vivo".into()
    }

    fn theme(&self, _: &State) -> Option<Theme> {
        let base = match self.preset {
            CatalogPreset::AppKitLight => {
                Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light)
            }
            CatalogPreset::AppKitDark => {
                Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Dark)
            }
            CatalogPreset::GlassLight => {
                Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light)
            }
            CatalogPreset::GlassDark => {
                Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark)
            }
            CatalogPreset::Kai => Theme::kai(self.density),
        };
        // Density moves apart from size, to explore them.
        Some(Theme {
            control_size: self.size,
            density: self.density,
            ..base
        })
    }
}

impl Screen {
    fn catalog(&self) -> Element<Msg> {
        let size = self.size;
        let field_density = match self.density {
            Density::Comfortable => FieldDensity::Default,
            Density::Compact => FieldDensity::Compact,
        };
        let preset_labels = [
            CatalogPreset::AppKitLight.label(),
            CatalogPreset::AppKitDark.label(),
            CatalogPreset::GlassLight.label(),
            CatalogPreset::GlassDark.label(),
            CatalogPreset::Kai.label(),
        ];
        let density_idx = match self.density {
            Density::Comfortable => 0,
            Density::Compact => 1,
        };
        let size_idx = size.index();
        let filters = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.filters")
            .children([
                card_header(
                    "Filtros",
                    Some("Tema (App::theme mientras esta sección está abierta), densidad y ControlSize"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-col gap-1.5").children([
                        caption("Tema"),
                        segmented_with_options(
                            &preset_labels,
                            self.preset.index(),
                            Msg::Preset,
                            false,
                        )
                        .test_id("catalog.preset"),
                    ]),
                    div().class("flex flex-col gap-1.5").children([
                        caption("Densidad"),
                        segmented_with_options(
                            &["Comfortable", "Compact"],
                            density_idx,
                            Msg::Density,
                            false,
                        )
                        .test_id("catalog.density"),
                    ]),
                    div().class("flex flex-col gap-1.5").children([
                        caption("Tamaño (ControlSize)"),
                        segmented_with_options(
                            &["Mini", "Small", "Regular", "Large", "XL"],
                            size_idx,
                            Msg::Size,
                            false,
                        )
                        .test_id("catalog.size"),
                    ]),
                ]),
            ]);

        let buttons = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.buttons")
            .children([
                card_header(
                    "Button",
                    Some("Variantes + disabled + loading al tamaño filtrado"),
                ),
                card_content().class("gap-3").children([div()
                    .class("flex flex-wrap items-center gap-2")
                    .children([
                        button("Primary", ButtonVariant::Primary)
                            .control_size(size)
                            .on_click(Msg::Pressed),
                        button("Secondary", ButtonVariant::Secondary)
                            .control_size(size)
                            .on_click(Msg::Pressed),
                        button("Outlined", ButtonVariant::Outlined)
                            .control_size(size)
                            .on_click(Msg::Pressed),
                        button("Danger", ButtonVariant::Danger)
                            .control_size(size)
                            .on_click(Msg::Pressed),
                        button("Disabled", ButtonVariant::Primary)
                            .control_size(size)
                            .disabled(true),
                        Button::new("Loading")
                            .accent(true)
                            .loading(true)
                            .control_size(size)
                            .on_click(Msg::Pressed)
                            .build(),
                    ])]),
            ]);

        let fields = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.fields")
            .children([
                card_header(
                    "TextField",
                    Some(
                        "Default / required / helper / error / disabled · FieldDensity del filtro",
                    ),
                ),
                card_content()
                    .class("gap-4")
                    .children([page_grid().children([
                        page_grid_item(6).child(
                            TextField::new("Nombre", &self.sample)
                                .placeholder("Valor de ejemplo")
                                .field_icon(FieldKind::Name)
                                .density(field_density)
                                .on_input(Msg::Sample)
                                .test_id("catalog.field.default")
                                .build()
                                .control_size(size),
                        ),
                        page_grid_item(6).child(
                            TextField::new("Correo", "ana@example.com")
                                .field_icon(FieldKind::Email)
                                .density(field_density)
                                .required(true)
                                .test_id("catalog.field.required")
                                .build()
                                .control_size(size),
                        ),
                        page_grid_item(6).child(
                            TextField::new("Búsqueda", "")
                                .placeholder("Escribe para filtrar")
                                .field_icon(FieldKind::Search)
                                .helper_text("Puedes usar palabras clave")
                                .density(field_density)
                                .test_id("catalog.field.helper")
                                .build()
                                .control_size(size),
                        ),
                        page_grid_item(6).child(
                            TextField::new("Correo", "no-es-un-correo")
                                .field_icon(FieldKind::Email)
                                .helper_text("Introduce un correo válido")
                                .error(true)
                                .density(field_density)
                                .disabled(false)
                                .test_id("catalog.field.error")
                                .build()
                                .control_size(size),
                        ),
                    ])]),
            ]);

        let switch_density = match self.density {
            Density::Comfortable => SwitchDensity::Default,
            Density::Compact => SwitchDensity::Compact,
        };
        let check_state = if self.check {
            ControlState::On
        } else {
            ControlState::Off
        };
        let choices = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.choice")
            .children([
                card_header(
                    "Checkbox · Switch · Select",
                    Some("Estados útiles; checkbox/select respetan ControlSize del tema; switch usa SwitchDensity"),
                ),
                card_content().class("gap-4").children([
                    div().class("flex flex-col gap-2").children([
                        caption("Checkbox"),
                        div()
                            .class("flex flex-wrap items-center gap-4")
                            .children([
                                checkbox_with_options(
                                    "Activo",
                                    check_state,
                                    Msg::ToggleCheck,
                                    false,
                                )
                                .test_id("catalog.check"),
                                checkbox_with_options(
                                    "Mixto",
                                    ControlState::Mixed,
                                    Msg::Pressed,
                                    false,
                                )
                                .test_id("catalog.check.mixed"),
                                checkbox_with_options(
                                    "Deshabilitado",
                                    ControlState::On,
                                    Msg::Pressed,
                                    true,
                                )
                                .test_id("catalog.check.disabled"),
                            ]),
                    ]),
                    div().class("flex flex-col gap-2").children([
                        caption("Switch"),
                        div()
                            .class("flex flex-wrap items-center gap-4")
                            .children([
                                switch_labeled(
                                    "Notificaciones",
                                    self.switch,
                                    Msg::ToggleSwitch,
                                    false,
                                    switch_density,
                                )
                                .test_id("catalog.switch"),
                                switch_labeled(
                                    "Deshabilitado",
                                    true,
                                    Msg::Pressed,
                                    true,
                                    switch_density,
                                )
                                .test_id("catalog.switch.disabled"),
                            ]),
                    ]),
                    div().class("flex flex-col gap-2 max-w-xs").children([
                        caption("Select"),
                        select_with_options(
                            &["Diseño", "Desarrollo", "Producto"],
                            self.select,
                            "Elegir rol",
                            Msg::Select,
                            false,
                        )
                        .label("Rol")
                        .control_size(size)
                        .test_id("catalog.select"),
                        select_with_options(
                            &["Diseño", "Desarrollo", "Producto"],
                            Some(1),
                            "Elegir rol",
                            Msg::Select,
                            true,
                        )
                        .label("Deshabilitado")
                        .control_size(size)
                        .test_id("catalog.select.disabled"),
                    ]),
                ]),
            ]);

        let badges = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.badges")
            .children([
                card_header(
                    "Badge · Alert",
                    Some("Variantes sólidas y outlined (no pills WIP)"),
                ),
                card_content().class("gap-3").children([
                    div().class("flex flex-wrap items-center gap-2").children([
                        badge("Primary", BadgeVariant::Primary),
                        badge("Success", BadgeVariant::Success),
                        badge("Warning", BadgeVariant::Warning),
                        badge("Error", BadgeVariant::Error),
                        badge("Info", BadgeVariant::Info),
                        badge("Outlined", BadgeVariant::PrimaryOutlined),
                        badge("Success □", BadgeVariant::SuccessOutlined),
                    ]),
                    alert(AlertVariant::Info, "Alerta informativa de ejemplo"),
                    alert(AlertVariant::Warning, "Revisa los campos requeridos"),
                ]),
            ]);

        let progress_size = match size {
            ControlSize::Mini | ControlSize::Small => DotSize::Sm,
            ControlSize::Regular => DotSize::Md,
            ControlSize::Large | ControlSize::ExtraLarge => DotSize::Lg,
        };
        let feedback = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.feedback")
            .children([
                card_header(
                    "DotProgress · Tabs",
                    Some("Progreso indeterminado (tamaño ≈ ControlSize) y pestañas"),
                ),
                card_content().class("gap-4").children([
                    div()
                        .class(
                            "flex flex-wrap items-center gap-6 text-foreground appkit:text-label",
                        )
                        .children([
                            div().class("flex flex-col gap-1").children([
                                caption("DotProgress"),
                                DotProgress::new()
                                    .size(progress_size)
                                    .label("Cargando catálogo")
                                    .test_id("catalog.progress")
                                    .build(),
                            ]),
                            div().class("flex flex-col gap-1").children([
                                caption("Sm (referencia)"),
                                DotProgress::new().size(DotSize::Sm).decorative().build(),
                            ]),
                        ]),
                    Tabs::new(self.tab, Msg::Tab)
                        .tab(
                            "General",
                            paragraph("Contenido de la pestaña General bajo el tema filtrado."),
                        )
                        .tab("Avanzado", paragraph("Contenido de la pestaña Avanzado."))
                        .build()
                        .test_id("catalog.tabs"),
                ]),
            ]);

        let empties = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.empty")
            .children([
                card_header(
                    "Empty filter vs empty resource",
                    Some("EmptyKind::Filter (búsqueda sin filas) · EmptyKind::Resource (colección vacía)"),
                ),
                card_content().class("gap-4").children([
                    page_grid().children([
                        page_grid_item(6).child(
                            div()
                                .class(
                                    "rounded-md border border-border appkit:border-separator \
                                     overflow-hidden bg-control",
                                )
                                .child(
                                    EmptyState::filter("No hay resultados")
                                        .description("Ningún ítem coincide con el filtro.")
                                        .action("Limpiar filtros", Msg::ClearFilter)
                                        .test_id("catalog.empty.filter")
                                        .build(),
                                ),
                        ),
                        page_grid_item(6).child(
                            div()
                                .class(
                                    "rounded-md border border-border appkit:border-separator \
                                     overflow-hidden bg-control",
                                )
                                .child(
                                    EmptyState::resource("Sin elementos")
                                        .description("Todavía no hay nada en esta colección.")
                                        .action("Crear elemento", Msg::Pressed)
                                        .test_id("catalog.empty.resource")
                                        .build(),
                                ),
                        ),
                    ]),
                ]),
            ]);

        let surface = |name: &str, class: &str| {
            div()
                .class(&format!(
                    "flex flex-col gap-1 min-h-20 rounded-md border border-border/60 \
                     appkit:border-separator p-3 {class}"
                ))
                .children([
                    caption(name),
                    text("Aa").class("text-sm text-foreground appkit:text-label"),
                ])
        };
        let layers = card()
            .class("w-full max-w-4xl")
            .test_id("catalog.layers")
            .children([
                card_header(
                    "Capas AppKit",
                    Some("window / under-page / control / fill — ver docs/dev/24-layering.md"),
                ),
                card_content()
                    .class("gap-3")
                    .children([page_grid().children([
                        page_grid_item(3).child(surface("bg-window", "bg-window")),
                        page_grid_item(3).child(surface("bg-under-page", "bg-under-page")),
                        page_grid_item(3).child(surface("bg-control", "bg-control")),
                        page_grid_item(3).child(surface("bg-fill-3", "bg-fill-3")),
                    ])]),
            ]);

        div()
            .class("flex flex-col gap-5 w-full")
            .test_id("catalog")
            .children([
                filters, buttons, fields, choices, badges, feedback, empties, layers,
            ])
    }
}
