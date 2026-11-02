//! `TextField` con `forms`: formato, cursor, `on_input` normalizado y
//! errores de validación.

use crate::components::TextField;
use crate::element::{Element, div};
use crate::forms::{Format, ValidateOn, Validators};
use crate::layout::Content;
use crate::runtime::SimpleApp;
use crate::testing::{By, MemoryClipboard, TestApp};
use crate::ui_engine::{Key, Modifiers};

#[derive(Default)]
struct Form {
    rut: String,
    email: String,
    rut_touched: bool,
    email_touched: bool,
    amount: String,
    phone: String,
    inputs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Rut(String),
    Email(String),
    RutBlur,
    EmailBlur,
    Amount(String),
    Phone(String),
}

impl SimpleApp for Form {
    type Msg = Msg;
    fn title(&self) -> String {
        "Formulario".into()
    }
    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Rut(value) => {
                self.inputs.push(value.clone());
                self.rut = value;
            }
            Msg::Email(value) => self.email = value,
            Msg::RutBlur => self.rut_touched = true,
            Msg::EmailBlur => self.email_touched = true,
            Msg::Amount(value) => self.amount = value,
            Msg::Phone(value) => self.phone = value,
        }
    }
    fn view(&self) -> Element<Msg> {
        div().class("flex flex-col gap-2 p-4").children([
            TextField::new("RUT", &self.rut)
                .format(Format::Rut)
                .validate(Validators::required())
                .validate(Validators::rut())
                .touched(self.rut_touched)
                .helper_text("Con dígito verificador")
                .on_input(Msg::Rut)
                .on_blur(Msg::RutBlur)
                .test_id("rut")
                .build(),
            TextField::new("Correo", &self.email)
                .format(Format::Email)
                .validate(Validators::email())
                .validate_on(ValidateOn::Change)
                .touched(self.email_touched)
                .on_input(Msg::Email)
                .on_blur(Msg::EmailBlur)
                .test_id("correo")
                .build(),
            TextField::new("Monto", &self.amount)
                .format(Format::Clp)
                .start_symbol("$")
                .validate(Validators::clp_range(1_000, 500_000))
                .validate_on(ValidateOn::Change)
                .on_input(Msg::Amount)
                .test_id("monto")
                .build(),
            TextField::new("Teléfono", &self.phone)
                .format(Format::TelefonoCl)
                .validate(Validators::phone_cl())
                .validate_on(ValidateOn::Change)
                .on_input(Msg::Phone)
                .test_id("fono")
                .build(),
        ])
    }
}

/// Pega `text` donde está el cursor (⌘V), sin mover el foco.
fn paste(app: &mut TestApp<Form>, clipboard: &MemoryClipboard, text: &str) {
    clipboard.set(text);
    app.press(
        Key::V,
        Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    );
}

fn with_clipboard() -> (TestApp<Form>, MemoryClipboard) {
    let clipboard = MemoryClipboard::new();
    let app = TestApp::builder(Form::default())
        .clipboard(clipboard.clone())
        .build();
    (app, clipboard)
}

fn helper(app: &TestApp<Form>, id: &str) -> Option<String> {
    let screen = app.screen();
    let found = screen.query(By::test_id(format!("{id}.helper")))?;
    match &found.node().content {
        Content::Text(text) => Some(text.clone()),
        _ => None,
    }
}

#[test]
fn typing_a_rut_formats_it_and_reports_the_normalized_value() {
    let (mut app, clipboard) = with_clipboard();
    app.type_text(By::test_id("rut"), "123456785");
    assert_eq!(app.value(By::test_id("rut")), "12.345.678-5");
    assert_eq!(app.app().rut, "12345678-5");
    assert_eq!(app.app().inputs[..3], ["1", "1-2", "12-3"]);
    // Un décimo carácter no cabe.
    paste(&mut app, &clipboard, "0");
    assert_eq!(app.value(By::test_id("rut")), "12.345.678-5");
}

#[test]
fn the_caret_stays_after_the_same_digit_when_dots_move() {
    let (mut app, clipboard) = with_clipboard();
    app.type_text(By::test_id("rut"), "1234567");
    assert_eq!(app.value(By::test_id("rut")), "123.456-7");
    app.press(Key::Home, Modifiers::NONE);
    app.press(Key::ArrowRight, Modifiers::NONE);
    paste(&mut app, &clipboard, "9");
    assert_eq!(app.value(By::test_id("rut")), "1.923.456-7");
    // El cursor quedó tras el 9, no donde estaba su byte.
    paste(&mut app, &clipboard, "8");
    assert_eq!(app.value(By::test_id("rut")), "19.823.456-7");
    assert_eq!(app.app().rut, "19823456-7");
}

#[test]
fn pasting_a_rut_with_dots_and_spaces_formats_it() {
    let (mut app, clipboard) = with_clipboard();
    app.click(By::test_id("rut"));
    paste(&mut app, &clipboard, " 12.345 678-k ");
    assert_eq!(app.value(By::test_id("rut")), "12.345.678-K");
    assert_eq!(app.app().rut, "12345678-K");
}

#[test]
fn an_invalid_rut_shows_its_error_on_blur_and_a_fix_clears_it() {
    let (mut app, clipboard) = with_clipboard();
    app.type_text(By::test_id("rut"), "123456784");
    assert_eq!(
        helper(&app, "rut").as_deref(),
        Some("Con dígito verificador"),
        "sin error mientras se escribe"
    );
    app.click(By::test_id("correo"));
    assert!(app.app().rut_touched);
    assert_eq!(helper(&app, "rut").as_deref(), Some("RUT no válido"));
    assert_eq!(
        app.screen()
            .get(By::test_id("rut.input"))
            .node()
            .description
            .as_deref(),
        Some("RUT no válido")
    );

    app.click(By::test_id("rut"));
    app.press(Key::End, Modifiers::NONE);
    app.press(Key::Backspace, Modifiers::NONE);
    paste(&mut app, &clipboard, "5");
    assert_eq!(app.app().rut, "12345678-5");
    assert_eq!(
        helper(&app, "rut").as_deref(),
        Some("Con dígito verificador")
    );
}

#[test]
fn an_empty_required_rut_fails_only_after_blur() {
    let mut app = TestApp::new(Form::default());
    assert_eq!(
        helper(&app, "rut").as_deref(),
        Some("Con dígito verificador")
    );
    app.click(By::test_id("rut"));
    app.click(By::test_id("correo"));
    assert_eq!(
        helper(&app, "rut").as_deref(),
        Some("Este campo es obligatorio")
    );
}

#[test]
fn email_validates_while_typing_and_keeps_the_local_part() {
    let mut app = TestApp::new(Form::default());
    app.type_text(By::test_id("correo"), "Ana@Ejemplo");
    assert_eq!(app.app().email, "Ana@ejemplo");
    assert_eq!(helper(&app, "correo").as_deref(), Some("Correo no válido"));
    app.type_text(By::test_id("correo"), ".CL");
    assert_eq!(app.value(By::test_id("correo")), "Ana@ejemplo.cl");
    assert_eq!(helper(&app, "correo"), None);
}

#[test]
fn a_formatted_value_from_the_app_is_shown_formatted() {
    let app = TestApp::new(Form {
        rut: "1000005k".into(),
        ..Form::default()
    });
    assert_eq!(app.value(By::test_id("rut")), "1.000.005-K");
}

#[test]
fn an_amount_gets_thousands_dots_and_reports_digits() {
    let (mut app, clipboard) = with_clipboard();
    app.type_text(By::test_id("monto"), "1234567");
    assert_eq!(app.value(By::test_id("monto")), "1.234.567");
    assert_eq!(app.app().amount, "1234567");
    assert_eq!(
        helper(&app, "monto").as_deref(),
        Some("Debe estar entre $ 1.000 y $ 500.000")
    );
    // Insertar en medio: el cursor sigue al dígito, no al punto.
    app.press(Key::Home, Modifiers::NONE);
    app.press(Key::ArrowRight, Modifiers::NONE);
    paste(&mut app, &clipboard, "0");
    assert_eq!(app.value(By::test_id("monto")), "10.234.567");
    paste(&mut app, &clipboard, "9");
    assert_eq!(app.value(By::test_id("monto")), "109.234.567");

    app.press(
        Key::A,
        Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    );
    paste(&mut app, &clipboard, "$ 25.000,");
    assert_eq!(app.value(By::test_id("monto")), "25.000");
    assert_eq!(app.app().amount, "25000");
    assert_eq!(helper(&app, "monto"), None);
}

#[test]
fn a_phone_gets_the_country_code_and_reports_e164() {
    let (mut app, clipboard) = with_clipboard();
    app.type_text(By::test_id("fono"), "91234");
    assert_eq!(app.value(By::test_id("fono")), "+56 9 1234");
    assert_eq!(app.app().phone, "+5691234");
    assert_eq!(helper(&app, "fono").as_deref(), Some("Teléfono no válido"));
    // Tras el 9 (Home + 5 → "+56 9|"), insertar mueve los grupos.
    app.press(Key::Home, Modifiers::NONE);
    for _ in 0..5 {
        app.press(Key::ArrowRight, Modifiers::NONE);
    }
    paste(&mut app, &clipboard, "8");
    assert_eq!(app.value(By::test_id("fono")), "+56 9 8123 4");
    paste(&mut app, &clipboard, "7");
    assert_eq!(app.value(By::test_id("fono")), "+56 9 8712 34");

    app.press(
        Key::A,
        Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    );
    paste(&mut app, &clipboard, "(+56) 2-2345-6789");
    assert_eq!(app.value(By::test_id("fono")), "+56 2 2345 6789");
    assert_eq!(app.app().phone, "+56223456789");
    assert_eq!(helper(&app, "fono"), None);
}

/// Sin `touched` en el modelo: la sesión recuerda qué campo perdió el foco.
#[derive(Default)]
struct Plain {
    rut: String,
    name: String,
}

#[derive(Clone, Debug, PartialEq)]
enum PlainMsg {
    Rut(String),
    Name(String),
    Clear,
}

impl SimpleApp for Plain {
    type Msg = PlainMsg;
    fn title(&self) -> String {
        "Sin estado".into()
    }
    fn update(&mut self, msg: PlainMsg) {
        match msg {
            PlainMsg::Rut(value) => self.rut = value,
            PlainMsg::Name(value) => self.name = value,
            PlainMsg::Clear => *self = Self::default(),
        }
    }
    fn view(&self) -> Element<PlainMsg> {
        div().class("flex flex-col gap-2 p-4").children([
            TextField::new("RUT", &self.rut)
                .format(Format::Rut)
                .validate(Validators::required().and(Validators::rut()))
                .on_input(PlainMsg::Rut)
                .test_id("rut")
                .build(),
            // Sin `test_id`: se recuerda por la etiqueta.
            TextField::new("Nombre", &self.name)
                .validate(Validators::min_len(3))
                .on_input(PlainMsg::Name)
                .build(),
            crate::components::button("Limpiar", crate::components::ButtonVariant::Text)
                .on_click(PlainMsg::Clear)
                .test_id("limpiar"),
        ])
    }
}

fn error_text(app: &TestApp<Plain>, text: &str) -> bool {
    app.screen().query(By::text(text)).is_some()
}

#[test]
fn the_session_remembers_blur_without_the_app_tracking_touched() {
    let mut app = TestApp::new(Plain::default());
    app.type_text(By::test_id("rut"), "123456784");
    assert!(
        !error_text(&app, "RUT no válido"),
        "sin error mientras se escribe"
    );
    app.press(Key::Tab, Modifiers::NONE);
    assert!(!app.is_focused(By::test_id("rut")));
    assert!(
        error_text(&app, "RUT no válido"),
        "error tras salir con Tab"
    );

    app.type_text(By::label("Nombre"), "Al");
    assert!(!error_text(&app, "Debe tener al menos 3 caracteres"));
    app.click(By::test_id("rut"));
    assert!(error_text(&app, "Debe tener al menos 3 caracteres"));
    assert!(error_text(&app, "RUT no válido"), "sigue tocado al volver");

    // La app limpia el formulario: los campos dejan de estar tocados.
    app.click(By::test_id("limpiar"));
    assert!(!error_text(&app, "Este campo es obligatorio"));
    assert!(!error_text(&app, "Debe tener al menos 3 caracteres"));
}

/// Un formulario atado a `FormState` con `FormState::text_field`.
struct Bound {
    form: crate::forms::FormState,
    sent: Option<Result<crate::forms::Values, crate::forms::Errors>>,
}

#[derive(Clone, Debug, PartialEq)]
enum BoundMsg {
    Form(crate::forms::FormMsg),
}

impl Default for Bound {
    fn default() -> Self {
        use crate::forms::{Field, FormState};
        Self {
            form: FormState::new()
                .field(
                    "rut",
                    Field::new()
                        .format(Format::Rut)
                        .validator(Validators::rut()),
                )
                .field("nombre", Field::new().validator(Validators::required())),
            sent: None,
        }
    }
}

impl SimpleApp for Bound {
    type Msg = BoundMsg;
    fn title(&self) -> String {
        "Atado".into()
    }
    fn update(&mut self, BoundMsg::Form(msg): BoundMsg) {
        if let Some(result) = self.form.update(msg) {
            self.sent = Some(result);
        }
    }
    fn view(&self) -> Element<BoundMsg> {
        div().class("flex flex-col gap-2 p-4").children([
            self.form.text_field("rut", "RUT", BoundMsg::Form).build(),
            self.form
                .text_field("nombre", "Nombre", BoundMsg::Form)
                .on_submit(BoundMsg::Form(crate::forms::FormMsg::Submit))
                .build(),
        ])
    }
}

#[test]
fn a_bound_field_formats_tracks_blur_and_submit_in_the_form_state() {
    let mut app = TestApp::new(Bound::default());
    app.type_text(By::test_id("rut"), "123456784");
    assert_eq!(app.value(By::test_id("rut")), "12.345.678-4");
    assert_eq!(app.app().form.value("rut"), Some("12345678-4"));
    assert!(app.app().form.is_dirty());
    assert!(app.screen().query(By::text("RUT no válido")).is_none());

    app.click(By::test_id("nombre"));
    assert!(app.app().form.is_touched("rut"), "on_blur → FormMsg::Blur");
    app.screen().get(By::text("RUT no válido"));
    assert!(
        app.screen()
            .query(By::text("Este campo es obligatorio"))
            .is_none()
    );

    app.press(Key::Enter, Modifiers::NONE);
    let errors = app.app().sent.clone().unwrap().unwrap_err();
    assert_eq!(errors.first_field(), Some("rut"));
    app.screen().get(By::text("Este campo es obligatorio"));
}
