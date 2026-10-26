//! /fields: the variants and states of TextField, the search field and
//! the text area.

use kai::components::{
    FieldBezel, FieldDensity, FieldKind, FieldVariant, LabelLayout, SearchVariant, TextField,
    caption, card, card_content, card_header, page, page_body, page_grid, page_grid_item,
    search_field, text_area,
};
use kai::element::{Element, div, icon, text};
use kai::icons::Icon;
use kai::router::{Context, Page};
use kai::runtime::Effects;

use crate::state::State;

/// The fields a person can type in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Search,
    Password,
    Amount,
    Discount,
    Commune,
    Code,
    Quantity,
    Autocomplete,
    Pin,
    Notes,
    Lookup,
}

#[derive(Default)]
pub struct Screen {
    search: String,
    password: String,
    show_password: bool,
    amount: String,
    discount: String,
    commune: String,
    code: String,
    quantity: String,
    autocomplete: String,
    pin: String,
    show_pin: bool,
    notes: String,
    lookup: String,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Edit(Field, String),
    TogglePassword,
    TogglePin,
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
                    Field::Pin => &mut self.pin,
                    Field::Notes => &mut self.notes,
                    Field::Lookup => &mut self.lookup,
                } = value;
            }
            Msg::TogglePassword => self.show_password = !self.show_password,
            Msg::TogglePin => self.show_pin = !self.show_pin,
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().children([self.variants(), self.more(), states()]))
    }

    fn title(&self, _: &State) -> String {
        "Campos".into()
    }
}

fn edit(field: Field) -> impl Fn(String) -> Msg {
    move |value| Msg::Edit(field, value)
}

impl Screen {
    fn variants(&self) -> Element<Msg> {
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
        let autocomplete = div()
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
        let cell = |field: Element<Msg>| page_grid_item(6).child(field);
        card()
            .class("w-full max-w-3xl")
            .test_id("fields.variants")
            .children([
                card_header(
                    "Variantes de TextField",
                    Some(
                        "Iconos Lucide, adornments, password, requerido, compacto, solo \
                         lectura, deshabilitado y autocomplete",
                    ),
                ),
                card_content().class("gap-4").children([
                    page_grid().children([
                        cell(
                            TextField::new("Buscar", &self.search)
                                .placeholder("Texto de búsqueda")
                                .field_icon(FieldKind::Search)
                                .on_input(edit(Field::Search))
                                .build(),
                        ),
                        cell(
                            TextField::new("Contraseña", &self.password)
                                .test_id("fields.password")
                                .field_icon(FieldKind::Password)
                                .password(self.show_password, Msg::TogglePassword)
                                .required(true)
                                .helper_text("Mínimo 8 caracteres")
                                .on_input(edit(Field::Password))
                                .build(),
                        ),
                        cell(
                            TextField::new("Monto", &self.amount)
                                .test_id("fields.amount")
                                .start_symbol("$")
                                .select_on_focus(true)
                                .on_input(edit(Field::Amount))
                                .build(),
                        ),
                        cell(
                            TextField::new("Descuento", &self.discount)
                                .end_symbol("%")
                                .on_input(edit(Field::Discount))
                                .build(),
                        ),
                        cell(
                            TextField::new("Comuna", &self.commune)
                                .required(true)
                                .helper_text("Donde despacharemos el pedido")
                                .on_input(edit(Field::Commune))
                                .build(),
                        ),
                        cell(
                            TextField::new("Código", &self.code)
                                .density(FieldDensity::Compact)
                                .placeholder("Ej: A-12")
                                .on_input(edit(Field::Code))
                                .build(),
                        ),
                        cell(
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
                        ),
                        cell(
                            TextField::new("Folio", "F-2026-0042")
                                .read_only(true)
                                .end_icon(Icon::CircleCheckBig)
                                .build(),
                        ),
                        cell(
                            TextField::new("Sucursal", "Santiago Centro")
                                .disabled(true)
                                .test_id("fields.branch")
                                .build(),
                        ),
                        cell(autocomplete),
                    ]),
                    contrast_bar,
                ]),
            ])
    }

    /// A PIN with its eye, the AppKit rounded bezel, adornments, the
    /// placeholder color, a search field and a text area.
    fn more(&self) -> Element<Msg> {
        card()
            .class("w-full max-w-3xl")
            .test_id("fields.more")
            .children([
                card_header(
                    "Más campos",
                    Some("Bisel redondeado, adornos, color del placeholder, búsqueda y notas"),
                ),
                card_content().class("gap-4").children([
                    div()
                        .class("grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-5")
                        .children([
                            TextField::new("PIN", &self.pin)
                                .test_id("fields.pin")
                                .password(self.show_pin, Msg::TogglePin)
                                .on_input(edit(Field::Pin))
                                .build(),
                            TextField::new("Usuario", &self.lookup)
                                .test_id("fields.rounded")
                                .bezel(FieldBezel::Rounded)
                                .placeholder("Placeholder en color primario")
                                .placeholder_color("primary/60")
                                .on_input(edit(Field::Lookup))
                                .build(),
                            TextField::new("Sitio", "kai.cl")
                                .test_id("fields.adornments")
                                .start_adornment(text("https://").class(
                                    "text-sm text-muted-foreground appkit:text-secondary-label",
                                ))
                                .end_adornment(
                                    icon(Icon::CircleCheckBig).class("size-4 text-success"),
                                )
                                .read_only(true)
                                .build(),
                            search_field(&self.lookup, edit(Field::Lookup))
                                .variant(SearchVariant::Field)
                                .placeholder("Buscar en la ayuda")
                                .label("Buscar en la ayuda")
                                .test_id("fields.search")
                                .build(),
                        ]),
                    text_area("Notas", &self.notes)
                        .test_id("fields.notes")
                        .placeholder("Varias líneas; Retorno agrega una")
                        .auto_grow(3, 6)
                        .max_length(280)
                        .counter(true)
                        .helper_text("Crece hasta 6 líneas")
                        .on_input(edit(Field::Notes))
                        .build(),
                ]),
            ])
    }
}

/// Default, with a value, disabled, required, helper and error.
fn states() -> Element<Msg> {
    let cell = |label: &str, field: Element<Msg>| {
        div()
            .class("flex flex-col gap-1.5 min-w-0")
            .children([caption(label).class("text-secondary-label"), field])
    };
    card()
        .class("w-full max-w-3xl")
        .test_id("fields.states")
        .children([
            card_header(
                "Estados de TextField",
                Some("Default, foco, disabled, required, helper y error"),
            ),
            card_content().child(
                div()
                    .class("grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-5")
                    .children([
                        cell(
                            "Default",
                            TextField::new("Nombre", "")
                                .placeholder("Valor de ejemplo")
                                .field_icon(FieldKind::Name)
                                .test_id("states.default")
                                .build(),
                        ),
                        cell(
                            "Con valor (foco vía Tab)",
                            TextField::new("Correo", "ana@example.com")
                                .field_icon(FieldKind::Email)
                                .test_id("states.focus")
                                .build(),
                        ),
                        cell(
                            "Disabled",
                            TextField::new("Teléfono", "+56 9 1234 5678")
                                .field_icon(FieldKind::Phone)
                                .disabled(true)
                                .test_id("states.disabled")
                                .build(),
                        ),
                        cell(
                            "Required",
                            TextField::new("Usuario", "")
                                .placeholder("Requerido")
                                .field_icon(FieldKind::User)
                                .required(true)
                                .test_id("states.required")
                                .build(),
                        ),
                        cell(
                            "Helper text",
                            TextField::new("Búsqueda", "")
                                .placeholder("Escribe para filtrar")
                                .field_icon(FieldKind::Search)
                                .helper_text("Puedes usar palabras clave")
                                .test_id("states.helper")
                                .build(),
                        ),
                        cell(
                            "Error",
                            TextField::new("Correo", "no-es-un-correo")
                                .field_icon(FieldKind::Email)
                                .helper_text("Introduce un correo válido")
                                .error(true)
                                .test_id("states.error")
                                .build(),
                        ),
                    ]),
            ),
        ])
}
