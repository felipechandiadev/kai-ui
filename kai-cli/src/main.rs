//! `kai`: development tools for Kai apps (13, Paso 18).
//!
//! `kai dev` rebuilds the app of the current Cargo package when a source
//! changes and restarts it with its state (window frame and whatever the
//! app returns from `App::dev_state`). A build that fails leaves the
//! running version open.

mod cargo;
mod dev;
mod watch;

use std::path::PathBuf;
use std::process::ExitCode;

const USO: &str = "\
Uso: kai dev [--bin NOMBRE] [--watch CARPETA]... [-- ARGUMENTOS DE LA APP]

  kai dev       Compila la app del paquete actual, la abre y la reinicia con
                su estado cada vez que cambia un fuente.

Opciones de kai dev:
  --bin NOMBRE      Binario a abrir, si el paquete tiene varios.
  --watch CARPETA   Carpeta extra a observar (se puede repetir).
  -- ARGUMENTOS     Se pasan a la app.

  kai --help | kai --version";

#[derive(Debug, PartialEq)]
enum Comando {
    Dev(dev::Opciones),
    Ayuda,
    Version,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Comando, String> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        None | Some("-h" | "--help" | "help") => Ok(Comando::Ayuda),
        Some("-V" | "--version") => Ok(Comando::Version),
        Some("dev") => parse_dev(args).map(Comando::Dev),
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
    fn help_version_and_mistakes() {
        assert_eq!(parse_str(&[]), Ok(Comando::Ayuda));
        assert_eq!(parse_str(&["--help"]), Ok(Comando::Ayuda));
        assert_eq!(parse_str(&["--version"]), Ok(Comando::Version));
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
