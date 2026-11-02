//! Teléfonos chilenos: `+56` y 9 dígitos nacionales.
//!
//! - Se conservan solo los dígitos. El `56` inicial se quita si el texto
//!   empieza con `+` (`+56 9…`, `(+56) 9…`) o si hay 11 dígitos o más
//!   (`56912345678` pegado). Así, se escribe el número nacional: el campo
//!   pone el `+56` solo. Un `+` escrito en un campo vacío se descarta.
//! - Hasta 9 dígitos nacionales; lo que sobra se descarta.
//! - Agrupación: móvil (`9`) y Santiago (`2`) usan código de 1 dígito y
//!   grupos de 4 (`+56 9 1234 5678`, `+56 2 2345 6789`); otro código de
//!   área usa 2 dígitos y grupos de 3 y 4 (`+56 32 234 5678`).
//! - Válido ([`is_valid`]): exactamente 9 dígitos nacionales y el primero
//!   entre 2 y 9 (`9` = móvil; `2` = Santiago; `3`–`8` = código de área de
//!   2 dígitos). No se revisa la lista oficial de códigos de área.

/// Lo que precede al número nacional en pantalla.
pub const PREFIX: &str = "+56 ";

fn strips_country_code(raw: &str) -> bool {
    let digits = raw.chars().filter(char::is_ascii_digit).count();
    let plus = raw
        .trim_start_matches(|c: char| c.is_whitespace() || c == '(')
        .starts_with('+');
    let starts_56 = raw
        .chars()
        .filter(char::is_ascii_digit)
        .take(2)
        .eq("56".chars());
    starts_56 && (plus || digits >= 11)
}

/// Dígitos de `raw` que se descartan como código de país.
pub(crate) fn dropped_prefix(raw: &str) -> usize {
    if strips_country_code(raw) { 2 } else { 0 }
}

/// Hasta 9 dígitos nacionales, sin el `56`.
pub fn national(raw: &str) -> String {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let mut national = digits[dropped_prefix(raw)..].to_owned();
    national.truncate(9);
    national
}

pub(crate) fn display_of_national(national: &str) -> String {
    if national.is_empty() {
        return String::new();
    }
    let short = national.starts_with('9') || national.starts_with('2');
    let head = if short { 1 } else { 2.min(national.len()) };
    let groups: &[usize] = if short { &[4, 4] } else { &[3, 4] };
    let mut out = format!("{PREFIX}{}", &national[..head]);
    let mut rest = &national[head..];
    for size in groups {
        if rest.is_empty() {
            break;
        }
        let take = (*size).min(rest.len());
        out.push(' ');
        out.push_str(&rest[..take]);
        rest = &rest[take..];
    }
    out
}

pub(crate) fn value_of_national(national: &str) -> String {
    if national.is_empty() {
        String::new()
    } else {
        format!("+56{national}")
    }
}

/// `+56912345678` (E.164), o lo que haya mientras se escribe.
pub fn normalize(raw: &str) -> String {
    value_of_national(&national(raw))
}

/// `+56 9 1234 5678`.
pub fn format(raw: &str) -> String {
    display_of_national(&national(raw))
}

/// ¿Es un teléfono chileno completo? Ver el detalle del módulo.
pub fn is_valid(raw: &str) -> bool {
    let all: String = raw.chars().filter(char::is_ascii_digit).collect();
    let national = &all[dropped_prefix(raw)..];
    national.len() == 9 && matches!(national.as_bytes()[0], b'2'..=b'9')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_and_invalid_numbers() {
        for ok in [
            "+56 9 1234 5678",
            "912345678",
            "56912345678",
            "+56 2 2345 6789",
            "+56 32 234 5678",
        ] {
            assert!(is_valid(ok), "{ok}");
        }
        for bad in [
            "",
            "12345678",
            "91234567",
            "9123456789",
            "112345678",
            "012345678",
            "+56 9 1234 567",
        ] {
            assert!(!is_valid(bad), "{bad}");
        }
        assert_eq!(normalize("+56 9 1234 5678"), "+56912345678");
        assert_eq!(format("+56912345678"), "+56 9 1234 5678");
    }
}
