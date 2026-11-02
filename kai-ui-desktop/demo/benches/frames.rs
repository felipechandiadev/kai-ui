//! Frame cost with criterion (docs/dev/30-benchmarks-de-frames.md): what
//! one frame spends on `Element::map`, on view + layout, on paint, on
//! both, and on rebuilding a large tree after a message. Headless: `TestApp` with
//! real text, no window and no GPU.
//!
//! ```text
//! cargo bench --bench frames                       # everything
//! cargo bench --bench frames -- layout_paint       # one group
//! cargo bench --bench frames -- --save-baseline main
//! cargo bench --bench frames -- --baseline main    # compare
//! ```

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use kai::components::ButtonVariant;
use kai::element::{Element, div, text};
use kai::runtime::{App as _, SimpleApp};
use kai::testing::{By, Key, Modifiers, TestApp};
use kai_demo::app::{self, AppRoot, RootMsg};
use kai_demo::state::State;

/// Sections of the demo measured, from light to heavy.
const PATHS: [&str; 5] = ["/home", "/fields", "/table", "/map", "/catalog"];
/// Widths that a resize alternates between: each frame lays out again.
const WIDTHS: [f32; 2] = [1024.0, 800.0];
/// Rows of the large tree (four elements each).
const ROWS: [usize; 3] = [250, 1_000, 4_000];

fn demo(path: &str) -> TestApp<AppRoot> {
    let mut app = TestApp::builder(app::app(State::default()))
        .real_text()
        .viewport(WIDTHS[0], 720.0)
        .build();
    app.navigate(path);
    app.run_until_idle();
    app
}

fn name(path: &str) -> &str {
    path.trim_start_matches('/')
}

/// [`Element::map`] on the whole view, the root's own tree wrapped once
/// more (what a layout around a page costs), against building the view.
fn element_map(c: &mut Criterion) {
    #[derive(Clone, Debug)]
    struct Wrapped(#[allow(dead_code)] RootMsg);

    let mut group = c.benchmark_group("element_map");
    for path in PATHS {
        let app = demo(path);
        let root: &AppRoot = app.app();
        group.bench_function(BenchmarkId::new("view", name(path)), |b| {
            b.iter(|| black_box(root.view()));
        });
        group.bench_function(BenchmarkId::new("map", name(path)), |b| {
            b.iter_batched(
                || root.view(),
                |tree: Element<RootMsg>| black_box(tree.map(Wrapped)),
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

/// A resize: the session builds the view and lays it out again (real text,
/// taffy), without painting.
fn layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout");
    for path in PATHS {
        let mut app = demo(path);
        let mut frame = 0;
        group.bench_function(name(path), |b| {
            b.iter(|| {
                frame += 1;
                app.resize(WIDTHS[frame % WIDTHS.len()], 720.0);
            });
        });
    }
    group.finish();
}

/// The draw list of a frame already laid out: paint alone, without the
/// GPU.
fn paint(c: &mut Criterion) {
    let mut group = c.benchmark_group("paint");
    for path in PATHS {
        let app = demo(path);
        group.bench_function(name(path), |b| b.iter(|| black_box(app.draw_list())));
    }
    group.finish();
}

/// [`layout`] plus the draw list of the frame (paint without the GPU).
fn layout_paint(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout_paint");
    for path in PATHS {
        let mut app = demo(path);
        let mut frame = 0;
        group.bench_function(name(path), |b| {
            b.iter(|| {
                frame += 1;
                app.resize(WIDTHS[frame % WIDTHS.len()], 720.0);
                black_box(app.draw_list())
            });
        });
    }
    group.finish();
}

/// A list of `rows` rows, not virtual: every message rebuilds and lays
/// out the whole tree.
struct Large {
    rows: usize,
    bumps: u32,
}

#[derive(Clone, Debug)]
enum LargeMsg {
    Bump,
}

impl SimpleApp for Large {
    type Msg = LargeMsg;

    fn title(&self) -> String {
        "Árbol grande".into()
    }

    fn update(&mut self, msg: LargeMsg) {
        match msg {
            LargeMsg::Bump => self.bumps += 1,
        }
    }

    fn view(&self) -> Element<LargeMsg> {
        let rows = (0..self.rows).map(|row| {
            div()
                .class("flex flex-row items-center gap-2 px-3 py-1 border-b border-border")
                .key(row.to_string())
                .children([
                    text(format!("Fila {row}")).class("text-sm flex-1"),
                    text(format!("{}", (row as u32).wrapping_mul(self.bumps + 1)))
                        .class("text-xs text-muted-foreground w-16"),
                    div().class("w-2 h-2 rounded-full bg-primary"),
                ])
        });
        div().class("flex flex-col w-full h-full").children([
            kai::components::button(
                format!("Cambiar ({})", self.bumps),
                ButtonVariant::Secondary,
            )
            .test_id("large.bump")
            .on_click(LargeMsg::Bump),
            div()
                .class("flex flex-col flex-1 overflow-y-auto")
                .children(rows),
        ])
    }
}

/// One message on a large tree: update, view, layout and paint, as the
/// window does after a click.
fn rebuild_large_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("rebuild_large_tree");
    group.sample_size(30);
    for rows in ROWS {
        let mut app = TestApp::builder(Large { rows, bumps: 0 })
            .real_text()
            .viewport(1024.0, 720.0)
            .build();
        assert_eq!(app.unknown_classes(), Vec::<&str>::new());
        // The button keeps the focus: each Space is one message.
        app.click(By::test_id("large.bump"));
        let before = app.app().bumps;
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(
            app.app().bumps,
            before + 1,
            "Space on the button sends Bump"
        );
        group.bench_with_input(BenchmarkId::from_parameter(rows), &rows, |b, _| {
            b.iter(|| {
                app.press(Key::Space, Modifiers::NONE);
                black_box(app.draw_list())
            });
        });
    }
    group.finish();
}

criterion_group!(
    frames,
    element_map,
    layout,
    paint,
    layout_paint,
    rebuild_large_tree
);
criterion_main!(frames);
