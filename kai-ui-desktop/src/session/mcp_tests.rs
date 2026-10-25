//! The MCP server in process: exposed commands become tools, calls run on
//! the app and destructive ones wait for the person ([`crate::mcp`]). They
//! run on [`Sample`], not the demo (docs/dev/27).

use serde_json::json;

use crate::commands::{Origin, Outcome};
use crate::element::Role;
use crate::testing::sample_app::Sample;
use crate::testing::{By, Confirm, MCP_CLIENT, TestApp};

fn agent(client: &str) -> Origin {
    Origin::Agent {
        client: client.to_owned(),
    }
}

#[test]
fn mcp_lists_the_exposed_commands_as_tools() {
    let mut app = TestApp::new(Sample::default());
    let mut client = app.mcp_client();
    assert!(client.initialized()["capabilities"]["tools"].is_object());
    let tools = client.tools();
    let described = crate::commands::describe::<Sample>();
    let exposed: Vec<&serde_json::Value> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|command| command["exposed"] == json!(true))
        .collect();
    assert_eq!(tools.len(), exposed.len());
    for (tool, command) in tools.iter().zip(exposed) {
        assert_eq!(tool["name"], command["id"]);
        assert_eq!(tool["title"], command["title"]);
        assert_eq!(tool["inputSchema"], command["inputSchema"]);
        let read_only = command["kind"] == "query";
        assert_eq!(tool["annotations"]["readOnlyHint"], json!(read_only));
        let destructive = command["safety"] == "destructive";
        assert_eq!(tool["annotations"]["destructiveHint"], json!(destructive));
        match command.get("outputSchema") {
            Some(output) if output["type"] == "object" => assert_eq!(
                tool["outputSchema"], *output,
                "{} entrega su objeto tal cual",
                command["id"]
            ),
            Some(output) => assert_eq!(
                tool["outputSchema"]["properties"]["result"], *output,
                "{} envuelve su salida",
                command["id"]
            ),
            None => assert!(tool.get("outputSchema").is_none()),
        }
    }
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(!names.contains(&"form.toggle_password"));
    assert!(
        client.close(),
        "el servidor termina cuando el cliente se va"
    );
}

#[test]
fn mcp_destructive_calls_wait_for_the_person() {
    let mut app = TestApp::new(Sample::default());
    let mut client = app.mcp_client();
    client
        .app()
        .screen()
        .get(By::role(Role::Status).name(format!("Agente: {MCP_CLIENT}")));
    client
        .call_tool("counter.add", json!({"count": 3}))
        .unwrap();
    client
        .app()
        .screen()
        .get(By::role(Role::Status).name(format!("Agente: {MCP_CLIENT} · Sumar")));

    let reset = client.send_call("counter.reset", json!({}));
    let screen = client.app().screen();
    let dialog = screen.get(By::role(Role::Dialog).name("¿Permitir «Reiniciar»?"));
    screen.within(dialog).get(By::text(format!(
        "{MCP_CLIENT} quiere ejecutar counter.reset."
    )));
    client.app().click(By::test_id("kai.confirm.deny"));
    let result = client.finish(reset).unwrap();
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["content"][0]["text"],
        json!("La persona rechazó «Reiniciar»")
    );
    assert_eq!(client.app().app().clicks, 3);
    let last = client.app().audit_log().last().unwrap().clone();
    assert_eq!(last.origin, agent(MCP_CLIENT));
    assert_eq!(last.outcome, Outcome::Denied);

    let reset = client.send_call("counter.reset", json!({}));
    client.cancel(reset);
    assert!(!client.app().confirmation_open());
    assert_eq!(client.app().app().clicks, 3);
    let last = client.app().audit_log().last().unwrap().outcome.clone();
    assert_eq!(last, Outcome::Cancelled);

    client.app().on_confirmation(Confirm::AllowOnce);
    let result = client.call_tool("counter.reset", json!({})).unwrap();
    assert_eq!(result["isError"], json!(false));
    assert_eq!(client.app().app().clicks, 0);
    let last = client.app().audit_log().last().unwrap().clone();
    assert_eq!(
        (last.origin, last.outcome),
        (agent(MCP_CLIENT), Outcome::Done)
    );
    assert!(client.close());
}

#[test]
fn mcp_tool_calls_run_on_the_app() {
    let mut app = TestApp::new(Sample::default());
    let mut client = app.mcp_client();

    let result = client
        .call_tool("counter.add", json!({"count": 2}))
        .unwrap();
    assert_eq!(result["isError"], json!(false));
    assert_eq!(client.app().app().clicks, 2);
    let screen = client.app().screen();
    screen
        .within(screen.get_by_test_id("counter"))
        .get(By::role(Role::Status).name("2"));

    let reset = client.send_call("counter.reset", json!({}));
    assert!(client.app().confirmation_open());
    client.app().click(By::test_id("kai.confirm.once"));
    assert_eq!(client.finish(reset).unwrap()["isError"], json!(false));
    assert_eq!(client.app().app().clicks, 0);
    let result = client.call_tool("counter.reset", json!({})).unwrap();
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["content"][0]["text"],
        json!("El contador ya está en cero"),
        "an action that cannot run fails without asking"
    );

    let result = client
        .call_tool("counter.add", json!({"count": "two"}))
        .unwrap();
    assert_eq!(result["isError"], json!(true));
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("entrada inválida para counter.add")
    );

    let error = client
        .call_tool("form.toggle_password", json!({}))
        .unwrap_err();
    assert!(error["message"].as_str().unwrap().contains("desconocida"));
    assert_eq!(client.app().app().clicks, 0);

    client.app().type_text(By::test_id("form.name"), "Ana");
    let result = client.call_tool("form.submit", json!({})).unwrap();
    assert_eq!(result["structuredContent"], json!({"result": "Ana"}));
    let result = client.call_tool("form.last_submission", json!({})).unwrap();
    assert_eq!(result["structuredContent"], json!({"result": "Ana"}));
    assert_eq!(result["content"][0]["text"], json!(r#"{"result":"Ana"}"#));
    assert!(client.close());
}
