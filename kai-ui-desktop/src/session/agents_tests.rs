//! Scripts and agents: JSON calls, queries, the confirmation dialog for
//! destructive actions, the call queue, the audit log and the agent
//! indicator. They run on [`Sample`], not the demo (docs/dev/27).

use serde_json::json;

use crate::commands::{CommandError, Origin, Outcome};
use crate::element::Role;
use crate::testing::sample_app::{Increment, LastSubmission, Msg, Sample, Submit};
use crate::testing::{By, Confirm, Key, Modifiers, TestApp};

const CMD_SHIFT: Modifiers = Modifiers {
    command: true,
    shift: true,
    ..Modifiers::NONE
};

fn agent(client: &str) -> Origin {
    Origin::Agent {
        client: client.to_owned(),
    }
}

fn denied() -> Option<Result<serde_json::Value, CommandError>> {
    Some(Err(CommandError::new("La persona rechazó «Reiniciar»")))
}

const DONE: Option<Result<serde_json::Value, CommandError>> = Some(Ok(serde_json::Value::Null));

#[test]
fn json_calls_run_actions_and_queries() {
    let mut app = TestApp::new(Sample::default());
    assert_eq!(
        app.call("counter.add", json!({"count": 3})),
        Ok(json!(null))
    );
    assert_eq!(app.app().clicks, 3);
    assert_eq!(app.messages(), [Msg::Add(3)]);
    let error = app.call("counter.add", json!({})).unwrap_err();
    assert_eq!(
        error.message,
        "entrada inválida para counter.add: missing field `count`"
    );
    assert!(app.call("counter.add", json!({"count": -1})).is_err());

    assert_eq!(app.call("form.last_submission", json!({})), Ok(json!(null)));
    let error = app.call("form.submit", json!({})).unwrap_err();
    assert_eq!(error.message, "Falta el nombre");
    app.type_text(By::test_id("form.name"), "Ana");
    // «Enviar» answers with «Último envío», read after its update.
    assert_eq!(app.call("form.submit", json!({})), Ok(json!("Ana")));
    assert_eq!(
        app.call("form.last_submission", json!({})),
        Ok(json!("Ana"))
    );
    assert_eq!(app.call("form.clear", json!({})), Ok(json!(null)));
    assert!(app.call("counter.nothing", json!({})).is_err());

    let exposed: Vec<&str> = app
        .commands()
        .iter()
        .filter(|command| command.exposed)
        .map(|command| command.id)
        .collect();
    assert_eq!(
        exposed,
        [
            "counter.increment",
            "counter.add",
            "counter.reset",
            "form.submit",
            "form.clear",
            "form.last_submission",
            "task.wait",
        ]
    );
}

#[test]
fn query_reads_the_model_without_msg() {
    let mut app = TestApp::new(Sample::default());
    assert_eq!(app.query(LastSubmission), Ok(None));
    app.type_text(By::test_id("form.name"), "Ana");
    app.invoke(Submit).unwrap();
    let sent = app.messages().len();
    assert_eq!(app.query(LastSubmission), Ok(Some("Ana".to_owned())));
    assert_eq!(app.messages().len(), sent, "a query sends nothing");
}

#[test]
fn agents_confirm_destructive_actions_in_a_dialog() {
    let mut app = TestApp::new(Sample::default());
    app.invoke(Increment).unwrap();
    app.invoke(Increment).unwrap();

    let reply = app.agent_call("Cursor", "counter.reset", json!({}));
    assert_eq!(reply.result(), None, "waits for the person");
    assert_eq!(app.app().clicks, 2);
    let screen = app.screen();
    let dialog = screen.get(By::role(Role::Dialog).name("¿Permitir «Reiniciar»?"));
    screen
        .within(dialog)
        .get(By::text("Cursor quiere ejecutar counter.reset."));
    assert!(app.is_focused(By::test_id("kai.confirm.deny")));
    assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
    assert!(!app.perform("counter.increment"), "the dialog is modal");
    app.press(Key::Char('p'), CMD_SHIFT);
    assert!(!app.session().palette_open());

    app.click(By::test_id("kai.confirm.deny"));
    assert_eq!(reply.result(), denied());
    assert_eq!(app.app().clicks, 2);
    assert!(app.screen().query(By::role(Role::Dialog)).is_none());
    let last = app.audit_log().last().unwrap();
    assert_eq!(last.command, "counter.reset");
    assert_eq!(last.origin, agent("Cursor"));
    assert_eq!(last.outcome, Outcome::Denied);

    let reply = app.agent_call("Cursor", "counter.reset", json!({}));
    app.press(Key::Escape, Modifiers::NONE);
    assert_eq!(reply.result(), denied(), "Escape rejects");
    assert_eq!(app.app().clicks, 2);

    let reply = app.agent_call("Cursor", "counter.reset", json!({}));
    app.click(By::test_id("kai.confirm.once"));
    assert_eq!(reply.result(), DONE);
    assert_eq!(app.app().clicks, 0);
    assert_eq!(app.audit_log().last().unwrap().outcome, Outcome::Done);

    app.invoke(Increment).unwrap();
    let reply = app.agent_call("Cursor", "counter.reset", json!({}));
    assert_eq!(reply.result(), None, "allowed once asks again");
    app.click(By::test_id("kai.confirm.session"));
    assert_eq!(reply.result(), DONE);
    app.invoke(Increment).unwrap();
    let reply = app.agent_call("Cursor", "counter.reset", json!({}));
    assert_eq!(reply.result(), DONE, "allowed for the session");
    assert_eq!(app.app().clicks, 0);
}

#[test]
fn an_agent_wait_command_answers_only_after_the_task_finishes() {
    let mut app = TestApp::new(Sample::default());
    let reply = app.agent_call("Cursor", "task.wait", json!({}));
    assert!(
        reply.result().is_none(),
        "queda abierta mientras corre la espera"
    );
    assert!(app.tasks_pending());
    app.run_until_idle();
    assert_eq!(reply.result(), Some(Ok(json!(null))));
    assert!(app.app().waited);
    assert_eq!(app.audit_log().last().unwrap().outcome, Outcome::Done);
}

#[test]
fn agent_calls_wait_behind_the_dialog_and_can_be_cancelled() {
    let mut app = TestApp::new(Sample::default());
    app.invoke(Increment).unwrap();
    let reset = app.agent_call("Cursor", "counter.reset", json!({}));
    let add = app.agent_call("Cursor", "counter.increment", json!({}));
    assert_eq!((reset.result(), add.result()), (None, None));
    assert_eq!(app.app().clicks, 1);

    app.cancel_agent_call(reset.id());
    assert!(reset.result().unwrap().is_err());
    assert!(!app.session().confirmation_open());
    assert_eq!(add.result(), DONE, "the queue moves on");
    assert_eq!(app.app().clicks, 2);
    let outcomes: Vec<_> = app
        .audit_log()
        .entries()
        .map(|entry| (entry.command.as_str(), entry.outcome.clone()))
        .collect();
    assert_eq!(
        outcomes,
        [
            ("counter.reset", Outcome::Cancelled),
            ("counter.increment", Outcome::Done),
        ]
    );

    let first = app.agent_call("Cursor", "counter.reset", json!({}));
    let second = app.agent_call("Cursor", "counter.reset", json!({}));
    app.cancel_agent_call(second.id());
    assert!(second.result().unwrap().is_err());
    assert_eq!(first.result(), None);
    app.on_confirmation(Confirm::Deny);
    assert_eq!(first.result(), denied());
    assert_eq!(app.app().clicks, 2);
}

#[test]
fn agent_calls_fail_before_asking_when_they_cannot_run() {
    let mut app = TestApp::new(Sample::default());
    let hidden = app.agent_call("Cursor", "form.toggle_password", json!({}));
    assert_eq!(
        hidden.result(),
        Some(Err(CommandError::new(
            "form.toggle_password no está expuesto a agentes"
        )))
    );
    let reset = app.agent_call("Cursor", "counter.reset", json!({}));
    assert_eq!(
        reset.result(),
        Some(Err(CommandError::new("El contador ya está en cero")))
    );
    let bad = app.agent_call("Cursor", "counter.add", json!({"count": "two"}));
    assert!(bad.result().unwrap().is_err());
    assert!(!app.session().confirmation_open());
    let query = app.agent_call("Cursor", "form.last_submission", json!({}));
    assert_eq!(query.result(), DONE);
    assert_eq!(app.audit_log().len(), 3, "queries are not audited");
    assert!(
        app.audit_log()
            .entries()
            .all(|entry| matches!(entry.outcome, Outcome::Failed(_)))
    );
}

#[test]
fn people_and_scripts_never_see_the_dialog() {
    let mut app = TestApp::new(Sample::default());
    app.invoke(Increment).unwrap();
    app.click(By::test_id("counter.reset"));
    assert_eq!(app.app().clicks, 0);
    assert!(!app.session().confirmation_open());
    assert!(app.audit_log().is_empty(), "the person is not audited");

    app.call("counter.add", json!({"count": 3})).unwrap();
    app.call("counter.reset", json!({})).unwrap();
    assert!(!app.session().confirmation_open());
    let entries: Vec<_> = app.audit_log().entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].origin, Origin::Script);
    assert_eq!(entries[0].input, json!({"count": 3}));
    assert_eq!(entries[1].outcome, Outcome::Done);
}

#[test]
fn the_indicator_shows_the_agent_and_its_last_action() {
    let mut app = TestApp::new(Sample::default());
    assert!(app.screen().query(By::test_id("kai.agente")).is_none());
    app.agent_connected("Cursor");
    app.screen()
        .get(By::role(Role::Status).name("Agente: Cursor"));
    assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
    app.on_confirmation(Confirm::AllowOnce);
    app.agent_call("Cursor", "counter.increment", json!({}));
    app.agent_call("Cursor", "counter.reset", json!({}));
    assert_eq!(app.app().clicks, 0);
    app.screen()
        .get(By::role(Role::Status).name("Agente: Cursor · Reiniciar"));
}

#[cfg(feature = "mcp")]
#[test]
fn the_indicator_goes_away_when_the_agent_disconnects() {
    let mut app = TestApp::new(Sample::default());
    app.agent_connected("Cursor");
    assert!(app.screen().query(By::test_id("kai.agente")).is_some());
    app.agent_disconnected();
    assert!(app.screen().query(By::test_id("kai.agente")).is_none());
}
