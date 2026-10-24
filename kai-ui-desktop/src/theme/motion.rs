//! Productive motion durations already used in the product.
//!
//! Kai desktop does **not** yet ship a general transition/animation token
//! system (no layout spring, no shared `duration-short` CSS). Until that
//! exists, this module documents the durations that *do* drive UI time:
//! caret blink, toast dismiss, typeahead reset. Prefer these names in docs;
//! the owning modules keep the runtime constants to avoid layering cycles.
//!
//! When adding motion, respect [`crate::theme::SystemPrefs::reduce_motion`].
//!
//! See `docs/dev/23-spacing-density.md` § Motion.

use std::time::Duration;

/// Caret blink half-period (AppKit-style). Must match
/// [`crate::text_field::CARET_BLINK_PERIOD`].
pub const CARET_BLINK: Duration = Duration::from_millis(530);

/// Toast auto-dismiss. Must match [`crate::components::TOAST_DURATION`].
pub const TOAST: Duration = Duration::from_secs(4);

/// Typeahead buffer reset in menus / selects. Must match
/// `session::TYPEAHEAD_RESET` (1 s).
pub const TYPEAHEAD_RESET: Duration = Duration::from_secs(1);

/// Placeholder for a future short UI transition (~120–150 ms). Not wired.
pub const TRANSITION_SHORT_STUB_MS: u64 = 120;
/// Placeholder for a future medium UI transition (~200–250 ms). Not wired.
pub const TRANSITION_MEDIUM_STUB_MS: u64 = 200;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::TOAST_DURATION;
    use crate::text_field::CARET_BLINK_PERIOD;

    #[test]
    fn documented_durations_match_runtime_constants() {
        assert_eq!(CARET_BLINK, CARET_BLINK_PERIOD);
        assert_eq!(TOAST, TOAST_DURATION);
        assert_eq!(TYPEAHEAD_RESET.as_secs(), 1);
    }
}
