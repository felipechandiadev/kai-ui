//! Experimental MCP server (13, Paso 12): the commands an app marks with
//! [`expose`](crate::commands::CommandBuilder::expose) become tools an agent
//! can list and call over stdio.
//!
//! The protocol runs on its own thread with a small tokio runtime; the app
//! stays on the UI thread. Each tool call travels as an [`Invocation`] to the
//! event loop, which runs it like any JSON call ([`TestApp::call`]) and sends
//! the answer back. Build with `--features mcp` and start the app with
//! [`run_with_mcp`](crate::runtime::run_with_mcp).
//!
//! [`TestApp::call`]: crate::testing::TestApp::call

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
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;

use crate::commands::{CommandError, CommandRegistry};
use crate::runtime::App;

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

/// Identifies one tool call while the app answers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvocationId(pub u64);

/// A tool call waiting for the app: run `command` with `input` on the UI
/// thread, then [`reply`](Self::reply).
#[derive(Debug)]
pub struct Invocation {
    pub id: InvocationId,
    pub command: String,
    pub input: Value,
    reply: oneshot::Sender<Result<Value, CommandError>>,
}

impl Invocation {
    /// Answer the agent. If it stopped waiting, the answer is dropped.
    pub fn reply(self, result: Result<Value, CommandError>) {
        let _ = self.reply.send(result);
    }
}

type Invoke = Arc<dyn Fn(Invocation) + Send + Sync>;

struct Server {
    tools: Tools,
    invoke: Invoke,
    next: AtomicU64,
}

impl Server {
    async fn run(&self, name: String, input: Value) -> Result<Value, CommandError> {
        let (reply, answer) = oneshot::channel();
        let id = InvocationId(self.next.fetch_add(1, Ordering::Relaxed));
        (self.invoke)(Invocation {
            id,
            command: name.clone(),
            input,
            reply,
        });
        match tokio::time::timeout(TIMEOUT, answer).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CommandError::new("La app se cerró antes de responder")),
            Err(_) => Err(CommandError::new(format!(
                "{name} no respondió en {} s",
                TIMEOUT.as_secs()
            ))),
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
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let name = request.name.to_string();
        if !self.tools.contains(&name) {
            return Err(ErrorData::invalid_params(
                format!("Herramienta desconocida: {name}"),
                None,
            ));
        }
        let input = Value::Object(request.arguments.unwrap_or_default());
        let result = self.run(name.clone(), input).await;
        Ok(self.result(&name, result).into())
    }
}

/// Serve `tools` over the stream `connect` opens, on a thread of its own.
/// Calls reach `invoke`; `on_close` runs once the client goes away.
pub fn serve<R, W>(
    tools: Tools,
    connect: impl FnOnce() -> (R, W) + Send + 'static,
    invoke: impl Fn(Invocation) + Send + Sync + 'static,
    on_close: impl FnOnce() + Send + 'static,
) -> JoinHandle<()>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    let server = Server {
        tools,
        invoke: Arc::new(invoke),
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
            on_close();
        })
        .expect("no se pudo crear el hilo de MCP")
}

/// [`serve`] over the process's stdin and stdout. Stdout then belongs to
/// the protocol: anything else the app prints must go to stderr.
pub fn serve_stdio(
    tools: Tools,
    invoke: impl Fn(Invocation) + Send + Sync + 'static,
    on_close: impl FnOnce() + Send + 'static,
) -> JoinHandle<()> {
    serve(tools, rmcp::transport::io::stdio, invoke, on_close)
}

#[cfg(test)]
mod tests {
    /// Stdout carries the protocol, so the code that runs while serving
    /// never prints to it.
    #[test]
    fn serving_code_never_prints_to_stdout() {
        let sources = [
            ("mcp/mod.rs", include_str!("mod.rs")),
            ("runtime.rs", include_str!("../runtime.rs")),
            ("session.rs", include_str!("../session.rs")),
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
}
