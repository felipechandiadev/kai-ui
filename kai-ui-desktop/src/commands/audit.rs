//! What scripts and agents did with the app (13, Paso 13): every action
//! called through JSON, with who asked, its input and how it ended,
//! rejected and cancelled calls included. Queries are not recorded, nor
//! what the person does in the window. The log lives in memory and keeps
//! the last [`AUDIT_CAPACITY`] entries.
//!
//! Inputs are stored redacted: a field whose schema says `writeOnly`
//! (`#[schemars(extend("writeOnly" = true))]`, for passwords, tokens and
//! the like) is replaced by [`REDACTED`].

use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use serde_json::Value;

/// Entries the log keeps; older ones are dropped.
pub const AUDIT_CAPACITY: usize = 500;

/// What a secret field becomes in the log and in confirmation dialogs.
pub const REDACTED: &str = "•••";

/// Who called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// An agent over MCP, by the name its client gave.
    Agent { client: String },
    /// The app's own code or a test (`TestApp::call`).
    Script,
}

/// How a call ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// The command refused or failed, with the reason the caller got.
    Failed(String),
    /// The person rejected it in the confirmation dialog.
    Denied,
    /// The caller gave up (time limit or cancellation) before it ran.
    Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuditEntry {
    /// When the call arrived.
    pub at: SystemTime,
    pub origin: Origin,
    pub command: String,
    /// Redacted input.
    pub input: Value,
    pub outcome: Outcome,
    /// From arrival to outcome, waiting for confirmation included.
    pub duration: Duration,
}

/// The last [`AUDIT_CAPACITY`] calls, oldest first.
#[derive(Clone, Debug, Default)]
pub struct AuditLog {
    entries: VecDeque<AuditEntry>,
}

impl AuditLog {
    pub fn push(&mut self, entry: AuditEntry) {
        if self.entries.len() == AUDIT_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = &AuditEntry> {
        self.entries.iter()
    }

    pub fn last(&self) -> Option<&AuditEntry> {
        self.entries.back()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// `input` with every field whose `schema` is `writeOnly` replaced by
/// [`REDACTED`], through objects, arrays and `oneOf`/`anyOf`/`allOf`
/// variants.
pub fn redact(input: &Value, schema: &Value) -> Value {
    if schema["writeOnly"] == Value::Bool(true) {
        return Value::String(REDACTED.to_owned());
    }
    match input {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(name, value)| {
                    let redacted = property_schemas(schema, name)
                        .iter()
                        .fold(value.clone(), |value, schema| redact(&value, schema));
                    (name.clone(), redacted)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact(item, &schema["items"]))
                .collect(),
        ),
        _ => input.clone(),
    }
}

/// Every schema `name` may have in `schema`, variants included.
fn property_schemas<'a>(schema: &'a Value, name: &str) -> Vec<&'a Value> {
    let mut found: Vec<&Value> = schema["properties"].get(name).into_iter().collect();
    for combinator in ["oneOf", "anyOf", "allOf"] {
        if let Some(variants) = schema[combinator].as_array() {
            for variant in variants {
                found.extend(property_schemas(variant, name));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::{AUDIT_CAPACITY, AuditEntry, AuditLog, Origin, Outcome, REDACTED, redact};
    use crate::commands::schema::input_schema;

    #[derive(Deserialize, schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Ingresar {
        usuario: String,
        #[schemars(extend("writeOnly" = true))]
        clave: String,
        tokens: Vec<Token>,
        #[serde(default)]
        pago: Option<Pago>,
    }

    #[derive(Deserialize, schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Token {
        #[schemars(extend("writeOnly" = true))]
        valor: String,
        nombre: String,
    }

    #[derive(Deserialize, schemars::JsonSchema)]
    #[serde(tag = "tipo")]
    #[allow(dead_code)]
    enum Pago {
        Tarjeta {
            #[schemars(extend("writeOnly" = true))]
            numero: String,
        },
        Efectivo {
            monto: u64,
        },
    }

    #[test]
    fn write_only_fields_are_redacted_at_any_depth() {
        let schema = input_schema::<Ingresar>();
        let input = json!({
            "usuario": "ana",
            "clave": "secreto",
            "tokens": [{"valor": "abc", "nombre": "api"}],
            "pago": {"tipo": "Tarjeta", "numero": "4111"},
        });
        assert_eq!(
            redact(&input, &schema),
            json!({
                "usuario": "ana",
                "clave": REDACTED,
                "tokens": [{"valor": REDACTED, "nombre": "api"}],
                "pago": {"tipo": "Tarjeta", "numero": REDACTED},
            })
        );
        let cash = json!({"pago": {"tipo": "Efectivo", "monto": 1000}});
        assert_eq!(redact(&cash, &schema), cash);
    }

    #[test]
    fn the_log_keeps_the_last_entries() {
        let mut log = AuditLog::default();
        for index in 0..AUDIT_CAPACITY + 2 {
            log.push(AuditEntry {
                at: std::time::SystemTime::UNIX_EPOCH,
                origin: Origin::Script,
                command: format!("c.{index}"),
                input: json!({}),
                outcome: Outcome::Done,
                duration: std::time::Duration::ZERO,
            });
        }
        assert_eq!(log.len(), AUDIT_CAPACITY);
        assert_eq!(log.entries().next().unwrap().command, "c.2");
        assert_eq!(
            log.last().unwrap().command,
            format!("c.{}", AUDIT_CAPACITY + 1)
        );
    }
}
