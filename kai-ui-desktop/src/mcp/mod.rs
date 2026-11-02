//! Experimental MCP server (13, Paso 12): the commands an app marks with
//! [`expose`](crate::commands::CommandBuilder::expose) become tools an agent
//! can list and call over stdio.
//!
//! The protocol runs on its own thread with a small tokio runtime; the app
//! stays on the UI thread. Each tool call travels as an [`Invocation`] to the
//! event loop, which runs it as an agent call ([`TestApp::agent_call`]):
//! a `Destructive` action waits for the person in a confirmation dialog
//! (13, Paso 13). Build with `--features mcp` and start the app with
//! [`run_with_mcp`](crate::runtime::run_with_mcp).
//!
//! [`TestApp::agent_call`]: crate::testing::TestApp::agent_call

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    ToolAnnotations,
};
use rmcp::service::{NotificationContext, Peer, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;

use crate::commands::{CommandError, CommandRegistry};
use crate::runtime::App;
use crate::session::AgentCall;

/// How long a tool call waits for the app before giving up.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// The tools of an app: its exposed commands, ids as names.
#[derive(Clone, Debug)]
pub struct Tools {
    name: String,
    tools: Vec<Tool>,
    /// Tools whose output is not an object: MCP wants `structuredContent`
    /// to be one, so these answer `{"result": ...}`.
    wrapped: HashSet<String>,
}

impl Tools {
    /// The exposed commands of `A`, for a server called `name`.
    pub fn of<A: App>(name: impl Into<String>) -> Self {
        let mut tools = Vec::new();
        let mut wrapped = HashSet::new();
        for command in CommandRegistry::<A>::of().descriptions() {
            if command["exposed"] != json!(true) {
                continue;
            }
            let (tool, wraps) = tool(&command);
            if wraps {
                wrapped.insert(tool.name.to_string());
            }
            tools.push(tool);
        }
        Self {
            name: name.into(),
            tools,
            wrapped,
        }
    }

    /// Tool names, in registration order.
    pub fn names(&self) -> Vec<&str> {
        self.tools.iter().map(|tool| tool.name.as_ref()).collect()
    }

    fn contains(&self, name: &str) -> bool {
        self.tools.iter().any(|tool| tool.name == name)
    }
}

fn tool(command: &Value) -> (Tool, bool) {
    let name = command["id"].as_str().unwrap_or_default().to_owned();
    let description = command["description"]
        .as_str()
        .map(|d| Cow::Owned(d.to_owned()));
    let input = object(&command["inputSchema"]);
    let query = command["kind"] == "query";
    let annotations = ToolAnnotations::new()
        .read_only(query)
        .destructive(command["safety"] == "destructive");
    let mut tool = Tool::new_with_raw(name, description, Arc::new(input))
        .with_title(command["title"].as_str().unwrap_or_default().to_owned())
        .with_annotations(annotations);
    let mut wraps = false;
    if let Some(output) = command.get("outputSchema") {
        let output = if output["type"] == "object" {
            output.clone()
        } else {
            wraps = true;
            json!({
                "type": "object",
                "properties": {"result": output},
                "required": ["result"],
                "additionalProperties": false,
            })
        };
        tool = tool.with_raw_output_schema(Arc::new(object(&output)));
    }
    (tool, wraps)
}

fn object(value: &Value) -> JsonObject {
    value.as_object().cloned().unwrap_or_default()
}

pub use crate::runtime::InvocationId;

/// A tool call waiting for the app: run `command` with `input` on the UI
/// thread (as an agent call, so a `Destructive` action asks the person
/// first), then [`reply`](Self::reply).
#[derive(Debug)]
pub struct Invocation {
    pub id: InvocationId,
    /// The client that asked, as it introduced itself.
    pub client: String,
    pub command: String,
    pub input: Value,
    reply: oneshot::Sender<Result<Value, CommandError>>,
}

impl Invocation {
    /// Answer the agent. If it stopped waiting, the answer is dropped.
    pub fn reply(self, result: Result<Value, CommandError>) {
        let _ = self.reply.send(result);
    }

    pub(crate) fn into_agent_call(self) -> AgentCall {
        let reply = self.reply;
        AgentCall {
            id: self.id.0,
            client: self.client,
            command: self.command,
            input: self.input,
            reply: Box::new(move |result| {
                let _ = reply.send(result);
            }),
        }
    }
}

/// What the server tells the app, in order.
#[derive(Debug)]
pub enum Event {
    /// A client finished the handshake, with its name.
    Connected(String),
    /// A tool call to run.
    Call(Invocation),
    /// The agent stopped waiting for this call (time limit or
    /// `notifications/cancelled`); if it did not run yet, it must not.
    Cancel(InvocationId),
    /// A client of a socket server went away; the server waits for the next
    /// ([`serve_socket`]).
    Disconnected,
    /// The client went away; the server thread is done.
    Closed,
}

type Notify = Arc<dyn Fn(Event) + Send + Sync>;

struct Server {
    tools: Tools,
    send: Notify,
    next: AtomicU64,
}

/// Name of the client: its title, or its name.
fn client_name(peer: &Peer<RoleServer>) -> String {
    peer.peer_info().map_or_else(
        || "agente".to_owned(),
        |info| {
            let client = &info.client_info;
            client.title.clone().unwrap_or_else(|| client.name.clone())
        },
    )
}

impl Server {
    async fn run(
        &self,
        name: String,
        input: Value,
        context: &RequestContext<RoleServer>,
    ) -> Result<Value, CommandError> {
        let (reply, answer) = oneshot::channel();
        let id = InvocationId(self.next.fetch_add(1, Ordering::Relaxed));
        (self.send)(Event::Call(Invocation {
            id,
            client: client_name(&context.peer),
            command: name.clone(),
            input,
            reply,
        }));
        tokio::select! {
            answer = answer => answer
                .unwrap_or_else(|_| Err(CommandError::new("La app se cerró antes de responder"))),
            () = tokio::time::sleep(TIMEOUT) => {
                (self.send)(Event::Cancel(id));
                Err(CommandError::new(format!(
                    "{name} no terminó en {} s (¿esperaba confirmación?)",
                    TIMEOUT.as_secs()
                )))
            }
            () = context.ct.cancelled() => {
                (self.send)(Event::Cancel(id));
                Err(CommandError::new(format!("{name} se canceló")))
            }
        }
    }

    fn result(&self, name: &str, result: Result<Value, CommandError>) -> CallToolResult {
        match result {
            Ok(value) if self.tools.wrapped.contains(name) => {
                CallToolResult::structured(json!({"result": value}))
            }
            Ok(value @ Value::Object(_)) => CallToolResult::structured(value),
            Ok(_) => CallToolResult::success(vec![ContentBlock::text("Hecho")]),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error.message)]),
        }
    }
}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new(self.tools.name.clone(), crate::version::VERSION.to_owned()),
        )
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        (self.send)(Event::Connected(client_name(&context.peer)));
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tools.tools.clone()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let name = request.name.to_string();
        if !self.tools.contains(&name) {
            return Err(ErrorData::invalid_params(
                format!("Herramienta desconocida: {name}"),
                None,
            ));
        }
        let input = Value::Object(request.arguments.unwrap_or_default());
        let result = self.run(name.clone(), input, &context).await;
        Ok(self.result(&name, result).into())
    }
}

/// Serve `tools` over the stream `connect` opens, on a thread of its own.
/// Everything the app must know arrives at `send` as an [`Event`], the
/// last one [`Event::Closed`].
pub fn serve<R, W>(
    tools: Tools,
    connect: impl FnOnce() -> (R, W) + Send + 'static,
    send: impl Fn(Event) + Send + Sync + 'static,
) -> JoinHandle<()>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let send: Notify = Arc::new(send);
    let server = Server {
        tools,
        send: Arc::clone(&send),
        next: AtomicU64::new(1),
    };
    std::thread::Builder::new()
        .name("kai-mcp".into())
        .spawn(move || {
            match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => {
                    runtime.block_on(async move {
                        match server.serve(connect()).await {
                            Ok(service) => {
                                let _ = service.waiting().await;
                            }
                            Err(error) => eprintln!("MCP: no se pudo iniciar la sesión: {error}"),
                        }
                    });
                    runtime.shutdown_background();
                }
                Err(error) => eprintln!("MCP: no se pudo crear el runtime: {error}"),
            }
            send(Event::Closed);
        })
        .expect("no se pudo crear el hilo de MCP")
}

/// [`serve`] over the process's stdin and stdout. Stdout then belongs to
/// the protocol: anything else the app prints must go to stderr.
pub fn serve_stdio(tools: Tools, send: impl Fn(Event) + Send + Sync + 'static) -> JoinHandle<()> {
    serve(tools, rmcp::transport::io::stdio, send)
}

/// Create the socket `path` for [`serve_socket`]: only its owner may
/// connect, and one left by an earlier run is replaced (but not a file that
/// is not a socket).
#[cfg(unix)]
pub fn bind_socket(path: &std::path::Path) -> std::io::Result<std::os::unix::net::UnixListener> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.file_type().is_socket() {
            return Err(std::io::Error::other(format!(
                "{} existe y no es un socket",
                path.display()
            )));
        }
        std::fs::remove_file(path)?;
    }
    let listener = std::os::unix::net::UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Serve `tools` to one client at a time over `listener`, on a thread of its
/// own. Each client is a whole MCP session; when it leaves, the app gets
/// [`Event::Disconnected`] (never [`Event::Closed`]) and the next may connect.
#[cfg(unix)]
pub fn serve_socket(
    tools: Tools,
    listener: std::os::unix::net::UnixListener,
    send: impl Fn(Event) + Send + Sync + 'static,
) -> JoinHandle<()> {
    let send: Notify = Arc::new(send);
    std::thread::Builder::new()
        .name("kai-mcp-socket".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let once = Arc::clone(&send);
                let session = serve(
                    tools.clone(),
                    move || {
                        let _ = stream.set_nonblocking(true);
                        let stream = tokio::net::UnixStream::from_std(stream)
                            .expect("el socket debe poder usarse con tokio");
                        stream.into_split()
                    },
                    move |event| {
                        once(match event {
                            Event::Closed => Event::Disconnected,
                            other => other,
                        });
                    },
                );
                let _ = session.join();
            }
        })
        .expect("no se pudo crear el hilo del socket MCP")
}

#[cfg(test)]
mod tests {
    /// Stdout carries the protocol, so the code that runs while serving
    /// never prints to it.
    #[test]
    fn serving_code_never_prints_to_stdout() {
        let sources = [
            ("mcp/mod.rs", include_str!("mod.rs")),
            ("devtools.rs", include_str!("../devtools.rs")),
            ("runtime.rs", include_str!("../runtime.rs")),
            ("session.rs", include_str!("../session.rs")),
            ("session/calls.rs", include_str!("../session/calls.rs")),
        ];
        for (file, source) in sources {
            for (at, _) in source.match_indices("print") {
                let before = source[..at].chars().next_back();
                let after = &source[at + "print".len()..];
                let is_macro = after.starts_with("!(") || after.starts_with("ln!(");
                let to_stdout = !before.is_some_and(|c| c.is_alphanumeric() || c == '_');
                assert!(!(is_macro && to_stdout), "{file} escribe en stdout");
            }
        }
    }

    #[cfg(unix)]
    mod socket {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixStream;
        use std::sync::mpsc;

        use super::super::*;

        fn path(tag: &str) -> std::path::PathBuf {
            std::env::temp_dir().join(format!("kai-mcp-{tag}-{}.sock", std::process::id()))
        }

        fn ask(stream: &mut UnixStream, reader: &mut BufReader<UnixStream>, line: &str) -> Value {
            writeln!(stream, "{line}").unwrap();
            let mut answer = String::new();
            reader.read_line(&mut answer).unwrap();
            serde_json::from_str(&answer).unwrap()
        }

        const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"prueba","version":"0"}}}"#;

        #[test]
        fn a_socket_serves_one_client_after_another_without_closing_the_app() {
            // Any app will do: the tools list is what matters.
            type A = crate::testing::sample_app::Sample;
            let path = path("sequence");
            let listener = bind_socket(&path).unwrap();
            let (events, seen) = mpsc::channel();
            let events = std::sync::Mutex::new(events);
            serve_socket(Tools::of::<A>("socket"), listener, move |event| {
                let _ = events.lock().unwrap().send(format!("{event:?}"));
            });
            for client in 0..2 {
                let mut stream = UnixStream::connect(&path).unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let hello = ask(&mut stream, &mut reader, INITIALIZE);
                assert_eq!(hello["result"]["serverInfo"]["name"], "socket", "client {client}");
                writeln!(stream, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#).unwrap();
                let tools = ask(&mut stream, &mut reader, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#);
                assert!(tools["result"]["tools"].is_array());
                drop(reader);
                drop(stream);
            }
            let log: Vec<String> = seen
                .recv_timeout(Duration::from_secs(5))
                .into_iter()
                .chain(seen.try_iter())
                .collect();
            let disconnected = (0..2)
                .filter(|_| {
                    seen.recv_timeout(Duration::from_secs(5))
                        .is_ok_and(|event| event == "Disconnected")
                })
                .count();
            assert!(!log.iter().any(|event| event == "Closed"), "{log:?}");
            assert!(disconnected >= 1, "{log:?}");
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn binding_replaces_an_old_socket_but_not_other_files() {
            let socket = path("replace");
            drop(bind_socket(&socket).unwrap());
            drop(bind_socket(&socket).unwrap());
            let _ = std::fs::remove_file(&socket);
            let file = path("file");
            std::fs::write(&file, "x").unwrap();
            assert!(bind_socket(&file).is_err());
            let _ = std::fs::remove_file(&file);
        }
    }
}
