//! What every section shares: the global settings of the top bar and where
//! they are saved, and the in-memory CRM of «App real». Everything else
//! stays in each section.

use kai::store::Store;
use kai::theme::Theme;

use crate::crm::Crm;
use crate::settings::{Settings, ThemeChoice};
use crate::showcase::Memory;

pub struct State {
    /// Theme, density, size and background chosen in the top bar.
    pub settings: Settings,
    /// The theme `main` gives the runtime: what «Inicial» means.
    pub initial_theme: Theme,
    /// `KAI_THEME` was given: it wins over the saved theme choice until
    /// one is picked in the bar (the choice on disk is kept).
    pub env_theme: bool,
    /// Where the settings are saved; `None` in tests, which keep them in
    /// memory.
    pub store: Option<Store>,
    /// The CRM of «App real» (/real-app), in memory.
    pub crm: Crm,
    /// The props changed on each component page (/components/<id>), so
    /// they are still there when it is opened again.
    pub components: Memory,
}

impl Default for State {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            initial_theme: Theme::glass(),
            env_theme: false,
            store: None,
            crm: Crm::default(),
            components: Memory::default(),
        }
    }
}

impl State {
    /// The settings saved in `store` (the defaults when there are none or
    /// they cannot be read), saved there again when they change.
    pub fn with_store(store: Store) -> Self {
        let settings = store.load::<Settings>().unwrap_or_else(|error| {
            eprintln!("Ajustes guardados ilegibles ({error}); se usan los de fábrica");
            None
        });
        Self {
            settings: settings.unwrap_or_default(),
            store: Some(store),
            ..Self::default()
        }
    }

    /// The theme choice in effect (`KAI_THEME` first, see `env_theme`).
    pub fn theme_choice(&self) -> ThemeChoice {
        if self.env_theme {
            ThemeChoice::Initial
        } else {
            self.settings.theme
        }
    }

    /// The window's theme the settings ask for, if they change it.
    pub fn theme(&self) -> Option<Theme> {
        self.settings
            .theme(&self.initial_theme, self.theme_choice())
    }
}
