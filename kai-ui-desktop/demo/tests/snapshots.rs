//! Captures of the demo for visual review, rendered offscreen as the
//! runtime draws them (they need a GPU, so they are ignored by default):
//!
//! - `cargo test --test snapshots -- --ignored snapshot_demo`: Kai theme,
//!   BMP under `target/snapshots/` (the baseline, `scripts/snapshot-baseline.sh`).
//! - `cargo test --test snapshots -- --ignored snapshots_appkit`: the static
//!   AppKit themes, light and dark, Aqua 11 and Glass 27, as PNG under
//!   `target/snapshots/appkit/`.

use std::path::{Path, PathBuf};

use kai::element::Role;
use kai::snapshot::{self, Image};
use kai::testing::{By, Key, Modifiers, TestApp};
use kai::theme::{AppKitGeneration, Appearance, Density, Theme};
use kai_demo::app::{self, AppRoot};
use kai_demo::state::State;

type App = TestApp<AppRoot>;

/// Where a set of captures goes and with which theme.
struct Out {
    theme: Theme,
    dir: PathBuf,
    /// Appended to every file name.
    suffix: &'static str,
    /// PNG instead of BMP.
    png: bool,
}

impl Out {
    fn new(theme: Theme, dir: &str, suffix: &'static str, png: bool) -> Self {
        // `target/` of kai-ui-desktop, where the demo's builds go.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self {
            theme,
            dir,
            suffix,
            png,
        }
    }

    fn app(&self, path: &str, width: f32, height: f32) -> App {
        let mut app = TestApp::builder(app::app(State::default()))
            .theme(self.theme)
            .viewport(width, height)
            .build();
        app.navigate(path);
        app.run_until_idle();
        app
    }

    fn wide(&self, path: &str) -> App {
        self.app(path, 1024.0, 720.0)
    }

    fn write(&self, name: &str, image: &Image) {
        let file = format!("{name}{}", self.suffix);
        if self.png {
            snapshot::write_png(&self.dir.join(format!("{file}.png")), image).unwrap();
        } else {
            snapshot::write_bmp(&self.dir.join(format!("{file}.bmp")), image).unwrap();
        }
    }

    fn shot(&self, name: &str, app: &App) {
        self.write(name, &app.capture_window().expect("snapshot render"));
    }
}

/// Scroll the page so the element `by` sits near the top of the window.
fn scroll_to(app: &mut App, by: By) {
    let top = app.screen().get(by.clone()).bounds().y;
    app.scroll_wheel(by, -(top - 40.0).max(0.0));
}

fn render_demo_shots(out: &Out) {
    let idle = out.wide("/home");
    out.shot("demo-idle", &idle);

    let mut typed = out.wide("/home");
    for _ in 0..3 {
        typed.click(By::test_id("counter.increment"));
    }
    typed.type_text(By::label("Nombre"), "Ana");
    typed.hover(By::role(Role::Button).name("Incrementar"));
    out.shot("demo-interaction", &typed);

    let mut narrow = out.app("/home", 420.0, 560.0);
    narrow.type_text(By::label("Nombre"), "Ana");
    out.shot("demo-narrow", &narrow);

    // The focus ring of a button reached with Tab.
    let mut buttons = out.wide("/buttons");
    scroll_to(&mut buttons, By::test_id("buttons.secondary"));
    buttons.click(By::test_id("buttons.secondary"));
    buttons.press(
        Key::Tab,
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    );
    buttons.press(Key::Tab, Modifiers::NONE);
    out.shot("demo-variants-focus", &buttons);

    let mut fields = out.app("/fields", 1024.0, 640.0);
    fields.type_text(By::label("Contraseña"), "kai-2026");
    out.shot("demo-fields", &fields);
    fields.type_text(By::label("Monto"), "1500");
    fields.type_text(By::label("Comuna"), "Parral");
    fields.click(By::label("Contraseña"));
    fields.hover(By::label("Contraseña"));
    out.shot("demo-fields-focus", &fields);

    for (name, path) in [
        ("demo-selection", "/selection"),
        ("demo-containers", "/containers"),
        ("demo-badges", "/badges"),
        ("demo-charts", "/charts"),
        ("demo-document", "/document"),
        ("demo-catalog", "/catalog"),
        ("demo-template-list", "/template-list"),
        ("demo-template-form", "/template-form"),
        ("demo-template-data", "/template-data"),
        ("demo-real-app", "/real-app"),
        ("demo-real-app-clients", "/real-app/clients"),
        ("demo-real-app-client", "/real-app/clients/4"),
        ("demo-real-app-form", "/real-app/clients/new"),
    ] {
        out.shot(name, &out.wide(path));
    }

    let mut table = out.wide("/table");
    table.click(By::role(Role::ColumnHeader).name("Monto"));
    table.click(By::role(Role::ColumnHeader).name("Monto"));
    table.click(By::test_id("table.clients.row.1"));
    out.shot("demo-table", &table);

    let mut map = out.wide("/map");
    out.shot("demo-map-areas", &map);
    map.click(By::role(Role::Switch).name("Sectores"));
    out.shot("demo-map", &map);
    let mut selected = out.wide("/map");
    selected.click(By::test_id("map.sector.AOP26000002"));
    selected.advance(std::time::Duration::from_secs(1));
    out.shot("demo-map-selection", &selected);
    selected.click(By::test_id("map.edit"));
    selected.advance(std::time::Duration::from_secs(1));
    out.shot("demo-map-editing", &selected);

    let mut real_map = out.wide("/real-app/map");
    real_map.click(By::test_id("real_app.map.client.4"));
    real_map.advance(std::time::Duration::from_secs(1));
    out.shot("demo-real-app-map", &real_map);

    let mut media = out.wide("/media");
    media.drop_files(By::test_id("media.logo.preview"), ["/no/existe/logo.gif"]);
    out.shot("demo-media", &media);
    media.hover_files();
    media.hover(By::test_id("media.cover.preview"));
    out.shot("demo-media-drag", &media);
    media.cancel_file_drag();
    media.hover(By::test_id("media.logo.preview"));
    out.shot("demo-media-hover", &media);

    let mut dialog = out.wide("/dialogs");
    dialog.click(By::test_id("dialogs.delete"));
    out.shot("demo-dialog", &dialog);

    let mut toasts = out.wide("/dialogs");
    for kind in ["success", "error", "warning", "info"] {
        toasts.click(By::test_id(format!("toasts.{kind}")));
    }
    out.shot("demo-toasts", &toasts);

    // Buttons waiting on their tasks, with the dots.
    let mut waiting = out.wide("/buttons");
    scroll_to(&mut waiting, By::test_id("buttons.wait"));
    for id in ["wait.primary", "wait.outlined", "wait.reload"] {
        waiting.click(By::test_id(id));
    }
    out.shot("demo-buttons-waiting", &waiting);

    let mut rail = out.wide("/table");
    rail.click(By::test_id("sections.toggle"));
    out.shot("demo-sidebar-collapsed", &rail);
}

#[test]
#[ignore = "needs a GPU; writes images for visual review"]
fn snapshot_demo() {
    render_demo_shots(&Out::new(
        Theme::kai(Density::Comfortable),
        "target/snapshots",
        "",
        false,
    ));
}

#[test]
#[ignore = "needs a GPU; writes images for visual review"]
fn snapshots_appkit() {
    for (generation, appearance, suffix) in [
        (AppKitGeneration::Aqua11, Appearance::Light, ""),
        (AppKitGeneration::Aqua11, Appearance::Dark, "-dark"),
        (AppKitGeneration::Glass27, Appearance::Light, "-glass27"),
        (AppKitGeneration::Glass27, Appearance::Dark, "-glass27-dark"),
    ] {
        render_demo_shots(&Out::new(
            Theme::appkit_static(generation, appearance),
            "target/snapshots/appkit",
            suffix,
            true,
        ));
    }
}
