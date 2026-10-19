//! [`McpClient`]: an agent talking MCP to a [`TestApp`], over an in-memory
//! pipe instead of stdio, with the same server the window runtime starts.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;

use super::TestApp;
use crate::mcp::{Invocation, Tools, serve};
use crate::runtime::App;

/// How long a request waits for the server before the test fails.
const WAIT: Duration = Duration::from_secs(10);

const PROTOCOL: &str = "2025-06-18";

enum Event {
    Call(Invocation),
    Line(String),
    Closed,
}

/// MCP client of a [`TestApp`]: requests go out as JSON-RPC lines and the
/// tool calls they cause run on the app, as in the window runtime. Each
/// request waits for its response (failing the test after ten seconds).
pub struct McpClient<'a, A: App> {
    app: &'a mut TestApp<A>,
    outgoing: Option<UnboundedSender<String>>,
    events: Receiver<Event>,
    next_id: u64,
    initialized: Value,
}

impl<A: App> TestApp<A> {
    /// Start the MCP server of this app's exposed commands and connect an
    /// initialized client to it.
    pub fn mcp_client(&mut self) -> McpClient<'_, A> {
        let tools = Tools::of::<A>(self.app().title());
        let (client_io, server_io) = tokio::io::duplex(1 << 16);
        let (events_tx, events) = mpsc::channel();
        let (outgoing, mut outgoing_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let calls = events_tx.clone();
        let closed = events_tx.clone();
        serve(
            tools,
            move || tokio::io::split(server_io),
            move |invocation| {
                let _ = calls.send(Event::Call(invocation));
            },
            move || {
                let _ = closed.send(Event::Closed);
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
        };
        let response = client.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL,
                "capabilities": {},
                "clientInfo": {"name": "kai-test", "version": "0"},
            }),
        );
        client.initialized = response["result"].clone();
        client.notify("notifications/initialized");
        client
    }
}

impl<A: App> McpClient<'_, A> {
    /// What the server answered to `initialize`.
    pub fn initialized(&self) -> &Value {
        &self.initialized
    }

    /// The app, to check what the calls did.
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

    /// Call tool `name`: `Ok` with the `CallToolResult` (which may be an
    /// `isError` result), `Err` with the JSON-RPC error.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, Value> {
        let response = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        match response.get("error") {
            Some(error) => Err(error.clone()),
            None => Ok(response["result"].clone()),
        }
    }

    /// Send a JSON-RPC request and return its whole response, running the
    /// tool calls it causes on the app meanwhile.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            match self.events.recv_timeout(WAIT) {
                Ok(Event::Call(mut invocation)) => {
                    let input = std::mem::take(&mut invocation.input);
                    let result = self.app.call(&invocation.command, input);
                    invocation.reply(result);
                }
                Ok(Event::Line(line)) => {
                    let message: Value = serde_json::from_str(&line)
                        .unwrap_or_else(|error| panic!("línea MCP inválida ({error}): {line}"));
                    if message["id"] == json!(id) {
                        return message;
                    }
                }
                Ok(Event::Closed) => panic!("el servidor MCP se cerró durante {method}"),
                Err(_) => panic!("el servidor MCP no respondió a {method}"),
            }
        }
    }

    /// Disconnect, as an agent quitting does. Whether the server noticed
    /// and finished.
    pub fn close(mut self) -> bool {
        self.outgoing = None;
        loop {
            match self.events.recv_timeout(WAIT) {
                Ok(Event::Closed) => return true,
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return false,
            }
        }
    }

    fn notify(&mut self, method: &str) {
        self.send(json!({"jsonrpc": "2.0", "method": method}));
    }

    fn send(&mut self, message: Value) {
        let sent = self
            .outgoing
            .as_ref()
            .is_some_and(|outgoing| outgoing.send(message.to_string()).is_ok());
        assert!(sent, "el cliente MCP ya está desconectado");
    }
}
