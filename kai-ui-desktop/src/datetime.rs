//! Fechas y horas sin zona horaria, sin dependencias: [`Date`] (calendario
//! gregoriano), [`Time`] (24 h), [`Weekday`], y los nombres en español en
//! un solo lugar ([`names`]) para que la tarea de i18n los reemplace.
//!
//! Formatos: `dd/mm/aaaa` en pantalla, `aaaa-mm-dd` (ISO 8601) como valor;
//! `HH:MM` para horas.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Nombres en español (es-CL). **Único lugar** con textos de calendario:
/// la i18n (pendiente) los cambiará por un catálogo.
pub mod names {
    /// Meses, enero primero.
    pub const MONTHS: [&str; 12] = [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ];
    /// Días, lunes primero.
    pub const WEEKDAYS: [&str; 7] = [
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
        "domingo",
    ];
    /// Iniciales de los días para la grilla, lunes primero.
    pub const WEEKDAY_INITIALS: [&str; 7] = ["L", "M", "M", "J", "V", "S", "D"];
    pub const PREVIOUS_MONTH: &str = "Mes anterior";
    pub const NEXT_MONTH: &str = "Mes siguiente";
    pub const OPEN_CALENDAR: &str = "Abrir calendario";
    pub const CALENDAR: &str = "Calendario";
    pub const TODAY: &str = "hoy";
}

/// Día de la semana; lunes = 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    /// 0 = lunes … 6 = domingo.
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(index: usize) -> Weekday {
        Self::ALL[index % 7]
    }

    /// `lunes`, `martes`…
    pub fn name(self) -> &'static str {
        names::WEEKDAYS[self.index()]
    }
}

/// Fecha del calendario gregoriano, siempre válida.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

/// ¿Año bisiesto?
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Días del mes (`month` de 1 a 12).
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl Date {
    /// `None` si no existe (31/04, 29/02 de un año no bisiesto). Años de
    /// 1 a 9999.
    pub fn new(year: i32, month: u8, day: u8) -> Option<Date> {
        ((1..=9999).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month))
        .then_some(Date { year, month, day })
    }

    pub fn year(self) -> i32 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    /// Días desde 1970-01-01 (algoritmo `days_from_civil` de H. Hinnant).
    pub fn days_since_epoch(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// La fecha `days` días después de 1970-01-01 (`civil_from_days`).
    pub fn from_days_since_epoch(days: i64) -> Option<Date> {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Date::new(i32::try_from(year).ok()?, month, day)
    }

    pub fn weekday(self) -> Weekday {
        // 1970-01-01 fue jueves (índice 3).
        Weekday::from_index((self.days_since_epoch() + 3).rem_euclid(7) as usize)
    }

    /// Suma (o resta) días; `None` fuera de los años 1–9999.
    pub fn add_days(self, days: i64) -> Option<Date> {
        Date::from_days_since_epoch(self.days_since_epoch() + days)
    }

    /// Suma meses; el día se ajusta al último del mes (31/01 + 1 → 28/02).
    pub fn add_months(self, months: i32) -> Option<Date> {
        let index = self.year * 12 + i32::from(self.month) - 1 + months;
        let year = index.div_euclid(12);
        let month = (index.rem_euclid(12) + 1) as u8;
        let day = self.day.min(days_in_month(year, month));
        Date::new(year, month, day)
    }

    /// Primer día del mes de esta fecha.
    pub fn first_of_month(self) -> Date {
        Date { day: 1, ..self }
    }

    /// `aaaa-mm-dd`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// `dd/mm/aaaa`.
    pub fn display(self) -> String {
        format!("{:02}/{:02}/{:04}", self.day, self.month, self.year)
    }

    /// `5 de marzo de 2026`, el nombre accesible de un día.
    pub fn long_name(self) -> String {
        format!(
            "{} {} de {} de {}",
            self.weekday().name(),
            self.day,
            names::MONTHS[usize::from(self.month) - 1],
            self.year
        )
    }

    /// Lee `aaaa-mm-dd`, `dd/mm/aaaa`, `d-m-aaaa` o `dd.mm.aaaa` (con o sin
    /// ceros); `None` si no es una fecha válida.
    pub fn parse(text: &str) -> Option<Date> {
        let parts: Vec<&str> = text
            .trim()
            .split(['/', '-', '.', ' '])
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() != 3 || !parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit())) {
            return None;
        }
        let (y, m, d) = if parts[0].len() == 4 {
            (parts[0], parts[1], parts[2])
        } else if parts[2].len() == 4 && parts[0].len() <= 2 && parts[1].len() <= 2 {
            (parts[2], parts[1], parts[0])
        } else {
            return None;
        };
        if m.len() > 2 || d.len() > 2 {
            return None;
        }
        Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
    }

    /// Hoy según el reloj del sistema, en UTC (sin zona horaria: cerca de
    /// medianoche puede diferir del día local). Para tests, pasa una fecha
    /// fija a los componentes en vez de llamar esto.
    pub fn today_utc() -> Date {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Date::from_days_since_epoch(secs.div_euclid(86_400)).expect("fecha del sistema")
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

/// Hora del día (24 h) con minutos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Time {
    hour: u8,
    minute: u8,
}

impl Time {
    pub fn new(hour: u8, minute: u8) -> Option<Time> {
        (hour < 24 && minute < 60).then_some(Time { hour, minute })
    }

    pub fn hour(self) -> u8 {
        self.hour
    }

    pub fn minute(self) -> u8 {
        self.minute
    }

    /// Minutos desde medianoche.
    pub fn minutes(self) -> u16 {
        u16::from(self.hour) * 60 + u16::from(self.minute)
    }

    /// Desde minutos tras medianoche, dando la vuelta al día.
    pub fn from_minutes(minutes: i64) -> Time {
        let m = minutes.rem_euclid(24 * 60);
        Time {
            hour: (m / 60) as u8,
            minute: (m % 60) as u8,
        }
    }

    /// Suma minutos, dando la vuelta a las 24 h.
    pub fn add_minutes(self, minutes: i64) -> Time {
        Time::from_minutes(i64::from(self.minutes()) + minutes)
    }

    /// `HH:MM`.
    pub fn display(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// Lee `HH:MM`, `H:MM` o `HH.MM`; `None` si no es válida.
    pub fn parse(text: &str) -> Option<Time> {
        let (h, m) = text.trim().split_once([':', '.'])?;
        if h.is_empty() || h.len() > 2 || m.len() != 2 {
            return None;
        }
        if !h.bytes().chain(m.bytes()).all(|b| b.is_ascii_digit()) {
            return None;
        }
        Time::new(h.parse().ok()?, m.parse().ok()?)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

/// Las 6 semanas (42 días) que muestra la grilla del mes de `month`, desde
/// el `first_weekday` anterior o igual al día 1.
pub fn month_grid(month: Date, first_weekday: Weekday) -> Vec<Date> {
    let first = month.first_of_month();
    let back = (first.weekday().index() + 7 - first_weekday.index()) % 7;
    let start = first.add_days(-(back as i64)).unwrap_or(first);
    (0..42).filter_map(|i| start.add_days(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn leap_years_and_month_lengths() {
        assert!(is_leap_year(2024) && is_leap_year(2000));
        assert!(!is_leap_year(1900) && !is_leap_year(2026));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 4), 30);
        assert!(Date::new(2026, 2, 29).is_none());
        assert!(Date::new(2026, 4, 31).is_none());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::new(2026, 1, 0).is_none());
    }

    /// Verificados con `datetime.date(...).isoweekday()` de Python.
    #[test]
    fn weekdays_of_known_dates() {
        assert_eq!(d(2026, 10, 31).weekday(), Weekday::Saturday);
        assert_eq!(d(2000, 2, 29).weekday(), Weekday::Tuesday);
        assert_eq!(d(1900, 1, 1).weekday(), Weekday::Monday);
        assert_eq!(d(2024, 2, 29).weekday(), Weekday::Thursday);
        assert_eq!(d(2026, 3, 5).weekday(), Weekday::Thursday);
        assert_eq!(d(1970, 1, 1).weekday(), Weekday::Thursday);
        assert_eq!(d(2026, 10, 31).days_since_epoch(), 20_757);
    }

    /// Verificados con `timedelta` de Python.
    #[test]
    fn adding_days_and_months() {
        assert_eq!(d(2026, 1, 31).add_days(30), Some(d(2026, 3, 2)));
        assert_eq!(d(2024, 12, 31).add_days(1), Some(d(2025, 1, 1)));
        assert_eq!(d(2026, 3, 1).add_days(-1), Some(d(2026, 2, 28)));
        assert_eq!(d(2024, 3, 1).add_days(-1), Some(d(2024, 2, 29)));
        assert_eq!(d(2026, 1, 31).add_months(1), Some(d(2026, 2, 28)));
        assert_eq!(d(2024, 1, 31).add_months(1), Some(d(2024, 2, 29)));
        assert_eq!(d(2026, 11, 15).add_months(3), Some(d(2027, 2, 15)));
        assert_eq!(d(2026, 1, 15).add_months(-1), Some(d(2025, 12, 15)));
        for days in [-700_000, -1, 0, 1, 59, 10_000, 2_000_000] {
            let date = Date::from_days_since_epoch(days).unwrap();
            assert_eq!(date.days_since_epoch(), days);
        }
    }

    #[test]
    fn parse_and_format() {
        for text in ["2026-03-05", "05/03/2026", "5/3/2026", "05-03-2026", "5.3.2026"] {
            assert_eq!(Date::parse(text), Some(d(2026, 3, 5)), "{text}");
        }
        for text in ["", "2026-02-30", "32/01/2026", "05/03/26", "a/b/c", "2026-3"] {
            assert_eq!(Date::parse(text), None, "{text}");
        }
        assert_eq!(d(2026, 3, 5).iso(), "2026-03-05");
        assert_eq!(d(2026, 3, 5).display(), "05/03/2026");
        assert_eq!(d(2026, 3, 5).long_name(), "jueves 5 de marzo de 2026");
    }

    #[test]
    fn times() {
        assert_eq!(Time::parse("09:05"), Time::new(9, 5));
        assert_eq!(Time::parse("9:05"), Time::new(9, 5));
        assert_eq!(Time::parse("24:00"), None);
        assert_eq!(Time::parse("12:60"), None);
        assert_eq!(Time::parse("12:5"), None);
        let t = Time::new(23, 50).unwrap();
        assert_eq!(t.add_minutes(15).display(), "00:05");
        assert_eq!(Time::new(0, 0).unwrap().add_minutes(-1).display(), "23:59");
    }

    #[test]
    fn the_grid_starts_on_the_first_weekday() {
        // Octubre 2026 empieza en jueves: la grilla parte el lunes 28/09.
        let grid = month_grid(d(2026, 10, 15), Weekday::Monday);
        assert_eq!(grid.len(), 42);
        assert_eq!(grid[0], d(2026, 9, 28));
        assert_eq!(grid[3], d(2026, 10, 1));
        let sunday = month_grid(d(2026, 10, 15), Weekday::Sunday);
        assert_eq!(sunday[0], d(2026, 9, 27));
    }
}
