//! `kai new`: andamiaje mínimo de una app Kai (13, Paso 18). Plantilla `basic`.

use std::fs;
use std::path::PathBuf;

use crate::manifest;

#[derive(Debug, Clone, PartialEq)]
pub struct Opciones {
    pub nombre: String,
    pub template: Template,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Template {
    Basic,
    /// Reservado (Paso 23); por ahora igual que basic con un aviso.
    Multiwindow,
}

pub fn run(opciones: &Opciones) -> Result<(), String> {
    let dir = PathBuf::from(&opciones.nombre);
    if dir.exists() {
        return Err(format!("{} ya existe", dir.display()));
    }
    if opciones.template == Template::Multiwindow {
        eprintln!(
            "kai new: plantilla multiwindow aún no tiene varias ventanas; se genera basic"
        );
    }

    let id = format!("dev.kai.{}", sanitize_id(&opciones.nombre));
    fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("tests")).map_err(|e| e.to_string())?;

    fs::write(
        dir.join("kai.toml"),
        manifest::example_toml(&opciones.nombre, &id, Some(&opciones.nombre)),
    )
    .map_err(|e| e.to_string())?;

    fs::write(dir.join("Cargo.toml"), cargo_toml(&opciones.nombre))
        .map_err(|e| e.to_string())?;
    fs::write(dir.join("src/main.rs"), MAIN_RS).map_err(|e| e.to_string())?;
    fs::write(dir.join("tests/smoke.rs"), SMOKE_RS).map_err(|e| e.to_string())?;
    fs::write(dir.join(".gitignore"), GITIGNORE).map_err(|e| e.to_string())?;
    fs::write(dir.join("README.md"), readme(&opciones.nombre)).map_err(|e| e.to_string())?;

    eprintln!(
        "kai new: creado {} (edita Cargo.toml para apuntar al crate `kai`)",
        dir.display()
    );
    eprintln!("  cd {} && kai doctor && kai test", dir.display());
    Ok(())
}

fn sanitize_id(nombre: &str) -> String {
    nombre
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn cargo_toml(nombre: &str) -> String {
    format!(
        r#"[package]
name = "{nombre}"
version = "0.1.0"
edition = "2024"
rust-version = "1.89"
publish = false

[[bin]]
name = "{nombre}"
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

fn readme(nombre: &str) -> String {
    format!(
        "# {nombre}\n\nGenerado con `kai new`. Flujo: `kai doctor` → `kai test` → `kai dev`.\n"
    )
}

#[cfg(test)]
mod tests {
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
        let nombre = base.join("demo-app");
        run(&Opciones {
            nombre: nombre.to_str().unwrap().into(),
            template: Template::Basic,
        })
        .unwrap();
        assert!(nombre.join("kai.toml").is_file());
        assert!(nombre.join("Cargo.toml").is_file());
        assert!(nombre.join("src/main.rs").is_file());
        let toml = fs::read_to_string(nombre.join("kai.toml")).unwrap();
        assert!(toml.contains("[app]"));
        assert!(toml.contains("[dev]"));
        let _ = fs::remove_dir_all(&base);
    }
}
