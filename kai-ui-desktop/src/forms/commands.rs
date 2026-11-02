//! Comandos genéricos de un formulario para scripts y agentes:
//! `<prefijo>.set_field`, `.errors`, `.values`, `.submit`, `.reset` y
//! `.last_submit` (la respuesta de `submit`).
//!
//! La página implementa [`HasForm`] (dónde está su [`FormState`], cómo
//! llevarle un [`FormMsg`] y los ids, con [`crate::form_ids!`]) y los
//! registra en `Page::commands` con [`register`]. Como todo comando, no
//! se publican a agentes sin `.expose()`.
//!
//! ```ignore
//! impl HasForm for Screen {
//!     const IDS: FormIds = kai::form_ids!("client");
//!     fn form(&self) -> &FormState { &self.form }
//!     fn form_msg(msg: FormMsg) -> Msg { Msg::Form(msg) }
//! }
//! fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
//!     kai::forms::commands::register::<Self, A>(registry).expose();
//! }
//! ```

use std::marker::PhantomData;

use schemars::JsonSchema;
use serde::Deserialize;

use super::{Errors, FormMsg, FormState, Values};
use crate::commands::{CommandError, CommandRegistry, PageAction, PageQuery, Safety};
use crate::router::{HasPage, Page};
use crate::runtime::App;

/// Ids de los seis comandos de un formulario.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormIds {
    pub set_field: &'static str,
    pub errors: &'static str,
    pub values: &'static str,
    pub submit: &'static str,
    pub reset: &'static str,
    pub last_submit: &'static str,
}

/// [`FormIds`] con un prefijo: `form_ids!("client")` da
/// `client.set_field`, `client.errors`, `client.values`, `client.submit` y
/// `client.reset`.
#[macro_export]
macro_rules! form_ids {
    ($prefix:literal) => {
        $crate::forms::commands::FormIds {
            set_field: concat!($prefix, ".set_field"),
            errors: concat!($prefix, ".errors"),
            values: concat!($prefix, ".values"),
            submit: concat!($prefix, ".submit"),
            reset: concat!($prefix, ".reset"),
            last_submit: concat!($prefix, ".last_submit"),
        }
    };
}

/// Una página con un [`FormState`] que expone sus comandos.
pub trait HasForm: Page {
    const IDS: FormIds;
    fn form(&self) -> &FormState;
    /// El mensaje de la página que lleva `msg` al formulario.
    fn form_msg(msg: FormMsg) -> Self::Msg;
}

/// Entrada de `<prefijo>.set_field`.
#[derive(Deserialize, JsonSchema)]
#[serde(bound = "", deny_unknown_fields)]
#[schemars(bound = "")]
pub struct SetField<P> {
    /// Nombre del campo (`rut`).
    #[schemars(description = "Nombre del campo, por ejemplo `rut`")]
    pub name: String,
    /// Valor tal como lo escribiría una persona; se normaliza.
    #[schemars(description = "Valor; se normaliza con el formato del campo")]
    pub value: String,
    #[serde(skip)]
    #[schemars(skip)]
    _page: PhantomData<P>,
}

/// Comando sin entrada (`errors`, `values`, `submit`, `reset`).
#[derive(Deserialize, JsonSchema)]
#[serde(bound = "", deny_unknown_fields)]
#[schemars(bound = "")]
pub struct NoInput<P, const KIND: u8> {
    #[serde(skip)]
    #[schemars(skip)]
    _page: PhantomData<P>,
}

impl<P> Clone for SetField<P> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            value: self.value.clone(),
            _page: PhantomData,
        }
    }
}

impl<P, const KIND: u8> Clone for NoInput<P, KIND> {
    fn clone(&self) -> Self {
        Self { _page: PhantomData }
    }
}

const ERRORS: u8 = 0;
const VALUES: u8 = 1;
const SUBMIT: u8 = 2;
const RESET: u8 = 3;
const LAST_SUBMIT: u8 = 4;

/// `<prefijo>.errors`: todos los errores actuales.
pub type ErrorsQuery<P> = NoInput<P, ERRORS>;
/// `<prefijo>.values`: los valores normalizados.
pub type ValuesQuery<P> = NoInput<P, VALUES>;
/// `<prefijo>.last_submit`: errores del último envío (`[]` = enviado).
pub type LastSubmitQuery<P> = NoInput<P, LAST_SUBMIT>;
/// `<prefijo>.submit`: envía; contesta con `<prefijo>.last_submit`.
pub type Submit<P> = NoInput<P, SUBMIT>;
/// `<prefijo>.reset`: vuelve a los valores iniciales.
pub type Reset<P> = NoInput<P, RESET>;

impl<P: HasForm> PageAction for SetField<P> {
    type Page = P;
    const ID: &'static str = P::IDS.set_field;
    const TITLE: &'static str = "Completar campo";
    fn description() -> &'static str {
        "Escribe un valor en un campo del formulario; contesta con los errores"
    }
    fn validate(&self) -> Result<(), CommandError> {
        if self.name.trim().is_empty() {
            return Err(CommandError::new("Falta el nombre del campo"));
        }
        Ok(())
    }
    fn into_msg(self) -> P::Msg {
        P::form_msg(FormMsg::Set {
            name: self.name,
            value: self.value,
        })
    }
}

impl<P: HasForm> PageAction for Submit<P> {
    type Page = P;
    const ID: &'static str = P::IDS.submit;
    const TITLE: &'static str = "Enviar formulario";
    fn description() -> &'static str {
        "Envía el formulario; contesta con los errores del envío ([] si se envió)"
    }
    fn into_msg(self) -> P::Msg {
        P::form_msg(FormMsg::Submit)
    }
}

impl<P: HasForm> PageAction for Reset<P> {
    type Page = P;
    const ID: &'static str = P::IDS.reset;
    const TITLE: &'static str = "Restablecer formulario";
    const SAFETY: Safety = Safety::Destructive;
    fn description() -> &'static str {
        "Vuelve el formulario a sus valores iniciales"
    }
    fn into_msg(self) -> P::Msg {
        P::form_msg(FormMsg::Reset)
    }
}

impl<P: HasForm> PageQuery for ErrorsQuery<P> {
    type Page = P;
    type Output = Errors;
    const ID: &'static str = P::IDS.errors;
    const TITLE: &'static str = "Errores del formulario";
    fn description() -> &'static str {
        "Errores actuales por campo, en orden ({field, message})"
    }
    fn run(page: &P, _: &P::State, _: Self) -> Result<Errors, CommandError> {
        Ok(page.form().errors())
    }
}

impl<P: HasForm> PageQuery for LastSubmitQuery<P> {
    type Page = P;
    type Output = Option<Errors>;
    const ID: &'static str = P::IDS.last_submit;
    const TITLE: &'static str = "Último envío del formulario";
    fn description() -> &'static str {
        "Errores del último envío: [] si se envió, null si nunca se intentó"
    }
    fn run(page: &P, _: &P::State, _: Self) -> Result<Option<Errors>, CommandError> {
        Ok(page.form().last_submit().cloned())
    }
}

impl<P: HasForm> PageQuery for ValuesQuery<P> {
    type Page = P;
    type Output = Values;
    const ID: &'static str = P::IDS.values;
    const TITLE: &'static str = "Valores del formulario";
    fn description() -> &'static str {
        "Valores normalizados por campo"
    }
    fn run(page: &P, _: &P::State, _: Self) -> Result<Values, CommandError> {
        Ok(page.form().values())
    }
}

/// Los comandos recién registrados, para publicarlos juntos.
pub struct Registered<'r, A: App> {
    registry: &'r mut CommandRegistry<A>,
    ids: FormIds,
}

impl<A: App> Registered<'_, A> {
    /// Publica los seis comandos a los agentes (MCP).
    pub fn expose(self) {
        let FormIds {
            set_field,
            errors,
            values,
            submit,
            reset,
            last_submit,
        } = self.ids;
        self.registry
            .expose_ids(&[set_field, errors, values, submit, reset, last_submit]);
    }
}

/// Registra los seis comandos de la página `P` (sin publicarlos).
pub fn register<P: HasForm, A: HasPage<P>>(registry: &mut CommandRegistry<A>) -> Registered<'_, A> {
    registry
        .page_action::<SetField<P>>()
        .returns_page::<ErrorsQuery<P>>();
    registry.page_query::<ErrorsQuery<P>>();
    registry.page_query::<ValuesQuery<P>>();
    registry.page_query::<LastSubmitQuery<P>>();
    registry
        .page_action::<Submit<P>>()
        .returns_page::<LastSubmitQuery<P>>();
    registry.page_action::<Reset<P>>();
    Registered {
        registry,
        ids: P::IDS,
    }
}
