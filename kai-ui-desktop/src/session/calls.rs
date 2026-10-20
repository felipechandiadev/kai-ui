//! Calls through JSON (13, Paso 13): scripts and agents run commands with
//! [`Session::call`] and [`Session::agent_call`], and every action they run
//! lands in the [`AuditLog`]. An agent never runs a `Destructive` action
//! without a person: the session shows a modal dialog in the window and
//! the call waits for «Permitir una vez», «Permitir en esta sesión» or
//! «Rechazar». Other agent calls wait behind it, in order.

use std::collections::{HashSet, VecDeque};
use std::time::{Instant, SystemTime};

use serde_json::Value;

use super::Session;
use crate::commands::{
    AuditEntry, AuditLog, CommandError, CommandKind, Origin, Outcome, Safety, redact,
};
use crate::components::{ButtonVariant, button, dialog, themed};
use crate::devtools::MsgSource;
use crate::element::{Element, Role, WidgetId, div, text};
use crate::runtime::App;
use crate::ui_engine::Intent;

/// Test id of the confirmation dialog. Its buttons are
/// `kai.confirmar.sesion`, `kai.confirmar.una_vez` and
/// `kai.confirmar.rechazar`.
pub const CONFIRMATION: &str = "kai.confirmar";

/// Test id of the «agent connected» indicator.
pub const AGENT_STATUS: &str = "kai.agente";

/// Longest summary of the arguments in the confirmation dialog.
const SUMMARY_LIMIT: usize = 160;

/// A command an agent asked for; `reply` gets the result once it ran, was
/// rejected or was cancelled.
pub(crate) struct AgentCall {
    pub id: u64,
    pub client: String,
    pub command: String,
    pub input: Value,
    pub reply: Box<dyn FnOnce(Result<Value, CommandError>) + Send>,
}

/// What the person chose in the confirmation dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    AllowOnce,
    AllowForSession,
    Deny,
}

/// The dialog's buttons, in order: label and test id suffix.
const DECISIONS: [(Decision, &str, &str); 3] = [
    (
        Decision::AllowForSession,
        "Permitir en esta sesión",
        "sesion",
    ),
    (Decision::AllowOnce, "Permitir una vez", "una_vez"),
    (Decision::Deny, "Rechazar", "rechazar"),
];

/// A call and when it arrived.
struct Pending {
    call: AgentCall,
    at: SystemTime,
    started: Instant,
}

struct Confirmation {
    pending: Pending,
    /// Focus to give back on close.
    previous_focus: Option<WidgetId>,
    panel: WidgetId,
    buttons: Vec<(WidgetId, Decision)>,
}

struct AgentStatus {
    client: String,
    /// Title of the last action the agent ran.
    last: Option<&'static str>,
}

/// Agent state of a session.
#[derive(Default)]
pub(super) struct Agents {
    confirmation: Option<Confirmation>,
    waiting: VecDeque<Pending>,
    /// Destructive actions the person allowed for the rest of the session.
    allowed: HashSet<String>,
    status: Option<AgentStatus>,
    log: AuditLog,
}

impl<A: App> Session<A> {
    /// Run command `id` on a JSON `input`, as a script does: a query
    /// returns its output; an action that can run sends its message and
    /// returns the output of its `returns` query, or `null`. Actions are
    /// audited; nothing asks for confirmation.
    pub fn call(&mut self, id: &str, input: Value) -> Result<Value, CommandError> {
        self.with_source(MsgSource::Script, |session| {
            session.call_as(Origin::Script, id, input, SystemTime::now(), Instant::now())
        })
    }

    /// Run an agent's call. Commands not exposed are refused. A
    /// `Destructive` action opens the confirmation dialog unless the
    /// person allowed it for the session; while the dialog is open, other
    /// calls wait. `reply` may run later, from [`Self::decide`] or
    /// [`Self::cancel_agent_call`].
    pub fn agent_call(&mut self, call: AgentCall) {
        let pending = Pending {
            call,
            at: SystemTime::now(),
            started: Instant::now(),
        };
        if self.agents.confirmation.is_some() {
            self.agents.waiting.push_back(pending);
        } else {
            self.start(pending);
        }
    }

    /// The agent stopped waiting for call `id`: close its dialog or drop it
    /// from the queue. The model does not change.
    pub fn cancel_agent_call(&mut self, id: u64) {
        let open = self.agents.confirmation.as_ref();
        if open.is_some_and(|confirmation| confirmation.pending.call.id == id) {
            if let Some(confirmation) = self.close_confirmation() {
                self.cancelled(confirmation.pending);
            }
            self.next_waiting();
        } else if let Some(index) = self
            .agents
            .waiting
            .iter()
            .position(|pending| pending.call.id == id)
            && let Some(pending) = self.agents.waiting.remove(index)
        {
            self.cancelled(pending);
        }
    }

    /// An agent connected: show the indicator.
    pub fn agent_connected(&mut self, client: &str) {
        self.agents.status = Some(AgentStatus {
            client: client.to_owned(),
            last: None,
        });
        self.invalidate();
    }

    pub fn audit_log(&self) -> &AuditLog {
        &self.agents.log
    }

    /// Whether the confirmation dialog is open.
    pub fn confirmation_open(&self) -> bool {
        self.agents.confirmation.is_some()
    }

    /// Answer the open confirmation dialog, then start the calls that
    /// waited behind it.
    pub(crate) fn decide(&mut self, decision: Decision) {
        let Some(confirmation) = self.close_confirmation() else {
            return;
        };
        let pending = confirmation.pending;
        match decision {
            Decision::Deny => {
                let title = self
                    .command_state(&pending.call.command)
                    .map_or(pending.call.command.as_str(), |state| state.title)
                    .to_owned();
                self.finish(
                    pending,
                    Outcome::Denied,
                    Err(CommandError::new(format!("La persona rechazó «{title}»"))),
                );
            }
            Decision::AllowForSession => {
                self.agents.allowed.insert(pending.call.command.clone());
                self.run(pending);
            }
            Decision::AllowOnce => self.run(pending),
        }
        self.next_waiting();
    }

    /// The decision of confirmation button `widget`, if it is one.
    pub(super) fn confirmation_button(&self, widget: WidgetId) -> Option<Decision> {
        let confirmation = self.agents.confirmation.as_ref()?;
        confirmation
            .buttons
            .iter()
            .find(|(id, _)| *id == widget)
            .map(|&(_, decision)| decision)
    }

    /// The panel of the open confirmation dialog: clicks outside it do
    /// nothing.
    pub(super) fn confirmation_panel(&self) -> Option<WidgetId> {
        self.agents.confirmation.as_ref().map(|c| c.panel)
    }

    /// Append the indicator and the confirmation dialog to the tree. The
    /// indicator goes first so an open dialog or palette stays last.
    pub(super) fn add_agent_status(&mut self) {
        if let Some(status) = &self.agents.status {
            self.tree.children.push(agent_status(status));
            self.tree.assign_ids();
        }
    }

    pub(super) fn add_confirmation(&mut self) {
        let Some(confirmation) = &self.agents.confirmation else {
            return;
        };
        let call = &confirmation.pending.call;
        let (title, description) = self
            .command_state(&call.command)
            .map_or((call.command.as_str(), ""), |state| {
                (state.title, state.description)
            });
        let input = self.redacted(&call.command, &call.input);
        let element = confirmation_dialog(title, description, &call.client, &call.command, &input);
        self.tree.children.push(element);
        self.tree.assign_ids();
        let find = |test_id: &str| {
            self.tree
                .find_test_id(test_id)
                .map_or(WidgetId(0), |element| element.id)
        };
        let panel = find(&format!("{CONFIRMATION}.panel"));
        let buttons = DECISIONS
            .iter()
            .map(|&(decision, _, part)| (find(&format!("{CONFIRMATION}.{part}")), decision))
            .collect();
        if let Some(confirmation) = &mut self.agents.confirmation {
            confirmation.panel = panel;
            confirmation.buttons = buttons;
        }
    }

    fn start(&mut self, pending: Pending) {
        let id = pending.call.command.as_str();
        let Some(state) = self.command_state(id).filter(|state| state.exposed) else {
            let error = CommandError::new(format!("{id} no está expuesto a agentes"));
            let outcome = Outcome::Failed(error.message.clone());
            self.finish(pending, outcome, Err(error));
            return;
        };
        let confirm = state.kind == CommandKind::Action
            && state.safety == Some(Safety::Destructive)
            && !self.agents.allowed.contains(id);
        if !confirm {
            self.run(pending);
            return;
        }
        // Bad input or an unavailable action fails now, without asking.
        match self
            .registry
            .message_from(&self.app, id, pending.call.input.clone())
        {
            Ok(_) => self.open_confirmation(pending),
            Err(error) => {
                let outcome = Outcome::Failed(error.message.clone());
                self.finish(pending, outcome, Err(error));
            }
        }
    }

    fn run(&mut self, pending: Pending) {
        let Pending { call, at, started } = pending;
        let origin = Origin::Agent {
            client: call.client.clone(),
        };
        let source = MsgSource::Agent(call.client.clone());
        let result = self.with_source(source, |session| {
            session.call_as(origin, &call.command, call.input, at, started)
        });
        if result.is_ok()
            && let Some(state) = self.command_state(&call.command)
            && state.kind == CommandKind::Action
        {
            let title = state.title;
            if let Some(status) = &mut self.agents.status {
                status.last = Some(title);
                self.invalidate();
            }
        }
        (call.reply)(result);
    }

    fn call_as(
        &mut self,
        origin: Origin,
        id: &str,
        input: Value,
        at: SystemTime,
        started: Instant,
    ) -> Result<Value, CommandError> {
        if self.registry.has_query(id) {
            return self.registry.run_query(&self.app, id, input);
        }
        let redacted = self.redacted(id, &input);
        let result = self.run_action(id, input);
        let outcome = match &result {
            Ok(_) => Outcome::Done,
            Err(error) => Outcome::Failed(error.message.clone()),
        };
        self.agents.log.push(AuditEntry {
            at,
            origin,
            command: id.to_owned(),
            input: redacted,
            outcome,
            duration: started.elapsed(),
        });
        result
    }

    fn run_action(&mut self, id: &str, input: Value) -> Result<Value, CommandError> {
        let msg = self.registry.message_from(&self.app, id, input)?;
        self.send(msg);
        match self.registry.returns(id) {
            Some(query) => {
                self.registry
                    .run_query(&self.app, query, Value::Object(Default::default()))
            }
            None => Ok(Value::Null),
        }
    }

    /// Record a call that did not run and answer it.
    fn finish(&mut self, pending: Pending, outcome: Outcome, result: Result<Value, CommandError>) {
        let Pending { call, at, started } = pending;
        self.agents.log.push(AuditEntry {
            at,
            origin: Origin::Agent {
                client: call.client.clone(),
            },
            command: call.command.clone(),
            input: self.redacted(&call.command, &call.input),
            outcome,
            duration: started.elapsed(),
        });
        (call.reply)(result);
    }

    fn cancelled(&mut self, pending: Pending) {
        let error = CommandError::new(format!(
            "{} se canceló antes de ejecutarse",
            pending.call.command
        ));
        self.finish(pending, Outcome::Cancelled, Err(error));
    }

    fn redacted(&self, id: &str, input: &Value) -> Value {
        match self.registry.input_schema(id) {
            Some(schema) => redact(input, &schema),
            None => input.clone(),
        }
    }

    fn open_confirmation(&mut self, pending: Pending) {
        self.close_palette();
        self.agents.confirmation = Some(Confirmation {
            pending,
            previous_focus: self.focused(),
            panel: WidgetId(0),
            buttons: Vec::new(),
        });
        self.rebuild();
        self.relayout();
        let deny = self.agents.confirmation.as_ref().and_then(|confirmation| {
            confirmation
                .buttons
                .iter()
                .find(|(_, decision)| *decision == Decision::Deny)
                .map(|&(id, _)| id)
        });
        self.apply_intent(Intent::SetFocus(deny));
        self.invalidate();
    }

    fn close_confirmation(&mut self) -> Option<Confirmation> {
        let confirmation = self.agents.confirmation.take()?;
        self.rebuild();
        self.relayout();
        self.apply_intent(Intent::SetFocus(confirmation.previous_focus));
        self.invalidate();
        Some(confirmation)
    }

    fn next_waiting(&mut self) {
        while self.agents.confirmation.is_none()
            && let Some(pending) = self.agents.waiting.pop_front()
        {
            self.start(pending);
        }
    }
}

fn agent_status<Msg>(status: &AgentStatus) -> Element<Msg> {
    let label = match status.last {
        Some(last) => format!("Agente: {} · {last}", status.client),
        None => format!("Agente: {}", status.client),
    };
    div()
        .class(&format!(
            "absolute bottom-3 right-3 z-40 px-3 py-1 rounded-full border {}",
            themed(
                "text-xs text-muted-foreground border-border bg-background",
                "text-caption1 text-secondary-label border-separator bg-window",
            )
        ))
        .role(Role::Status)
        .label(label.clone())
        .test_id(AGENT_STATUS)
        .child(text(label))
}

fn confirmation_dialog<Msg>(
    title: &str,
    description: &str,
    client: &str,
    command: &str,
    input: &Value,
) -> Element<Msg> {
    let line = |content: String| {
        text(content).class(&themed("text-sm text-foreground", "text-body text-label"))
    };
    let mut body = div()
        .class("flex flex-col gap-2")
        .child(line(format!("{client} quiere ejecutar {command}.")));
    if !description.is_empty() {
        body = body.child(line(description.to_owned()));
    }
    if let Some(summary) = summary(input) {
        body = body.child(line(format!("Datos: {summary}")));
    }
    let actions = DECISIONS.iter().map(|&(decision, label, part)| {
        let variant = if decision == Decision::Deny {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Outlined
        };
        let mut button = button(label, variant).test_id(format!("{CONFIRMATION}.{part}"));
        button.command = Some(CONFIRMATION);
        button
    });
    dialog(&format!("¿Permitir «{title}»?"), body, actions).test_id(CONFIRMATION)
}

/// `campo: valor, ...` of an object input, shortened; `None` without
/// fields.
fn summary(input: &Value) -> Option<String> {
    let fields = input.as_object().filter(|fields| !fields.is_empty())?;
    let summary = fields
        .iter()
        .map(|(name, value)| match value {
            Value::String(text) => format!("{name}: {text}"),
            other => format!("{name}: {other}"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if summary.chars().count() <= SUMMARY_LIMIT {
        return Some(summary);
    }
    let short: String = summary.chars().take(SUMMARY_LIMIT).collect();
    Some(format!("{short}…"))
}
