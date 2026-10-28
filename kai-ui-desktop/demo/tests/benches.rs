//! Benchmarks of the demo; ignored, run them in release:
//! `cargo test --release --test benches -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use kai::element::Element;
use kai::geometry::Point;
use kai::runtime::App as _;
use kai::serde_json::json;
use kai::testing::{By, TestApp};
use kai_demo::app::{self, AppRoot, RootMsg};
use kai_demo::state::State;

fn percentile(samples: &mut [Duration], p: f64) -> Duration {
    samples.sort();
    samples[((samples.len() - 1) as f64 * p) as usize]
}

/// Cost of [`Element::map`] on whole views of the demo, against building
/// them.
#[test]
#[ignore = "benchmark; run explicitly in release mode"]
fn bench_element_map() {
    #[derive(Clone, Debug)]
    struct Wrapped(#[allow(dead_code)] RootMsg);

    fn count(element: &Element<Wrapped>) -> usize {
        1 + element.children.iter().map(count).sum::<usize>()
    }

    const ITERATIONS: usize = 300;
    for path in ["/home", "/fields", "/table", "/map", "/catalog"] {
        let mut app = TestApp::new(app::app(State::default()));
        app.navigate(path);
        let root: &AppRoot = app.app();
        let mut build = Vec::with_capacity(ITERATIONS);
        let mut mapped = Vec::with_capacity(ITERATIONS);
        let mut nodes = 0;
        for _ in 0..ITERATIONS {
            let start = Instant::now();
            let tree = root.view();
            build.push(start.elapsed());
            let start = Instant::now();
            let tree = tree.map(Wrapped);
            mapped.push(start.elapsed());
            nodes = count(&tree);
        }
        let (b50, m50) = (percentile(&mut build, 0.5), percentile(&mut mapped, 0.5));
        let (b95, m95) = (percentile(&mut build, 0.95), percentile(&mut mapped, 0.95));
        println!(
            "{path}: {nodes} nodos · view p50 {b50:?} p95 {b95:?} · map p50 {m50:?} p95 {m95:?} \
             ({:.1} % de view)",
            100.0 * m50.as_secs_f64() / b50.as_secs_f64()
        );
    }
}

/// Cost of a layout around a view of the demo (docs/dev/28): the layout's
/// toolbar built, mapped and its outlet filled with the page, against
/// building the page.
#[test]
#[ignore = "benchmark; run explicitly in release mode"]
fn bench_layout_fold() {
    use kai::components::{toolbar, toolbar_item, toolbar_spacer, toolbar_title};
    use kai::icons::Icon;
    use kai::router::outlet;

    #[derive(Clone, Debug)]
    enum LayoutMsg {
        Back,
        Forward,
        New,
    }
    #[derive(Clone, Debug)]
    enum Wrapped {
        Page(#[allow(dead_code)] RootMsg),
        Layout(#[allow(dead_code)] LayoutMsg),
    }

    fn frame(title: &str) -> Element<LayoutMsg> {
        kai::element::div()
            .class("flex flex-col w-full h-full")
            .children([
                toolbar([
                    toolbar_item(Icon::ArrowUp, "Atrás").on_click(LayoutMsg::Back),
                    toolbar_item(Icon::ChevronRight, "Adelante").on_click(LayoutMsg::Forward),
                    toolbar_title(title, None),
                    toolbar_spacer(),
                    toolbar_item(Icon::Plus, "Nuevo").on_click(LayoutMsg::New),
                ]),
                outlet(),
            ])
    }

    const ITERATIONS: usize = 300;
    for path in ["/home", "/fields", "/table", "/map", "/catalog"] {
        let mut app = TestApp::new(app::app(State::default()));
        app.navigate(path);
        let root: &AppRoot = app.app();
        let mut build = Vec::with_capacity(ITERATIONS);
        let mut folded = Vec::with_capacity(ITERATIONS);
        for _ in 0..ITERATIONS {
            let start = Instant::now();
            let page = root.view();
            build.push(start.elapsed());
            let page = page.map(Wrapped::Page);
            let start = Instant::now();
            let tree = frame("Título").map(Wrapped::Layout).fill_outlet(page);
            folded.push(start.elapsed());
            std::hint::black_box(&tree);
        }
        let (b50, f50) = (percentile(&mut build, 0.5), percentile(&mut folded, 0.5));
        let (b95, f95) = (percentile(&mut build, 0.95), percentile(&mut folded, 0.95));
        println!(
            "{path}: view p50 {b50:?} p95 {b95:?} · layout (build + map + fill) p50 {f50:?} \
             p95 {f95:?} ({:.1} % de view)",
            100.0 * f50.as_secs_f64() / b50.as_secs_f64()
        );
    }
}

/// Inicio resized to another width each frame: view, layout with real
/// text and paint.
#[test]
#[ignore = "benchmark; run explicitly in release mode"]
fn bench_layout_and_paint() {
    const ITERATIONS: usize = 400;
    const WIDTHS: [f32; 4] = [1024.0, 800.0, 600.0, 420.0];
    let mut app = TestApp::builder(app::app(State::default()))
        .real_text()
        .viewport(1024.0, 720.0)
        .build();
    app.type_text(By::label("Nombre"), "Ana");
    let mut samples = Vec::with_capacity(ITERATIONS);
    let mut ops = 0;
    for index in 0..ITERATIONS {
        let start = Instant::now();
        app.resize(WIDTHS[index % WIDTHS.len()], 720.0);
        let list = app.draw_list();
        samples.push(start.elapsed());
        ops += list.ops.len();
    }
    let total: Duration = samples.iter().sum();
    eprintln!(
        "view+layout+paint: {ITERATIONS} iteraciones, media {:?}, p50 {:?}, p95 {:?}, máx {:?} \
         ({} ops/frame)",
        total / ITERATIONS as u32,
        percentile(&mut samples, 0.5),
        percentile(&mut samples, 0.95),
        percentile(&mut samples, 1.0),
        ops / ITERATIONS,
    );
    assert!(ops > 0);
}

/// Frames of «Mapa» while a drag pans Parral at zoom 15: view, layout with
/// real text, tiles, labels and paint. Under 4 ms per frame in release.
#[test]
#[ignore = "benchmark; run explicitly in release mode"]
fn bench_map_pan() {
    let mut app = TestApp::builder(app::app(State::default()))
        .real_text()
        .viewport(1024.0, 720.0)
        .build();
    app.navigate("/map");
    let call =
        json!({ "center": {"lat": -36.1415, "lon": -71.8265}, "zoom": 15, "animate": false });
    app.call("map.set_view", call).unwrap();
    app.run_until_idle();
    let bounds = app.screen().get(By::test_id("map.view")).bounds();
    let start = Point {
        x: bounds.x + bounds.width / 2.0,
        y: bounds.y + bounds.height / 2.0,
    };
    let view = |app: &mut TestApp<AppRoot>| app.call("map.get_view", json!({})).unwrap();
    let before = view(&mut app);
    app.move_pointer(start);
    app.press_pointer();
    let mut samples = Vec::new();
    let mut offset = 0.0;
    for frame in 0..400 {
        offset += if (frame / 50) % 2 == 0 { -4.0 } else { 4.0 };
        let started = Instant::now();
        app.move_pointer(Point {
            x: start.x + offset,
            y: start.y + offset / 3.0,
        });
        let list = app.draw_list();
        samples.push(started.elapsed());
        assert!(!list.ops.is_empty());
    }
    app.release_pointer();
    assert_ne!(view(&mut app), before, "the drag pans");
    let p50 = percentile(&mut samples, 0.5);
    let p95 = percentile(&mut samples, 0.95);
    println!("pan del mapa: p50 {p50:?}, p95 {p95:?} por frame");
    if !cfg!(debug_assertions) {
        assert!(p95 < Duration::from_millis(4), "p95 {p95:?}");
    }
}
