//! Global settings of the demo, chosen in the top bar (`src/layout.rs`):
//! theme, density, control size and window background. They apply to every
//! section and are saved in the app's data folder (`kai::store`), so the
//! next run starts with them. Precedence: docs/dev/31-ajustes-globales.md.

use kai::background::{BackgroundEffect, BackgroundKind};
use kai::element::ControlSize;
use kai::store::Stored;
use kai::theme::{AppKitGeneration, Appearance, Density, Theme};
use serde::{Deserialize, Serialize};

/// What the bar offers; the default leaves the window as `main` set it up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub theme: ThemeChoice,
    pub density: DensityChoice,
    pub size: SizeChoice,
    pub background: BackgroundChoice,
}

impl Stored for Settings {
    const KEY: &'static str = "demo.settings";
    const VERSION: u32 = 1;
}

/// A theme preset, or the one the demo started with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    /// The theme `main` gives the runtime (`KAI_THEME`, Glass by default).
    #[default]
    Initial,
    Glass,
    GlassDark,
    AppKit,
    AppKitDark,
    Kai,
}

impl ThemeChoice {
    pub const ALL: [Self; 6] = [
        Self::Initial,
        Self::Glass,
        Self::GlassDark,
        Self::AppKit,
        Self::AppKitDark,
        Self::Kai,
    ];
    pub const LABELS: [&'static str; 6] = [
        "Inicial",
        "Glass",
        "Glass Dark",
        "AppKit",
        "AppKit Dark",
        "Kai",
    ];
}

/// The theme's own density, or a forced one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityChoice {
    #[default]
    Theme,
    Comfortable,
    Compact,
}

impl DensityChoice {
    pub const ALL: [Self; 3] = [Self::Theme, Self::Comfortable, Self::Compact];
    pub const LABELS: [&'static str; 3] = ["Del tema", "Comfortable", "Compact"];

    fn density(self) -> Option<Density> {
        match self {
            Self::Theme => None,
            Self::Comfortable => Some(Density::Comfortable),
            Self::Compact => Some(Density::Compact),
        }
    }
}

/// The theme's own control size, or a forced one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeChoice {
    #[default]
    Theme,
    Mini,
    Small,
    Regular,
    Large,
    ExtraLarge,
}

impl SizeChoice {
    pub const ALL: [Self; 6] = [
        Self::Theme,
        Self::Mini,
        Self::Small,
        Self::Regular,
        Self::Large,
        Self::ExtraLarge,
    ];
    pub const LABELS: [&'static str; 6] = ["Del tema", "Mini", "Small", "Regular", "Large", "XL"];

    fn size(self) -> Option<ControlSize> {
        match self {
            Self::Theme => None,
            Self::Mini => Some(ControlSize::Mini),
            Self::Small => Some(ControlSize::Small),
            Self::Regular => Some(ControlSize::Regular),
            Self::Large => Some(ControlSize::Large),
            Self::ExtraLarge => Some(ControlSize::ExtraLarge),
        }
    }
}

/// The window background under every section that has none of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundChoice {
    #[default]
    None,
    Mesh,
    Aurora,
    Glow,
}

impl BackgroundChoice {
    pub const ALL: [Self; 4] = [Self::None, Self::Mesh, Self::Aurora, Self::Glow];
    pub const LABELS: [&'static str; 4] = ["Ninguno", "Mesh", "Aurora", "Glow"];
}

/// Where `value` is in `all` (the segment to show selected).
pub fn index_of<T: PartialEq>(all: &[T], value: &T) -> usize {
    all.iter().position(|item| item == value).unwrap_or(0)
}

impl Settings {
    /// The window's theme with these settings over `base` (the theme the
    /// demo started with), or `None` when they change nothing, so the
    /// runtime keeps `base` as it is. A preset keeps the experiments of
    /// `base` (hover, mix, field labels: `KAI_HOVER`, `KAI_MIX`,
    /// `KAI_LABEL`).
    pub fn theme(&self, base: &Theme, choice: ThemeChoice) -> Option<Theme> {
        let density = self.density.density();
        let size = self.size.size();
        if choice == ThemeChoice::Initial && density.is_none() && size.is_none() {
            return None;
        }
        let forced = |generation, appearance| {
            Theme::appkit_static(generation, appearance)
                .with_mix(base.mix)
                .with_hover(base.hover_effects)
                .with_field_labels(base.field_labels)
        };
        let theme = match choice {
            ThemeChoice::Initial => *base,
            ThemeChoice::Glass => Theme {
                control_size: ControlSize::Large,
                ..forced(AppKitGeneration::Glass27, Appearance::Light)
            },
            ThemeChoice::GlassDark => Theme {
                control_size: ControlSize::Large,
                ..forced(AppKitGeneration::Glass27, Appearance::Dark)
            },
            ThemeChoice::AppKit => forced(AppKitGeneration::Aqua11, Appearance::Light),
            ThemeChoice::AppKitDark => forced(AppKitGeneration::Aqua11, Appearance::Dark),
            // Kai's spacing tables depend on the density.
            ThemeChoice::Kai => {
                Theme::kai(density.unwrap_or(Density::Comfortable)).with_hover(base.hover_effects)
            }
        };
        Some(Theme {
            density: density.unwrap_or(theme.density),
            control_size: size.unwrap_or(theme.control_size),
            ..theme
        })
    }

    /// The chosen background, or `None` for "Ninguno" so sections keep
    /// their own.
    pub fn background(&self) -> Option<BackgroundEffect> {
        let kind = match self.background {
            BackgroundChoice::None => return None,
            BackgroundChoice::Mesh => BackgroundKind::Mesh,
            BackgroundChoice::Aurora => BackgroundKind::Aurora,
            BackgroundChoice::Glow => BackgroundKind::Glow,
        };
        Some(BackgroundEffect::new(kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_leave_the_initial_theme_alone() {
        let settings = Settings::default();
        assert_eq!(settings.theme(&Theme::glass(), ThemeChoice::Initial), None);
        assert_eq!(settings.background(), None);
    }

    #[test]
    fn density_and_size_apply_over_the_initial_theme() {
        let settings = Settings {
            density: DensityChoice::Compact,
            size: SizeChoice::Small,
            ..Settings::default()
        };
        let theme = settings
            .theme(&Theme::glass(), ThemeChoice::Initial)
            .unwrap();
        assert!(theme.is_appkit());
        assert_eq!(theme.density, Density::Compact);
        assert_eq!(theme.control_size, ControlSize::Small);
    }

    #[test]
    fn a_preset_keeps_the_experiments_of_the_initial_theme() {
        let base = Theme::glass().with_hover(false);
        let theme = Settings::default()
            .theme(&base, ThemeChoice::AppKitDark)
            .unwrap();
        assert_eq!(theme.appearance, Appearance::Dark);
        assert!(!theme.hover_effects);
        let kai = Settings {
            density: DensityChoice::Compact,
            ..Settings::default()
        }
        .theme(&base, ThemeChoice::Kai)
        .unwrap();
        assert_eq!(kai, Theme::kai(Density::Compact).with_hover(false));
    }

    #[test]
    fn every_choice_has_its_label() {
        assert_eq!(ThemeChoice::ALL.len(), ThemeChoice::LABELS.len());
        assert_eq!(index_of(&SizeChoice::ALL, &SizeChoice::ExtraLarge), 5);
        assert_eq!(
            Settings {
                background: BackgroundChoice::Aurora,
                ..Settings::default()
            }
            .background()
            .map(|effect| effect.kind),
            Some(BackgroundKind::Aurora)
        );
    }
}
