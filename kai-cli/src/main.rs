//! `kai`: herramientas de desarrollo de apps Kai (13, Pasos 17–18).
//!
//! Lee `kai.toml` cuando existe. Comandos: `dev`, `doctor`, `test`, `new`,
//! `icons add` (Paso 19), `bundle`, `dmg`, `sign` y `notarize` (Paso 20).
//! `commands list` y `mcp list` leen el registro de la app.

mod bundle;
mod cargo;
mod commands_cmd;
mod dev;
mod dmg;
mod doctor;
mod icons_cmd;
mod manifest;
mod new_cmd;
mod notarize;
mod sign;
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
  kai new section <nombre>
                          Crea src/sections/<nombre>/ con section.rs y page.rs
  kai new layout [<sección>[/<carpeta>]]
                          Crea layout.rs (sin argumento, el de toda la app) con su outlet()
  kai icons add <nombre> [--from RUTA]
  kai icons add <archivo.svg> --name <nombre>
                          Copia un SVG a assets/icons/ (Lucide del crate kai o --from)
  kai bundle [--bin NOMBRE] [--debug]
                          Genera target/kai/<Nombre>.app (sin firmar) desde kai.toml
  kai dmg [--bundle RUTA]
                          Imagen .dmg del .app con un acceso a /Applications (firma el .app antes)
  kai sign [--identity 'Developer ID Application: …'] [--bundle RUTA]
                          codesign del .app; sin --identity firma ad hoc (solo este Mac)
  kai notarize --profile NOMBRE [--bundle RUTA]
                          Envía a Apple, espera y grapa (perfil de notarytool en el llavero)
  kai commands list [--bin NOMBRE] [--exposed]
                          Los comandos de la app (los lee de su binario con --describe-commands)
  kai mcp list [--bin NOMBRE]
                          Solo los que un agente puede llamar
  kai dev [--bin NOMBRE] [--watch CARPETA]... [-- ARGUMENTOS]
                          Compila, abre y reinicia conservando estado

  kai --help | kai --version

Si hay kai.toml, `kai dev` usa [dev].bin y observa el manifiesto.";

#[derive(Debug, PartialEq)]
enum Command {
    Bundle(bundle::Options),
    Sign(sign::Options),
    Dmg(dmg::Options),
    Notarize(notarize::Options),
    Dev(dev::Options),
    Commands(commands_cmd::Options),
    Doctor,
    Test(test_cmd::Options),
    New(new_cmd::Options),
    NewSection(String),
    NewLayout(Option<String>),
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
        Some("bundle") => parse_bundle(args).map(Command::Bundle),
        Some("sign") => parse_sign(args).map(Command::Sign),
        Some("dmg") => parse_dmg(args).map(Command::Dmg),
        Some("notarize") => parse_notarize(args).map(Command::Notarize),
        Some("doctor") => {
            if args.next().is_some() {
                return Err("kai doctor no acepta argumentos".into());
            }
            Ok(Command::Doctor)
        }
        Some("test") => parse_test(args).map(Command::Test),
        Some("new") => parse_new(args),
        Some("icons") => parse_icons(args).map(Command::IconsAdd),
        Some("commands") => match args.next().as_deref() {
            Some("list") => commands_cmd::parse_args(args, false).map(Command::Commands),
            Some(other) => Err(format!(
                "subcomando desconocido para kai commands: {other} (list)"
            )),
            None => Err("kai commands necesita un subcomando (list)".into()),
        },
        Some("mcp") => match args.next().as_deref() {
            Some("list") => commands_cmd::parse_args(args, true).map(Command::Commands),
            Some(other) => Err(format!(
                "subcomando desconocido para kai mcp: {other} (list)"
            )),
            None => Err("kai mcp necesita un subcomando (list)".into()),
        },
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

fn parse_bundle(mut args: impl Iterator<Item = String>) -> Result<bundle::Options, String> {
    let mut options = bundle::Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bin" => {
                options.bin = Some(args.next().ok_or("--bin necesita un nombre")?);
            }
            "--debug" => options.debug = true,
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumento desconocido para kai bundle: {other}")),
        }
    }
    Ok(options)
}

fn parse_dmg(mut args: impl Iterator<Item = String>) -> Result<dmg::Options, String> {
    let mut options = dmg::Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bundle" => {
                options.bundle = Some(args.next().ok_or("--bundle necesita una ruta")?.into());
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumento desconocido para kai dmg: {other}")),
        }
    }
    Ok(options)
}

fn parse_sign(mut args: impl Iterator<Item = String>) -> Result<sign::Options, String> {
    let mut options = sign::Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--identity" => {
                options.identity = Some(args.next().ok_or("--identity necesita un nombre")?);
            }
            "--bundle" => {
                options.bundle = Some(args.next().ok_or("--bundle necesita una ruta")?.into());
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumento desconocido para kai sign: {other}")),
        }
    }
    Ok(options)
}

fn parse_notarize(mut args: impl Iterator<Item = String>) -> Result<notarize::Options, String> {
    let mut options = notarize::Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--profile" => {
                options.profile = Some(args.next().ok_or("--profile necesita un nombre")?);
            }
            "--bundle" => {
                options.bundle = Some(args.next().ok_or("--bundle necesita una ruta")?.into());
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumento desconocido para kai notarize: {other}")),
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

fn parse_new(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let name = args
        .next()
        .ok_or_else(|| "kai new necesita un nombre de proyecto".to_string())?;
    if name == "section" {
        let section = args
            .next()
            .ok_or("kai new section necesita un nombre (snake_case en inglés)")?;
        if let Some(extra) = args.next() {
            return Err(format!(
                "argumento desconocido para kai new section: {extra}"
            ));
        }
        return Ok(Command::NewSection(section));
    }
    if name == "layout" {
        let folder = args.next();
        if let Some(extra) = args.next() {
            return Err(format!(
                "argumento desconocido para kai new layout: {extra}"
            ));
        }
        return Ok(Command::NewLayout(folder));
    }
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
    Ok(Command::New(new_cmd::Options { name, template }))
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
        Ok(Command::Commands(options)) => match commands_cmd::run(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai commands: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Bundle(options)) => match bundle::run(&options) {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai bundle: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Dmg(options)) => match dmg::run(&options) {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai dmg: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Sign(options)) => match sign::run(&options) {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai sign: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Notarize(options)) => match notarize::run(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai notarize: {error}");
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
        Ok(Command::NewSection(name)) => match new_cmd::section(std::path::Path::new("."), &name) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai new section: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::NewLayout(folder)) => {
            match new_cmd::layout(std::path::Path::new("."), folder.as_deref()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("kai new layout: {error}");
                    ExitCode::FAILURE
                }
            }
        }
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
    fn bundle_takes_a_bin_and_debug() {
        assert_eq!(
            parse_str(&["bundle"]),
            Ok(Command::Bundle(bundle::Options::default()))
        );
        assert_eq!(
            parse_str(&["bundle", "--bin", "demo", "--debug"]),
            Ok(Command::Bundle(bundle::Options {
                bin: Some("demo".into()),
                debug: true,
            }))
        );
        assert!(parse_str(&["bundle", "--release"]).is_err());
    }

    #[test]
    fn dmg_commands_and_mcp_take_their_options() {
        assert_eq!(
            parse_str(&["dmg", "--bundle", "x/App.app"]),
            Ok(Command::Dmg(dmg::Options {
                bundle: Some("x/App.app".into())
            }))
        );
        assert!(parse_str(&["dmg", "--mas"]).is_err());
        assert_eq!(
            parse_str(&["commands", "list", "--exposed"]),
            Ok(Command::Commands(commands_cmd::Options {
                bin: None,
                exposed_only: true
            }))
        );
        assert_eq!(
            parse_str(&["mcp", "list", "--bin", "demo"]),
            Ok(Command::Commands(commands_cmd::Options {
                bin: Some("demo".into()),
                exposed_only: true
            }))
        );
        assert!(parse_str(&["commands"]).is_err());
        assert!(parse_str(&["mcp", "run"]).is_err());
    }

    #[test]
    fn sign_and_notarize_take_their_options() {
        assert_eq!(
            parse_str(&["sign"]),
            Ok(Command::Sign(sign::Options::default()))
        );
        assert_eq!(
            parse_str(&["sign", "--identity", "Developer ID Application: Kai"]),
            Ok(Command::Sign(sign::Options {
                identity: Some("Developer ID Application: Kai".into()),
                bundle: None,
            }))
        );
        assert_eq!(
            parse_str(&["notarize", "--profile", "kai-notary"]),
            Ok(Command::Notarize(notarize::Options {
                profile: Some("kai-notary".into()),
                bundle: None,
            }))
        );
        assert!(parse_str(&["sign", "--password", "x"]).is_err());
        assert!(parse_str(&["notarize", "--profile"]).is_err());
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
            parse_str(&["new", "section", "customers"]),
            Ok(Command::NewSection("customers".into()))
        );
        assert!(parse_str(&["new", "section"]).is_err());
        assert_eq!(parse_str(&["new", "layout"]), Ok(Command::NewLayout(None)));
        assert_eq!(
            parse_str(&["new", "layout", "customers/new"]),
            Ok(Command::NewLayout(Some("customers/new".into())))
        );
        assert!(parse_str(&["new", "layout", "a", "b"]).is_err());
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
