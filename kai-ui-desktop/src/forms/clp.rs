//! Montos en pesos chilenos (CLP): enteros con puntos de miles.

/// Dígitos que caben en un monto (hasta 999.999.999.999).
pub const MAX_DIGITS: usize = 12;

/// Solo dígitos, sin ceros a la izquierda (`000` → `0`), hasta
/// [`MAX_DIGITS`].
pub fn clean(raw: &str) -> String {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let trimmed = digits.trim_start_matches('0');
    let mut out = if trimmed.is_empty() && !digits.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    };
    out.truncate(MAX_DIGITS);
    out
}

/// Ceros a la izquierda que [`clean`] descarta.
pub(crate) fn leading_zeros(raw: &str) -> usize {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let zeros = digits.len() - digits.trim_start_matches('0').len();
    if zeros == digits.len() {
        zeros.saturating_sub(1)
    } else {
        zeros
    }
}

/// `1234567` → `1.234.567` (es-CL, sin decimales).
pub fn group(digits: &str) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Monto de `raw` (`$ 1.234.567` → `1234567`); `None` sin dígitos.
pub fn parse(raw: &str) -> Option<u64> {
    clean(raw).parse().ok()
}

/// `$ 1.234.567`.
pub fn format(amount: u64) -> String {
    format!("$ {}", group(&amount.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_format_amounts() {
        assert_eq!(parse("$ 1.234.567"), Some(1_234_567));
        assert_eq!(parse("abc"), None);
        assert_eq!(format(500_000), "$ 500.000");
        assert_eq!(format(0), "$ 0");
        assert_eq!(leading_zeros("00150"), 2);
        assert_eq!(leading_zeros("000"), 2);
    }
}
