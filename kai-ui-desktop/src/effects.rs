//! Work off the UI thread for [`Effects::perform`]: a pool of
//! [`POOL_THREADS`] threads runs each task, and its result waits in an
//! [`Inbox`] until the session turns it into a message on the UI thread.
//! The window runtime is woken with a `UserEvent::Task`; `TestApp` waits
//! with [`Inbox::wait`] (13, Paso 15, without Tokio: it stays optional,
//! for MCP only).
//!
//! [`Effects::perform`]: crate::runtime::Effects::perform

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// Threads of the shared task pool.
pub const POOL_THREADS: usize = 4;

/// A task's result: `None` when it panicked.
pub(crate) type TaskOutput = Option<Box<dyn Any + Send>>;

type Job = Box<dyn FnOnce() + Send>;

/// Results of finished tasks, by task id, and who to wake when one lands.
#[derive(Default)]
pub(crate) struct Inbox {
    done: Mutex<Vec<(u64, TaskOutput)>>,
    landed: Condvar,
    waker: Mutex<Option<Box<dyn Fn() + Send>>>,
}

impl Inbox {
    fn put(&self, id: u64, output: TaskOutput) {
        self.done
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push((id, output));
        self.landed.notify_all();
        if let Some(wake) = self
            .waker
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .as_ref()
        {
            wake();
        }
    }

    /// Call `wake` from the pool each time a result lands.
    pub fn set_waker(&self, wake: Box<dyn Fn() + Send>) {
        *self
            .waker
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(wake);
    }

    /// The results that landed, oldest first.
    pub fn take(&self) -> Vec<(u64, TaskOutput)> {
        std::mem::take(
            &mut *self
                .done
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()),
        )
    }

    /// Block until some result is waiting or `timeout` passes; returns
    /// whether one is.
    pub fn wait(&self, timeout: Duration) -> bool {
        let done = self
            .done
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let (done, _) = self
            .landed
            .wait_timeout_while(done, timeout, |done| done.is_empty())
            .unwrap_or_else(|poison| poison.into_inner());
        !done.is_empty()
    }
}

struct Pool {
    sender: Mutex<mpsc::Sender<Job>>,
}

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        for index in 0..POOL_THREADS {
            let receiver = Arc::clone(&receiver);
            thread::Builder::new()
                .name(format!("kai-tarea-{index}"))
                .spawn(move || {
                    loop {
                        let job = receiver
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .recv();
                        match job {
                            Ok(job) => job(),
                            Err(_) => return,
                        }
                    }
                })
                .expect("hilo del pool de tareas");
        }
        Pool {
            sender: Mutex::new(sender),
        }
    })
}

/// Run `task` on the pool and leave its result in `inbox` under `id`. A
/// panic is reported and lands as `None`, so nobody waits for it forever.
pub(crate) fn spawn(
    inbox: &Arc<Inbox>,
    id: u64,
    task: Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>,
) {
    let inbox = Arc::clone(inbox);
    let job: Job = Box::new(move || {
        let output = panic::catch_unwind(AssertUnwindSafe(task)).ok();
        if output.is_none() {
            eprintln!("Una tarea de Effects::perform entró en pánico");
        }
        inbox.put(id, output);
    });
    let _ = pool()
        .sender
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .send(job);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_land_in_the_inbox_and_wake_whoever_listens() {
        let inbox = Arc::new(Inbox::default());
        let (woke, woken) = mpsc::channel();
        inbox.set_waker(Box::new(move || {
            let _ = woke.send(());
        }));
        spawn(&inbox, 7, Box::new(|| Box::new(6 * 7)));
        assert!(inbox.wait(Duration::from_secs(5)));
        woken.recv_timeout(Duration::from_secs(5)).unwrap();
        let done = inbox.take();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].0, 7);
        let value = done.into_iter().next().unwrap().1.unwrap();
        assert_eq!(*value.downcast::<i32>().unwrap(), 42);
        assert!(inbox.take().is_empty());
    }

    use crate::components::{ButtonVariant, button};
    use crate::element::{Element, Role, div, text};
    use crate::runtime::{App, Effects};
    use crate::testing::{By, TestApp};

    /// Closed until [`Gate::open`]: tasks that wait on it stay running.
    #[derive(Clone, Default)]
    struct Gate(Arc<(Mutex<bool>, Condvar)>);

    impl Gate {
        fn open(&self) {
            *self.0.0.lock().unwrap() = true;
            self.0.1.notify_all();
        }

        fn pass(&self) {
            let open = self.0.0.lock().unwrap();
            drop(self.0.1.wait_while(open, |open| !*open).unwrap());
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Search(u32),
        Cancel,
        Found(u32),
        Twice,
        Click,
    }

    #[derive(Default)]
    struct Searches {
        gate: Gate,
        found: Vec<u32>,
        clicks: u32,
    }

    impl App for Searches {
        type Msg = Msg;

        fn title(&self) -> String {
            "Tareas".into()
        }

        fn update(&mut self, msg: Msg) -> Effects<Msg> {
            match msg {
                Msg::Search(n) => {
                    let gate = self.gate.clone();
                    return Effects::perform(
                        move || {
                            gate.pass();
                            n * 10
                        },
                        Msg::Found,
                    )
                    .keyed("buscar");
                }
                Msg::Cancel => return Effects::cancel("buscar"),
                Msg::Found(n) => {
                    self.found.push(n);
                    if n == 10 {
                        return Effects::perform(|| 99, Msg::Found);
                    }
                }
                Msg::Twice => {
                    return Effects::batch([
                        Effects::perform(|| 1, Msg::Found),
                        Effects::perform(|| 2, Msg::Found),
                    ]);
                }
                Msg::Click => self.clicks += 1,
            }
            Effects::none()
        }

        fn view(&self) -> Element<Msg> {
            div().children([
                button("Buscar 1", ButtonVariant::Primary).on_click(Msg::Search(1)),
                button("Buscar 2", ButtonVariant::Primary).on_click(Msg::Search(2)),
                button("Cancelar", ButtonVariant::Secondary).on_click(Msg::Cancel),
                button("Dos", ButtonVariant::Secondary).on_click(Msg::Twice),
                button("Clic", ButtonVariant::Secondary).on_click(Msg::Click),
                text(format!("{} clics", self.clicks)).role(Role::Status),
            ])
        }
    }

    fn press(app: &mut TestApp<Searches>, name: &str) {
        app.click(By::role(Role::Button).name(name));
    }

    #[test]
    fn a_slow_task_does_not_hold_the_ui_and_its_result_comes_back_as_a_message() {
        let mut app = TestApp::new(Searches::default());
        press(&mut app, "Buscar 2");
        assert!(app.tasks_pending());
        for _ in 0..3 {
            press(&mut app, "Clic");
        }
        app.screen().get(By::role(Role::Status).name("3 clics"));
        assert!(app.app().found.is_empty());

        app.app().gate.open();
        app.run_until_idle();
        assert_eq!(app.app().found, [20]);
        assert!(!app.tasks_pending());
        assert!(app.messages().contains(&Msg::Found(20)));
    }

    #[test]
    fn a_keyed_task_replaces_the_pending_one_and_cancel_drops_it() {
        let mut app = TestApp::new(Searches::default());
        press(&mut app, "Buscar 2");
        press(&mut app, "Buscar 1");
        app.app().gate.open();
        app.run_until_idle();
        assert_eq!(
            app.app().found,
            [10, 99],
            "only the last search, then the one it started"
        );

        let mut app = TestApp::new(Searches::default());
        press(&mut app, "Buscar 2");
        press(&mut app, "Cancelar");
        assert!(!app.tasks_pending());
        app.app().gate.open();
        app.run_until_idle();
        assert!(app.app().found.is_empty());
    }

    #[test]
    fn unkeyed_tasks_all_send_their_message() {
        let mut app = TestApp::new(Searches::default());
        press(&mut app, "Dos");
        app.run_until_idle();
        let mut found = app.app().found.clone();
        found.sort_unstable();
        assert_eq!(found, [1, 2]);
        assert_eq!(
            format!("{:?}", Effects::perform(|| 1, Msg::Found).keyed("k")),
            r#"[perform(key: Some("k"))]"#
        );
    }

    #[test]
    fn a_panicking_task_lands_as_none() {
        let inbox = Arc::new(Inbox::default());
        spawn(&inbox, 1, Box::new(|| panic!("falla a propósito")));
        assert!(inbox.wait(Duration::from_secs(5)));
        let done = inbox.take();
        assert!(matches!(done.as_slice(), [(1, None)]));
    }
}
