//! What an app knows how to do, as a typed list (13 §7.2): [`Action`]s
//! produce a message for `update`, [`Query`]s read the model and change
//! nothing. The app declares them once in [`App::commands`]; menus,
//! shortcuts, the command palette, buttons and tests all read the same
//! [`CommandRegistry`].
//!
//! A command is its own input: it deserializes from JSON (serde) and
//! describes itself with JSON Schema 2020-12 (schemars), so scripts and
//! agents can call it (13, Paso 11). An action without parameters is a
//! unit struct; it takes `{}` and is published as a closed empty object.
//! Other inputs use `#[serde(deny_unknown_fields)]`.

mod registry;
mod schema;

use std::error::Error;
use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::element::KeyEquivalent;
use crate::runtime::App;

pub use registry::{CommandBuilder, CommandRegistry};

/// Shortcut of a command: a key plus modifiers, as a control's key
/// equivalent.
pub type Shortcut = KeyEquivalent;

/// Id of the command palette, a command of the framework.
pub const PALETTE: &str = "kai.paleta";

/// Effect of an action on domain data. Changing a filter, the route or
/// the selection is UI state: a `Mutating` action of the `ui.*` family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Safety {
    /// Changes domain data in an expected or reversible way.
    Mutating,
    /// Deletes, sends, pays or cannot be undone: an agent needs human
    /// confirmation.
    Destructive,
}

/// Whether a command can run now, evaluated after every `update`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    Enabled,
    /// Shown dimmed in menus and the palette; `reason` is the accessible
    /// description of buttons bound to it.
    Disabled {
        reason: String,
    },
    /// Not listed in menus or the palette.
    Hidden,
}

impl Availability {
    pub fn disabled(reason: impl Into<String>) -> Self {
        Self::Disabled {
            reason: reason.into(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        *self == Self::Enabled
    }
}

/// Why a command could not run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandError {
    pub message: String,
}

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for CommandError {}

/// A command that changes the app: it becomes a message that goes through
/// `update`, like a click.
///
/// ```
/// use kai::commands::{Action, Availability};
/// # use kai::element::{Element, div};
/// # use kai::runtime::SimpleApp;
/// # #[derive(Default)]
/// # struct Counter(u32);
/// # #[derive(Clone, Debug)]
/// # enum Msg { Reset }
/// # impl SimpleApp for Counter {
/// #     type Msg = Msg;
/// #     fn title(&self) -> String { String::new() }
/// #     fn update(&mut self, _: Msg) { self.0 = 0 }
/// #     fn view(&self) -> Element<Msg> { div() }
/// # }
///
/// #[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
/// struct Reset;
///
/// impl Action for Reset {
///     type App = Counter;
///     const ID: &'static str = "contador.reiniciar";
///     const TITLE: &'static str = "Reiniciar";
///     fn available(app: &Counter) -> Availability {
///         if app.0 == 0 {
///             Availability::disabled("El contador ya está en 0")
///         } else {
///             Availability::Enabled
///         }
///     }
///     fn into_msg(self) -> Msg {
///         Msg::Reset
///     }
/// }
/// ```
pub trait Action: Clone + DeserializeOwned + JsonSchema + 'static {
    type App: App;
    /// Lowercase segments separated by `.` (`clientes.crear`); `kai.*` is
    /// reserved.
    const ID: &'static str;
    const TITLE: &'static str;
    const SAFETY: Safety = Safety::Mutating;

    fn description() -> &'static str {
        ""
    }

    fn available(_app: &Self::App) -> Availability {
        Availability::Enabled
    }

    fn into_msg(self) -> <Self::App as App>::Msg;
}

/// A command that reads the app and changes nothing: no message, no
/// `update`.
pub trait Query: DeserializeOwned + JsonSchema + 'static {
    type App: App;
    type Output: Serialize + JsonSchema;
    const ID: &'static str;
    const TITLE: &'static str;

    fn description() -> &'static str {
        ""
    }

    fn run(app: &Self::App, input: Self) -> Result<Self::Output, CommandError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Action,
    Query,
}

/// A registered command and its availability after the last `update`:
/// what menus, the palette and buttons show.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandState {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub kind: CommandKind,
    /// `None` for queries and the framework's own `kai.*` commands.
    pub safety: Option<Safety>,
    pub shortcut: Option<Shortcut>,
    /// Menu bar menu it appears in (`"Contador"`).
    pub menu: Option<&'static str>,
    /// An action that `{}` does not satisfy: only scripts and agents can
    /// run it, so menus and the palette leave it out.
    pub needs_input: bool,
    pub availability: Availability,
}
