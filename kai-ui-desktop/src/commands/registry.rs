use std::any::type_name;

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
    /// The message of an action without parameters; `None` for queries
    /// and `kai.*` commands (the session runs those itself).
    message: Option<fn() -> A::Msg>,
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
}

fn always<A>(_: &A) -> Availability {
    Availability::Enabled
}

fn message_of<T: Action + Default>() -> <T::App as App>::Msg {
    T::default().into_msg()
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
            message: None,
        });
        registry
    }

    fn push(&mut self, entry: Entry<A>) -> CommandBuilder<'_, A> {
        self.entries.push(entry);
        let entry = self.entries.last_mut().expect("just pushed");
        CommandBuilder { entry }
    }

    pub fn action<T: Action<App = A> + Default>(&mut self) -> CommandBuilder<'_, A> {
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
            message: Some(message_of::<T>),
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
            message: None,
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
                availability: (entry.available)(app),
            })
            .collect()
    }

    /// The message of action `id` if it can run now.
    pub fn message(&self, app: &A, id: &str) -> Result<A::Msg, CommandError> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CommandError::new(format!("comando desconocido: {id}")))?;
        let message = entry
            .message
            .ok_or_else(|| CommandError::new(format!("{id} no produce un mensaje")))?;
        match (entry.available)(app) {
            Availability::Enabled => Ok(message()),
            Availability::Disabled { reason } => Err(CommandError::new(reason)),
            Availability::Hidden => Err(CommandError::new(format!("{id} no está disponible"))),
        }
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
                Msg::Reset => self.0 = 0,
            }
        }
        fn view(&self) -> Element<Msg> {
            div()
        }
    }

    #[derive(Clone, Default)]
    struct Add;

    impl Action for Add {
        type App = Counter;
        const ID: &'static str = "contador.sumar";
        const TITLE: &'static str = "Sumar";
        fn into_msg(self) -> Msg {
            Msg::Add
        }
    }

    #[derive(Clone, Default)]
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
    #[derive(Clone, Default)]
    struct Twin;

    impl Action for Twin {
        type App = Counter;
        const ID: &'static str = "contador.sumar";
        const TITLE: &'static str = "Sumar otra vez";
        fn into_msg(self) -> Msg {
            Msg::Add
        }
    }

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
        #[derive(Clone, Default)]
        struct Intruder;
        impl Action for Intruder {
            type App = Counter;
            const ID: &'static str = "kai.paleta";
            const TITLE: &'static str = "Otra paleta";
            fn into_msg(self) -> Msg {
                Msg::Add
            }
        }
        #[derive(Clone, Default)]
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
    }
}
