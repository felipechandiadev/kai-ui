//! [`FormState`]: el modelo de un formulario (valores, tocados, enviado y
//! errores) sin dependencia del renderer.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Format, ValidateOn, Validator};

/// Valores normalizados por nombre de campo.
pub type Values = BTreeMap<String, String>;

/// Error de un campo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct FieldError {
    /// Nombre del campo (`rut`).
    pub field: String,
    /// Mensaje para mostrar (`RUT no válido`).
    pub message: String,
}

/// Errores de un formulario, en el orden en que se registraron los campos.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Errors(pub Vec<FieldError>);

impl Errors {
    /// El campo del primer error: al que conviene llevar el foco.
    pub fn first_field(&self) -> Option<&str> {
        self.0.first().map(|error| error.field.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Mensaje del campo `name`, si tiene error.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|error| error.field == name)
            .map(|error| error.message.as_str())
    }
}

/// Definición de un campo: formato, reglas, cuándo mostrar el error y
/// valor inicial.
#[derive(Clone, Debug, Default)]
pub struct Field {
    format: Option<Format>,
    validator: Validator,
    validate_on: ValidateOn,
    initial: String,
}

impl Field {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Reglas del campo; se agregan a las anteriores.
    pub fn validator(mut self, validator: Validator) -> Self {
        self.validator = std::mem::take(&mut self.validator).and(validator);
        self
    }

    pub fn validate_on(mut self, when: ValidateOn) -> Self {
        self.validate_on = when;
        self
    }

    /// Valor inicial (y al que vuelve [`FormState::reset`]); se normaliza.
    pub fn initial(mut self, value: impl Into<String>) -> Self {
        self.initial = value.into();
        self
    }

    fn normalize(&self, value: &str) -> String {
        match self.format {
            Some(format) => format.format(value).value,
            None => value.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
struct Slot {
    name: String,
    field: Field,
    value: String,
    touched: bool,
}

/// Mensajes de un formulario, para el `Msg` de la app
/// (`Msg::Form(FormMsg)`) y para los comandos `<prefijo>.*`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormMsg {
    /// Nuevo valor de un campo (se normaliza).
    Set { name: String, value: String },
    /// El campo perdió el foco.
    Blur(String),
    /// Intento de envío ([`FormState::submit`]).
    Submit,
    /// Volver a los valores iniciales ([`FormState::reset`]).
    Reset,
}

/// Modelo de un formulario: campos con nombre, valores normalizados,
/// tocados, enviado y errores calculados con [`Validator`].
///
/// ```
/// use kai::forms::{Field, Format, FormState, Validators};
///
/// let mut form = FormState::new()
///     .field("nombre", Field::new().validator(Validators::required()))
///     .field("rut", Field::new().format(Format::Rut).validator(Validators::rut()));
/// form.set("rut", "12.345.678-5");
/// assert_eq!(form.value("rut"), Some("12345678-5"));
/// let errors = form.submit().unwrap_err();
/// assert_eq!(errors.first_field(), Some("nombre"));
/// ```
#[derive(Clone, Debug, Default)]
pub struct FormState {
    slots: Vec<Slot>,
    submitted: bool,
    last_submit: Option<Errors>,
}

impl FormState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra el campo `name` (el orden define el de [`Self::errors`]).
    /// Un nombre repetido reemplaza la definición anterior.
    pub fn field(mut self, name: impl Into<String>, field: Field) -> Self {
        let name = name.into();
        let value = field.normalize(&field.initial);
        let slot = Slot {
            name: name.clone(),
            field,
            value,
            touched: false,
        };
        match self.slots.iter_mut().find(|slot| slot.name == name) {
            Some(existing) => *existing = slot,
            None => self.slots.push(slot),
        }
        self
    }

    fn slot(&self, name: &str) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.name == name)
    }

    fn slot_mut(&mut self, name: &str) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|slot| slot.name == name)
    }

    /// Nombres de los campos, en orden.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.slots.iter().map(|slot| slot.name.as_str())
    }

    /// ¿Existe el campo `name`?
    pub fn has(&self, name: &str) -> bool {
        self.slot(name).is_some()
    }

    /// Cambia el valor (normalizado con el formato del campo). `false` si
    /// el campo no existe.
    pub fn set(&mut self, name: &str, value: &str) -> bool {
        match self.slot_mut(name) {
            Some(slot) => {
                slot.value = slot.field.normalize(value);
                true
            }
            None => false,
        }
    }

    /// Valor normalizado (`12345678-5`).
    pub fn value(&self, name: &str) -> Option<&str> {
        self.slot(name).map(|slot| slot.value.as_str())
    }

    /// Texto formateado para mostrar (`12.345.678-5`).
    pub fn display(&self, name: &str) -> Option<String> {
        let slot = self.slot(name)?;
        Some(match slot.field.format {
            Some(format) => format.format(&slot.value).display,
            None => slot.value.clone(),
        })
    }

    /// Formato del campo, si tiene.
    pub fn format_of(&self, name: &str) -> Option<Format> {
        self.slot(name)?.field.format
    }

    /// Reglas del campo.
    pub fn validator_of(&self, name: &str) -> Option<&Validator> {
        self.slot(name).map(|slot| &slot.field.validator)
    }

    /// Cuándo muestra su error el campo.
    pub fn validate_on_of(&self, name: &str) -> Option<ValidateOn> {
        self.slot(name).map(|slot| slot.field.validate_on)
    }

    /// Todos los valores normalizados.
    pub fn values(&self) -> Values {
        self.slots
            .iter()
            .map(|slot| (slot.name.clone(), slot.value.clone()))
            .collect()
    }

    /// Error del valor actual aunque aún no deba verse.
    pub fn check(&self, name: &str) -> Option<String> {
        let slot = self.slot(name)?;
        slot.field.validator.validate(&slot.value).err()
    }

    /// Error visible del campo según su [`ValidateOn`], tocado y enviado.
    pub fn error(&self, name: &str) -> Option<String> {
        let slot = self.slot(name)?;
        if !slot
            .field
            .validate_on
            .shows_error(&slot.value, slot.touched, self.submitted)
        {
            return None;
        }
        self.check(name)
    }

    /// Todos los errores actuales (visibles o no), en orden de registro.
    pub fn errors(&self) -> Errors {
        Errors(
            self.slots
                .iter()
                .filter_map(|slot| {
                    let message = slot.field.validator.validate(&slot.value).err()?;
                    Some(FieldError {
                        field: slot.name.clone(),
                        message,
                    })
                })
                .collect(),
        )
    }

    /// El primer campo con error, o `None`.
    pub fn first_error_field(&self) -> Option<String> {
        self.errors().first_field().map(str::to_owned)
    }

    pub fn is_valid(&self) -> bool {
        self.errors().is_empty()
    }

    /// ¿Algún valor difiere del inicial?
    pub fn is_dirty(&self) -> bool {
        self.names().any(|name| self.is_field_dirty(name))
    }

    pub fn is_field_dirty(&self, name: &str) -> bool {
        self.slot(name)
            .is_some_and(|slot| slot.value != slot.field.normalize(&slot.field.initial))
    }

    pub fn is_touched(&self, name: &str) -> bool {
        self.slot(name).is_some_and(|slot| slot.touched)
    }

    pub fn is_submitted(&self) -> bool {
        self.submitted
    }

    pub fn touch(&mut self, name: &str) {
        if let Some(slot) = self.slot_mut(name) {
            slot.touched = true;
        }
    }

    pub fn touch_all(&mut self) {
        for slot in &mut self.slots {
            slot.touched = true;
        }
    }

    /// Marca el formulario como enviado y todos sus campos como tocados:
    /// `Ok(valores)` si no hay errores, o todos los errores
    /// ([`Errors::first_field`] = el campo al que llevar el foco).
    pub fn submit(&mut self) -> Result<Values, Errors> {
        self.submitted = true;
        self.touch_all();
        let errors = self.errors();
        self.last_submit = Some(errors.clone());
        if errors.is_empty() {
            Ok(self.values())
        } else {
            Err(errors)
        }
    }

    /// Errores del último [`Self::submit`] (vacío si se envió bien), aunque
    /// después se haya hecho [`Self::reset`]; `None` si nunca se envió.
    pub fn last_submit(&self) -> Option<&Errors> {
        self.last_submit.as_ref()
    }

    /// Vuelve a los valores iniciales, sin tocados ni envío (conserva
    /// [`Self::last_submit`]).
    pub fn reset(&mut self) {
        self.submitted = false;
        for slot in &mut self.slots {
            slot.value = slot.field.normalize(&slot.field.initial);
            slot.touched = false;
        }
    }

    /// Aplica `msg`: `Set`, `Blur` y `Reset` cambian el modelo y devuelven
    /// `None`; `Submit` devuelve el resultado de [`Self::submit`].
    pub fn update(&mut self, msg: FormMsg) -> Option<Result<Values, Errors>> {
        match msg {
            FormMsg::Set { name, value } => {
                self.set(&name, &value);
                None
            }
            FormMsg::Blur(name) => {
                self.touch(&name);
                None
            }
            FormMsg::Submit => Some(self.submit()),
            FormMsg::Reset => {
                self.reset();
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::Validators;

    fn client() -> FormState {
        FormState::new()
            .field("nombre", Field::new().validator(Validators::required()))
            .field(
                "correo",
                Field::new()
                    .format(Format::Email)
                    .validator(Validators::required().and(Validators::email())),
            )
            .field(
                "rut",
                Field::new()
                    .format(Format::Rut)
                    .validator(Validators::rut())
                    .initial("11.111.111-1"),
            )
            .field(
                "cupo",
                Field::new()
                    .format(Format::Clp)
                    .validate_on(ValidateOn::Change)
                    .validator(Validators::clp_range(1_000, 10_000)),
            )
    }

    #[test]
    fn values_are_normalized_and_displayed_formatted() {
        let mut form = client();
        assert_eq!(form.value("rut"), Some("11111111-1"));
        assert!(form.set("rut", "12.345.678-5"));
        assert_eq!(form.value("rut"), Some("12345678-5"));
        assert_eq!(form.display("rut").as_deref(), Some("12.345.678-5"));
        assert!(!form.set("nada", "x"));
        assert_eq!(form.value("nada"), None);
        assert_eq!(
            form.values().get("rut").map(String::as_str),
            Some("12345678-5")
        );
    }

    #[test]
    fn errors_show_by_validate_on_and_submit_shows_them_all() {
        let mut form = client();
        assert_eq!(form.error("nombre"), None, "sin tocar");
        assert_eq!(form.errors().len(), 2, "nombre y correo vacíos");
        form.set("cupo", "50");
        assert_eq!(
            form.error("cupo").as_deref(),
            Some("Debe estar entre $ 1.000 y $ 10.000"),
            "Change: al escribir"
        );
        form.update(FormMsg::Blur("nombre".into()));
        assert_eq!(
            form.error("nombre").as_deref(),
            Some("Este campo es obligatorio")
        );

        let errors = form.submit().unwrap_err();
        assert!(form.is_submitted());
        assert_eq!(errors.first_field(), Some("nombre"));
        let fields: Vec<&str> = errors.0.iter().map(|e| e.field.as_str()).collect();
        assert_eq!(fields, ["nombre", "correo", "cupo"]);
        assert_eq!(
            form.error("correo").as_deref(),
            Some("Este campo es obligatorio")
        );
        assert!(!form.is_valid());
        assert_eq!(form.first_error_field().as_deref(), Some("nombre"));
    }

    #[test]
    fn a_valid_submit_returns_the_values() {
        let mut form = client();
        form.set("nombre", "Ana");
        form.set("correo", " Ana@Ejemplo.CL ");
        form.set("cupo", "$ 5.000");
        let values = form.submit().unwrap();
        form.reset();
        assert_eq!(form.last_submit(), Some(&Errors::default()));
        assert_eq!(values["correo"], "Ana@ejemplo.cl");
        assert_eq!(values["cupo"], "5000");
        assert_eq!(values["rut"], "11111111-1");
    }

    #[test]
    fn dirty_tracking_and_reset() {
        let mut form = client();
        assert!(!form.is_dirty());
        form.set("rut", "11111111-1");
        assert!(!form.is_dirty(), "mismo valor normalizado");
        form.set("nombre", "Ana");
        assert!(form.is_dirty() && form.is_field_dirty("nombre"));
        form.touch("rut");
        let _ = form.submit();
        form.update(FormMsg::Reset);
        assert!(!form.is_dirty() && !form.is_submitted() && !form.is_touched("rut"));
        assert_eq!(form.value("nombre"), Some(""));
        assert_eq!(form.error("nombre"), None);
    }
}
