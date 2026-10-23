//! Extended spacing scale for layout (`gap-*`, `p-*`, `m-*`, …).
//!
//! The Tailwind-like engine already resolves **any non-negative step** as
//! `steps × `[`Theme::spacing_unit`](super::Theme::spacing_unit). This module
//! names the **productive** steps (through ~64 pt at comfortable density) and
//! the semantic aliases used by page templates — it does **not** invent a
//! second grid (PageGrid stays 12; Carbon’s 16-col / `spacing-01…13` IDs are
//! not copied).
//!
//! | Step | Comfortable (unit 4 pt) | Compact (unit 3 pt) | Typical use |
//! |------|-------------------------|---------------------|-------------|
//! | 0    | 0                       | 0                   | flush       |
//! | 1    | 4                       | 3                   | hairline / icon pad |
//! | 2    | 8                       | 6                   | control cluster (`gap-2`) |
//! | 3    | 12                      | 9                   | data stack (`PageDensity::Compact`) |
//! | 4    | 16                      | 12                  | page gutter / `--space-4` |
//! | 5    | 20                      | 15                  | form stack (`PageDensity::Comfortable`) |
//! | 6    | 24                      | 18                  | section inside a card |
//! | 8    | 32                      | 24                  | page body margin (`md:px-8`) |
//! | 10   | 40                      | 30                  | large section break |
//! | 12   | 48                      | 36                  | hero / sparse chrome |
//! | 16   | 64                      | 48                  | top-of-scale block |
//!
//! Prefer these named steps in docs and helpers; arbitrary values
//! (`gap-[18px]`) remain available for one-offs.
//!
//! See [`docs/dev/23-spacing-density.md`].

use super::Theme;

/// Productive step numbers on the Tailwind scale (`gap-N`, `p-N`, …).
pub mod steps {
    pub const S0: u16 = 0;
    pub const S1: u16 = 1;
    pub const S2: u16 = 2;
    pub const S3: u16 = 3;
    pub const S4: u16 = 4;
    pub const S5: u16 = 5;
    pub const S6: u16 = 6;
    pub const S8: u16 = 8;
    pub const S10: u16 = 10;
    pub const S12: u16 = 12;
    /// Top of the documented scale (~64 pt when `spacing_unit` is 4).
    pub const S16: u16 = 16;

    /// Every named productive step, ascending (for docs / tests).
    pub const ALL: [u16; 11] = [S0, S1, S2, S3, S4, S5, S6, S8, S10, S12, S16];
}

/// Canonical pt at **comfortable** density (`spacing_unit` = 4). Compact
/// scales these by `Theme::spacing_unit()`; do not hard-code pt in widgets.
pub mod comfortable_pt {
    pub const PT_0: f32 = 0.0;
    pub const PT_4: f32 = 4.0;
    pub const PT_8: f32 = 8.0;
    pub const PT_12: f32 = 12.0;
    pub const PT_16: f32 = 16.0;
    pub const PT_20: f32 = 20.0;
    pub const PT_24: f32 = 24.0;
    pub const PT_32: f32 = 32.0;
    pub const PT_40: f32 = 40.0;
    pub const PT_48: f32 = 48.0;
    pub const PT_64: f32 = 64.0;
}

/// Vertical rhythm between blocks in a form template (`gap-5`).
pub const FORM_STACK_STEPS: u16 = steps::S5;
/// Vertical rhythm between blocks in a data template (`gap-3`).
pub const DATA_STACK_STEPS: u16 = steps::S3;
/// PageGrid gutter (`gap-4`) — same as [`crate::components::page_tokens::GUTTER_PT`] at comfortable.
pub const PAGE_GUTTER_STEPS: u16 = steps::S4;
/// Gap between neighboring actions (`gap-2`).
pub const CONTROL_CLUSTER_STEPS: u16 = steps::S2;
/// Card / section padding often paired with `--space-4` (`p-4`).
pub const CARD_PADDING_STEPS: u16 = steps::S4;

/// `steps × unit` in logical points.
pub fn pt(steps: u16, unit: f32) -> f32 {
    f32::from(steps) * unit
}

/// Resolve a step against a theme’s [`Theme::spacing_unit`].
pub fn pt_for(theme: &Theme, steps: u16) -> f32 {
    pt(steps, theme.spacing_unit())
}

/// Utility class `gap-{steps}` (vertical/horizontal flex/grid gap).
pub fn gap_class(steps: u16) -> String {
    format!("gap-{steps}")
}

/// Utility class `p-{steps}`.
pub fn p_class(steps: u16) -> String {
    format!("p-{steps}")
}

/// Utility class `px-{steps}`.
pub fn px_class(steps: u16) -> String {
    format!("px-{steps}")
}

/// Utility class `py-{steps}`.
pub fn py_class(steps: u16) -> String {
    format!("py-{steps}")
}

/// Form template stack class (`gap-5`).
pub fn form_stack_gap_class() -> String {
    gap_class(FORM_STACK_STEPS)
}

/// Data template stack class (`gap-3`).
pub fn data_stack_gap_class() -> String {
    gap_class(DATA_STACK_STEPS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Density, Theme};
    use crate::tw::{ClassList, State};

    #[test]
    fn comfortable_scale_reaches_sixty_four() {
        let theme = Theme::kai(Density::Comfortable);
        assert_eq!(theme.spacing_unit(), 4.0);
        assert_eq!(pt_for(&theme, steps::S0), comfortable_pt::PT_0);
        assert_eq!(pt_for(&theme, steps::S1), comfortable_pt::PT_4);
        assert_eq!(pt_for(&theme, steps::S4), comfortable_pt::PT_16);
        assert_eq!(pt_for(&theme, steps::S5), comfortable_pt::PT_20);
        assert_eq!(pt_for(&theme, steps::S16), comfortable_pt::PT_64);
        assert_eq!(
            pt_for(&theme, PAGE_GUTTER_STEPS),
            crate::components::page_tokens::GUTTER_PT
        );
    }

    #[test]
    fn compact_shrinks_every_step() {
        let comfortable = Theme::kai(Density::Comfortable);
        let compact = Theme::kai(Density::Compact);
        for step in steps::ALL {
            assert!(
                pt_for(&compact, step) <= pt_for(&comfortable, step),
                "step {step}"
            );
        }
        assert!(compact.spacing_unit() < comfortable.spacing_unit());
    }

    #[test]
    fn gap_classes_resolve_through_tw() {
        let theme = Theme::kai(Density::Comfortable);
        for step in [steps::S3, steps::S5, steps::S16] {
            let class = gap_class(step);
            let style = ClassList::parse(&class, &theme).resolve(State::default());
            assert_eq!(style.gap_x, pt_for(&theme, step), "{class}");
            assert_eq!(style.gap_y, pt_for(&theme, step), "{class}");
        }
    }

    #[test]
    fn semantic_aliases_match_page_density() {
        assert_eq!(form_stack_gap_class(), "gap-5");
        assert_eq!(data_stack_gap_class(), "gap-3");
        assert_eq!(FORM_STACK_STEPS, 5);
        assert_eq!(DATA_STACK_STEPS, 3);
    }
}
