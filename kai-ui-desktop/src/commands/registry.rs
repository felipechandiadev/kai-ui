use std::any::type_name;

use serde_json::{Map, Value, json};

use super::schema::{decode, input_schema, output_schema};
use super::{
    Action, Availability, CommandError, CommandKind, CommandState, PALETTE, Query, Safety, Shortcut,
};
use crate::element::KeyEquivalent;
use crate::runtime::App;
use crate::ui_engine::Key;

/// Shortcuts a text field needs for editing; no command may take them, or
/// the native menu would steal them from the focused field.
const EDITING: [Shortcut; 6] = [
    KeyEquivalent::cmd(Key::A),
    KeyEquivalent::cmd(Key::C),
    KeyEquivalent::cmd(Key::X),
    KeyEquivalent::cmd(Key::V),
    KeyEquivalent::cmd(Key::Char('z')),
    KeyEquivalent::cmd_shift(Key::Char('z')),
];

type Decode<A> = fn(Value) -> Result<<A as App>::Msg, CommandError>;

/// A query run on a JSON input, with its output as JSON.
type RunQuery<A> = fn(&A, Value) -> Result<Value, CommandError>;

struct Entry<A: App> {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    kind: CommandKind,
    safety: Option<Safety>,
    shortcut: Option<Shortcut>,
    menu: Option<&'static str>,
    /// Type that declared it, for error messages.
    origin: &'static str,
    available: fn(&A) -> Availability,
    /// The message of an action for a JSON input; `None` for queries and
    /// `kai.*` commands (the session runs those itself).
    decode: Option<Decode<A>>,
    run_query: Option<RunQuery<A>>,
    /// `None` for `kai.*` commands, which are not described.
    input_schema: Option<fn() -> Value>,
    output_schema: Option<fn() -> Value>,
    needs_input: bool,
    /// Listed for agents (13 §7.4); nothing is by default.
    exposed: bool,
    /// Query whose output answers the caller after this action's `update`.
    returns: Option<&'static str>,
}

/// Every command of an app, declared once in [`App::commands`]:
///
/// ```ignore
/// fn commands(registry: &mut CommandRegistry<Self>) {
///     registry.action::<Incrementar>().shortcut(Shortcut::cmd(Key::Char('i'))).menu("Contador");
///     registry.query::<Saludo>();
/// }
/// ```
pub struct CommandRegistry<A: App> {
    entries: Vec<Entry<A>>,
}

/// Options of the command just registered.
pub struct CommandBuilder<'r, A: App> {
    entry: &'r mut Entry<A>,
}

impl<A: App> CommandBuilder<'_, A> {
    pub fn shortcut(self, shortcut: Shortcut) -> Self {
        self.entry.shortcut = Some(shortcut);
        self
    }

    /// Menu bar menu that lists it.
    pub fn menu(self, menu: &'static str) -> Self {
        self.entry.menu = Some(menu);
        self
    }

    /// Let agents see and call it (MCP). Nothing is exposed by default.
    pub fn expose(self) -> Self {
        self.entry.exposed = true;
        self
    }

    /// Answer whoever called this action (a script, an agent) with the
    /// output of query `Q`, run without input after `update`.
    pub fn returns<Q: Query<App = A>>(self) -> Self {
        self.entry.returns = Some(Q::ID);
        self.entry.output_schema = Some(output_schema::<Q::Output>);
        self
    }
}

fn always<A>(_: &A) -> Availability {
    Availability::Enabled
}

fn empty() -> Value {
    Value::Object(Map::new())
}

fn message_of<T: Action>(input: Value) -> Result<<T::App as App>::Msg, CommandError> {
    decode::<T>(T::ID, input).map(T::into_msg)
}

fn run_query_of<T: Query>(app: &T::App, input: Value) -> Result<Value, CommandError> {
    let output = T::run(app, decode::<T>(T::ID, input)?)?;
    serde_json::to_value(output)
        .map_err(|error| CommandError::new(format!("salida inválida de {}: {error}", T::ID)))
}

impl<A: App> Default for CommandRegistry<A> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<A: App> CommandRegistry<A> {
    /// The app's commands ([`App::commands`]) plus the framework's own:
    /// the command palette (`kai.paleta`, ⇧⌘P).
    pub fn of() -> Self {
        let mut registry = Self::default();
        A::commands(&mut registry);
        registry.entries.push(Entry {
            id: PALETTE,
            title: "Paleta de comandos",
            description: "Buscar y ejecutar un comando",
            kind: CommandKind::Action,
            safety: None,
            shortcut: Some(KeyEquivalent::cmd_shift(Key::Char('p'))),
            menu: Some("Ver"),
            origin: "kai",
            available: always,
            decode: None,
            run_query: None,
            input_schema: None,
            output_schema: None,
            needs_input: false,
            exposed: false,
            returns: None,
        });
        registry
    }

    fn push(&mut self, entry: Entry<A>) -> CommandBuilder<'_, A> {
        self.entries.push(entry);
        let entry = self.entries.last_mut().expect("just pushed");
        CommandBuilder { entry }
    }

    pub fn action<T: Action<App = A>>(&mut self) -> CommandBuilder<'_, A> {
        self.push(Entry {
            id: T::ID,
            title: T::TITLE,
            description: T::description(),
            kind: CommandKind::Action,
            safety: Some(T::SAFETY),
            shortcut: None,
            menu: None,
            origin: type_name::<T>(),
            available: T::available,
            decode: Some(message_of::<T>),
            run_query: None,
            input_schema: Some(input_schema::<T>),
            output_schema: None,
            needs_input: decode::<T>(T::ID, empty()).is_err(),
            exposed: false,
            returns: None,
        })
    }

    pub fn query<T: Query<App = A>>(&mut self) -> CommandBuilder<'_, A> {
        self.push(Entry {
            id: T::ID,
            title: T::TITLE,
            description: T::description(),
            kind: CommandKind::Query,
            safety: None,
            shortcut: None,
            menu: None,
            origin: type_name::<T>(),
            available: always,
            decode: None,
            run_query: Some(run_query_of::<T>),
            input_schema: Some(input_schema::<T>),
            output_schema: Some(output_schema::<T::Output>),
            needs_input: decode::<T>(T::ID, empty()).is_err(),
            exposed: false,
            returns: None,
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.entries.iter().any(|entry| entry.id == id)
    }

    /// Every command with its availability for `app`, in registration
    /// order.
    pub fn states(&self, app: &A) -> Vec<CommandState> {
        self.entries
            .iter()
            .map(|entry| CommandState {
                id: entry.id,
                title: entry.title,
                description: entry.description,
                kind: entry.kind,
                safety: entry.safety,
                shortcut: entry.shortcut,
                menu: entry.menu,
                needs_input: entry.needs_input,
                exposed: entry.exposed,
                availability: (entry.available)(app),
            })
            .collect()
    }

    fn entry(&self, id: &str) -> Result<&Entry<A>, CommandError> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CommandError::new(format!("comando desconocido: {id}")))
    }

    /// Availability of action `id` for `app`; an error for unknown ids
    /// and queries.
    pub fn availability(&self, app: &A, id: &str) -> Result<Availability, CommandError> {
        let entry = self.entry(id)?;
        if entry.kind != CommandKind::Action {
            return Err(CommandError::new(format!(
                "{id} es una consulta, no una acción"
            )));
        }
        Ok((entry.available)(app))
    }

    /// Whether query `id` is registered.
    pub fn has_query(&self, id: &str) -> bool {
        self.entry(id)
            .is_ok_and(|entry| entry.kind == CommandKind::Query)
    }

    /// The message of action `id` without parameters (input `{}`) if it
    /// can run now.
    pub fn message(&self, app: &A, id: &str) -> Result<A::Msg, CommandError> {
        self.message_from(app, id, empty())
    }

    /// The message of action `id` for a JSON `input` if it can run now:
    /// availability first, then the input.
    pub fn message_from(&self, app: &A, id: &str, input: Value) -> Result<A::Msg, CommandError> {
        let entry = self.entry(id)?;
        let decode = entry
            .decode
            .ok_or_else(|| CommandError::new(format!("{id} no produce un mensaje")))?;
        match (entry.available)(app) {
            Availability::Enabled => decode(input),
            Availability::Disabled { reason } => Err(CommandError::new(reason)),
            Availability::Hidden => Err(CommandError::new(format!("{id} no está disponible"))),
        }
    }

    /// Run query `id` on a JSON `input`; its output as JSON.
    pub fn run_query(&self, app: &A, id: &str, input: Value) -> Result<Value, CommandError> {
        let run = self
            .entry(id)?
            .run_query
            .ok_or_else(|| CommandError::new(format!("{id} es una acción, no una consulta")))?;
        run(app, input)
    }

    /// JSON Schema 2020-12 of the input of command `id`, without
    /// `$schema`; `None` for unknown ids and `kai.*` commands.
    pub fn input_schema(&self, id: &str) -> Option<Value> {
        self.entry(id).ok()?.input_schema.map(|schema| schema())
    }

    /// The query that answers whoever calls action `id`
    /// ([`CommandBuilder::returns`]).
    pub fn returns(&self, id: &str) -> Option<&'static str> {
        self.entry(id).ok()?.returns
    }

    /// JSON Schema 2020-12 of the output of query `id`, or of the query
    /// that answers action `id`.
    pub fn output_schema(&self, id: &str) -> Option<Value> {
        self.entry(id).ok()?.output_schema.map(|schema| schema())
    }

    /// The app's commands as JSON, in registration order, without the
    /// framework's `kai.*`: id, title, description, kind, safety, whether
    /// agents see it, and the schemas of input and output.
    pub(super) fn descriptions(&self) -> Vec<Value> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let mut command = Map::new();
                command.insert("id".into(), json!(entry.id));
                command.insert("title".into(), json!(entry.title));
                if !entry.description.is_empty() {
                    command.insert("description".into(), json!(entry.description));
                }
                let kind = match entry.kind {
                    CommandKind::Action => "action",
                    CommandKind::Query => "query",
                };
                command.insert("kind".into(), json!(kind));
                command.insert("exposed".into(), json!(entry.exposed));
                if let Some(safety) = entry.safety {
                    let safety = match safety {
                        Safety::Mutating => "mutating",
                        Safety::Destructive => "destructive",
                    };
                    command.insert("safety".into(), json!(safety));
                }
                command.insert("inputSchema".into(), (entry.input_schema?)());
                if let Some(output) = entry.output_schema {
                    command.insert("outputSchema".into(), output());
                }
                Some(Value::Object(command))
            })
            .collect()
    }

    /// Repeated ids and shortcuts (with the types that declared them),
    /// app ids in `kai.*`, malformed ids and shortcuts reserved for text
    /// editing. Empty when the registry is sound.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            let earlier = &self.entries[..index];
            if let Some(first) = earlier.iter().find(|other| other.id == entry.id) {
                problems.push(format!(
                    "comando \"{}\" repetido: {} y {}",
                    entry.id, first.origin, entry.origin
                ));
            }
            if let Some(shortcut) = entry.shortcut
                && let Some(first) = earlier
                    .iter()
                    .find(|other| other.shortcut == Some(shortcut))
            {
                problems.push(format!(
                    "atajo {} repetido: {} ({}) y {} ({})",
                    shortcut.label(),
                    first.id,
                    first.origin,
                    entry.id,
                    entry.origin
                ));
            }
            if entry.origin == "kai" {
                continue;
            }
            if entry.id.starts_with("kai.") {
                problems.push(format!(
                    "comando \"{}\" ({}): los ids kai.* son del framework",
                    entry.id, entry.origin
                ));
            }
            if !well_formed(entry.id) {
                problems.push(format!(
                    "comando \"{}\" ({}): el id debe ser segmentos en minúsculas separados por \
                     punto",
                    entry.id, entry.origin
                ));
            }
            if let Some(shortcut) = entry.shortcut
                && EDITING.contains(&shortcut)
            {
                problems.push(format!(
                    "comando \"{}\" ({}): el atajo {} es de la edición de texto",
                    entry.id,
                    entry.origin,
                    shortcut.label()
                ));
            }
            if entry.needs_input && (entry.shortcut.is_some() || entry.menu.is_some()) {
                problems.push(format!(
                    "comando \"{}\" ({}): necesita entrada, así que no puede tener atajo ni menú",
                    entry.id, entry.origin
                ));
            }
            if let Some(query) = entry.returns {
                let target = self.entries.iter().find(|other| other.id == query);
                let problem = if entry.kind != CommandKind::Action {
                    Some("solo una acción responde con una consulta".to_owned())
                } else {
                    match target {
                        None => Some(format!("responde con \"{query}\", que no está registrada")),
                        Some(target) if target.kind != CommandKind::Query => {
                            Some(format!("responde con \"{query}\", que no es una consulta"))
                        }
                        Some(target) if target.needs_input => {
                            Some(format!("responde con \"{query}\", que necesita entrada"))
                        }
                        Some(_) => None,
                    }
                };
                if let Some(problem) = problem {
                    problems.push(format!(
                        "comando \"{}\" ({}): {problem}",
                        entry.id, entry.origin
                    ));
                }
            }
        }
        problems
    }
}

/// `clientes.crear`, `form.alternar_contrasena`: non-empty segments of
/// lowercase ASCII letters, digits and `_`.
fn well_formed(id: &str) -> bool {
    id.split('.').all(|segment| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    })
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;

    use super::super::{Action, Availability, CommandKind, PALETTE, Query, Shortcut};
    use super::CommandRegistry;
    use crate::element::{Element, div};
    use crate::runtime::SimpleApp;
    use crate::ui_engine::Key;

    #[derive(Default)]
    struct Counter(u32);

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Add,
        AddMany(u32),
        Reset,
    }

    impl SimpleApp for Counter {
        type Msg = Msg;
        fn title(&self) -> String {
            "Contador".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Add => self.0 += 1,
                Msg::AddMany(amount) => self.0 += amount,
                Msg::Reset => self.0 = 0,
            }
        }
        fn view(&self) -> Element<Msg> {
            div()
        }
    }

    #[derive(Clone, Deserialize, JsonSchema)]
    struct Add;

    impl Action for Add {
        type App = Counter;
        const ID: &'static str = "contador.sumar";
        const TITLE: &'static str = "Sumar";
        fn into_msg(self) -> Msg {
            Msg::Add
        }
    }

    #[derive(Clone, Deserialize, JsonSchema)]
    struct Reset;

    impl Action for Reset {
        type App = Counter;
        const ID: &'static str = "contador.reiniciar";
        const TITLE: &'static str = "Reiniciar";
        fn available(app: &Counter) -> Availability {
            if app.0 == 0 {
                Availability::disabled("Ya está en 0")
            } else {
                Availability::Enabled
            }
        }
        fn into_msg(self) -> Msg {
            Msg::Reset
        }
    }

    /// Same id as `Add`.
    #[derive(Clone, Deserialize, JsonSchema)]
    struct Twin;

    impl Action for Twin {
        type App = Counter;
        const ID: &'static str = "contador.sumar";
        const TITLE: &'static str = "Sumar otra vez";
        fn into_msg(self) -> Msg {
            Msg::Add
        }
    }

    #[derive(Clone, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct AddMany {
        amount: u32,
    }

    impl Action for AddMany {
        type App = Counter;
        const ID: &'static str = "contador.sumar_varios";
        const TITLE: &'static str = "Sumar varios";
        fn into_msg(self) -> Msg {
            Msg::AddMany(self.amount)
        }
    }

    #[derive(Deserialize, JsonSchema)]
    struct Total;

    impl Query for Total {
        type App = Counter;
        type Output = u32;
        const ID: &'static str = "contador.total";
        const TITLE: &'static str = "Total";
        fn run(app: &Counter, _: Self) -> Result<u32, super::CommandError> {
            Ok(app.0)
        }
    }

    fn registry(build: impl FnOnce(&mut CommandRegistry<Counter>)) -> CommandRegistry<Counter> {
        let mut registry = CommandRegistry::default();
        build(&mut registry);
        registry
    }

    #[test]
    fn duplicate_command_ids_are_reported_with_both_types() {
        let registry = registry(|r| {
            r.action::<Add>();
            r.action::<Twin>();
        });
        let problems = registry.validate();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("\"contador.sumar\" repetido"));
        assert!(problems[0].contains("::Add") && problems[0].contains("::Twin"));
    }

    #[test]
    fn duplicate_shortcuts_are_reported() {
        let cmd_k = Shortcut::cmd(Key::Char('k'));
        let registry = registry(|r| {
            r.action::<Add>().shortcut(cmd_k);
            r.action::<Reset>().shortcut(cmd_k);
        });
        let problems = registry.validate();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("atajo ⌘K repetido"));
        assert!(problems[0].contains("contador.sumar") && problems[0].contains("::Reset"));
    }

    #[test]
    fn kai_ids_and_editing_shortcuts_are_reserved() {
        #[derive(Clone, Deserialize, JsonSchema)]
        struct Intruder;
        impl Action for Intruder {
            type App = Counter;
            const ID: &'static str = "kai.paleta";
            const TITLE: &'static str = "Otra paleta";
            fn into_msg(self) -> Msg {
                Msg::Add
            }
        }
        #[derive(Clone, Deserialize, JsonSchema)]
        struct Shouting;
        impl Action for Shouting {
            type App = Counter;
            const ID: &'static str = "Contador.Gritar";
            const TITLE: &'static str = "Gritar";
            fn into_msg(self) -> Msg {
                Msg::Add
            }
        }
        let mut registry = registry(|r| {
            r.action::<Intruder>();
            r.action::<Shouting>();
            r.action::<Add>().shortcut(Shortcut::cmd(Key::C));
        });
        let problems = registry.validate();
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].contains("kai.* son del framework"));
        assert!(problems[1].contains("\"Contador.Gritar\""));
        assert!(problems[2].contains("⌘C es de la edición de texto"));

        // The framework's palette is fine on its own.
        registry = CommandRegistry::of();
        assert!(registry.contains(PALETTE));
        assert_eq!(registry.validate(), Vec::<String>::new());
    }

    #[test]
    fn availability_and_messages_follow_the_model() {
        let registry = registry(|r| {
            r.action::<Add>().menu("Contador");
            r.action::<Reset>().menu("Contador");
        });
        let mut app = Counter::default();
        let reset = |app: &Counter| registry.states(app)[1].availability.clone();
        assert_eq!(reset(&app), Availability::disabled("Ya está en 0"));
        assert_eq!(
            registry
                .message(&app, "contador.reiniciar")
                .unwrap_err()
                .message,
            "Ya está en 0"
        );
        app.update(registry.message(&app, "contador.sumar").unwrap());
        assert_eq!(reset(&app), Availability::Enabled);
        assert_eq!(registry.message(&app, "contador.reiniciar"), Ok(Msg::Reset));
        assert!(registry.message(&app, "contador.nada").is_err());
    }

    #[test]
    fn query_runs_without_msg() {
        let registry = registry(|r| {
            r.query::<Total>();
        });
        let state = &registry.states(&Counter(3))[0];
        assert_eq!(state.kind, CommandKind::Query);
        assert_eq!(state.safety, None);
        assert!(registry.message(&Counter(3), Total::ID).is_err());
        assert_eq!(Total::run(&Counter(3), Total), Ok(3));
        assert_eq!(
            registry.run_query(&Counter(3), Total::ID, json!({})),
            Ok(json!(3))
        );
        assert_eq!(
            registry.output_schema(Total::ID).unwrap()["type"],
            "integer"
        );
    }

    #[test]
    fn actions_with_input_decode_json_and_need_it() {
        let registry = registry(|r| {
            r.action::<Add>();
            r.action::<AddMany>();
        });
        let app = Counter::default();
        let states = registry.states(&app);
        assert!(!states[0].needs_input);
        assert!(states[1].needs_input);
        assert_eq!(
            registry.message_from(&app, AddMany::ID, json!({"amount": 3})),
            Ok(Msg::AddMany(3))
        );
        assert_eq!(
            registry.message(&app, AddMany::ID).unwrap_err().message,
            "entrada inválida para contador.sumar_varios: missing field `amount`"
        );
        assert_eq!(
            registry.message_from(&app, Add::ID, json!(null)),
            Ok(Msg::Add)
        );
        assert_eq!(
            registry
                .run_query(&app, Add::ID, json!({}))
                .unwrap_err()
                .message,
            "contador.sumar es una acción, no una consulta"
        );
        let schema = registry.input_schema(AddMany::ID).unwrap();
        assert_eq!(schema["required"], json!(["amount"]));
        assert_eq!(registry.input_schema(PALETTE), None);
    }

    #[test]
    fn exposed_actions_can_answer_with_a_query() {
        let mut registry = registry(|r| {
            r.action::<Add>().expose().returns::<Total>();
            r.action::<Reset>();
            r.query::<Total>().expose();
        });
        assert_eq!(registry.validate(), Vec::<String>::new());
        let states = registry.states(&Counter(0));
        let exposed: Vec<bool> = states.iter().map(|state| state.exposed).collect();
        assert_eq!(exposed, [true, false, true]);
        assert_eq!(registry.returns(Add::ID), Some(Total::ID));
        assert_eq!(registry.returns(Reset::ID), None);
        assert_eq!(registry.output_schema(Add::ID).unwrap()["type"], "integer");
        assert_eq!(registry.descriptions()[1]["exposed"], false);

        registry = super::CommandRegistry::default();
        registry.action::<Add>().returns::<Total>();
        let problems = registry.validate();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("\"contador.total\", que no está registrada"));

        registry = super::CommandRegistry::default();
        registry.query::<Total>().returns::<Total>();
        let problems = registry.validate();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("solo una acción responde con una consulta"));
    }

    #[test]
    fn actions_that_need_input_have_no_shortcut_or_menu() {
        let registry = registry(|r| {
            r.action::<AddMany>().menu("Contador");
        });
        let problems = registry.validate();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("necesita entrada"), "{problems:?}");
    }
}
