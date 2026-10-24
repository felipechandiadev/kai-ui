//! `kai`: herramientas de desarrollo de apps Kai (13, Pasos 17–18).
//!
//! Lee `kai.toml` cuando existe. Comandos: `dev`, `doctor`, `test`, `new`,
//! `icons add` (Paso 19). Firma/bundle (Paso 20) y `commands`/`mcp` quedan fuera.

mod cargo;
mod dev;
mod doctor;
mod icons_cmd;
mod manifest;
mod new_cmd;
mod test_cmd;
mod watch;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Uso: kai <comando> [opciones]

Comandos:
  kai doctor              Comprueba Rust ≥ MSRV, Xcode CLT (macOS) y kai.toml
  kai test [--snapshots] [-- ARGS]
                          cargo test con features de [dev] en kai.toml
  kai new <nombre> [--template basic|multiwindow]
                          Andamiaje mínimo con kai.toml y tests/
  kai icons add <nombre> [--from RUTA]
  kai icons add <archivo.svg> --name <nombre>
                          Copia un SVG a assets/icons/ (Lucide del crate kai o --from)
  kai dev [--bin NOMBRE] [--watch CARPETA]... [-- ARGUMENTOS]
                          Compila, abre y reinicia conservando estado

  kai --help | kai --version

Si hay kai.toml, `kai dev` usa [dev].bin y observa el manifiesto.";

#[derive(Debug, PartialEq)]
enum Command {
    Dev(dev::Options),
    Doctor,
    Test(test_cmd::Options),
    New(new_cmd::Options),
    IconsAdd(icons_cmd::Options),
    Help,
    Version,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        None | Some("-h" | "--help" | "help") => Ok(Command::Help),
        Some("-V" | "--version") => Ok(Command::Version),
        Some("dev") => parse_dev(args).map(Command::Dev),
        Some("doctor") => {
            if args.next().is_some() {
                return Err("kai doctor no acepta argumentos".into());
            }
            Ok(Command::Doctor)
        }
        Some("test") => parse_test(args).map(Command::Test),
        Some("new") => parse_new(args).map(Command::New),
        Some("icons") => parse_icons(args).map(Command::IconsAdd),
        Some(other) => Err(format!("comando desconocido: {other}")),
    }
}

fn parse_dev(mut args: impl Iterator<Item = String>) -> Result<dev::Options, String> {
    let mut options = dev::Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bin" => {
                let bin = args.next().ok_or("--bin necesita un nombre")?;
                options.bin = Some(bin);
            }
            "--watch" => {
                let folder = args.next().ok_or("--watch necesita una carpeta")?;
                options.watch.push(PathBuf::from(folder));
            }
            "--" => {
                options.args = args.collect();
                break;
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumento desconocido para kai dev: {other}")),
        }
    }
    Ok(options)
}

fn parse_test(mut args: impl Iterator<Item = String>) -> Result<test_cmd::Options, String> {
    let mut options = test_cmd::Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--snapshots" => options.snapshots = true,
            "--" => {
                options.cargo_args.push("--".into());
                options.cargo_args.extend(args);
                break;
            }
            "-h" | "--help" => return Err(String::new()),
            other => {
                // Cargo-style passthrough (e.g. --lib, filtro).
                options.cargo_args.push(other.into());
                options.cargo_args.extend(args);
                break;
            }
        }
    }
    Ok(options)
}

fn parse_new(mut args: impl Iterator<Item = String>) -> Result<new_cmd::Options, String> {
    let name = args
        .next()
        .ok_or_else(|| "kai new necesita un nombre de proyecto".to_string())?;
    if name.starts_with('-') {
        return Err(format!("nombre inválido: {name}"));
    }
    let mut template = new_cmd::Template::Basic;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--template" => {
                let t = args
                    .next()
                    .ok_or("--template necesita basic o multiwindow")?;
                template = match t.as_str() {
                    "basic" => new_cmd::Template::Basic,
                    "multiwindow" => new_cmd::Template::Multiwindow,
                    other => {
                        return Err(format!(
                            "plantilla desconocida: {other} (basic|multiwindow)"
                        ));
                    }
                };
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumento desconocido para kai new: {other}")),
        }
    }
    Ok(new_cmd::Options { name, template })
}

fn parse_icons(mut args: impl Iterator<Item = String>) -> Result<icons_cmd::Options, String> {
    match args.next().as_deref() {
        Some("add") => icons_cmd::parse_args(args),
        Some(other) => Err(format!(
            "subcomando desconocido para kai icons: {other} (add)"
        )),
        None => Err("kai icons necesita un subcomando (add)".into()),
    }
}

fn main() -> ExitCode {
    match parse(std::env::args().skip(1)) {
        Ok(Command::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("kai {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Dev(options)) => match dev::run(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai dev: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Doctor) => match doctor::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai doctor: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Test(options)) => match test_cmd::run(&options) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("kai test: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::New(options)) => match new_cmd::run(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai new: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::IconsAdd(options)) => match icons_cmd::run(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai icons add: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            if !error.is_empty() {
                eprintln!("kai: {error}\n");
            }
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn dev_takes_a_bin_folders_to_watch_and_the_app_arguments() {
        assert_eq!(
            parse_str(&["dev"]),
            Ok(Command::Dev(dev::Options::default()))
        );
        assert_eq!(
            parse_str(&[
                "dev", "--bin", "demo", "--watch", "../temas", "--watch", "datos", "--", "--mcp",
                "--bin",
            ]),
            Ok(Command::Dev(dev::Options {
                bin: Some("demo".into()),
                watch: vec!["../temas".into(), "datos".into()],
                args: vec!["--mcp".into(), "--bin".into()],
            }))
        );
    }

    #[test]
    fn doctor_test_new_and_help() {
        assert_eq!(parse_str(&[]), Ok(Command::Help));
        assert_eq!(parse_str(&["--help"]), Ok(Command::Help));
        assert_eq!(parse_str(&["--version"]), Ok(Command::Version));
        assert_eq!(parse_str(&["doctor"]), Ok(Command::Doctor));
        assert_eq!(
            parse_str(&["test", "--snapshots"]),
            Ok(Command::Test(test_cmd::Options {
                snapshots: true,
                cargo_args: vec![],
            }))
        );
        assert_eq!(
            parse_str(&["new", "hola", "--template", "basic"]),
            Ok(Command::New(new_cmd::Options {
                name: "hola".into(),
                template: new_cmd::Template::Basic,
            }))
        );
        assert_eq!(
            parse_str(&["icons", "add", "search"]),
            Ok(Command::IconsAdd(icons_cmd::Options {
                name: "search".into(),
                from: None,
            }))
        );
        assert_eq!(
            parse_str(&["nuevo"]),
            Err("comando desconocido: nuevo".into())
        );
        assert_eq!(
            parse_str(&["dev", "--bin"]),
            Err("--bin necesita un nombre".into())
        );
        assert_eq!(
            parse_str(&["dev", "--release"]),
            Err("argumento desconocido para kai dev: --release".into())
        );
    }
}
