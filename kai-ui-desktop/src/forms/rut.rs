//! RUT chileno: limpieza, formato y dígito verificador (módulo 11).

/// Solo dígitos y una `K` final (en mayúscula), máximo 9 caracteres.
pub(crate) fn clean(raw: &str) -> String {
    let chars: Vec<char> = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'k' || *c == 'K')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let last = chars.len().saturating_sub(1);
    let mut out: String = chars
        .iter()
        .enumerate()
        .filter(|(i, c)| **c != 'K' || *i == last)
        .map(|(_, c)| *c)
        .collect();
    out.truncate(9);
    // Al truncar, una K puede quedar en medio: solo vale al final.
    if out.len() == 9 && out[..8].contains('K') {
        out = out.chars().filter(|c| *c != 'K').collect();
    }
    out
}

pub(crate) fn display_of_clean(clean: &str) -> String {
    if clean.len() < 2 {
        return clean.to_owned();
    }
    let (body, dv) = clean.split_at(clean.len() - 1);
    let mut grouped = String::new();
    for (i, c) in body.chars().enumerate() {
        if i > 0 && (body.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!("{grouped}-{dv}")
}

pub(crate) fn value_of_clean(clean: &str) -> String {
    if clean.len() < 2 {
        return clean.to_owned();
    }
    let (body, dv) = clean.split_at(clean.len() - 1);
    format!("{body}-{dv}")
}

/// `12.345.678-5` → `12345678-5` (sin puntos, `K` mayúscula). Acepta
/// puntos, espacios y guiones; un texto incompleto queda como esté.
pub fn normalize(raw: &str) -> String {
    value_of_clean(&clean(raw))
}

/// Formato de pantalla: `12.345.678-5`, `1.000.005-K`.
pub fn format(raw: &str) -> String {
    display_of_clean(&clean(raw))
}

/// Dígito verificador del cuerpo (solo dígitos): `'0'`–`'9'` o `'K'`.
/// `None` si `body` está vacío o tiene otros caracteres.
pub fn check_digit(body: &str) -> Option<char> {
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut sum = 0u32;
    let mut factor = 2;
    for b in body.bytes().rev() {
        sum += u32::from(b - b'0') * factor;
        factor = if factor == 7 { 2 } else { factor + 1 };
    }
    Some(match 11 - sum % 11 {
        11 => '0',
        10 => 'K',
        r => char::from_digit(r, 10).expect("dígito"),
    })
}

/// ¿Es un RUT válido? Cuerpo de 1 a 8 dígitos distinto de cero y dígito
/// verificador correcto; acepta puntos, espacios, guion y `k` minúscula.
pub fn is_valid(raw: &str) -> bool {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '.' | '-' | ' '))
        .collect();
    if cleaned.len() < 2 || cleaned.len() > 9 || !cleaned.is_ascii() {
        return false;
    }
    let (body, dv) = cleaned.split_at(cleaned.len() - 1);
    if body.bytes().all(|b| b == b'0') {
        return false;
    }
    check_digit(body) == dv.chars().next().map(|c| c.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verificados a mano con el algoritmo (2,3,4,5,6,7,2,3… desde la
    /// derecha).
    const VALID: [&str; 9] = [
        "12.345.678-5",
        "11.111.111-1",
        "1.000.005-K",
        "6-K",
        "23-K",
        "1-9",
        "1.000.013-0",
        "9.876.543-3",
        "7.775.998-0",
    ];

    #[test]
    fn check_digits_match_known_ruts() {
        for rut in VALID {
            let normalized = normalize(rut);
            let (body, dv) = normalized.split_once('-').unwrap();
            assert_eq!(check_digit(body), dv.chars().next(), "{rut}");
            assert!(is_valid(rut), "{rut}");
        }
        assert_eq!(check_digit(""), None);
        assert_eq!(check_digit("12a"), None);
    }

    #[test]
    fn accepts_lowercase_k_spaces_and_no_dots() {
        assert!(is_valid("1000005-k"));
        assert!(is_valid("12345678-5"));
        assert!(is_valid("123456785"));
        assert!(is_valid(" 12 345 678 5 "));
    }

    #[test]
    fn rejects_wrong_digits_and_malformed_input() {
        for rut in [
            "12.345.678-4",
            "12.345.678-K",
            "1.000.005-0",
            "0-0",
            "00000000-0",
            "5",
            "",
            "123.456.789-0",
            "12.345.67A-5",
            "12.345.678-",
        ] {
            assert!(!is_valid(rut), "{rut}");
        }
    }

    #[test]
    fn normalize_and_format() {
        assert_eq!(normalize("12.345.678-5"), "12345678-5");
        assert_eq!(normalize("1.000.005-k"), "1000005-K");
        assert_eq!(format("123456785"), "12.345.678-5");
        assert_eq!(format("6k"), "6-K");
    }
}
