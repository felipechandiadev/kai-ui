//! [`McpClient`]: an agent talking MCP to a [`TestApp`], over an in-memory
//! pipe instead of stdio, with the same server the window runtime starts.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;

use super::TestApp;
use crate::mcp::{self, Tools, serve};
use crate::runtime::App;

/// How long the client waits for the server before the test fails.
const WAIT: Duration = Duration::from_secs(10);

const PROTOCOL: &str = "2025-06-18";

/// Name the client gives in `initialize`.
pub const MCP_CLIENT: &str = "kai-test";

enum Event {
    Server(mcp::Event),
    Line(String),
}

/// MCP client of a [`TestApp`]: requests go out as JSON-RPC lines, and the
/// events they cause (tool calls, cancellations) reach the app as in the
/// window runtime: a `Destructive` action opens the confirmation dialog.
/// Waiting fails the test after ten seconds.
pub struct McpClient<'a, A: App> {
    app: &'a mut TestApp<A>,
    outgoing: Option<UnboundedSender<String>>,
    events: Receiver<Event>,
    next_id: u64,
    initialized: Value,
    /// Responses that arrived before anyone asked for them.
    responses: HashMap<u64, Value>,
}

impl<A: App> TestApp<A> {
    /// Start the MCP server of this app's exposed commands and connect an
    /// initialized client to it, named [`MCP_CLIENT`].
    pub fn mcp_client(&mut self) -> McpClient<'_, A> {
        let tools = Tools::of::<A>(self.app().title());
        let (client_io, server_io) = tokio::io::duplex(1 << 16);
        let (events_tx, events) = mpsc::channel();
        let (outgoing, mut outgoing_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let server_events = events_tx.clone();
        serve(
            tools,
            move || tokio::io::split(server_io),
            move |event| {
                let _ = server_events.send(Event::Server(event));
            },
        );

        let lines = events_tx;
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("runtime del cliente MCP");
            runtime.block_on(async move {
                let (read, mut write) = tokio::io::split(client_io);
                let reader = async move {
                    let mut incoming = BufReader::new(read).lines();
                    while let Ok(Some(line)) = incoming.next_line().await {
                        if lines.send(Event::Line(line)).is_err() {
                            break;
                        }
                    }
                };
                let writer = async move {
                    while let Some(line) = outgoing_rx.recv().await {
                        let sent = write.write_all(format!("{line}\n").as_bytes()).await;
                        if sent.is_err() || write.flush().await.is_err() {
                            break;
                        }
                    }
                    let _ = write.shutdown().await;
                };
                tokio::join!(reader, writer);
            });
        });

        let mut client = McpClient {
            app: self,
            outgoing: Some(outgoing),
            events,
            next_id: 1,
            initialized: Value::Null,
            responses: HashMap::new(),
        };
        let response = client.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL,
                "capabilities": {},
                "clientInfo": {"name": MCP_CLIENT, "version": "0"},
            }),
        );
        client.initialized = response["result"].clone();
        client.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        client.wait_for("la conexión", |event| {
            matches!(event, mcp::Event::Connected(_))
        });
        client
    }
}

impl<A: App> McpClient<'_, A> {
    /// What the server answered to `initialize`.
    pub fn initialized(&self) -> &Value {
        &self.initialized
    }

    /// The app, to check what the calls did or to answer the dialog.
    pub fn app(&mut self) -> &mut TestApp<A> {
        self.app
    }

    /// The listed tools, as JSON.
    pub fn tools(&mut self) -> Vec<Value> {
        let response = self.request("tools/list", json!({}));
        response["result"]["tools"]
            .as_array()
            .cloned()
            .unwrap_or_else(|| panic!("tools/list sin herramientas: {response}"))
    }

    /// Call tool `name` and wait for its answer: `Ok` with the
    /// `CallToolResult` (which may be an `isError` result), `Err` with the
    /// JSON-RPC error. A call that opens the confirmation dialog needs the
    /// app to answer it ([`TestApp::on_confirmation`]); otherwise use
    /// [`Self::send_call`] and [`Self::finish`].
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, Value> {
        let id = self.send_call(name, arguments);
        self.finish(id)
    }

    /// Call tool `name` without waiting for the answer: return once the
    /// app has the call, ran or waiting for confirmation. The request id
    /// is for [`Self::finish`] and [`Self::cancel`].
    pub fn send_call(&mut self, name: &str, arguments: Value) -> u64 {
        let id = self.send_request("tools/call", json!({"name": name, "arguments": arguments}));
        loop {
            match self.next_event("tools/call") {
                Event::Server(event) => {
                    let call = matches!(event, mcp::Event::Call(_));
                    self.handle(event);
                    if call && self.app.confirmation_open() {
                        return id;
                    }
                }
                Event::Line(line) => {
                    if self.store(&line) == Some(id) {
                        return id;
                    }
                }
            }
        }
    }

    /// Wait for the answer of tool call `id`, as [`Self::call_tool`].
    pub fn finish(&mut self, id: u64) -> Result<Value, Value> {
        let response = self.response(id, "tools/call");
        match response.get("error") {
            Some(error) => Err(error.clone()),
            None => Ok(response["result"].clone()),
        }
    }

    /// Cancel tool call `id` (`notifications/cancelled`) and wait until the
    /// app heard of it.
    pub fn cancel(&mut self, id: u64) {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {"requestId": id, "reason": "el test dejó de esperar"},
        }));
        self.wait_for("la cancelación", |event| {
            matches!(event, mcp::Event::Cancel(_))
        });
    }

    /// Send a JSON-RPC request and return its whole response, handing the
    /// events it causes to the app meanwhile.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.send_request(method, params);
        self.response(id, method)
    }

    /// Disconnect, as an agent quitting does. Whether the server noticed
    /// and finished.
    pub fn close(mut self) -> bool {
        self.outgoing = None;
        loop {
            match self.events.recv_timeout(WAIT) {
                Ok(Event::Server(mcp::Event::Closed)) => return true,
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return false,
            }
        }
    }

    fn send_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    fn response(&mut self, id: u64, method: &str) -> Value {
        loop {
            if let Some(response) = self.responses.remove(&id) {
                return response;
            }
            match self.next_event(method) {
                Event::Server(event) => self.handle(event),
                Event::Line(line) => {
                    self.store(&line);
                }
            }
        }
    }

    /// Hand server events to the app until one matches `wanted`.
    fn wait_for(&mut self, what: &str, wanted: impl Fn(&mcp::Event) -> bool) {
        loop {
            match self.next_event(what) {
                Event::Server(event) => {
                    let found = wanted(&event);
                    self.handle(event);
                    if found {
                        return;
                    }
                }
                Event::Line(line) => {
                    self.store(&line);
                }
            }
        }
    }

    fn next_event(&mut self, what: &str) -> Event {
        match self.events.recv_timeout(WAIT) {
            Ok(Event::Server(mcp::Event::Closed)) => {
                panic!("el servidor MCP se cerró esperando {what}")
            }
            Ok(event) => event,
            Err(_) => panic!("el servidor MCP no respondió a {what}"),
        }
    }

    fn handle(&mut self, event: mcp::Event) {
        match event {
            mcp::Event::Connected(client) => self.app.agent_connected(&client),
            mcp::Event::Call(invocation) => self.app.run_agent_call(invocation.into_agent_call()),
            mcp::Event::Cancel(id) => self.app.cancel_agent_call(id.0),
            mcp::Event::Closed => {}
        }
    }

    /// Keep a response for later; its id, if it is one.
    fn store(&mut self, line: &str) -> Option<u64> {
        let message: Value = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("línea MCP inválida ({error}): {line}"));
        let id = message["id"].as_u64()?;
        self.responses.insert(id, message);
        Some(id)
    }

    fn send(&mut self, message: Value) {
        let sent = self
            .outgoing
            .as_ref()
            .is_some_and(|outgoing| outgoing.send(message.to_string()).is_ok());
        assert!(sent, "el cliente MCP ya está desconectado");
    }
}
