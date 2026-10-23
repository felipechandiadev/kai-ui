//! `kai dev`: build, open, and on every change rebuild and restart the app
//! with its state.
//!
//! The app is started with `KAI_DEV_STATE=<file>` and its stdin piped. To
//! restart it, `kai dev` writes `guardar-y-salir` on that stdin: a Kai app
//! in a debug build saves its window frame and `App::dev_state` there and
//! quits, and the next run reads them back. An app that does not answer in
//! [`PLAZO_SALIDA`] is killed. When the build fails, the running version
//! stays open. Ctrl+C reaches the whole process group, the app included.
//!
//! If a `kai.toml` is found, `[dev].bin` fills in `--bin` when omitted and
//! `[dev].features` are passed to `cargo build`. The manifesto path is
//! watched (via `cargo` metadata roots).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::cargo::{self, Fallo};
use crate::manifest;
use crate::watch::{self, Snapshot};

/// Variable and line the Kai runtime listens for (`kai::dev_state`).
const ENV_ESTADO: &str = "KAI_DEV_STATE";
const GUARDAR_Y_SALIR: &str = "guardar-y-salir";

/// How long the app has to save its state and quit.
const PLAZO_SALIDA: Duration = Duration::from_secs(3);
const SONDEO: Duration = Duration::from_millis(300);
/// Quiet time after a change before building, so a save that touches
/// several files builds once.
const CALMA: Duration = Duration::from_millis(200);

#[derive(Debug, Default, PartialEq)]
pub struct Opciones {
    pub bin: Option<String>,
    /// Extra folders to watch, besides the sources Cargo knows about.
    pub watch: Vec<PathBuf>,
    /// Arguments for the app.
    pub args: Vec<String>,
}

pub fn run(opciones: &Opciones) -> Result<(), String> {
    let loaded = manifest::find(
        &std::env::current_dir().map_err(|error| format!("cwd: {error}"))?,
    )
    .and_then(|path| match manifest::load(&path) {
        Ok(loaded) => Some(loaded),
        Err(error) => {
            eprintln!("kai dev: aviso: {error}");
            None
        }
    });

    let mut bin = opciones.bin.clone();
    let mut features: Vec<String> = Vec::new();
    if let Some(ref loaded) = loaded {
        eprintln!(
            "kai dev: manifiesto {} ({})",
            loaded.path.display(),
            loaded.manifest.app.name
        );
        if bin.is_none() {
            bin = loaded.manifest.dev.bin.clone();
        }
        features = loaded.manifest.dev.features.clone();
    }

    let proyecto = cargo::proyecto()?;
    let mut carpetas = proyecto.watch;
    carpetas.extend(opciones.watch.iter().cloned());
    if let Some(ref loaded) = loaded {
        carpetas.push(loaded.path.clone());
        carpetas.sort();
        carpetas.dedup();
    }
    let estado = proyecto
        .target_directory
        .join("kai-dev")
        .join("estado.json");
    if let Some(carpeta) = estado.parent() {
        std::fs::create_dir_all(carpeta)
            .map_err(|error| format!("no se pudo crear {}: {error}", carpeta.display()))?;
    }
    // Each `kai dev` session starts from a fresh state.
    let _ = std::fs::remove_file(&estado);
    eprintln!(
        "kai dev: observando {} rutas; Ctrl+C para terminar",
        carpetas.len()
    );

    let mut app: Option<Child> = None;
    let mut fotos: Snapshot;
    loop {
        eprintln!("kai dev: compilando…");
        let inicio = Instant::now();
        match cargo::build(bin.as_deref(), &features) {
            Ok(binario) => {
                let reinicio = app.is_some();
                if let Some(anterior) = app.take() {
                    cerrar(anterior);
                }
                match abrir(&binario, &estado, &opciones.args) {
                    Ok(nueva) => {
                        app = Some(nueva);
                        let segundos = segundos(inicio.elapsed());
                        if reinicio {
                            eprintln!("kai dev: reiniciado en {segundos} s");
                        } else {
                            eprintln!("kai dev: abierto en {segundos} s");
                        }
                    }
                    Err(error) => eprintln!("kai dev: {error}"),
                }
            }
            Err(Fallo::Compilacion) if app.is_some() => {
                eprintln!("kai dev: error de compilación; sigue la versión anterior");
            }
            Err(Fallo::Compilacion) => {
                eprintln!("kai dev: error de compilación; se abre al corregirlo");
            }
            Err(Fallo::Otro(error)) => eprintln!("kai dev: {error}"),
        }
        // Cambios durante la compilación no deben reiniciar la app al terminar.
        fotos = watch::snapshot(&carpetas);
        esperar_cambio(&carpetas, &mut fotos, &mut app);
    }
}

/// Start the app, which will read and later write `estado`.
fn abrir(binario: &Path, estado: &Path, args: &[String]) -> Result<Child, String> {
    Command::new(binario)
        .args(args)
        .env(ENV_ESTADO, estado)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("no se pudo abrir {}: {error}", binario.display()))
}

/// Ask the app to save its state and quit; kill it after [`PLAZO_SALIDA`].
fn cerrar(mut app: Child) {
    if let Some(mut stdin) = app.stdin.take() {
        let _ = writeln!(stdin, "{GUARDAR_Y_SALIR}");
        let _ = stdin.flush();
    }
    let limite = Instant::now() + PLAZO_SALIDA;
    while Instant::now() < limite {
        match app.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => thread::sleep(Duration::from_millis(25)),
        }
    }
    eprintln!("kai dev: la app no respondió en 3 s; se cierra a la fuerza");
    let _ = app.kill();
    let _ = app.wait();
}

/// Block until a watched file changes and stays quiet for [`CALMA`].
/// Meanwhile, notice when the person closes the app.
fn esperar_cambio(carpetas: &[PathBuf], fotos: &mut Snapshot, app: &mut Option<Child>) {
    loop {
        thread::sleep(SONDEO);
        if let Some(child) = app
            && !matches!(child.try_wait(), Ok(None))
        {
            *app = None;
            eprintln!("kai dev: la app se cerró; se vuelve a abrir con el próximo cambio");
        }
        let ahora = watch::snapshot(carpetas);
        if watch::changes(fotos, &ahora).is_empty() {
            continue;
        }
        *fotos = ahora;
        loop {
            thread::sleep(CALMA);
            let ahora = watch::snapshot(carpetas);
            if watch::changes(fotos, &ahora).is_empty() {
                return;
            }
            *fotos = ahora;
        }
    }
}

/// Seconds with one decimal and a decimal comma: «2,3».
fn segundos(duracion: Duration) -> String {
    format!("{:.1}", duracion.as_secs_f64()).replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_use_a_decimal_comma() {
        assert_eq!(segundos(Duration::from_millis(2_340)), "2,3");
        assert_eq!(segundos(Duration::from_millis(400)), "0,4");
    }
}
