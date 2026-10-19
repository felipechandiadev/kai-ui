//! Commands as JSON (13, Paso 11): decoding an input and the JSON Schema
//! 2020-12 of inputs and outputs.

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::CommandError;

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
        let description = schema.remove("description");
        schema = Map::new();
        schema.insert("type".into(), json!("object"));
        schema.insert("properties".into(), json!({}));
        schema.insert("additionalProperties".into(), json!(false));
        if let Some(description) = description {
            schema.insert("description".into(), description);
        }
    }
    Value::Object(schema)
}

/// Schema of the output of a query.
pub(super) fn output_schema<T: JsonSchema>() -> Value {
    Value::Object(root::<T>())
}

/// Schema of `T` without `$schema` (the document declares the dialect once)
/// nor `title` (the Rust type name; the command has its own title).
fn root<T: JsonSchema>() -> Map<String, Value> {
    let schema = SchemaSettings::draft2020_12()
        .into_generator()
        .into_root_schema_for::<T>();
    let mut map = match Value::from(schema) {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    map.remove("$schema");
    map.remove("title");
    map
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;

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
                "additionalProperties": false,
                "description": "Some text."
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
}
