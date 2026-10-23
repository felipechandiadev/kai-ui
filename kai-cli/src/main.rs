//! `kai`: herramientas de desarrollo de apps Kai (13, Pasos 17–18).
//!
//! Lee `kai.toml` cuando existe. Comandos: `dev`, `doctor`, `test`, `new`,
//! `icons add` (Paso 19). Firma/bundle (Paso 20) y `commands`/`mcp` quedan fuera.

mod cargo;
mod dev;
mod doctor;
mod manifest;
mod icons_cmd;
mod new_cmd;
mod test_cmd;
mod watch;

use std::path::PathBuf;
use std::process::ExitCode;

const USO: &str = "\
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
enum Comando {
    Dev(dev::Opciones),
    Doctor,
    Test(test_cmd::Opciones),
    New(new_cmd::Opciones),
    IconsAdd(icons_cmd::Opciones),
    Ayuda,
    Version,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Comando, String> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        None | Some("-h" | "--help" | "help") => Ok(Comando::Ayuda),
        Some("-V" | "--version") => Ok(Comando::Version),
        Some("dev") => parse_dev(args).map(Comando::Dev),
        Some("doctor") => {
            if args.next().is_some() {
                return Err("kai doctor no acepta argumentos".into());
            }
            Ok(Comando::Doctor)
        }
        Some("test") => parse_test(args).map(Comando::Test),
        Some("new") => parse_new(args).map(Comando::New),
        Some("icons") => parse_icons(args).map(Comando::IconsAdd),
        Some(otro) => Err(format!("comando desconocido: {otro}")),
    }
}

fn parse_dev(mut args: impl Iterator<Item = String>) -> Result<dev::Opciones, String> {
    let mut opciones = dev::Opciones::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bin" => {
                let bin = args.next().ok_or("--bin necesita un nombre")?;
                opciones.bin = Some(bin);
            }
            "--watch" => {
                let carpeta = args.next().ok_or("--watch necesita una carpeta")?;
                opciones.watch.push(PathBuf::from(carpeta));
            }
            "--" => {
                opciones.args = args.collect();
                break;
            }
            "-h" | "--help" => return Err(String::new()),
            otro => return Err(format!("argumento desconocido para kai dev: {otro}")),
        }
    }
    Ok(opciones)
}

fn parse_test(mut args: impl Iterator<Item = String>) -> Result<test_cmd::Opciones, String> {
    let mut opciones = test_cmd::Opciones::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--snapshots" => opciones.snapshots = true,
            "--" => {
                opciones.cargo_args.push("--".into());
                opciones.cargo_args.extend(args);
                break;
            }
            "-h" | "--help" => return Err(String::new()),
            otro => {
                // Cargo-style passthrough (e.g. --lib, filtro).
                opciones.cargo_args.push(otro.into());
                opciones.cargo_args.extend(args);
                break;
            }
        }
    }
    Ok(opciones)
}

fn parse_new(mut args: impl Iterator<Item = String>) -> Result<new_cmd::Opciones, String> {
    let nombre = args
        .next()
        .ok_or_else(|| "kai new necesita un nombre de proyecto".to_string())?;
    if nombre.starts_with('-') {
        return Err(format!("nombre inválido: {nombre}"));
    }
    let mut template = new_cmd::Template::Basic;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--template" => {
                let t = args.next().ok_or("--template necesita basic o multiwindow")?;
                template = match t.as_str() {
                    "basic" => new_cmd::Template::Basic,
                    "multiwindow" => new_cmd::Template::Multiwindow,
                    otro => {
                        return Err(format!(
                            "plantilla desconocida: {otro} (basic|multiwindow)"
                        ));
                    }
                };
            }
            "-h" | "--help" => return Err(String::new()),
            otro => return Err(format!("argumento desconocido para kai new: {otro}")),
        }
    }
    Ok(new_cmd::Opciones { nombre, template })
}

fn parse_icons(mut args: impl Iterator<Item = String>) -> Result<icons_cmd::Opciones, String> {
    match args.next().as_deref() {
        Some("add") => icons_cmd::parse_args(args),
        Some(otro) => Err(format!(
            "subcomando desconocido para kai icons: {otro} (add)"
        )),
        None => Err("kai icons necesita un subcomando (add)".into()),
    }
}

fn main() -> ExitCode {
    match parse(std::env::args().skip(1)) {
        Ok(Comando::Ayuda) => {
            println!("{USO}");
            ExitCode::SUCCESS
        }
        Ok(Comando::Version) => {
            println!("kai {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Comando::Dev(opciones)) => match dev::run(&opciones) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai dev: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Comando::Doctor) => match doctor::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai doctor: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Comando::Test(opciones)) => match test_cmd::run(&opciones) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("kai test: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Comando::New(opciones)) => match new_cmd::run(&opciones) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kai new: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Comando::IconsAdd(opciones)) => match icons_cmd::run(&opciones) {
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
            eprintln!("{USO}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> Result<Comando, String> {
        parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn dev_takes_a_bin_folders_to_watch_and_the_app_arguments() {
        assert_eq!(
            parse_str(&["dev"]),
            Ok(Comando::Dev(dev::Opciones::default()))
        );
        assert_eq!(
            parse_str(&[
                "dev", "--bin", "demo", "--watch", "../temas", "--watch", "datos", "--", "--mcp",
                "--bin",
            ]),
            Ok(Comando::Dev(dev::Opciones {
                bin: Some("demo".into()),
                watch: vec!["../temas".into(), "datos".into()],
                args: vec!["--mcp".into(), "--bin".into()],
            }))
        );
    }

    #[test]
    fn doctor_test_new_and_help() {
        assert_eq!(parse_str(&[]), Ok(Comando::Ayuda));
        assert_eq!(parse_str(&["--help"]), Ok(Comando::Ayuda));
        assert_eq!(parse_str(&["--version"]), Ok(Comando::Version));
        assert_eq!(parse_str(&["doctor"]), Ok(Comando::Doctor));
        assert_eq!(
            parse_str(&["test", "--snapshots"]),
            Ok(Comando::Test(test_cmd::Opciones {
                snapshots: true,
                cargo_args: vec![],
            }))
        );
        assert_eq!(
            parse_str(&["new", "hola", "--template", "basic"]),
            Ok(Comando::New(new_cmd::Opciones {
                nombre: "hola".into(),
                template: new_cmd::Template::Basic,
            }))
        );
        assert_eq!(
            parse_str(&["icons", "add", "search"]),
            Ok(Comando::IconsAdd(icons_cmd::Opciones {
                nombre: "search".into(),
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
