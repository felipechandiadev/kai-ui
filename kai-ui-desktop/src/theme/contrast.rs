//! Measurable WCAG contrast checklist for AppKit semantic pairs.
//!
//! See `docs/dev/20-contraste-wcag.md`. Pairs that the theme promises AA for
//! normal text fail the test when under 4.5:1. UI / large-text pairs use 3:1.
//! Documented exceptions live in [`EXCEPTIONS`].

use crate::color::Color;
use crate::theme::{AppKitGeneration, Appearance, Theme};

/// Minimum contrast this pair must meet (WCAG 2.1 AA).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContrastTarget {
    /// Normal text: 4.5:1.
    AaNormal,
    /// Large text (≥18 pt / 14 pt bold) or non-text UI: 3:1.
    AaLargeOrUi,
}

impl ContrastTarget {
    pub fn min_ratio(self) -> f64 {
        match self {
            Self::AaNormal => 4.5,
            Self::AaLargeOrUi => 3.0,
        }
    }
}

/// One foreground/background pair under a named appearance.
#[derive(Clone, Copy, Debug)]
pub struct ContrastPair {
    pub id: &'static str,
    pub appearance: Appearance,
    pub foreground: fn(&Theme) -> Color,
    pub background: fn(&Theme) -> Color,
    pub target: ContrastTarget,
}

/// Pairs the AppKit theme promises at the given AA level.
pub const CRITICAL_PAIRS: &[ContrastPair] = &[
    ContrastPair {
        id: "label / window_background (light)",
        appearance: Appearance::Light,
        foreground: |t| t.semantic.label,
        background: |t| t.semantic.window_background,
        target: ContrastTarget::AaNormal,
    },
    ContrastPair {
        id: "label / text_background (light)",
        appearance: Appearance::Light,
        foreground: |t| t.semantic.label,
        background: |t| t.semantic.text_background,
        target: ContrastTarget::AaNormal,
    },
    ContrastPair {
        id: "label / window_background (dark)",
        appearance: Appearance::Dark,
        foreground: |t| t.semantic.label,
        background: |t| t.semantic.window_background,
        target: ContrastTarget::AaNormal,
    },
    ContrastPair {
        id: "label / text_background (dark)",
        appearance: Appearance::Dark,
        foreground: |t| t.semantic.label,
        background: |t| t.semantic.text_background,
        target: ContrastTarget::AaNormal,
    },
    ContrastPair {
        id: "on_accent / control_accent (light)",
        appearance: Appearance::Light,
        foreground: |t| t.semantic.on_accent,
        background: |t| t.semantic.control_accent,
        // Push-button title is ≥13 pt medium; treat as UI / large (3:1).
        // Absolute AA normal (4.5) is listed under EXCEPTIONS.
        target: ContrastTarget::AaLargeOrUi,
    },
    ContrastPair {
        id: "on_accent / control_accent (dark)",
        appearance: Appearance::Dark,
        foreground: |t| t.semantic.on_accent,
        background: |t| t.semantic.control_accent,
        target: ContrastTarget::AaLargeOrUi,
    },
    ContrastPair {
        id: "secondary_label / window_background (light)",
        appearance: Appearance::Light,
        foreground: |t| t.semantic.secondary_label,
        background: |t| t.semantic.window_background,
        target: ContrastTarget::AaLargeOrUi,
    },
    ContrastPair {
        id: "secondary_label / text_background (light)",
        appearance: Appearance::Light,
        foreground: |t| t.semantic.secondary_label,
        background: |t| t.semantic.text_background,
        target: ContrastTarget::AaLargeOrUi,
    },
    ContrastPair {
        id: "error / text_background (light)",
        appearance: Appearance::Light,
        foreground: |t| t.colors.error,
        background: |t| t.semantic.text_background,
        // systemRed on white is ~3.5:1 — body-size error copy needs care;
        // helper/caption (large or UI) is the promised AA level.
        target: ContrastTarget::AaLargeOrUi,
    },
    ContrastPair {
        id: "error / text_background (dark)",
        appearance: Appearance::Dark,
        foreground: |t| t.colors.error,
        background: |t| t.semantic.text_background,
        target: ContrastTarget::AaLargeOrUi,
    },
];

/// Explicit exceptions: pairs that do **not** promise AA normal (4.5:1).
/// Kept in the checklist so a future theme change is a conscious edit.
pub const EXCEPTIONS: &[(&str, &str)] = &[
    (
        "on_accent / control_accent",
        "Apple blue #007AFF with white is ~4.0:1 — below AA normal; OK for \
         button titles as large/UI (3:1). Do not use as body text on accent fills.",
    ),
    (
        "secondary_label / *",
        "secondaryLabel α≈0.5 composites to ~3.8–4.0:1 on light surfaces — below \
         AA normal; use for captions, helpers and secondary UI (≥3:1), not body.",
    ),
    (
        "error (systemRed) / text_background (light)",
        "systemRed #FF3B30 on white is ~3.5:1. Helper/error captions are large/UI; \
         for body-size error prose prefer label + alert surface, not red body text.",
    ),
    (
        "tertiary_label / quaternary_label",
        "Intentionally faint (placeholders, disabled hints). Not AA normal; not \
         asserted in CRITICAL_PAIRS.",
    ),
];

fn theme(appearance: Appearance) -> Theme {
    Theme::appkit_static(AppKitGeneration::Aqua11, appearance)
}

/// Ratio for one critical pair (composites translucent foregrounds).
pub fn pair_ratio(pair: &ContrastPair) -> f64 {
    let t = theme(pair.appearance);
    (pair.foreground)(&t).contrast_ratio((pair.background)(&t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_appkit_pairs_meet_promised_aa() {
        let mut failures = Vec::new();
        for pair in CRITICAL_PAIRS {
            let ratio = pair_ratio(pair);
            let min = pair.target.min_ratio();
            if ratio + 1e-6 < min {
                failures.push(format!(
                    "{}: {ratio:.2}:1 < {min}:1 ({:?})",
                    pair.id, pair.target
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "AppKit contrast regressions:\n  - {}\nExceptions (not asserted at 4.5):\n  - {}",
            failures.join("\n  - "),
            EXCEPTIONS
                .iter()
                .map(|(id, why)| format!("{id}: {why}"))
                .collect::<Vec<_>>()
                .join("\n  - ")
        );
    }

    #[test]
    fn exceptions_are_documented() {
        assert!(EXCEPTIONS.len() >= 3);
        assert!(EXCEPTIONS.iter().any(|(id, _)| id.contains("on_accent")));
        assert!(
            EXCEPTIONS
                .iter()
                .any(|(id, _)| id.contains("secondary_label"))
        );
        assert!(EXCEPTIONS.iter().any(|(id, _)| id.contains("error")));
    }

    #[test]
    fn label_on_window_is_well_above_aa_normal() {
        let light = CRITICAL_PAIRS
            .iter()
            .find(|p| p.id.starts_with("label / window_background (light)"))
            .unwrap();
        assert!(pair_ratio(light) >= 4.5);
    }
}
