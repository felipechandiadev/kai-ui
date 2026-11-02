//! /fields: the variants and states of TextField, the autocompletes, the
//! search field and the text area.

use std::time::Duration;

use kai::components::{
    AutocompleteItem, AutocompleteMsg, AutocompleteState, FieldBezel, FieldDensity, FieldKind,
    FieldVariant, LabelLayout, SearchVariant, TextField, autocomplete, caption, card, card_content,
    card_header, page, page_body, page_grid, page_grid_item, search_field, text_area,
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
    Pin,
    Notes,
    Lookup,
}

/// A client the autocomplete finds: name and RUT.
#[derive(Clone, Debug, PartialEq)]
pub struct Client {
    pub name: &'static str,
    pub rut: &'static str,
}

impl AutocompleteItem for Client {
    fn label(&self) -> String {
        self.name.to_owned()
    }

    fn detail(&self) -> Option<String> {
        Some(self.rut.to_owned())
    }
}

const fn client(name: &'static str, rut: &'static str) -> Client {
    Client { name, rut }
}

const CLIENTS: &[Client] = &[
    client("Ana Pérez", "11.111.111-1"),
    client("Andrés Soto", "12.345.678-5"),
    client("Anita Rojas", "9.876.543-3"),
    client("Bruno Díaz", "7.654.321-K"),
    client("Camila Fuentes", "15.222.333-4"),
    client("Carlos Muñoz", "16.444.555-6"),
];

const COMMUNES: &[&str] = &[
    "Concepción",
    "Coquimbo",
    "Curicó",
    "La Florida",
    "La Reina",
    "Las Condes",
    "Maipú",
    "Ñuñoa",
    "Parral",
    "Providencia",
    "Puente Alto",
    "Santiago",
    "Temuco",
    "Valdivia",
    "Valparaíso",
    "Viña del Mar",
];

/// What the demo's client search waits, as a slow server would.
const SEARCH_LATENCY: Duration = Duration::from_millis(400);

/// The clients whose name contains `query`, after [`SEARCH_LATENCY`];
/// "error" fails, to show the error state.
fn search_clients(query: &str) -> Result<Vec<Client>, String> {
    std::thread::sleep(SEARCH_LATENCY);
    if query.eq_ignore_ascii_case("error") {
        return Err("No se pudo conectar con el servidor".into());
    }
    let query = query.to_lowercase();
    Ok(CLIENTS
        .iter()
        .filter(|client| client.name.to_lowercase().contains(&query))
        .cloned()
        .collect())
}

pub struct Screen {
    search: String,
    password: String,
    show_password: bool,
    amount: String,
    discount: String,
    commune: String,
    code: String,
    quantity: String,
    client: AutocompleteState<Client>,
    commune_pick: AutocompleteState<&'static str>,
    pin: String,
    show_pin: bool,
    notes: String,
    lookup: String,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            search: String::new(),
            password: String::new(),
            show_password: false,
            amount: String::new(),
            discount: String::new(),
            commune: String::new(),
            code: String::new(),
            quantity: String::new(),
            client: AutocompleteState::search("fields.client", search_clients),
            commune_pick: AutocompleteState::list("fields.commune", COMMUNES.iter().copied()),
            pin: String::new(),
            show_pin: false,
            notes: String::new(),
            lookup: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Edit(Field, String),
    Client(AutocompleteMsg<Client>),
    CommunePick(AutocompleteMsg<&'static str>),
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
                    Field::Pin => &mut self.pin,
                    Field::Notes => &mut self.notes,
                    Field::Lookup => &mut self.lookup,
                } = value;
            }
            Msg::Client(msg) => return self.client.update(msg).map(Msg::Client),
            Msg::CommunePick(msg) => return self.commune_pick.update(msg).map(Msg::CommunePick),
            Msg::TogglePassword => self.show_password = !self.show_password,
            Msg::TogglePin => self.show_pin = !self.show_pin,
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().children([
            self.variants(),
            self.autocompletes(),
            self.more(),
            states(),
        ]))
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
        let cell = |field: Element<Msg>| page_grid_item(6).child(field);
        card()
            .class("w-full max-w-3xl")
            .test_id("fields.variants")
            .children([
                card_header(
                    "Variantes de TextField",
                    Some(
                        "Iconos Lucide, adornments, password, requerido, compacto, solo \
                         lectura y deshabilitado",
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
                    ]),
                    contrast_bar,
                ]),
            ])
    }

    /// The autocompletes: clients from a slow search and communes from a
    /// list.
    fn autocompletes(&self) -> Element<Msg> {
        let picked = |label: &str, value: Option<String>| {
            caption(format!(
                "{label}: {}",
                value.unwrap_or_else(|| "ninguno".into())
            ))
            .class("text-secondary-label")
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("fields.autocomplete")
            .children([
                card_header(
                    "Autocompletar",
                    Some(
                        "Búsqueda con pausa y estados (escribe «error» para ver uno) y \
                         lista fija; ↑ ↓ eligen, Retorno toma y Escape cierra",
                    ),
                ),
                card_content().class("gap-4").child(
                    page_grid().children([
                        page_grid_item(6).class("flex flex-col gap-1.5").children([
                            autocomplete("Cliente", &self.client, Msg::Client)
                                .test_id("fields.client")
                                .placeholder("Nombre del cliente")
                                .helper_text("Desde 2 letras; busca en el servidor")
                                .build(),
                            picked(
                                "Cliente elegido",
                                self.client
                                    .selected()
                                    .map(|client| format!("{} ({})", client.name, client.rut)),
                            ),
                        ]),
                        page_grid_item(6).class("flex flex-col gap-1.5").children([
                            autocomplete(
                                "Comuna de despacho",
                                &self.commune_pick,
                                Msg::CommunePick,
                            )
                            .test_id("fields.commune")
                            .placeholder("↓ muestra todas")
                            .build(),
                            picked(
                                "Comuna elegida",
                                self.commune_pick
                                    .selected()
                                    .map(|commune| (*commune).to_owned()),
                            ),
                        ]),
                    ]),
                ),
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
