//! `kai new`: andamiaje mínimo de una app Kai (13, Paso 18). Plantilla `basic`.

use std::fs;
use std::path::PathBuf;

use crate::manifest;

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub name: String,
    pub template: Template,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Template {
    Basic,
    /// Reservado (Paso 23); por ahora igual que basic con un aviso.
    Multiwindow,
}

pub fn run(options: &Options) -> Result<(), String> {
    let dir = PathBuf::from(&options.name);
    if dir.exists() {
        return Err(format!("{} ya existe", dir.display()));
    }
    if options.template == Template::Multiwindow {
        eprintln!("kai new: plantilla multiwindow aún no tiene varias ventanas; se genera basic");
    }

    let crate_name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "kai new: el nombre debe terminar en un segmento de carpeta".to_string())?;
    let id = format!("dev.kai.{}", sanitize_id(crate_name));
    fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("tests")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("assets/icons")).map_err(|e| e.to_string())?;
    fs::write(dir.join("assets/icons/.gitkeep"), "").map_err(|e| e.to_string())?;

    fs::write(
        dir.join("kai.toml"),
        manifest::example_toml(crate_name, &id, Some(crate_name)),
    )
    .map_err(|e| e.to_string())?;

    fs::write(dir.join("Cargo.toml"), cargo_toml(crate_name)).map_err(|e| e.to_string())?;
    fs::write(dir.join("src/main.rs"), MAIN_RS).map_err(|e| e.to_string())?;
    fs::write(dir.join("tests/smoke.rs"), SMOKE_RS).map_err(|e| e.to_string())?;
    fs::write(dir.join(".gitignore"), GITIGNORE).map_err(|e| e.to_string())?;
    fs::write(dir.join("README.md"), readme(crate_name)).map_err(|e| e.to_string())?;

    eprintln!(
        "kai new: creado {} (edita Cargo.toml para apuntar al crate `kai`)",
        dir.display()
    );
    eprintln!("  cd {} && kai doctor && kai test", dir.display());
    Ok(())
}

fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
publish = false

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
# Ajusta el path (o versión) al crate `kai` de kai-ui-desktop:
kai = {{ package = "kai-ui-desktop-prototype", path = "../kai-ui-desktop" }}
serde_json = "1"
"#
    )
}

const MAIN_RS: &str = r#"//! App mínima generada por `kai new`.

use kai::element::{div, text, Element};
use kai::runtime::{App, Effects};
use kai::theme::{Density, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    kai::runtime::run(Demo, Theme::kai(Density::Comfortable))
}

struct Demo;

#[derive(Clone, Debug)]
enum Msg {}

impl App for Demo {
    type Msg = Msg;

    fn title(&self) -> String {
        "Kai demo".into()
    }

    fn update(&mut self, _msg: Msg) -> Effects<Msg> {
        Effects::none()
    }

    fn view(&self) -> Element<Msg> {
        div().child(text("Hola desde kai new"))
    }
}
"#;

const SMOKE_RS: &str = r#"//! Ejemplo de prueba con `TestApp` (ajusta cuando la app crezca).

#[test]
fn placeholder() {
    // use kai::testing::{By, TestApp};
    // let mut app = TestApp::new(Demo);
    // assert!(app.find(By::text("Hola")).is_some());
    assert!(true);
}
"#;

const GITIGNORE: &str = "/target/\n/Cargo.lock\n";

fn readme(name: &str) -> String {
    format!(
        "# {name}\n\nGenerado con `kai new`. Flujo: `kai doctor` → `kai test` → `kai dev`.\n\n\
Iconos propios: `kai icons add <nombre>` (Lucide del crate kai) o\n\
`kai icons add ./mi.svg --name foo`. Quedan en `assets/icons/`; en Rust:\n\
`kai::element::icon_svg(include_str!(\"../assets/icons/foo.svg\"))`.\n"
    )
}

/// `kai new section <name>`: `src/sections/<name>/section.rs` and
/// `page.rs` in the app at `root`, plus `src/state.rs` if it is missing.
pub fn section(root: &std::path::Path, name: &str) -> Result<(), String> {
    if !kai_build::scan::is_valid_name(name) {
        return Err(format!(
            "nombre inválido: {name} (snake_case en inglés, como customers o sales_report)"
        ));
    }
    let dir = root.join("src/sections").join(name);
    if dir.exists() {
        return Err(format!("{} ya existe", dir.display()));
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let title = title_of(name);
    fs::write(dir.join("section.rs"), section_rs(&title)).map_err(|e| e.to_string())?;
    fs::write(dir.join("page.rs"), page_rs(&title)).map_err(|e| e.to_string())?;
    let state = root.join("src/state.rs");
    if !state.exists() {
        fs::write(&state, STATE_RS).map_err(|e| e.to_string())?;
        eprintln!("kai new section: creado {}", state.display());
    }
    eprintln!("kai new section: creado {}", dir.display());
    if !root.join("build.rs").exists() {
        eprintln!(
            "  falta build.rs con kai_build::app().generate() y kai::include_app!() en src/main.rs \
             (ver docs/dev/26-router-y-kai-build.md)"
        );
    }
    Ok(())
}

/// `sales_report` → `Sales report`: the visible title until someone writes
/// the real one.
fn title_of(name: &str) -> String {
    let text = name.replace('_', " ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => text,
    }
}

fn section_rs(title: &str) -> String {
    format!(
        r#"use kai::icons::Icon;
use kai::router::SectionMeta;

/// How this section shows in the sidebar (title and group are visible text).
pub const SECTION: SectionMeta = SectionMeta {{
    title: "{title}",
    icon: Icon::LayoutGrid,
    order: 100,
    group: None,
}};
"#
    )
}

fn page_rs(title: &str) -> String {
    format!(
        r#"use kai::element::{{Element, text}};
use kai::router::{{Context, Page}};
use kai::runtime::Effects;

use crate::state::State;

pub struct Screen;

#[derive(Clone, Debug)]
pub enum Msg {{}}

impl Page for Screen {{
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {{
        (Self, Effects::none())
    }}

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {{
        match msg {{}}
    }}

    fn view(&self, _: &State) -> Element<Msg> {{
        text("{title}")
    }}

    fn title(&self, _: &State) -> String {{
        "{title}".into()
    }}
}}
"#
    )
}

const STATE_RS: &str =
    "/// What every page shares (session, loaded data).\n#[derive(Default)]\npub struct State {}\n";

#[cfg(test)]
mod tests {

    #[test]
    fn a_new_section_follows_the_folder_conventions() {
        let root = std::env::temp_dir().join(format!("kai-new-section-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        super::section(&root, "sales_report").unwrap();
        let tree = kai_build::scan::scan(&root).expect("valid folders");
        assert_eq!(tree.sections[0].name, "sales_report");
        let meta =
            std::fs::read_to_string(root.join("src/sections/sales_report/section.rs")).unwrap();
        assert!(meta.contains("title: \"Sales report\""));
        assert!(
            super::section(&root, "sales_report").is_err(),
            "it already exists"
        );
        assert!(
            super::section(&root, "Ventas").is_err(),
            "not snake_case English"
        );
        let _ = std::fs::remove_dir_all(root);
    }
    use super::*;

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_id("Mi App"), "mi-app");
    }

    #[test]
    fn scaffolds_basic() {
        let base = std::env::temp_dir().join(format!("kai-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let name = base.join("demo-app");
        run(&Options {
            name: name.to_str().unwrap().into(),
            template: Template::Basic,
        })
        .unwrap();
        assert!(name.join("kai.toml").is_file());
        assert!(name.join("Cargo.toml").is_file());
        assert!(name.join("src/main.rs").is_file());
        assert!(name.join("assets/icons").is_dir());
        let toml = fs::read_to_string(name.join("kai.toml")).unwrap();
        assert!(toml.contains("[app]"));
        assert!(toml.contains("[dev]"));
        let _ = fs::remove_dir_all(&base);
    }
}
