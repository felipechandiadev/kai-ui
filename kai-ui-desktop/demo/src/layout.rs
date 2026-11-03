//! The bar across the top of the window, around every page: the sidebar
//! toggle, back and forward, the title of the page on screen and the
//! global settings (`src/settings.rs`), and the thin status bar across the
//! bottom. Both bars span the whole window (`WRAPS_SIDEBAR`) and have slots
//! the layouts and pages inside can fill. Navigation buttons run the
//! framework's commands; the settings are the layout's own messages.

use kai::background::BackgroundEffect;
use kai::components::{
    caption, segmented, status_bar, status_item, status_text, toolbar, toolbar_areas,
    toolbar_item, toolbar_spacer, toolbar_title,
};
use kai::element::{Element, div};
use kai::icons::Icon;
use kai::router::{Context, Layout, RouteInfo, outlet, slot};
use kai::runtime::Effects;
use kai::store::StoreError;
use kai::theme::Theme;

use crate::settings::{BackgroundChoice, DensityChoice, SizeChoice, ThemeChoice, index_of};
use crate::state::State;

/// The frame around the content, to the right of the sidebar.
pub struct Frame {
    /// The settings strip under the bar is showing.
    settings_open: bool,
}

#[derive(Clone, Debug)]
pub enum Msg {
    ToggleSettings,
    Theme(usize),
    Density(usize),
    Size(usize),
    Background(usize),
    Saved(Result<(), StoreError>),
}

impl Layout for Frame {
    type Msg = Msg;
    type State = State;

    fn init(_: &State) -> (Self, Effects<Msg>) {
        (
            Self {
                settings_open: false,
            },
            Effects::none(),
        )
    }

    fn update(&mut self, msg: Msg, cx: &mut Context<'_, State>) -> Effects<Msg> {
        let state = cx.state();
        let settings = &mut state.settings;
        match msg {
            Msg::ToggleSettings => {
                self.settings_open = !self.settings_open;
                return Effects::none();
            }
            Msg::Theme(index) => {
                settings.theme = ThemeChoice::ALL[index.min(ThemeChoice::ALL.len() - 1)];
                // A choice in the bar ends the `KAI_THEME` override.
                state.env_theme = false;
            }
            Msg::Density(index) => {
                settings.density = DensityChoice::ALL[index.min(DensityChoice::ALL.len() - 1)];
            }
            Msg::Size(index) => {
                settings.size = SizeChoice::ALL[index.min(SizeChoice::ALL.len() - 1)];
            }
            Msg::Background(index) => {
                settings.background =
                    BackgroundChoice::ALL[index.min(BackgroundChoice::ALL.len() - 1)];
            }
            Msg::Saved(result) => {
                if let Err(error) = result {
                    eprintln!("No se pudieron guardar los ajustes: {error}");
                }
                return Effects::none();
            }
        }
        match &state.store {
            Some(store) => store.save_effect(&state.settings, Msg::Saved),
            None => Effects::none(),
        }
    }

    /// The settings' theme; a page with its own (the catalog) wins.
    fn theme(&self, state: &State) -> Option<Theme> {
        state.theme()
    }

    /// The settings' background; a page showing its own wins.
    fn background(&self, state: &State) -> Option<BackgroundEffect> {
        state.settings.background()
    }

    /// The bars go across the whole window, over and under the sidebar.
    const WRAPS_SIDEBAR: bool = true;

    fn view(&self, state: &State, route: &RouteInfo<'_>) -> Element<Msg> {
        let app = format!("Kai UI Desktop v{}", kai::version::VERSION);
        let sidebar = if route.sidebar_collapsed {
            "Mostrar barra lateral"
        } else {
            "Ocultar barra lateral"
        };
        let top = toolbar([toolbar_areas(
            [
                toolbar_item(Icon::PanelLeft, sidebar)
                    .run_command("kai.toggle_sidebar")
                    .test_id("bar.sidebar"),
                toolbar_item(Icon::ChevronLeft, "Atrás")
                    .run_command("kai.back")
                    .test_id("bar.back"),
                toolbar_item(Icon::ChevronRight, "Adelante")
                    .run_command("kai.forward")
                    .test_id("bar.forward"),
                toolbar_title(route.title, Some(&app)).test_id("bar.title"),
                slot(SLOT_TOP_LEADING).class("gap-1").test_id("bar.leading"),
            ],
            // Later: the tabs of the section.
            [slot(SLOT_TOP_CENTER).class("gap-1").test_id("bar.center")],
            [
                // What the page brings: the search of the table, for one.
                slot("actions").class("gap-1").test_id("bar.actions"),
                slot(SLOT_TOP_TRAILING).class("gap-1").test_id("bar.trailing"),
                toolbar_item(Icon::Settings, "Ajustes")
                    .on_click(Msg::ToggleSettings)
                    .test_id("bar.settings"),
            ],
        )])
        .test_id("bar");
        div()
            .class("flex flex-col flex-1 min-w-0 min-h-0")
            .child(top)
            .children(self.settings_open.then(|| settings_strip(state)))
            .child(outlet())
            .child(bottom_bar(state))
    }
}

/// Slots of the top bar a section's layout or a page can fill.
pub const SLOT_TOP_LEADING: &str = "top.leading";
pub const SLOT_TOP_CENTER: &str = "top.center";
pub const SLOT_TOP_TRAILING: &str = "top.trailing";
/// Slots of the bottom bar, at each end of the status.
pub const SLOT_BOTTOM_LEADING: &str = "bottom.leading";
pub const SLOT_BOTTOM_TRAILING: &str = "bottom.trailing";

/// The thin status bar across the bottom of the window.
fn bottom_bar(state: &State) -> Element<Msg> {
    let clients = state.crm.clients.len();
    status_bar([
        status_item(Icon::CircleCheckBig, "", "Sincronizado"),
        status_text(format!("Sincronizado hace 2 min · {clients} clientes"))
            .test_id("bar.status"),
        slot(SLOT_BOTTOM_LEADING)
            .class("gap-2")
            .test_id("bar.bottom.leading"),
        toolbar_spacer(),
        slot(SLOT_BOTTOM_TRAILING)
            .class("gap-2")
            .test_id("bar.bottom.trailing"),
        status_item(Icon::Users, "", format!("{clients} clientes")),
        status_item(Icon::Bell, "", "Sin avisos"),
    ])
    .test_id("bar.bottom")
}

/// The global settings, one segmented control each, under the bar.
fn settings_strip(state: &State) -> Element<Msg> {
    let settings = &state.settings;
    let group = |label: &str, control: Element<Msg>| {
        div()
            .class("flex flex-col gap-1")
            .children([caption(label), control])
    };
    div()
        .class("flex flex-row flex-wrap items-end gap-4 px-4 py-2 border-b border-border")
        .test_id("settings")
        .children([
            group(
                "Tema",
                segmented(
                    &ThemeChoice::LABELS,
                    index_of(&ThemeChoice::ALL, &state.theme_choice()),
                    Msg::Theme,
                )
                .label("Tema")
                .test_id("settings.theme"),
            ),
            group(
                "Densidad",
                segmented(
                    &DensityChoice::LABELS,
                    index_of(&DensityChoice::ALL, &settings.density),
                    Msg::Density,
                )
                .label("Densidad")
                .test_id("settings.density"),
            ),
            group(
                "Tamaño",
                segmented(
                    &SizeChoice::LABELS,
                    index_of(&SizeChoice::ALL, &settings.size),
                    Msg::Size,
                )
                .label("Tamaño")
                .test_id("settings.size"),
            ),
            group(
                "Fondo",
                segmented(
                    &BackgroundChoice::LABELS,
                    index_of(&BackgroundChoice::ALL, &settings.background),
                    Msg::Background,
                )
                .label("Fondo")
                .test_id("settings.background"),
            ),
        ])
}
