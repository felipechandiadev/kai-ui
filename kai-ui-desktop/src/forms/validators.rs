use std::fmt;
use std::rc::Rc;

use super::{clp, email, phone, rut};
use crate::datetime::{Date, Time};

type CustomFn = Rc<dyn Fn(&str) -> Result<(), String>>;

#[derive(Clone)]
enum Rule {
    Required,
    Email,
    Rut,
    MinLen(usize),
    MaxLen(usize),
    PhoneCl,
    ClpRange(u64, u64),
    Date,
    DateRange(Date, Date),
    Time,
    TimeRange(Time, Time),
    Custom(CustomFn),
}

impl Rule {
    fn default_message(&self) -> String {
        match self {
            Rule::Required => "Este campo es obligatorio".into(),
            Rule::Email => "Correo no válido".into(),
            Rule::Rut => "RUT no válido".into(),
            Rule::MinLen(n) => format!("Debe tener al menos {n} caracteres"),
            Rule::MaxLen(n) => format!("Debe tener como máximo {n} caracteres"),
            Rule::PhoneCl => "Teléfono no válido".into(),
            Rule::ClpRange(min, max) => format!(
                "Debe estar entre {} y {}",
                clp::format(*min),
                clp::format(*max)
            ),
            Rule::Date => "Fecha no válida".into(),
            Rule::DateRange(min, max) => format!("Debe estar entre el {min} y el {max}"),
            Rule::Time => "Hora no válida".into(),
            Rule::TimeRange(min, max) => format!("Debe estar entre las {min} y las {max}"),
            Rule::Custom(_) => "Valor no válido".into(),
        }
    }
}

#[derive(Clone)]
struct Check {
    rule: Rule,
    message: Option<String>,
}

/// Reglas que se revisan en orden; la primera que falla da el mensaje.
/// Se arma con [`Validators`] y se combina con [`Validator::and`].
///
/// Un valor vacío (o solo espacios) pasa todas las reglas salvo
/// [`Validators::required`], así los campos opcionales funcionan.
#[derive(Clone, Default)]
pub struct Validator {
    checks: Vec<Check>,
}

impl fmt::Debug for Validator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Validator")
            .field("checks", &self.checks.len())
            .finish()
    }
}

/// Constructores de [`Validator`].
pub struct Validators;

impl Validators {
    /// Falla con un valor vacío o solo de espacios.
    pub fn required() -> Validator {
        Validator::rule(Rule::Required)
    }

    /// Correo práctico ([`crate::forms::email::is_valid`]).
    pub fn email() -> Validator {
        Validator::rule(Rule::Email)
    }

    /// RUT con dígito verificador correcto ([`crate::forms::rut::is_valid`]).
    pub fn rut() -> Validator {
        Validator::rule(Rule::Rut)
    }

    /// Al menos `n` caracteres (no bytes).
    pub fn min_len(n: usize) -> Validator {
        Validator::rule(Rule::MinLen(n))
    }

    /// Como máximo `n` caracteres (no bytes).
    pub fn max_len(n: usize) -> Validator {
        Validator::rule(Rule::MaxLen(n))
    }

    /// Teléfono chileno completo ([`crate::forms::phone::is_valid`]): 9
    /// dígitos nacionales que empiezan entre 2 y 9, con o sin `+56`.
    pub fn phone_cl() -> Validator {
        Validator::rule(Rule::PhoneCl)
    }

    /// Monto CLP entre `min` y `max`, ambos incluidos; lee los dígitos
    /// ([`crate::forms::clp::parse`]), así sirve con `1234` o `$ 1.234`.
    pub fn clp_range(min: u64, max: u64) -> Validator {
        Validator::rule(Rule::ClpRange(min, max))
    }

    /// Fecha que existe (`2026-03-05` o `05/03/2026`).
    pub fn date() -> Validator {
        Validator::rule(Rule::Date)
    }

    /// Fecha válida entre `min` y `max`, ambos incluidos.
    pub fn date_range(min: Date, max: Date) -> Validator {
        Validator::rule(Rule::DateRange(min, max))
    }

    /// Hora válida `HH:MM` (24 h).
    pub fn time() -> Validator {
        Validator::rule(Rule::Time)
    }

    /// Hora válida entre `min` y `max`, ambas incluidas.
    pub fn time_range(min: Time, max: Time) -> Validator {
        Validator::rule(Rule::TimeRange(min, max))
    }

    /// Regla propia: `Err(mensaje)` si el valor no sirve. Como las demás,
    /// no se llama con un valor vacío.
    pub fn custom(check: impl Fn(&str) -> Result<(), String> + 'static) -> Validator {
        Validator::rule(Rule::Custom(Rc::new(check)))
    }
}

impl Validator {
    fn rule(rule: Rule) -> Self {
        Self {
            checks: vec![Check {
                rule,
                message: None,
            }],
        }
    }

    /// Agrega las reglas de `other` después de estas.
    pub fn and(mut self, other: Validator) -> Self {
        self.checks.extend(other.checks);
        self
    }

    /// Cambia el mensaje de la última regla agregada.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        if let Some(last) = self.checks.last_mut() {
            last.message = Some(message.into());
        }
        self
    }

    /// `Ok(())` o el mensaje de la primera regla que falla.
    pub fn validate(&self, value: &str) -> Result<(), String> {
        let empty = value.trim().is_empty();
        for check in &self.checks {
            let result = match &check.rule {
                Rule::Required => (!empty).then_some(()).ok_or(None),
                _ if empty => Ok(()),
                Rule::Email => email::is_valid(value).then_some(()).ok_or(None),
                Rule::Rut => rut::is_valid(value).then_some(()).ok_or(None),
                Rule::MinLen(n) => (value.chars().count() >= *n).then_some(()).ok_or(None),
                Rule::MaxLen(n) => (value.chars().count() <= *n).then_some(()).ok_or(None),
                Rule::PhoneCl => phone::is_valid(value).then_some(()).ok_or(None),
                Rule::ClpRange(min, max) => clp::parse(value)
                    .filter(|amount| (*min..=*max).contains(amount))
                    .map(|_| ())
                    .ok_or(None),
                Rule::Date => Date::parse(value).map(|_| ()).ok_or(None),
                Rule::DateRange(min, max) => Date::parse(value)
                    .filter(|date| (*min..=*max).contains(date))
                    .map(|_| ())
                    .ok_or(None),
                Rule::Time => Time::parse(value).map(|_| ()).ok_or(None),
                Rule::TimeRange(min, max) => Time::parse(value)
                    .filter(|time| (*min..=*max).contains(time))
                    .map(|_| ())
                    .ok_or(None),
                Rule::Custom(f) => f(value).map_err(Some),
            };
            if let Err(own) = result {
                return Err(check
                    .message
                    .clone()
                    .or(own)
                    .unwrap_or_else(|| check.rule.default_message()));
            }
        }
        Ok(())
    }

    /// ¿No tiene reglas?
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }
}

impl FromIterator<Validator> for Validator {
    fn from_iter<I: IntoIterator<Item = Validator>>(iter: I) -> Self {
        iter.into_iter().fold(Validator::default(), Validator::and)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_messages_in_spanish() {
        assert_eq!(
            Validators::required().validate(" "),
            Err("Este campo es obligatorio".into())
        );
        assert_eq!(
            Validators::email().validate("x"),
            Err("Correo no válido".into())
        );
        assert_eq!(
            Validators::rut().validate("1-1"),
            Err("RUT no válido".into())
        );
        assert_eq!(
            Validators::min_len(3).validate("ñu"),
            Err("Debe tener al menos 3 caracteres".into())
        );
        assert_eq!(
            Validators::max_len(2).validate("ñuu"),
            Err("Debe tener como máximo 2 caracteres".into())
        );
        assert_eq!(Validators::min_len(2).validate("ñu"), Ok(()));
    }

    #[test]
    fn phone_and_clp_rules() {
        let phone = Validators::phone_cl();
        assert_eq!(phone.validate("+56912345678"), Ok(()));
        assert_eq!(phone.validate("+5691234"), Err("Teléfono no válido".into()));
        let amount = Validators::clp_range(1_000, 500_000);
        assert_eq!(amount.validate("1000"), Ok(()));
        assert_eq!(amount.validate("$ 500.000"), Ok(()));
        let message = Err("Debe estar entre $ 1.000 y $ 500.000".to_owned());
        assert_eq!(amount.validate("999"), message);
        assert_eq!(amount.validate("500001"), message);
        assert_eq!(amount.validate("abc"), message);
        assert_eq!(amount.validate(""), Ok(()));
    }

    #[test]
    fn date_and_time_rules() {
        let min = Date::new(2026, 1, 1).unwrap();
        let max = Date::new(2026, 12, 31).unwrap();
        assert_eq!(Validators::date().validate("2026-02-28"), Ok(()));
        assert_eq!(Validators::date().validate("30/02/2026"), Err("Fecha no válida".into()));
        let range = Validators::date_range(min, max);
        assert_eq!(range.validate("2026-03-05"), Ok(()));
        assert_eq!(
            range.validate("2027-01-01"),
            Err("Debe estar entre el 01/01/2026 y el 31/12/2026".into())
        );
        assert_eq!(Validators::time().validate("23:59"), Ok(()));
        assert_eq!(Validators::time().validate("24:00"), Err("Hora no válida".into()));
        let office = Validators::time_range(Time::new(9, 0).unwrap(), Time::new(18, 0).unwrap());
        assert_eq!(office.validate("18:00"), Ok(()));
        assert_eq!(
            office.validate("08:59"),
            Err("Debe estar entre las 09:00 y las 18:00".into())
        );
    }

    #[test]
    fn empty_values_only_fail_required() {
        let optional = Validators::email()
            .and(Validators::rut())
            .and(Validators::min_len(5))
            .and(Validators::custom(|_| Err("nunca".into())));
        assert_eq!(optional.validate(""), Ok(()));
        assert!(optional.validate("x").is_err());
    }

    #[test]
    fn rules_run_in_order_and_messages_override() {
        let v = Validators::required()
            .message("Ingresa tu RUT")
            .and(Validators::rut().message("Revisa el dígito verificador"));
        assert_eq!(v.validate(""), Err("Ingresa tu RUT".into()));
        assert_eq!(
            v.validate("12.345.678-4"),
            Err("Revisa el dígito verificador".into())
        );
        assert_eq!(v.validate("12.345.678-5"), Ok(()));

        let list: Validator = [Validators::min_len(2), Validators::max_len(3)]
            .into_iter()
            .collect();
        assert_eq!(
            list.validate("abcd"),
            Err("Debe tener como máximo 3 caracteres".into())
        );
    }

    #[test]
    fn custom_rules_return_their_own_message() {
        let even = Validators::custom(|v| {
            if v.len() % 2 == 0 {
                Ok(())
            } else {
                Err("Largo impar".into())
            }
        });
        assert_eq!(even.validate("ab"), Ok(()));
        assert_eq!(even.validate("abc"), Err("Largo impar".into()));
        assert_eq!(
            even.clone().message("Otro").validate("abc"),
            Err("Otro".into())
        );
    }
}
