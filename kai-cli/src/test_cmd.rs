//! `kai test`: delega en `cargo test` con features del manifiesto (13, Paso 18).

use std::process::{Command, ExitCode};

use crate::manifest;

#[derive(Debug, Default, PartialEq)]
pub struct Opciones {
    /// Activa la feature `snapshots` e incluye tests `#[ignore]` (capturas).
    pub snapshots: bool,
    /// Argumentos tras `--` para el binario de tests / filtros de cargo.
    pub cargo_args: Vec<String>,
}

pub fn run(opciones: &Opciones) -> Result<ExitCode, String> {
    let loaded = manifest::load_from_cwd().ok();
    let mut command = Command::new(cargo());
    command.arg("test");

    if let Some(ref loaded) = loaded {
        if let Some(package) = &loaded.manifest.dev.package {
            command.args(["-p", package]);
        }
        let mut features = loaded.manifest.dev.features.clone();
        if opciones.snapshots && !features.iter().any(|f| f == "snapshots") {
            features.push("snapshots".into());
        }
        if !features.is_empty() {
            command.args(["--features", &features.join(",")]);
        }
        eprintln!(
            "kai test: usando {} ({})",
            loaded.path.display(),
            loaded.manifest.app.name
        );
    } else {
        eprintln!("kai test: sin kai.toml; se llama a cargo test en el cwd");
        if opciones.snapshots {
            command.args(["--features", "snapshots"]);
        }
    }

    // Forward remaining args to cargo; if --snapshots, also include ignored.
    let mut forwarded = opciones.cargo_args.clone();
    if opciones.snapshots {
        // Ensure libtest sees --include-ignored after a `--` separator.
        if !forwarded.iter().any(|a| a == "--") {
            forwarded.push("--".into());
        }
        if !forwarded.iter().any(|a| a == "--include-ignored") {
            forwarded.push("--include-ignored".into());
        }
    }
    command.args(&forwarded);

    let status = command
        .status()
        .map_err(|error| format!("no se pudo ejecutar cargo test: {error}"))?;
    if status.success() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
    }
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_opciones() {
        assert_eq!(Opciones::default(), Opciones { snapshots: false, cargo_args: vec![] });
    }
}
