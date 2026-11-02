//! `kai commands list` and `kai mcp list`: what the app publishes, read from
//! its own binary (`--describe-commands`, which `kai::commands::describe_json`
//! prints), so the list is the registry's, not a copy.

use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::cargo::{self, Failure};

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    pub bin: Option<String>,
    /// `kai mcp list`: only what an agent can call.
    pub exposed_only: bool,
}

pub fn parse_args(
    mut args: impl Iterator<Item = String>,
    exposed_only: bool,
) -> Result<Options, String> {
    let mut options = Options {
        bin: None,
        exposed_only,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bin" => options.bin = Some(args.next().ok_or("--bin necesita un nombre")?),
            "--exposed" => options.exposed_only = true,
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumento desconocido: {other}")),
        }
    }
    Ok(options)
}

pub fn run(options: &Options) -> Result<(), String> {
    let binary = match cargo::build(options.bin.as_deref(), &[], false) {
        Ok(path) => path,
        Err(Failure::Build) => return Err("la app no compila".into()),
        Err(Failure::Other(error)) => return Err(error),
    };
    let output = Command::new(&binary)
        .arg("--describe-commands")
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("no se pudo ejecutar {}: {error}", binary.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} --describe-commands falló; la app debe aceptar ese argumento \
             (kai::commands::describe_json)",
            binary.display()
        ));
    }
    let json = String::from_utf8_lossy(&output.stdout);
    print!("{}", table(&json, options.exposed_only)?);
    Ok(())
}

#[derive(Deserialize)]
struct Schema {
    commands: Vec<Described>,
}

#[derive(Deserialize)]
struct Described {
    id: String,
    title: String,
    kind: String,
    #[serde(default)]
    safety: String,
    #[serde(default)]
    exposed: bool,
}

/// The commands as aligned columns: id, kind, safety, exposed, title.
fn table(json: &str, exposed_only: bool) -> Result<String, String> {
    let schema: Schema = serde_json::from_str(json)
        .map_err(|error| format!("la app devolvió algo que no es el esquema: {error}"))?;
    let rows: Vec<_> = schema
        .commands
        .iter()
        .filter(|command| !exposed_only || command.exposed)
        .collect();
    let width = |pick: fn(&Described) -> &str| {
        rows.iter()
            .map(|command| pick(command).chars().count())
            .max()
            .unwrap_or(0)
    };
    let (id, kind, safety) = (
        width(|c| &c.id).max(2),
        width(|c| &c.kind).max(4),
        width(|c| &c.safety).max(9),
    );
    let mut out = format!(
        "{:<id$}  {:<kind$}  {:<safety$}  MCP  TÍTULO\n",
        "ID", "TIPO", "SEGURIDAD"
    );
    for command in &rows {
        out.push_str(&format!(
            "{:<id$}  {:<kind$}  {:<safety$}  {:<3}  {}\n",
            command.id,
            command.kind,
            command.safety,
            if command.exposed { "sí" } else { "no" },
            command.title
        ));
    }
    out.push_str(&format!("{} comandos\n", rows.len()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"commands":[
        {"id":"kai.back","title":"Atrás","kind":"action","safety":"mutating","exposed":true},
        {"id":"form.toggle_password","title":"Mostrar contraseña","kind":"action","safety":"mutating","exposed":false},
        {"id":"counter.reset","title":"Reiniciar","kind":"action","safety":"destructive","exposed":true}
    ],"format":"x"}"#;

    #[test]
    fn the_list_shows_every_command_or_only_the_exposed_ones() {
        let all = table(JSON, false).unwrap();
        assert!(all.contains("form.toggle_password"));
        assert!(all.ends_with("3 comandos\n"));
        let exposed = table(JSON, true).unwrap();
        assert!(!exposed.contains("form.toggle_password"));
        assert!(exposed.contains("counter.reset"));
        assert!(exposed.ends_with("2 comandos\n"));
    }

    #[test]
    fn the_columns_line_up() {
        let all = table(JSON, false).unwrap();
        let column = |line: &str| line.find("sí").or_else(|| line.find("no "));
        let mut lines = all.lines().skip(1);
        let first = lines.next().unwrap();
        let second = lines.next().unwrap();
        assert_eq!(column(first), column(second));
    }

    #[test]
    fn something_that_is_not_the_schema_is_an_error() {
        assert!(table("no es json", false).is_err());
    }

    #[test]
    fn the_arguments_are_a_binary_and_the_exposed_filter() {
        let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            parse_args(args(&["--bin", "demo"]).into_iter(), false),
            Ok(Options {
                bin: Some("demo".into()),
                exposed_only: false
            })
        );
        assert!(
            parse_args(args(&["--exposed"]).into_iter(), false)
                .unwrap()
                .exposed_only
        );
        assert!(parse_args(args(&["--mas"]).into_iter(), false).is_err());
    }
}
