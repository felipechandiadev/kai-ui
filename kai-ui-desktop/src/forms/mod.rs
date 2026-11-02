//! Formularios: formato de entrada ([`Format`]) y validación
//! ([`Validator`], [`Validators`]) para campos de texto.
//!
//! Módulo puro, sin dependencia del renderer: sirve tanto en
//! [`crate::components::TextField`] (`.format`, `.validate`,
//! `.validate_on`) como para validar el modelo al enviar un formulario.
//!
//! ```
//! use kai::forms::{Format, Validators};
//!
//! let rut = Format::Rut.format("12345678-5");
//! assert_eq!(rut.display, "12.345.678-5");
//! assert_eq!(rut.value, "12345678-5");
//!
//! let validator = Validators::required().and(Validators::rut());
//! assert_eq!(validator.validate(""), Err("Este campo es obligatorio".into()));
//! assert_eq!(validator.validate("12.345.678-5"), Ok(()));
//! ```

pub mod clp;
pub mod commands;
pub mod email;
mod form;
pub mod phone;
pub mod rut;
mod when;
mod validators;

pub use form::{Errors, Field, FieldError, FormMsg, FormState, Values};
pub use validators::{Validator, Validators};

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

thread_local! {
    /// Campos que la sesión vio perder el foco, mientras arma la vista.
    static REMEMBERED: RefCell<Option<Rc<HashSet<String>>>> = const { RefCell::new(None) };
}

/// Clave de un campo para recordar que fue tocado: el `test_id` de su input
/// (`<id>.input`) o, sin `test_id`, su etiqueta.
pub(crate) fn field_key(test_id: Option<&str>, label: &str) -> String {
    match test_id {
        Some(id) => format!("{id}.input"),
        None => label.to_owned(),
    }
}

/// Corre `view` con los campos tocados que recuerda la sesión.
pub(crate) fn with_touched<R>(touched: Rc<HashSet<String>>, view: impl FnOnce() -> R) -> R {
    let previous = REMEMBERED.with(|cell| cell.replace(Some(touched)));
    let result = view();
    REMEMBERED.with(|cell| *cell.borrow_mut() = previous);
    result
}

/// ¿La sesión recuerda que el campo `key` perdió el foco?
pub(crate) fn remembered_touched(key: &str) -> bool {
    REMEMBERED.with(|cell| cell.borrow().as_ref().is_some_and(|set| set.contains(key)))
}

/// Cuándo muestra su error un campo con validación
/// ([`crate::components::TextField::validate_on`]). Los campos no guardan
/// estado: la app marca `touched` (al recibir `on_blur`) y `submitted` (al
/// enviar) y los pasa al campo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ValidateOn {
    /// Mientras se escribe: apenas hay texto, o si ya se tocó o envió.
    Change,
    /// Al salir del campo: cuando la app lo marca `touched` o `submitted`.
    #[default]
    Blur,
    /// Solo al enviar: cuando la app lo marca `submitted`.
    Submit,
}

impl ValidateOn {
    /// ¿Se muestra el error de `value` con este estado del campo?
    pub fn shows_error(self, value: &str, touched: bool, submitted: bool) -> bool {
        match self {
            ValidateOn::Change => submitted || touched || !value.trim().is_empty(),
            ValidateOn::Blur => submitted || touched,
            ValidateOn::Submit => submitted,
        }
    }
}

/// Texto de un campo formateado: lo que se muestra y lo que se guarda.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formatted {
    /// Lo que muestra el campo (`12.345.678-5`).
    pub display: String,
    /// Valor normalizado que recibe la app (`12345678-5`).
    pub value: String,
}

/// Formato de un campo de texto: se aplica mientras se escribe o pega, y
/// la app recibe el valor normalizado ([`Formatted::value`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    /// RUT chileno: `12.345.678-5` en pantalla, `12345678-5` como valor.
    /// Solo conserva dígitos y `K` (hasta 9: 8 de cuerpo + verificador).
    Rut,
    /// Correo: sin espacios alrededor y con el dominio en minúsculas (la
    /// parte local se respeta).
    Email,
    /// Monto en pesos chilenos: `1.234.567` en pantalla, `1234567` como
    /// valor. Solo dígitos (pegar `$ 1.234.567` o `1,234,567` sirve), sin
    /// ceros a la izquierda ni decimales, hasta 12 dígitos. El signo `$` no
    /// va en el texto: ponlo con `TextField::start_symbol("$")`.
    Clp,
    /// Teléfono chileno: `+56 9 1234 5678` (móvil), `+56 2 2345 6789`
    /// (Santiago) o `+56 32 234 5678` (otras áreas, código de 2 dígitos) en
    /// pantalla; `+56912345678` (E.164) como valor. Ver [`phone`].
    TelefonoCl,
    /// Fecha: `05/03/2026` en pantalla, `2026-03-05` (ISO) como valor
    /// cuando está completa y existe; incompleta o inválida, el valor es lo
    /// que se ve. Pegar `5/3/2026`, `05-03-2026` o `2026-03-05` sirve. ↑/↓
    /// cambian el día, mes o año según el cursor.
    Fecha,
    /// Hora 24 h: `09:05` en pantalla y como valor. ↑/↓ cambian la hora o
    /// `step` minutos según el cursor.
    Hora { step: u8 },
}

impl Format {
    /// [`Format::Hora`] de minuto en minuto.
    pub const HORA: Format = Format::Hora { step: 1 };

    /// Formatea `raw` (escrito a medias o pegado con puntos, espacios o
    /// guiones).
    pub fn format(&self, raw: &str) -> Formatted {
        match self {
            Format::Rut => {
                let clean = rut::clean(raw);
                Formatted {
                    display: rut::display_of_clean(&clean),
                    value: rut::value_of_clean(&clean),
                }
            }
            Format::Email => {
                let value = email::normalize(raw);
                Formatted {
                    display: value.clone(),
                    value,
                }
            }
            Format::Clp => {
                let value = clp::clean(raw);
                Formatted {
                    display: clp::group(&value),
                    value,
                }
            }
            Format::Fecha => {
                let digits = when::date_digits(raw);
                Formatted {
                    display: when::date_display(&digits),
                    value: when::date_value(&digits),
                }
            }
            Format::Hora { .. } => {
                let display = when::time_display(&when::time_digits(raw));
                Formatted {
                    value: display.clone(),
                    display,
                }
            }
            Format::TelefonoCl => {
                let national = phone::national(raw);
                Formatted {
                    display: phone::display_of_national(&national),
                    value: phone::value_of_national(&national),
                }
            }
        }
    }

    /// ¿Cuenta `c` para ubicar el cursor? Los separadores que agrega el
    /// formato (puntos, guion, espacios) no cuentan.
    pub fn is_significant(&self, c: char) -> bool {
        match self {
            Format::Rut => c.is_ascii_digit() || c == 'k' || c == 'K',
            Format::Email => !c.is_whitespace(),
            Format::Clp | Format::TelefonoCl | Format::Fecha | Format::Hora { .. } => {
                c.is_ascii_digit()
            }
        }
    }

    /// Caracteres significativos antes del byte `caret` de `text`, sin
    /// contar los que el formato descarta al inicio (ceros a la izquierda
    /// de un monto, el `56` de un teléfono).
    pub fn significant_before(&self, text: &str, caret: usize) -> usize {
        let count = text
            .char_indices()
            .take_while(|(i, _)| *i < caret)
            .filter(|(_, c)| self.is_significant(*c))
            .count();
        let dropped = match self {
            Format::Clp => clp::leading_zeros(text),
            Format::TelefonoCl => phone::dropped_prefix(text),
            _ => 0,
        };
        count.saturating_sub(dropped)
    }

    /// Byte de `display` justo después del carácter significativo número
    /// `count` (0 = al inicio): ahí queda el cursor tras reformatear, así no
    /// salta al agregar o quitar puntos.
    pub fn caret_in(&self, display: &str, count: usize) -> usize {
        // El `+56 ` del teléfono no se escribe: el cursor parte después.
        let start = match self {
            Format::TelefonoCl => phone::PREFIX.len().min(display.len()),
            _ => 0,
        };
        if count == 0 {
            return start;
        }
        let mut seen = 0;
        for (i, c) in display.char_indices().skip_while(|(i, _)| *i < start) {
            if self.is_significant(c) {
                seen += 1;
                if seen == count {
                    return i + c.len_utf8();
                }
            }
        }
        display.len()
    }

    /// ↑ (`delta` 1) o ↓ (-1) con el cursor en el byte `caret` del texto
    /// mostrado: el nuevo texto, o `None` si este formato no tiene pasos o
    /// el valor no está completo.
    pub fn step(&self, text: &str, caret: usize, delta: i32) -> Option<String> {
        match self {
            Format::Fecha => when::step_date(text, caret, delta),
            Format::Hora { step } => when::step_time(text, caret, delta, *step),
            _ => None,
        }
    }

    /// Reformatea `text` con el cursor en el byte `caret`: devuelve el
    /// resultado y la nueva posición del cursor en `display`.
    pub fn reformat(&self, text: &str, caret: usize) -> (Formatted, usize) {
        let formatted = self.format(text);
        let count = self.significant_before(text, caret);
        let caret = self.caret_in(&formatted.display, count);
        (formatted, caret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rut_formats_while_typing() {
        let cases = [
            ("", "", ""),
            ("1", "1", "1"),
            ("12", "1-2", "1-2"),
            ("1234", "123-4", "123-4"),
            ("12345", "1.234-5", "1234-5"),
            ("1234567", "123.456-7", "123456-7"),
            ("12345678", "1.234.567-8", "1234567-8"),
            ("123456785", "12.345.678-5", "12345678-5"),
            ("1000005k", "1.000.005-K", "1000005-K"),
        ];
        for (raw, display, value) in cases {
            let f = Format::Rut.format(raw);
            assert_eq!(f.display, display, "{raw}");
            assert_eq!(f.value, value, "{raw}");
        }
    }

    #[test]
    fn rut_accepts_pasted_text_and_caps_at_nine_chars() {
        for raw in [
            " 12.345.678-5 ",
            "12 345 678 5",
            "12345678-5",
            "12.345.6785",
        ] {
            assert_eq!(Format::Rut.format(raw).display, "12.345.678-5", "{raw}");
        }
        assert_eq!(Format::Rut.format("1234567890").display, "12.345.678-9");
        // Una K solo vale como dígito verificador.
        assert_eq!(Format::Rut.format("1k23").display, "12-3");
        assert_eq!(Format::Rut.format("abc").display, "");
    }

    #[test]
    fn caret_follows_significant_chars() {
        // "1234567|" + "8" → "1.234.567-8|"
        let (f, caret) = Format::Rut.reformat("123.456-78", 10);
        assert_eq!(f.display, "1.234.567-8");
        assert_eq!(caret, f.display.len());
        // Cursor tras el "9" insertado en "1|23.456-7" → "1.9|23.456-7"
        let (f, caret) = Format::Rut.reformat("1923.456-7", 2);
        assert_eq!(f.display, "1.923.456-7");
        assert_eq!(&f.display[..caret], "1.9");
        assert_eq!(Format::Rut.caret_in("1.923.456-7", 0), 0);
        assert_eq!(Format::Rut.significant_before("12.345", 4), 3);
    }

    #[test]
    fn validate_on_decides_when_errors_show() {
        assert!(!ValidateOn::Change.shows_error("", false, false));
        assert!(ValidateOn::Change.shows_error("x", false, false));
        assert!(!ValidateOn::Blur.shows_error("x", false, false));
        assert!(ValidateOn::Blur.shows_error("", true, false));
        assert!(!ValidateOn::Submit.shows_error("x", true, false));
        assert!(ValidateOn::Submit.shows_error("", false, true));
        assert_eq!(ValidateOn::default(), ValidateOn::Blur);
    }

    #[test]
    fn clp_formats_thousands_and_keeps_only_digits() {
        let cases = [
            ("", "", ""),
            ("abc", "", ""),
            ("0", "0", "0"),
            ("000", "0", "0"),
            ("7", "7", "7"),
            ("1500", "1.500", "1500"),
            ("0001500", "1.500", "1500"),
            ("1234567", "1.234.567", "1234567"),
            ("$ 1.234.567", "1.234.567", "1234567"),
            ("1,234,567", "1.234.567", "1234567"),
            ("1234567890123", "123.456.789.012", "123456789012"),
        ];
        for (raw, display, value) in cases {
            let f = Format::Clp.format(raw);
            assert_eq!(
                (f.display.as_str(), f.value.as_str()),
                (display, value),
                "{raw}"
            );
        }
    }

    #[test]
    fn clp_caret_skips_dots_and_dropped_zeros() {
        // "1.23|4" + "5" tras el 3 → "12.35|4"... cuenta 4 dígitos.
        let (f, caret) = Format::Clp.reformat("1.2354", 5);
        assert_eq!(f.display, "12.354");
        assert_eq!(&f.display[..caret], "12.35");
        // Un cero a la izquierda no corre el cursor.
        let (f, caret) = Format::Clp.reformat("05", 2);
        assert_eq!((f.display.as_str(), caret), ("5", 1));
    }

    #[test]
    fn phone_formats_mobile_landline_and_partial_input() {
        let cases = [
            ("", "", ""),
            ("9", "+56 9", "+569"),
            ("91234", "+56 9 1234", "+5691234"),
            ("912345", "+56 9 1234 5", "+56912345"),
            ("912345678", "+56 9 1234 5678", "+56912345678"),
            ("+56912345678", "+56 9 1234 5678", "+56912345678"),
            ("56 9 1234 5678", "+56 9 1234 5678", "+56912345678"),
            ("(+56) 9-1234-5678", "+56 9 1234 5678", "+56912345678"),
            ("223456789", "+56 2 2345 6789", "+56223456789"),
            ("32", "+56 32", "+5632"),
            ("3223", "+56 32 23", "+563223"),
            ("322345678", "+56 32 234 5678", "+56322345678"),
            ("9123456789", "+56 9 1234 5678", "+56912345678"),
        ];
        for (raw, display, value) in cases {
            let f = Format::TelefonoCl.format(raw);
            assert_eq!(
                (f.display.as_str(), f.value.as_str()),
                (display, value),
                "{raw}"
            );
        }
    }

    #[test]
    fn phone_caret_ignores_the_country_code() {
        // "+56 9|1234" + "8" → "+56 9 8|123 4"
        let (f, caret) = Format::TelefonoCl.reformat("+56 981234", 6);
        assert_eq!(f.display, "+56 9 8123 4");
        assert_eq!(&f.display[..caret], "+56 9 8");
        // El primer dígito escrito queda tras el `+56 `.
        let (f, caret) = Format::TelefonoCl.reformat("9", 1);
        assert_eq!((f.display.as_str(), caret), ("+56 9", 5));
        assert_eq!(Format::TelefonoCl.caret_in("+56 9", 0), 4);
    }

    #[test]
    fn dates_format_while_typing_and_when_pasted() {
        let cases = [
            ("", "", ""),
            ("0", "0", "0"),
            ("05", "05", "05"),
            ("050", "05/0", "05/0"),
            ("0503", "05/03", "05/03"),
            ("050320", "05/03/20", "05/03/20"),
            ("05032026", "05/03/2026", "2026-03-05"),
            ("5/3/2026", "05/03/2026", "2026-03-05"),
            ("05-03-2026", "05/03/2026", "2026-03-05"),
            ("2026-03-05", "05/03/2026", "2026-03-05"),
            ("30022026", "30/02/2026", "30/02/2026"),
            ("050320261", "05/03/2026", "2026-03-05"),
        ];
        for (raw, display, value) in cases {
            let f = Format::Fecha.format(raw);
            assert_eq!((f.display.as_str(), f.value.as_str()), (display, value), "{raw}");
        }
        let (f, caret) = Format::Fecha.reformat("05/032", 6);
        assert_eq!(f.display, "05/03/2");
        assert_eq!(caret, f.display.len());
    }

    #[test]
    fn times_format_and_step() {
        for (raw, display) in [("", ""), ("9", "9"), ("093", "09:3"), ("0930", "09:30"), ("9:05", "09:05"), ("12345", "12:34")] {
            assert_eq!(Format::HORA.format(raw).display, display, "{raw}");
        }
        let quarter = Format::Hora { step: 15 };
        assert_eq!(quarter.step("09:05", 4, 1).as_deref(), Some("09:15"));
        assert_eq!(quarter.step("09:05", 4, -1).as_deref(), Some("09:00"));
        assert_eq!(quarter.step("09:00", 4, -1).as_deref(), Some("08:45"));
        assert_eq!(quarter.step("23:50", 4, 1).as_deref(), Some("00:00"));
        assert_eq!(Format::HORA.step("23:10", 1, 1).as_deref(), Some("00:10"));
        assert_eq!(Format::HORA.step("09:3", 4, 1), None);
    }

    #[test]
    fn dates_step_by_day_month_or_year() {
        assert_eq!(Format::Fecha.step("31/01/2026", 1, 1).as_deref(), Some("01/02/2026"));
        assert_eq!(Format::Fecha.step("31/01/2026", 4, 1).as_deref(), Some("28/02/2026"));
        assert_eq!(Format::Fecha.step("29/02/2024", 8, 1).as_deref(), Some("28/02/2025"));
        assert_eq!(Format::Fecha.step("05/03", 1, 1), None);
        assert_eq!(Format::Rut.step("1-9", 0, 1), None);
    }

    #[test]
    fn email_format_trims_and_lowercases_the_domain() {
        let f = Format::Email.format("  Ana.Perez@Ejemplo.CL ");
        assert_eq!(f.display, "Ana.Perez@ejemplo.cl");
        assert_eq!(f.value, "Ana.Perez@ejemplo.cl");
        assert_eq!(Format::Email.format("Ana").value, "Ana");
    }
}
