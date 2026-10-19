//! Commands as JSON (13, Paso 11): decoding an input, the JSON Schema
//! 2020-12 of inputs and outputs, and the document that describes every
//! command of an app (`mcp-schema.json`, `--describe-commands`).
//!
//! Conventions for domain types: a RUT is a `string` with a `pattern`,
//! money is an integer in pesos (never `f64`), and enums use
//! `#[serde(tag = "tipo")]` so their schema is a readable `oneOf`.
//! Subschemas are inlined: no `$ref` for clients to resolve.

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::{CommandError, CommandRegistry};
use crate::runtime::App;

/// The JSON Schema dialect of every schema in the document.
pub const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Version of the document's own shape.
const FORMAT: u32 = 1;

/// Every command of `A` (its [`App::commands`], without `kai.*`) with the
/// schemas of its input and output, for agents and scripts.
pub fn describe<A: App>() -> Value {
    json!({
        "$schema": DIALECT,
        "format": FORMAT,
        "commands": CommandRegistry::<A>::of().descriptions(),
    })
}

/// [`describe`] as indented JSON, what `--describe-commands` prints.
pub fn describe_json<A: App>() -> String {
    serde_json::to_string_pretty(&describe::<A>()).expect("a JSON value always serializes")
}

/// Decode the input of command `id`. A unit struct, whose JSON is `null`,
/// also takes `{}`: that is what a client sends to a command without
/// parameters.
pub(super) fn decode<T: DeserializeOwned>(id: &str, input: Value) -> Result<T, CommandError> {
    let empty = matches!(&input, Value::Object(map) if map.is_empty());
    T::deserialize(&input)
        .or_else(|error| {
            if empty {
                T::deserialize(Value::Null).map_err(|_| error)
            } else {
                Err(error)
            }
        })
        .map_err(|error| CommandError::new(format!("entrada inválida para {id}: {error}")))
}

/// Schema of the input of a command: always an object, as MCP expects. A
/// unit struct is a closed empty object.
pub(super) fn input_schema<T: JsonSchema>() -> Value {
    let mut schema = root::<T>();
    if schema.get("type") == Some(&json!("null")) {
        schema = Map::new();
        schema.insert("type".into(), json!("object"));
        schema.insert("properties".into(), json!({}));
        schema.insert("additionalProperties".into(), json!(false));
    }
    Value::Object(schema)
}

/// Schema of the output of a query.
pub(super) fn output_schema<T: JsonSchema>() -> Value {
    Value::Object(root::<T>())
}

/// Schema of `T` without `$schema` (the document declares the dialect
/// once), `title` (the Rust type name) or `description` (the type's doc
/// comment): the command has its own title and description. Fields keep
/// theirs; text meant for agents goes in `#[schemars(description = ..)]`.
fn root<T: JsonSchema>() -> Map<String, Value> {
    let schema = SchemaSettings::draft2020_12()
        .with(|settings| settings.inline_subschemas = true)
        .into_generator()
        .into_root_schema_for::<T>();
    let mut map = match Value::from(schema) {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    for key in ["$schema", "title", "description"] {
        map.remove(key);
    }
    map
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::{decode, input_schema, output_schema};

    /// Some text.
    #[derive(Debug, Deserialize, JsonSchema, PartialEq)]
    struct Unit;

    #[derive(Debug, Deserialize, JsonSchema, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Add {
        amount: u32,
    }

    #[test]
    fn a_unit_input_is_a_closed_empty_object() {
        assert_eq!(decode::<Unit>("x.unit", json!({})), Ok(Unit));
        assert_eq!(decode::<Unit>("x.unit", json!(null)), Ok(Unit));
        assert!(decode::<Unit>("x.unit", json!({"a": 1})).is_err());
        assert_eq!(
            input_schema::<Unit>(),
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
        );
    }

    #[test]
    fn invalid_inputs_say_what_is_wrong() {
        assert_eq!(
            decode::<Add>("x.add", json!({"amount": 3})),
            Ok(Add { amount: 3 })
        );
        let missing = decode::<Add>("x.add", json!({})).unwrap_err();
        assert_eq!(
            missing.message,
            "entrada inválida para x.add: missing field `amount`"
        );
        let unknown = decode::<Add>("x.add", json!({"amount": 1, "extra": 2})).unwrap_err();
        assert!(
            unknown.message.contains("unknown field `extra`"),
            "{unknown}"
        );
        let wrong = decode::<Add>("x.add", json!({"amount": "tres"})).unwrap_err();
        assert!(wrong.message.contains("invalid type"), "{wrong}");
    }

    #[test]
    fn schemas_drop_dialect_and_type_name() {
        let schema = input_schema::<Add>();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["amount"]["type"], "integer");
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema.get("$schema").is_none() && schema.get("title").is_none());
        let output = output_schema::<Option<String>>();
        assert_eq!(output["type"], json!(["string", "null"]));
    }

    /// A RUT: a string with its pattern, never a number.
    struct Rut;

    impl JsonSchema for Rut {
        fn schema_name() -> Cow<'static, str> {
            "Rut".into()
        }
        fn json_schema(_: &mut SchemaGenerator) -> Schema {
            json_schema!({
                "type": "string",
                "pattern": r"^\d{1,2}\.\d{3}\.\d{3}-[\dkK]$",
                "examples": ["12.345.678-5"]
            })
        }
    }

    #[derive(JsonSchema)]
    #[serde(tag = "tipo")]
    #[allow(dead_code)]
    enum Medio {
        Efectivo,
        Transferencia { banco: String },
    }

    #[derive(JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Pagar {
        rut: Rut,
        /// Pesos.
        monto: u64,
        medio: Medio,
    }

    #[test]
    fn domain_types_follow_the_conventions_inline() {
        let schema = input_schema::<Pagar>();
        let properties = &schema["properties"];
        assert_eq!(properties["rut"]["type"], "string");
        assert!(properties["rut"]["pattern"].is_string());
        assert_eq!(properties["monto"]["type"], "integer");
        assert_eq!(properties["monto"]["minimum"], 0);
        let variants = properties["medio"]["oneOf"].as_array().expect("oneOf");
        let tags: Vec<&Value> = variants
            .iter()
            .map(|variant| &variant["properties"]["tipo"]["const"])
            .collect();
        assert_eq!(tags, [&json!("Efectivo"), &json!("Transferencia")]);
        assert!(!schema.to_string().contains("$ref"), "{schema:#}");
    }
}
