//! `kai dev`: build, open, and on every change rebuild and restart the app
//! with its state.
//!
//! The app is started with `KAI_DEV_STATE=<file>` and its stdin piped. To
//! restart it, `kai dev` writes `save-and-quit` on that stdin: a Kai app
//! in a debug build saves its window frame and `App::dev_state` there and
//! quits, and the next run reads them back. An app that does not answer in
//! [`EXIT_DEADLINE`] is killed. When the build fails, the running version
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

use crate::cargo::{self, Failure};
use crate::manifest;
use crate::watch::{self, Snapshot};

/// Variable and line the Kai runtime listens for (`kai::dev_state`).
const ENV_STATE: &str = "KAI_DEV_STATE";
const SAVE_AND_QUIT_LINE: &str = "save-and-quit";

/// How long the app has to save its state and quit.
const EXIT_DEADLINE: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(300);
/// Quiet time after a change before building, so a save that touches
/// several files builds once.
const QUIET: Duration = Duration::from_millis(200);

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    pub bin: Option<String>,
    /// Extra folders to watch, besides the sources Cargo knows about.
    pub watch: Vec<PathBuf>,
    /// Arguments for the app.
    pub args: Vec<String>,
}

pub fn run(options: &Options) -> Result<(), String> {
    let loaded = manifest::find(&std::env::current_dir().map_err(|error| format!("cwd: {error}"))?)
        .and_then(|path| match manifest::load(&path) {
            Ok(loaded) => Some(loaded),
            Err(error) => {
                eprintln!("kai dev: aviso: {error}");
                None
            }
        });

    let mut bin = options.bin.clone();
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

    let project = cargo::project()?;
    let mut folders = project.watch;
    folders.extend(options.watch.iter().cloned());
    if let Some(ref loaded) = loaded {
        folders.push(loaded.path.clone());
        folders.sort();
        folders.dedup();
    }
    let state_value = project.target_directory.join("kai-dev").join("estado.json");
    if let Some(folder) = state_value.parent() {
        std::fs::create_dir_all(folder)
            .map_err(|error| format!("no se pudo crear {}: {error}", folder.display()))?;
    }
    // Each `kai dev` session starts from a fresh state.
    let _ = std::fs::remove_file(&state_value);
    eprintln!(
        "kai dev: observando {} rutas; Ctrl+C para terminar",
        folders.len()
    );

    let mut app: Option<Child> = None;
    let mut snapshots: Snapshot;
    loop {
        eprintln!("kai dev: compilando…");
        let home_view = Instant::now();
        match cargo::build(bin.as_deref(), &features, false) {
            Ok(binary) => {
                let restart = app.is_some();
                if let Some(anterior) = app.take() {
                    close_child(anterior);
                }
                match open(&binary, &state_value, &options.args) {
                    Ok(new_one) => {
                        app = Some(new_one);
                        let seconds = seconds(home_view.elapsed());
                        if restart {
                            eprintln!("kai dev: reiniciado en {seconds} s");
                        } else {
                            eprintln!("kai dev: abierto en {seconds} s");
                        }
                    }
                    Err(error) => eprintln!("kai dev: {error}"),
                }
            }
            Err(Failure::Build) if app.is_some() => {
                eprintln!("kai dev: error de compilación; sigue la versión anterior");
            }
            Err(Failure::Build) => {
                eprintln!("kai dev: error de compilación; se abre al corregirlo");
            }
            Err(Failure::Other(error)) => eprintln!("kai dev: {error}"),
        }
        // Cambios durante la compilación no deben reiniciar la app al terminar.
        snapshots = watch::snapshot(&folders);
        wait_for_change(&folders, &mut snapshots, &mut app);
    }
}

/// Start the app, which will read and later write `state_value`.
fn open(binary: &Path, state_value: &Path, args: &[String]) -> Result<Child, String> {
    Command::new(binary)
        .args(args)
        .env(ENV_STATE, state_value)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("no se pudo abrir {}: {error}", binary.display()))
}

/// Ask the app to save its state and quit; kill it after [`EXIT_DEADLINE`].
fn close_child(mut app: Child) {
    if let Some(mut stdin) = app.stdin.take() {
        let _ = writeln!(stdin, "{SAVE_AND_QUIT_LINE}");
        let _ = stdin.flush();
    }
    let limit = Instant::now() + EXIT_DEADLINE;
    while Instant::now() < limit {
        match app.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => thread::sleep(Duration::from_millis(25)),
        }
    }
    eprintln!("kai dev: la app no respondió en 3 s; se cierra a la fuerza");
    let _ = app.kill();
    let _ = app.wait();
}

/// Block until a watched file changes and stays quiet for [`QUIET`].
/// Meanwhile, notice when the person closes the app.
fn wait_for_change(folders: &[PathBuf], snapshots: &mut Snapshot, app: &mut Option<Child>) {
    loop {
        thread::sleep(POLL);
        if let Some(child) = app
            && !matches!(child.try_wait(), Ok(None))
        {
            *app = None;
            eprintln!("kai dev: la app se cerró; se vuelve a abrir con el próximo cambio");
        }
        let now_at = watch::snapshot(folders);
        if watch::changes(snapshots, &now_at).is_empty() {
            continue;
        }
        *snapshots = now_at;
        loop {
            thread::sleep(QUIET);
            let now_at = watch::snapshot(folders);
            if watch::changes(snapshots, &now_at).is_empty() {
                return;
            }
            *snapshots = now_at;
        }
    }
}

/// Seconds with one decimal and a decimal comma: «2,3».
fn seconds(duration: Duration) -> String {
    format!("{:.1}", duration.as_secs_f64()).replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_use_a_decimal_comma() {
        assert_eq!(seconds(Duration::from_millis(2_340)), "2,3");
        assert_eq!(seconds(Duration::from_millis(400)), "0,4");
    }
}
