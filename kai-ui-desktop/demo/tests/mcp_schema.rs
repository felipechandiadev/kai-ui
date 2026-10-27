//! `mcp-schema.json` is what the demo publishes to agents; a change must be
//! deliberate. `KAI_UPDATE_SCHEMA=1 cargo test --test mcp_schema` rewrites
//! it.

use kai::commands::{DIALECT, describe_json};
use kai::serde_json::{self, Value};
use kai_demo::app::AppRoot;

/// Lines that differ between `expected` and `actual`, numbered; `None`
/// when they are equal.
fn line_diff(expected: &str, actual: &str) -> Option<String> {
    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();
    let mut diff = String::new();
    for line in 0..expected.len().max(actual.len()) {
        let (old, new) = (expected.get(line), actual.get(line));
        if old != new {
            for (sign, text) in [("-", old), ("+", new)] {
                if let Some(text) = text {
                    diff.push_str(&format!("{:>4} {sign} {text}\n", line + 1));
                }
            }
        }
    }
    (!diff.is_empty()).then_some(diff)
}

#[test]
fn mcp_schema_matches_snapshot() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("mcp-schema.json");
    let actual = describe_json::<AppRoot>() + "\n";
    if std::env::var_os("KAI_UPDATE_SCHEMA").is_some() {
        std::fs::write(&path, &actual).expect("write mcp-schema.json");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    if let Some(diff) = line_diff(&expected, &actual) {
        panic!(
            "mcp-schema.json cambió; si es a propósito, KAI_UPDATE_SCHEMA=1 cargo test \
             --test mcp_schema\n{diff}"
        );
    }
    let document: Value = serde_json::from_str(&actual).unwrap();
    assert_eq!(document["$schema"], DIALECT);
    let ids: Vec<&str> = document["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|command| command["id"].as_str())
        .collect();
    // One command of each kind of section: an action, a page query, a
    // framework command.
    for id in [
        "counter.add",
        "form.last_submission",
        "areas.edit_geometry",
        "kai.navigate",
    ] {
        assert!(ids.contains(&id), "{id}: {ids:?}");
    }
}
