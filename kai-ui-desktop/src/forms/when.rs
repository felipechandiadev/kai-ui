//! Máscaras de fecha (`dd/mm/aaaa`) y hora (`HH:MM`) para [`super::Format`].

use crate::datetime::{Date, Time};

fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

fn numeric_parts(raw: &str) -> Vec<&str> {
    raw.split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect()
}

/// Dígitos `ddmmaaaa` (hasta 8) de lo escrito o pegado. Con tres partes y
/// un año de 4 dígitos (`5/3/2026`, `2026-03-05`) rellena día y mes con
/// ceros; si no, toma los dígitos en orden.
pub(crate) fn date_digits(raw: &str) -> String {
    let parts = numeric_parts(raw);
    if parts.len() == 3 {
        let (y, m, d) = if parts[0].len() == 4 && parts[1].len() <= 2 && parts[2].len() <= 2 {
            (parts[0], parts[1], parts[2])
        } else if parts[2].len() == 4 && parts[0].len() <= 2 && parts[1].len() <= 2 {
            (parts[2], parts[1], parts[0])
        } else {
            ("", "", "")
        };
        if !y.is_empty() {
            return format!("{d:0>2}{m:0>2}{y}");
        }
    }
    let mut all = digits(raw);
    all.truncate(8);
    all
}

/// `05032026` → `05/03/2026`; parcial: `050` → `05/0`.
pub(crate) fn date_display(digits: &str) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i == 2 || i == 4 {
            out.push('/');
        }
        out.push(c);
    }
    out
}

/// ISO si la fecha está completa y existe; si no, lo que se ve.
pub(crate) fn date_value(digits: &str) -> String {
    let display = date_display(digits);
    match Date::parse(&display) {
        Some(date) if digits.len() == 8 => date.iso(),
        _ => display,
    }
}

/// Dígitos `HHMM` (hasta 4); `9:05` pegado → `0905`.
pub(crate) fn time_digits(raw: &str) -> String {
    let parts = numeric_parts(raw);
    if parts.len() == 2 && parts[0].len() == 1 && parts[1].len() == 2 {
        return format!("0{}{}", parts[0], parts[1]);
    }
    let mut all = digits(raw);
    all.truncate(4);
    all
}

pub(crate) fn time_display(digits: &str) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i == 2 {
            out.push(':');
        }
        out.push(c);
    }
    out
}

/// Flechas ↑/↓ en una fecha completa: el cursor en el día (bytes 0–2), el
/// mes (3–5) o el año elige qué cambia.
pub(crate) fn step_date(text: &str, caret: usize, delta: i32) -> Option<String> {
    let date = Date::parse(text)?;
    let next = if caret <= 2 {
        date.add_days(i64::from(delta))
    } else if caret <= 5 {
        date.add_months(delta)
    } else {
        date.add_months(delta * 12)
    }?;
    Some(next.display())
}

/// Flechas ↑/↓ en una hora completa: en la hora (bytes 0–2) suma horas; en
/// los minutos, `step` minutos, ajustando al múltiplo de `step`.
pub(crate) fn step_time(text: &str, caret: usize, delta: i32, step: u8) -> Option<String> {
    let time = Time::parse(text)?;
    let next = if caret <= 2 {
        time.add_minutes(i64::from(delta) * 60)
    } else {
        let step = i64::from(step.max(1));
        let minutes = i64::from(time.minutes());
        let snapped = if delta > 0 {
            (minutes / step + 1) * step
        } else {
            (minutes + step - 1) / step * step - step
        };
        Time::from_minutes(snapped)
    };
    Some(next.display())
}
