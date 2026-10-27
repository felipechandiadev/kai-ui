//! The demo: `cargo run` here, or `kai dev` for a rebuild on every save.
//!
//! - `KAI_THEME=kai|appkit|appkit-dark|aqua11|aqua11-dark|glass27|glass27-dark`
//!   (default: Glass 27 light, large controls), `KAI_MIX=appkit|a|b|c`,
//!   `KAI_LABEL=near|appkit|top|tab` and `KAI_HOVER=0`.
//! - `--describe-commands` prints what the demo publishes to agents.
//! - `--mcp` (with `--features mcp`) serves its commands over stdio.

use kai::commands::describe_json;
use kai::element::ControlSize;
use kai::store::Store;
use kai::theme::{
    AppKitGeneration, Appearance, Density, FieldLabels, MacVersion, MixStyle, SystemInfo, Theme,
};
use kai_demo::app::{self, AppRoot};
use kai_demo::state::State;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flag = |name: &str| std::env::args().skip(1).any(|arg| arg == name);
    if flag("--describe-commands") {
        println!("{}", describe_json::<AppRoot>());
        return Ok(());
    }
    let mcp = flag("--mcp");
    if mcp && !cfg!(feature = "mcp") {
        return Err("--mcp necesita compilar con --features mcp".into());
    }
    let theme = theme_from_env();
    // As `app::run` does: the last section, kept in the data folder.
    let mut root = app::app(State::default());
    match Store::for_app(env!("CARGO_PKG_NAME")) {
        Ok(store) => root = root.with_store(store),
        Err(error) => eprintln!("Sin carpeta de datos: {error}"),
    }
    #[cfg(feature = "mcp")]
    if mcp {
        return kai::runtime::run_with_mcp(root, theme);
    }
    kai::runtime::run(root, theme)
}

/// The theme `KAI_THEME`, `KAI_HOVER`, `KAI_MIX` and `KAI_LABEL` ask for.
fn theme_from_env() -> Theme {
    let forced = |generation, appearance| Theme::appkit_static(generation, appearance);
    let theme = match std::env::var("KAI_THEME").as_deref() {
        // Por defecto: Glass claro, controles grandes y densidad cómoda.
        Ok("") | Err(_) => Theme {
            control_size: ControlSize::Large,
            density: Density::Comfortable,
            ..forced(AppKitGeneration::Glass27, Appearance::Light)
        },
        Ok("appkit" | "appkit-light") => Theme::appkit_current(Appearance::Light),
        Ok("appkit-dark") => Theme::appkit_current(Appearance::Dark),
        Ok("aqua11") => forced(AppKitGeneration::Aqua11, Appearance::Light),
        Ok("aqua11-dark") => forced(AppKitGeneration::Aqua11, Appearance::Dark),
        Ok("glass27") => forced(AppKitGeneration::Glass27, Appearance::Light),
        Ok("glass27-dark") => forced(AppKitGeneration::Glass27, Appearance::Dark),
        Ok("kai") => Theme::kai(Density::Comfortable),
        Ok(other) => {
            eprintln!(
                "KAI_THEME={other} desconocido (kai, appkit, appkit-dark, aqua11, aqua11-dark, \
                 glass27, glass27-dark); se usa appkit"
            );
            Theme::appkit_current(Appearance::Light)
        }
    };
    let theme = theme.with_hover(std::env::var("KAI_HOVER").as_deref() != Ok("0"));
    let mix = match std::env::var("KAI_MIX").as_deref() {
        Ok("appkit" | "strict") => MixStyle::AppKit,
        Ok("a" | "A") => MixStyle::A,
        Ok("b" | "B") => MixStyle::B,
        Ok("c" | "C" | "") | Err(_) => MixStyle::C,
        Ok(other) => {
            eprintln!("KAI_MIX={other} desconocido (appkit, a, b, c); se usa c");
            MixStyle::C
        }
    };
    let theme = theme.with_mix(mix);
    // Experiment: where AppKit fields put their label.
    let labels = match std::env::var("KAI_LABEL").as_deref() {
        Ok("appkit") => FieldLabels::AppKit,
        Ok("top") => FieldLabels::Top,
        Ok("tab") => FieldLabels::Tab,
        Ok("near" | "") | Err(_) => FieldLabels::Near,
        Ok(other) => {
            eprintln!("KAI_LABEL={other} desconocido (near, appkit, top, tab); se usa near");
            FieldLabels::Near
        }
    };
    let theme = theme.with_field_labels(labels);
    if theme.is_appkit() {
        let system = SystemInfo::detect();
        let show = |v: Option<MacVersion>| v.map_or("?".to_owned(), |v| v.to_string());
        eprintln!(
            "tema {:?} {:?}, mezcla {:?}, hover {}, etiquetas {:?} (macOS {}, SDK {}: nativo {:?})",
            theme.kind,
            theme.appearance,
            theme.mix,
            if theme.hover_effects { "sí" } else { "no" },
            theme.field_labels,
            show(system.os),
            show(system.sdk),
            system.generation()
        );
    }
    theme
}
