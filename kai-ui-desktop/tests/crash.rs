//! `kai::crash::install` end to end, in its own process: the panic hook is
//! global, so it cannot share a binary with the library's tests.

#[test]
fn an_installed_hook_reports_a_panic_of_the_ui_thread_once() {
    let base = std::env::temp_dir().join(format!("kai-crash-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    // SAFETY: the only test of this binary; no other thread reads the
    // environment yet.
    unsafe {
        std::env::set_var("HOME", &base);
        std::env::set_var("XDG_STATE_HOME", &base);
        std::env::set_var("LOCALAPPDATA", &base);
    }
    let id = "dev.kai.crash-e2e";
    // This test's thread plays the UI thread.
    kai::crash::install(id);
    assert_eq!(kai::crash::take_last(), None);

    let result = std::panic::catch_unwind(|| panic!("fallo de prueba"));
    assert!(result.is_err());
    let worker = std::thread::spawn(|| panic!("fallo en otro hilo")).join();
    assert!(worker.is_err());

    let crash = kai::crash::take_last().expect("the UI thread's panic");
    assert_eq!(crash.message, "fallo de prueba");
    assert!(
        crash
            .location
            .as_deref()
            .unwrap()
            .contains("tests/crash.rs")
    );
    assert!(
        crash.log.starts_with(kai::crash::dir(id)),
        "{:?}",
        crash.log
    );
    let log = std::fs::read_to_string(&crash.log).unwrap();
    assert!(log.contains("fallo de prueba"), "{log}");
    assert_eq!(kai::crash::take_last(), None, "shown once");
    let logs = std::fs::read_dir(kai::crash::dir(id)).unwrap().count();
    assert_eq!(logs, 2, "both panics are logged");
    let _ = std::fs::remove_dir_all(base);
}
