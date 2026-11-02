//! How the pages write their code: `use` and a builder chain, with texts
//! as Rust literals.

/// `use kai::components::{..};`, a blank line and `body`.
pub fn snippet(uses: &[&str], body: impl Into<String>) -> String {
    format!(
        "use kai::components::{{{}}};\n\n{}",
        uses.join(", "),
        body.into()
    )
}

/// `head` followed by one `    .call` line per present call.
pub fn chain(head: impl Into<String>, calls: impl IntoIterator<Item = Option<String>>) -> String {
    let mut out = head.into();
    for call in calls.into_iter().flatten() {
        out.push_str("\n    .");
        out.push_str(&call);
    }
    out
}

/// A text as a Rust literal (`"Guardar"`, escaped).
pub fn lit(text: &str) -> String {
    format!("{text:?}")
}

/// The call, only if `on`.
pub fn when(on: bool, call: impl Into<String>) -> Option<String> {
    on.then(|| call.into())
}

/// `.call` always.
pub fn call(call: impl Into<String>) -> Option<String> {
    Some(call.into())
}

/// `Some("texto")`, or `None` if empty.
pub fn option(text: &str) -> String {
    if text.is_empty() {
        "None".to_owned()
    } else {
        format!("Some({})", lit(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_skips_absent_calls_and_quotes_literals() {
        let code = chain(
            format!("button({}, ButtonVariant::Primary)", lit("Di \"hola\"")),
            [when(false, "disabled(true)"), call("on_click(Msg::Save)")],
        );
        assert_eq!(
            code,
            "button(\"Di \\\"hola\\\"\", ButtonVariant::Primary)\n    .on_click(Msg::Save)"
        );
        assert_eq!(
            snippet(&["button", "ButtonVariant"], "x"),
            "use kai::components::{button, ButtonVariant};\n\nx"
        );
        assert_eq!(option(""), "None");
        assert_eq!(option("Hoy"), "Some(\"Hoy\")");
    }
}
