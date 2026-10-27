//! /media: the image fields take a picked or dropped image and refuse
//! other files; `media.load_url` shows a downloaded image in the avatar.

use kai::media::FakeImageLoader;
use kai::serde_json::json;
use kai::testing::{By, TestApp};
use kai_demo::app;
use kai_demo::state::State;

/// A 1×1 PNG.
const PNG: [u8; 70] = [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 100, 248, 207, 80, 15, 0, 3,
    134, 1, 128, 90, 52, 125, 107, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn media() -> TestApp<app::AppRoot> {
    let mut app = TestApp::new(app::app(State::default()));
    app.navigate("/media");
    app
}

#[test]
fn image_fields_take_picked_and_dropped_files_and_refuse_others() {
    let dir = std::env::temp_dir().join(format!("kai-demo-media-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cover = dir.join("portada.png");
    std::fs::write(&cover, PNG).unwrap();
    let mut app = media();
    assert_eq!(app.unknown_classes(), Vec::<&str>::new());
    let preview = |id: &str| By::test_id(format!("media.{id}.preview"));
    let name = |app: &TestApp<app::AppRoot>, id: &str| {
        app.screen().get(preview(id)).name().unwrap_or_default()
    };
    assert_eq!(name(&app, "cover"), "Agregar imagen");

    app.click(preview("cover"));
    let request = app.file_pick_request().expect("the panel is open");
    assert_eq!(request.title, "Elegir portada");
    app.on_file_pick(Some(cover));
    app.run_until_idle();
    assert_eq!(name(&app, "cover"), "Cambiar imagen");

    app.on_file_pick(None);
    app.click(preview("photo"));
    app.run_until_idle();
    assert_eq!(name(&app, "photo"), "Cambiar imagen", "cancelled");

    app.drop_files(preview("logo"), [dir.join("nota.txt")]);
    assert_eq!(
        app.screen()
            .get(By::test_id("media.logo.error"))
            .name()
            .as_deref(),
        Some("Tipo no permitido: png, jpg, jpeg, webp")
    );
    assert_eq!(name(&app, "logo"), "Cambiar imagen", "the logo stays");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn load_url_puts_a_downloaded_image_in_the_avatar() {
    const URL: &str = "https://kai.test/ana.png";
    let mut app = TestApp::builder(app::app(State::default()))
        .images(FakeImageLoader::new().with(URL, PNG))
        .build();
    app.navigate("/media");
    let image = By::test_id("media.avatar_url.image");
    assert!(app.screen().query(image.clone()).is_none(), "the icon");

    assert_eq!(
        app.call("media.load_url", json!({ "url": URL })),
        Ok(json!(null))
    );
    app.run_until_idle();
    app.screen().get(image.clone());
    assert_eq!(
        app.value(By::test_id("media.url")),
        URL,
        "the field shows it"
    );

    app.type_text(By::test_id("media.url"), " ");
    app.click(By::test_id("media.load"));
    assert_eq!(app.value(By::test_id("media.url")), URL, "trimmed");
    app.call("media.load_url", json!({ "url": "" })).unwrap();
    assert!(app.screen().query(image).is_none(), "back to the icon");
}
