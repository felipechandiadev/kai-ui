//! Renders for visual review (they need a GPU, so they are ignored), as PNG
//! under `target/snapshots/`:
//!
//! - `snapshots_mix`: the mixed-theme candidates side by side
//!   (`mix/mix-compare-{light,dark}.png`, rows Kai | strict AppKit | A | B |
//!   C; columns buttons card | fields card), plus each candidate's focus
//!   and fields shots.
//! - `snapshots_renders`: the form row wide and narrow, the whole demo in
//!   mix C, the control sizes card, field labels and fields next to
//!   buttons (`renders/`; `KAI_RENDER_TAG` names the form-row sheet).
//! - `snapshots_backgrounds`: the window background effects, static and
//!   animated (`renders/bg-*.png`).
//!
//! `cargo test --test review -- --ignored snapshots_mix`

use std::path::{Path, PathBuf};

use kai::background::BackgroundKind;
use kai::components::{ButtonVariant, FieldDensity, TextField, button, card, card_content};
use kai::element::{ControlSize, Element, Kind, Role, div, text};
use kai::geometry::Size;
use kai::runtime::App as _;
use kai::snapshot::{self, Image, Shot};
use kai::snapshot::{Interaction, ScrollOffsets};
use kai::testing::{By, Key, Modifiers, TestApp};
use kai::theme::{AppKitGeneration, Appearance, Density, FieldLabels, MixStyle, Theme};
use kai_demo::app::{self, AppRoot};
use kai_demo::state::State;

type App = TestApp<AppRoot>;

const BUTTONS_CARD: &str = "Botones, Tamaños e IconButtons";
const FIELDS_CARD: &str = "Variantes de TextField";

fn out(dir: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/snapshots")
        .join(dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: PathBuf, image: &Image) {
    snapshot::write_png(&path, image).unwrap();
    println!("{}", path.display());
}

fn app_at(theme: Theme, path: &str, width: f32, height: f32) -> App {
    let mut app = TestApp::builder(app::app(State::default()))
        .theme(theme)
        .viewport(width, height)
        .build();
    app.navigate(path);
    app.run_until_idle();
    app
}

/// Scroll so the card titled `title` sits 40 pt below the top.
fn scroll_to_card(app: &mut App, title: &str) {
    let top = app.screen().get(By::text(title)).bounds().y;
    app.scroll_wheel(By::text(title), -(top - 40.0).max(0.0));
}

/// The card titled `title`, `width` × `height` points from just above its
/// top, at 2x.
fn card_shot(app: &mut App, title: &str, height: f32) -> Image {
    scroll_to_card(app, title);
    let top = app.screen().get(By::text(title)).bounds().y - 28.0;
    let full = app.capture_window().expect("snapshot render");
    crop(&full, (top.max(0.0) * 2.0) as u32, (height * 2.0) as u32)
}

/// Buttons with three clicks and the password filled, as the old demo's
/// sections were seeded.
fn buttons(theme: Theme, width: f32, height: f32) -> App {
    let mut app = app_at(theme, "/buttons", width, height);
    for _ in 0..3 {
        app.click(By::test_id("buttons.secondary"));
    }
    app
}

fn fields(theme: Theme, width: f32, height: f32) -> App {
    let mut app = app_at(theme, "/fields", width, height);
    app.type_text(By::label("Contraseña"), "kai-2026");
    app
}

/// The Secondary button focused from the keyboard, its ring drawn.
fn focus_secondary(app: &mut App) {
    app.click(By::test_id("buttons.secondary"));
    app.press(
        Key::Tab,
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    );
    app.press(Key::Tab, Modifiers::NONE);
}

/// A label on its own row, for the comparison sheets.
fn label(caption: &str) -> Image {
    let mut tree: Element<()> = div()
        .class("w-full h-full flex items-center px-4 bg-neutral")
        .child(text(caption).class("text-lg font-semibold text-foreground"));
    tree.assign_ids();
    render_tree(&tree, &Theme::kai(Density::Comfortable), 1600.0, 36.0, None)
}

fn render_tree<Msg>(
    tree: &Element<Msg>,
    theme: &Theme,
    width: f32,
    height: f32,
    focused: Option<kai::element::WidgetId>,
) -> Image {
    snapshot::render(&Shot {
        tree,
        theme,
        logical: Size { width, height },
        scale: 2.0,
        interaction: &Interaction {
            focused,
            ..Interaction::default()
        },
        scroll: &ScrollOffsets::new(),
    })
    .expect("render")
}

/// Rows `top..top + height` of `image` (clamped).
fn crop(image: &Image, top: u32, height: u32) -> Image {
    let top = top.min(image.height);
    let height = height.min(image.height - top);
    let row = (image.width * 4) as usize;
    let start = top as usize * row;
    Image {
        width: image.width,
        height,
        rgba: image.rgba[start..start + height as usize * row].to_vec(),
    }
}

/// `a` and `b` next to each other, top-aligned, on white.
fn side_by_side(a: &Image, b: &Image) -> Image {
    let (width, height) = (a.width + b.width, a.height.max(b.height));
    let mut rgba = vec![255; (width * height * 4) as usize];
    for (image, x0) in [(a, 0), (b, a.width)] {
        for y in 0..image.height {
            let src = (y * image.width * 4) as usize;
            let dst = ((y * width + x0) * 4) as usize;
            rgba[dst..dst + (image.width * 4) as usize]
                .copy_from_slice(&image.rgba[src..src + (image.width * 4) as usize]);
        }
    }
    Image {
        width,
        height,
        rgba,
    }
}

/// `images` one below the other, left-aligned, on white.
fn stack(images: &[Image]) -> Image {
    let width = images.iter().map(|i| i.width).max().unwrap_or(0);
    let mut rgba = Vec::new();
    let mut height = 0;
    for image in images {
        for y in 0..image.height {
            let src = (y * image.width * 4) as usize;
            rgba.extend_from_slice(&image.rgba[src..src + (image.width * 4) as usize]);
            rgba.resize(rgba.len() + ((width - image.width) * 4) as usize, 255);
        }
        height += image.height;
    }
    Image {
        width,
        height,
        rgba,
    }
}

fn mix_c(appearance: Appearance) -> Theme {
    Theme::appkit_static(AppKitGeneration::Aqua11, appearance).with_mix(MixStyle::C)
}

#[test]
#[ignore = "needs a GPU; writes images for visual review"]
fn snapshots_mix() {
    let dir = out("mix");
    for (generation, appearance, name) in [
        (AppKitGeneration::Aqua11, Appearance::Light, "light"),
        (AppKitGeneration::Glass27, Appearance::Dark, "dark"),
    ] {
        let strict = Theme::appkit_static(generation, appearance);
        let rows = [
            ("Kai (referencia)", Theme::kai(Density::Comfortable), "kai"),
            ("AppKit estricto (KAI_MIX=appkit)", strict, "appkit"),
            (
                "A · cerca de AppKit (KAI_MIX=a)",
                strict.with_mix(MixStyle::A),
                "a",
            ),
            (
                "B · intermedio (KAI_MIX=b)",
                strict.with_mix(MixStyle::B),
                "b",
            ),
            (
                "C · cerca de Kai (KAI_MIX=c, por defecto)",
                strict.with_mix(MixStyle::C),
                "c",
            ),
        ];
        let mut sheet = Vec::new();
        for (caption, theme, tag) in rows {
            let buttons_card = card_shot(&mut buttons(theme, 800.0, 460.0), BUTTONS_CARD, 300.0);
            let fields_card = card_shot(&mut fields(theme, 800.0, 630.0), FIELDS_CARD, 470.0);
            sheet.push(label(&format!("{caption} — {generation:?} {appearance:?}")));
            sheet.push(side_by_side(&buttons_card, &fields_card));
            if tag != "kai" {
                let mut focused = buttons(theme, 1024.0, 720.0);
                scroll_to_card(&mut focused, BUTTONS_CARD);
                focus_secondary(&mut focused);
                write(
                    dir.join(format!("demo-variants-focus-{name}-{tag}.png")),
                    &focused.capture_window().unwrap(),
                );
                let mut filled = fields(theme, 1024.0, 640.0);
                scroll_to_card(&mut filled, FIELDS_CARD);
                write(
                    dir.join(format!("demo-fields-{name}-{tag}.png")),
                    &filled.capture_window().unwrap(),
                );
            }
        }
        write(dir.join(format!("mix-compare-{name}.png")), &stack(&sheet));
    }
}

#[test]
#[ignore = "needs a GPU; writes images for visual review"]
fn snapshots_renders() {
    let dir = out("renders");
    let tag = std::env::var("KAI_RENDER_TAG").unwrap_or_else(|_| "after".to_owned());

    // The form row: Kai and mix C, wide | narrow.
    let mut rows = Vec::new();
    for theme in [Theme::kai(Density::Comfortable), mix_c(Appearance::Light)] {
        let shot = |width: f32| {
            let mut app = app_at(theme, "/home", width, 420.0);
            app.type_text(By::label("Nombre"), "Ana");
            scroll_to_card(&mut app, "Formulario controlado");
            app.capture_window().unwrap()
        };
        rows.push(side_by_side(&shot(1024.0), &shot(420.0)));
    }
    write(dir.join(format!("form-row-{tag}.png")), &stack(&rows));

    // Inicio in mix C light (white window), tall enough for every card.
    let mut whole = app_at(mix_c(Appearance::Light), "/home", 1024.0, 3000.0);
    whole.type_text(By::label("Nombre"), "Ana");
    write(
        dir.join("demo-white-light.png"),
        &whole.capture_window().unwrap(),
    );

    // The control sizes card: Kai, mix C light and dark.
    for (name, theme) in [
        ("kai", Theme::kai(Density::Comfortable)),
        ("appkit-light", mix_c(Appearance::Light)),
        ("appkit-dark", mix_c(Appearance::Dark)),
    ] {
        let mut app = buttons(theme, 900.0, 860.0);
        write(
            dir.join(format!("sizes-{name}.png")),
            &card_shot(&mut app, "Tamaños de control", 700.0),
        );
    }

    // Mix C field labels, appkit | top | near (default) | tab, on a group
    // box whose fill differs from the fields': empty, filled, focused
    // (empty) and required filled.
    for (name, appearance) in [("light", Appearance::Light), ("dark", Appearance::Dark)] {
        let theme = mix_c(appearance);
        let columns = [
            ("KAI_LABEL=appkit", FieldLabels::AppKit),
            ("KAI_LABEL=top", FieldLabels::Top),
            ("near (default)", FieldLabels::Near),
            ("KAI_LABEL=tab", FieldLabels::Tab),
        ]
        .map(|(title, labels)| {
            let mut tree: Element<()> = div()
                .class("w-full h-full flex flex-col gap-3 p-4 bg-window")
                .children([
                    text(title).class("text-headline text-label"),
                    card()
                        .class("w-full")
                        .child(card_content().class("gap-4").children([
                            TextField::new("Nombre", "").build(),
                            TextField::new("Correo", "ana@kai.cl").build(),
                            TextField::new("Ciudad", "").build(),
                            TextField::new("Comuna", "Ñuñoa").required(true).build(),
                        ])),
                ]);
            tree.assign_ids();
            let mut focused = None;
            tree.walk(&mut |e| {
                if matches!(e.kind, Kind::Input { .. }) && e.label.as_deref() == Some("Ciudad") {
                    focused = Some(e.id);
                }
            });
            render_tree(
                &tree,
                &theme.with_field_labels(labels),
                300.0,
                400.0,
                focused,
            )
        });
        let sheet = columns[1..]
            .iter()
            .fold(columns[0].clone(), |sheet, column| {
                side_by_side(&sheet, column)
            });
        write(dir.join(format!("labels-compare-{name}.png")), &sheet);
    }

    // A mix C field (near label) next to every button size, in an
    // `items-end` and an `items-center` row, regular and compact.
    for (name, appearance) in [
        ("field-button-align", Appearance::Light),
        ("field-button-align-dark", Appearance::Dark),
    ] {
        let row = |items: &str, density: FieldDensity, variant: ButtonVariant| {
            div()
                .class(&format!("flex gap-2 {items}"))
                .child(
                    div()
                        .class("w-[220px]")
                        .child(TextField::new("Campo", "").density(density).build()),
                )
                .children(
                    ControlSize::ALL
                        .map(|size| button(format!("{size:?}"), variant).control_size(size)),
                )
        };
        let mut tree: Element<()> = div()
            .class("w-full h-full flex flex-col gap-4 p-4 bg-window")
            .children(["items-end", "items-center"].map(|items| {
                div().class("flex flex-col gap-3").children([
                    text(format!("{items} (campo normal y compacto)"))
                        .class("text-headline text-label"),
                    row(items, FieldDensity::Default, ButtonVariant::Primary),
                    row(items, FieldDensity::Default, ButtonVariant::Outlined),
                    row(items, FieldDensity::Compact, ButtonVariant::Primary),
                    row(items, FieldDensity::Compact, ButtonVariant::Outlined),
                ])
            }));
        tree.assign_ids();
        write(
            dir.join(format!("{name}.png")),
            &render_tree(&tree, &mix_c(appearance), 760.0, 640.0, None),
        );
    }
}

/// Inicio with background `kind`, animated or not.
fn with_background(theme: Theme, kind: BackgroundKind, animated: bool) -> App {
    let mut app = app_at(theme, "/home", 1024.0, 720.0);
    let index = BackgroundKind::ALL.iter().position(|k| *k == kind).unwrap();
    app.click(By::test_id(format!("background.effect.{index}")));
    if animated {
        app.click(By::role(Role::Switch).name("Animado"));
    }
    app
}

#[test]
#[ignore = "needs a GPU; writes images for visual review"]
fn snapshots_backgrounds() {
    let dir = out("renders");
    let themes = [
        ("kai", Theme::kai(Density::Comfortable)),
        ("appkit-light", mix_c(Appearance::Light)),
        ("appkit-dark", mix_c(Appearance::Dark)),
    ];
    for (name, theme) in themes {
        let mut row: Option<Image> = None;
        for kind in BackgroundKind::ALL {
            let image = with_background(theme, kind, false)
                .capture_window()
                .unwrap();
            write(dir.join(format!("bg-{}-{name}.png", kind.name())), &image);
            row = Some(match row {
                None => image,
                Some(row) => side_by_side(&row, &image),
            });
        }
        write(dir.join(format!("bg-sheet-{name}.png")), &row.unwrap());
    }

    // Motion: the animated effects at 0, 4 and 8 s of a 16 s cycle.
    let theme = mix_c(Appearance::Light);
    for kind in [
        BackgroundKind::Mesh,
        BackgroundKind::Aurora,
        BackgroundKind::Glow,
    ] {
        let app = with_background(theme, kind, true);
        assert!(app.app().background().is_animated());
        let frames: Vec<Image> = [0.0, 4.0, 8.0]
            .into_iter()
            .map(|seconds| {
                let image = app.capture_window_at(seconds).unwrap();
                write(
                    dir.join(format!("bg-{}-t{seconds}.png", kind.name())),
                    &image,
                );
                image
            })
            .collect();
        assert_ne!(frames[0], frames[1], "{kind:?} moves");
        let sheet = side_by_side(&side_by_side(&frames[0], &frames[1]), &frames[2]);
        write(dir.join(format!("bg-{}-motion.png", kind.name())), &sheet);
    }

    // Static effects ignore the time.
    let still = with_background(theme, BackgroundKind::Aurora, false);
    assert_eq!(
        still.capture_window_at(0.0).unwrap(),
        still.capture_window_at(5.0).unwrap()
    );
}

/// «App real», Resumen in Glass: the pane footer with the pointer over its
/// search button (the hover fill must stay inside the row) and the
/// destructive confirmation (`renders/panel-*.png`).
#[test]
#[ignore = "needs a GPU; writes images for visual review"]
fn snapshots_real_app_panel() {
    let dir = out("renders");
    let theme = Theme {
        control_size: ControlSize::Large,
        ..Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light)
    };
    let mut app = app_at(theme, "/real-app", 1280.0, 800.0);
    app.hover(By::test_id("real_app.list.search"));
    write(dir.join("panel-hover.png"), &app.capture_window().unwrap());
    app.click(By::test_id("real_app.detail.delete"));
    write(dir.join("panel-delete.png"), &app.capture_window().unwrap());
}

/// The list toolbar of «App real» in Glass: at rest, the search halfway
/// through unfolding, open with text («N de M»), and the filter menu
/// (`renders/toolbar-*.png`).
#[test]
#[ignore = "needs a GPU; writes images for visual review"]
fn snapshots_list_toolbar() {
    let dir = out("renders");
    let theme = Theme {
        control_size: ControlSize::Large,
        ..Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light)
    };
    let mut app = app_at(theme, "/real-app", 1280.0, 800.0);
    write(
        dir.join("toolbar2-normal.png"),
        &app.capture_window().unwrap(),
    );
    app.click(By::test_id("real_app.list.bar.search"));
    app.advance(std::time::Duration::from_millis(64));
    write(
        dir.join("toolbar2-animando.png"),
        &app.capture_window().unwrap(),
    );
    app.advance(std::time::Duration::from_millis(250));
    app.run_until_idle();
    app.type_text(By::test_id("real_app.list.bar.field.input"), "terminal");
    write(
        dir.join("toolbar2-busqueda.png"),
        &app.capture_window().unwrap(),
    );
    app.press(Key::Escape, Modifiers::NONE);
    app.press(Key::Escape, Modifiers::NONE);
    app.advance(std::time::Duration::from_millis(250));
    app.run_until_idle();
    app.click(By::test_id("real_app.list.filter"));
    write(
        dir.join("toolbar2-filtro.png"),
        &app.capture_window().unwrap(),
    );
}
