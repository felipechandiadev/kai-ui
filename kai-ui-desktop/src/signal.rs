//! Small, explicit, local observable state primitive for the desktop UI.
//!
//! A signal does not schedule work, notify global subscribers, or depend on a
//! window/render backend. Callers decide what to invalidate when `set` returns
//! `true`.

#[derive(Clone, Debug, PartialEq)]
pub struct Signal<T> {
    value: T,
    revision: u64,
}

impl<T> Signal<T> {
    pub fn new(value: T) -> Self {
        Self { value, revision: 0 }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

impl<T: PartialEq> Signal<T> {
    /// Set a new value and return whether it differs from the current value.
    /// The revision advances only for an actual state transition.
    pub fn set(&mut self, value: T) -> bool {
        if self.value == value {
            return false;
        }

        self.value = value;
        self.revision = self.revision.wrapping_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Signal;

    #[test]
    fn starts_at_revision_zero_with_initial_value() {
        let signal = Signal::new("idle");

        assert_eq!(signal.get(), &"idle");
        assert_eq!(signal.revision(), 0);
    }

    #[test]
    fn changing_value_advances_revision_once() {
        let mut signal = Signal::new(false);

        assert!(signal.set(true));
        assert_eq!(signal.get(), &true);
        assert_eq!(signal.revision(), 1);
    }

    #[test]
    fn setting_same_value_does_not_report_change_or_advance_revision() {
        let mut signal = Signal::new(false);

        assert!(!signal.set(false));
        assert_eq!(signal.revision(), 0);
        assert!(signal.set(true));
        assert!(!signal.set(true));
        assert_eq!(signal.revision(), 1);
    }

    #[test]
    fn supports_non_copy_typed_state() {
        let mut signal = Signal::new(String::from("ready"));

        assert!(signal.set(String::from("changed")));
        assert_eq!(signal.get(), "changed");
        assert_eq!(signal.revision(), 1);
    }
}
