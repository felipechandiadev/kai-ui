//! Fatal errors (docs/dev/29-errores-fatales.md): what is left behind when
//! the app panics, and what the next launch can tell the person.
//!
//! Opt-in: call [`install`] once, at the top of `main`, before
//! [`crate::runtime::run`]. From then on a panic
//!
//! - writes a log (message, place, thread, backtrace) to [`dir`], the
//!   app's log folder, even without `HOME`;
//! - when it happens on the UI thread, also writes a marker, the
//!   [`Crash`] that [`take_last`] returns on the next launch so the app
//!   can show it ([`Crash::toast`]);
//! - when it unwinds out of the event loop, saves [`App::dev_state`] and
//!   the window's frame with [`Store`]; the next [`crate::runtime::run`]
//!   restores them once, as `kai dev` does after a rebuild.
//!
//! A panic in a task of the effects pool is already caught there (the
//! task sends nothing): it is logged, but it is not a crash. A run that
//! panicked right after restoring a saved state does not save it again,
//! so a state that makes the app panic is not restored forever.
//!
//! [`App::dev_state`]: crate::runtime::App::dev_state

use std::any::Any;
use std::ffi::OsString;
use std::panic::{AssertUnwindSafe, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::ThreadId;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::components::{AlertVariant, Toast};
use crate::dev_state::DevState;
use crate::store::{Store, Stored};

/// How many logs [`dir`] keeps; older ones are deleted when a new one is
/// written.
pub const KEPT_LOGS: usize = 10;

/// What the last crash left: [`take_last`] returns it once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crash {
    /// Seconds since the Unix epoch (UTC).
    pub time: u64,
    /// The panic's message.
    pub message: String,
    /// `file:line:column` where it panicked, when known.
    pub location: Option<String>,
    /// Name of the thread that panicked.
    pub thread: String,
    /// The log with the details and the backtrace.
    pub log: PathBuf,
    /// The window and the app's state were saved, and the next run
    /// restores them.
    pub state_saved: bool,
}

impl Stored for Crash {
    const KEY: &'static str = "last-crash";
    const VERSION: u32 = 1;
}

impl Crash {
    /// An error toast for the next launch (errors stay until closed). Its
    /// text says where the log is and whether the state came back.
    pub fn toast<Msg>(&self, id: u64) -> Toast<Msg> {
        let mut message = format!("Se guardó un informe en {}.", self.log.display());
        if self.state_saved {
            message.push_str(" Se recuperó la ventana y lo que tenías abierto.");
        }
        Toast::new(id, AlertVariant::Error, "La app se cerró por un error").message(message)
    }
}

/// The window and the app's state when the crash happened.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CrashState(DevState);

impl Stored for CrashState {
    const KEY: &'static str = "crash-state";
    const VERSION: u32 = 1;
}

static REPORTER: OnceLock<Reporter> = OnceLock::new();

/// Report panics of the app `id` (like `dev.kai.ui-desktop`) to [`dir`].
/// The thread that calls it is the UI thread: call it from `main`. The
/// hook that was installed before keeps running after Kai's (the message
/// still reaches stderr). Later calls do nothing.
pub fn install(id: &str) {
    let mut installed = false;
    let reporter = REPORTER.get_or_init(|| {
        installed = true;
        Reporter::new(dir(id), std::thread::current().id())
    });
    if !installed {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        reporter.report(&Panic::of(info));
        previous(info);
    }));
}

/// The crash of the previous run, once: the marker is deleted when read.
/// `None` without [`install`] or when the previous run ended well.
pub fn take_last() -> Option<Crash> {
    REPORTER.get()?.take_last()
}

/// The app's log folder, created on the first log:
/// `~/Library/Logs/<id>` on macOS, `%LOCALAPPDATA%\<id>\Logs` on Windows,
/// `$XDG_STATE_HOME/<id>` (or `~/.local/state/<id>`) elsewhere. Without
/// those variables, `<temp>/<id>`: never an error.
pub fn dir(id: &str) -> PathBuf {
    dir_from(id, Os::current(), |name| std::env::var_os(name))
}

/// The state that the previous run saved when it crashed, once (the
/// runtime calls it at startup).
pub(crate) fn take_saved_state() -> Option<DevState> {
    REPORTER.get()?.take_state()
}

/// The event loop unwound with a panic: save what `state` returns.
pub(crate) fn save_state(state: impl FnOnce() -> DevState) {
    if let Some(reporter) = REPORTER.get() {
        reporter.save_state(state);
    }
}

/// The event loop ended without a panic: a marker written in this run
/// came from a panic that the app caught, not from a crash.
pub(crate) fn clean_exit() {
    if let Some(reporter) = REPORTER.get() {
        reporter.clean_exit();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Os {
    Mac,
    Windows,
    Other,
}

impl Os {
    fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

fn dir_from(id: &str, os: Os, var: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    let var = |name: &str| {
        var(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let found = match os {
        Os::Mac => var("HOME").map(|home| home.join("Library/Logs").join(id)),
        Os::Windows => var("LOCALAPPDATA")
            .or_else(|| var("APPDATA"))
            .map(|base| base.join(id).join("Logs")),
        Os::Other => var("XDG_STATE_HOME")
            .or_else(|| var("HOME").map(|home| home.join(".local/state")))
            .map(|base| base.join(id)),
    };
    found.unwrap_or_else(|| std::env::temp_dir().join(id))
}

/// One panic, as the hook sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Panic {
    message: String,
    location: Option<String>,
    thread: ThreadId,
    thread_name: String,
    backtrace: String,
}

impl Panic {
    fn of(info: &PanicHookInfo<'_>) -> Self {
        let thread = std::thread::current();
        Self {
            message: message_of(info.payload()),
            location: info.location().map(ToString::to_string),
            thread: thread.id(),
            thread_name: thread.name().unwrap_or("sin nombre").to_owned(),
            backtrace: std::backtrace::Backtrace::force_capture().to_string(),
        }
    }
}

/// The text of a panic's payload (`panic!("…")` gives `&str` or `String`).
fn message_of(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "(panic sin mensaje de texto)".to_owned()
    }
}

/// Where the logs, the marker and the saved state of one app go.
struct Reporter {
    store: Store,
    ui_thread: ThreadId,
    /// This run wrote the marker (see [`Reporter::clean_exit`]).
    wrote_marker: AtomicBool,
    /// This run started from a saved state: it does not save another.
    restored: AtomicBool,
    /// Saving the state; a panic while doing it is only logged.
    saving: AtomicBool,
}

impl Reporter {
    fn new(dir: PathBuf, ui_thread: ThreadId) -> Self {
        Self {
            store: Store::at(dir),
            ui_thread,
            wrote_marker: AtomicBool::new(false),
            restored: AtomicBool::new(false),
            saving: AtomicBool::new(false),
        }
    }

    fn dir(&self) -> &Path {
        self.store.dir()
    }

    /// Log `panic` and, on the UI thread, leave the marker. Errors are
    /// ignored: a hook must not panic.
    fn report(&self, panic: &Panic) {
        let time = now();
        let log = self.write_log(panic, time);
        let fatal = panic.thread == self.ui_thread && !self.saving.load(Ordering::SeqCst);
        if !fatal {
            return;
        }
        let crash = Crash {
            time,
            message: panic.message.clone(),
            location: panic.location.clone(),
            thread: panic.thread_name.clone(),
            log: log.unwrap_or_else(|| self.dir().to_owned()),
            state_saved: false,
        };
        if self.store.save(&crash).is_ok() {
            self.wrote_marker.store(true, Ordering::SeqCst);
        }
    }

    fn write_log(&self, panic: &Panic, time: u64) -> Option<PathBuf> {
        std::fs::create_dir_all(self.dir()).ok()?;
        let path = self.dir().join(format!(
            "panic-{time}-{}-{}.log",
            std::process::id(),
            next_log()
        ));
        let text = format!(
            "Kai {version}: panic en el hilo «{thread}»{ui}\n\
             Hora (Unix, UTC): {time}\n\
             Lugar: {location}\n\
             Mensaje: {message}\n\n\
             Backtrace:\n{backtrace}\n",
            version = env!("CARGO_PKG_VERSION"),
            thread = panic.thread_name,
            ui = if panic.thread == self.ui_thread {
                " (hilo de la interfaz)"
            } else {
                ""
            },
            location = panic.location.as_deref().unwrap_or("desconocido"),
            message = panic.message,
            backtrace = panic.backtrace,
        );
        std::fs::write(&path, text).ok()?;
        self.prune_logs();
        Some(path)
    }

    /// Keep the newest [`KEPT_LOGS`] logs.
    fn prune_logs(&self) {
        let Ok(entries) = std::fs::read_dir(self.dir()) else {
            return;
        };
        let mut logs: Vec<(SystemTime, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.starts_with("panic-") && name.ends_with(".log")
            })
            .map(|entry| {
                let modified = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .unwrap_or(UNIX_EPOCH);
                (modified, entry.path())
            })
            .collect();
        logs.sort();
        let extra = logs.len().saturating_sub(KEPT_LOGS);
        for (_, path) in logs.into_iter().take(extra) {
            let _ = std::fs::remove_file(path);
        }
    }

    fn take_last(&self) -> Option<Crash> {
        let crash = self.store.load::<Crash>().ok().flatten();
        let _ = self.store.remove::<Crash>();
        crash
    }

    fn take_state(&self) -> Option<DevState> {
        let state = self.store.load::<CrashState>().ok().flatten()?;
        let _ = self.store.remove::<CrashState>();
        self.restored.store(true, Ordering::SeqCst);
        Some(state.0)
    }

    fn save_state(&self, state: impl FnOnce() -> DevState) {
        if self.restored.load(Ordering::SeqCst) {
            return;
        }
        self.saving.store(true, Ordering::SeqCst);
        // The model may be half updated: if reading it panics too, the
        // crash is reported without a state.
        let state = std::panic::catch_unwind(AssertUnwindSafe(state));
        self.saving.store(false, Ordering::SeqCst);
        let Ok(state) = state else {
            return;
        };
        if self.store.save(&CrashState(state)).is_err() {
            return;
        }
        if let Ok(Some(mut crash)) = self.store.load::<Crash>() {
            crash.state_saved = true;
            let _ = self.store.save(&crash);
        }
    }

    fn clean_exit(&self) {
        if self.wrote_marker.swap(false, Ordering::SeqCst) {
            let _ = self.store.remove::<Crash>();
        }
    }
}

/// Tells apart the logs of one second (a panic often brings another).
fn next_log() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::SeqCst)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;
    use crate::dev_state::WindowFrame;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kai-crash-{name}-{}-{:?}",
            std::process::id(),
            SystemTime::now()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let vars: Vec<(String, OsString)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        move |name| {
            vars.iter()
                .find(|(var, _)| var == name)
                .map(|(_, value)| value.clone())
        }
    }

    fn panic_on(thread: ThreadId, message: &str) -> Panic {
        Panic {
            message: message.to_owned(),
            location: Some("src/app.rs:10:5".to_owned()),
            thread,
            thread_name: "main".to_owned(),
            backtrace: "0: kai::app::update".to_owned(),
        }
    }

    fn state() -> DevState {
        DevState {
            window: Some(WindowFrame {
                x: 10.0,
                y: 20.0,
                width: 800.0,
                height: 600.0,
            }),
            focused: true,
            app: Some(json!({ "section": "mapa" })),
        }
    }

    fn logs(dir: &Path) -> Vec<PathBuf> {
        let mut logs: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "log"))
            .collect();
        logs.sort();
        logs
    }

    #[test]
    fn the_log_folder_follows_each_system_and_never_needs_home() {
        let id = "dev.kai.prueba";
        assert_eq!(
            dir_from(id, Os::Mac, env(&[("HOME", "/Users/ana")])),
            PathBuf::from("/Users/ana/Library/Logs/dev.kai.prueba")
        );
        assert_eq!(
            dir_from(
                id,
                Os::Windows,
                env(&[("LOCALAPPDATA", "C:/Users/ana/AppData/Local")])
            ),
            PathBuf::from("C:/Users/ana/AppData/Local/dev.kai.prueba/Logs")
        );
        assert_eq!(
            dir_from(id, Os::Windows, env(&[("APPDATA", "C:/Roaming")])),
            PathBuf::from("C:/Roaming/dev.kai.prueba/Logs")
        );
        assert_eq!(
            dir_from(
                id,
                Os::Other,
                env(&[("XDG_STATE_HOME", "/state"), ("HOME", "/home/ana")])
            ),
            PathBuf::from("/state/dev.kai.prueba")
        );
        assert_eq!(
            dir_from(id, Os::Other, env(&[("HOME", "/home/ana")])),
            PathBuf::from("/home/ana/.local/state/dev.kai.prueba")
        );
        for os in [Os::Mac, Os::Windows, Os::Other] {
            assert_eq!(
                dir_from(id, os, env(&[("HOME", "")])),
                std::env::temp_dir().join(id),
                "{os:?} without variables"
            );
        }
    }

    #[test]
    fn the_message_of_a_panic_is_its_text() {
        let payload: Box<dyn Any + Send> = Box::new("se rompió");
        assert_eq!(message_of(payload.as_ref()), "se rompió");
        let payload: Box<dyn Any + Send> = Box::new(String::from("índice 3"));
        assert_eq!(message_of(payload.as_ref()), "índice 3");
        let payload: Box<dyn Any + Send> = Box::new(7_u8);
        assert_eq!(message_of(payload.as_ref()), "(panic sin mensaje de texto)");
    }

    #[test]
    fn a_panic_on_the_ui_thread_leaves_a_log_and_a_marker_read_once() {
        let dir = temp("ui");
        let ui = std::thread::current().id();
        let reporter = Reporter::new(dir.clone(), ui);
        reporter.report(&panic_on(ui, "índice fuera de rango"));

        let logs = logs(&dir);
        assert_eq!(logs.len(), 1);
        let log = std::fs::read_to_string(&logs[0]).unwrap();
        assert!(log.contains("índice fuera de rango"), "{log}");
        assert!(log.contains("src/app.rs:10:5"), "{log}");
        assert!(log.contains("kai::app::update"), "{log}");

        let crash = reporter.take_last().expect("a marker");
        assert_eq!(crash.message, "índice fuera de rango");
        assert_eq!(crash.location.as_deref(), Some("src/app.rs:10:5"));
        assert_eq!(crash.thread, "main");
        assert_eq!(crash.log, logs[0]);
        assert!(!crash.state_saved);
        assert_eq!(reporter.take_last(), None, "read once");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_panic_on_another_thread_is_only_logged() {
        let dir = temp("other");
        let reporter = Reporter::new(dir.clone(), std::thread::current().id());
        let other = std::thread::spawn(|| std::thread::current().id())
            .join()
            .unwrap();
        reporter.report(&panic_on(other, "una tarea falló"));
        assert_eq!(logs(&dir).len(), 1);
        assert_eq!(reporter.take_last(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_the_newest_logs_are_kept() {
        let dir = temp("prune");
        std::fs::create_dir_all(&dir).unwrap();
        for n in 0..KEPT_LOGS + 3 {
            std::fs::write(dir.join(format!("panic-{n:03}-0.log")), "viejo").unwrap();
        }
        std::fs::write(dir.join("otro.txt"), "no es un log").unwrap();
        let ui = std::thread::current().id();
        let reporter = Reporter::new(dir.clone(), ui);
        reporter.report(&panic_on(ui, "nuevo"));
        let kept = logs(&dir);
        assert_eq!(kept.len(), KEPT_LOGS);
        assert!(
            kept.iter()
                .any(|log| std::fs::read_to_string(log).unwrap().contains("nuevo"))
        );
        assert!(dir.join("otro.txt").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_saved_state_comes_back_once_and_marks_the_crash() {
        let dir = temp("state");
        let ui = std::thread::current().id();
        let reporter = Reporter::new(dir.clone(), ui);
        reporter.report(&panic_on(ui, "boom"));
        reporter.save_state(state);
        assert!(reporter.take_last().unwrap().state_saved);

        let next = Reporter::new(dir.clone(), ui);
        assert_eq!(next.take_state(), Some(state()));
        assert_eq!(next.take_state(), None, "restored once");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_run_restored_from_a_crash_does_not_save_its_state_again() {
        let dir = temp("loop");
        let ui = std::thread::current().id();
        let first = Reporter::new(dir.clone(), ui);
        first.save_state(state);
        let second = Reporter::new(dir.clone(), ui);
        assert!(second.take_state().is_some());
        second.report(&panic_on(ui, "otra vez"));
        second.save_state(state);
        assert!(!second.take_last().unwrap().state_saved);
        assert_eq!(Reporter::new(dir.clone(), ui).take_state(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_state_that_panics_while_saving_is_not_saved() {
        let dir = temp("panicky");
        let ui = std::thread::current().id();
        let reporter = Reporter::new(dir.clone(), ui);
        reporter.report(&panic_on(ui, "primero"));
        reporter.save_state(|| panic!("el modelo quedó a medias"));
        let crash = reporter.take_last().unwrap();
        assert_eq!(crash.message, "primero");
        assert!(!crash.state_saved);
        assert_eq!(reporter.take_state(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_clean_exit_drops_the_marker_of_a_caught_panic_only() {
        let dir = temp("clean");
        let ui = std::thread::current().id();
        let previous = Reporter::new(dir.clone(), ui);
        previous.report(&panic_on(ui, "de la ejecución anterior"));

        let reporter = Reporter::new(dir.clone(), ui);
        reporter.clean_exit();
        assert!(
            reporter.store.load::<Crash>().unwrap().is_some(),
            "not this run's"
        );
        reporter.report(&panic_on(ui, "atrapado por la app"));
        reporter.clean_exit();
        assert_eq!(reporter.take_last(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_toast_is_an_error_that_says_where_the_log_is() {
        let crash = Crash {
            time: 0,
            message: "boom".into(),
            location: None,
            thread: "main".into(),
            log: PathBuf::from("/tmp/panic-1.log"),
            state_saved: true,
        };
        let toast: Toast<()> = crash.toast(7);
        assert_eq!(toast.id, 7);
        assert_eq!(toast.variant, AlertVariant::Error);
        assert_eq!(toast.title, "La app se cerró por un error");
        let message = toast.message.unwrap();
        assert!(message.contains("/tmp/panic-1.log"), "{message}");
        assert!(message.contains("Se recuperó"), "{message}");
    }
}
