//! Avatars and the image slot of a form (`MultimediaSingleSlot` in
//! `kai-ui-web/src/components/Multimedia`). Showing ([`Avatar`]) is apart
//! from editing ([`MediaSlot`]): the app keeps the [`ImageSource`] in its
//! model and the slot only sends messages (pick, drop).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::{AlertVariant, alert};
use crate::element::{Element, Role, div, icon, image};
use crate::icons::Icon;
use crate::media::ImageSource;
use crate::runtime::IMAGE_FILE_TYPES;

/// Diameter of an [`Avatar`] or of the circle of a [`MediaSlot`]: `Sm`,
/// `Md` and `Lg` as `.fs-mm-slot__area--avatar-*` (4, 6 and 7 rem); `Xs`
/// (1.5 rem) for rows and lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
}

impl AvatarSize {
    /// Circle and placeholder icon classes.
    fn classes(self) -> (&'static str, &'static str) {
        match self {
            Self::Xs => ("size-6", "size-4"),
            Self::Sm => ("size-16", "size-7"),
            Self::Md => ("size-24", "size-10"),
            Self::Lg => ("size-28", "size-12"),
        }
    }
}

/// A person's picture: a circle with the Lucide `User` icon and, on top,
/// their image cropped to the circle once it loads. An image named after
/// the person (`Role::Image`).
#[derive(Clone, Debug, PartialEq)]
pub struct Avatar {
    name: String,
    image: Option<ImageSource>,
    size: AvatarSize,
    test_id: Option<String>,
}

impl Avatar {
    /// The avatar of `name` (empty: anonymous).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            image: None,
            size: AvatarSize::default(),
            test_id: None,
        }
    }

    pub fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.image = Some(source.into());
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build<Msg>(self) -> Element<Msg> {
        let (circle, icon_size) = self.size.classes();
        let placeholder = icon(Icon::User).class(icon_size);
        let mut root = div()
            .class(&format!(
                "relative inline-flex shrink-0 items-center justify-center overflow-hidden \
                 rounded-full bg-secondary/15 text-secondary {circle}"
            ))
            .alt(if self.name.trim().is_empty() {
                "Usuario".to_owned()
            } else {
                self.name.clone()
            })
            .parts_root()
            .child(placeholder.part("placeholder"));
        if let Some(source) = self.image {
            root = root.child(
                image(source)
                    .class("absolute inset-0 size-full object-cover rounded-full")
                    .part("image"),
            );
        }
        root.test_id = self.test_id;
        root
    }
}

/// [`Avatar`] of `name` with an optional image, medium size.
pub fn avatar<Msg>(name: impl Into<String>, image: Option<ImageSource>) -> Element<Msg> {
    let avatar = Avatar::new(name);
    match image {
        Some(source) => avatar.image(source),
        None => avatar,
    }
    .build()
}

/// What a [`MediaSlot`] holds: a profile picture (a circle), a 16:9
/// banner or a logo (square, the whole image shown).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaVariant {
    Avatar(AvatarSize),
    Banner,
    Logo,
}

/// The image field of a form: the preview is the control. A click, Space
/// or Return on it opens the file panel, and a file dropped on it from
/// Finder replaces the image. Under the pointer (and while a file is
/// dragged over it) it takes the choice look ([`super::CHOICE_ACTIVE`]):
/// a `choice` border and tint over the image, with a pill naming the
/// action («Agregar imagen» or «Cambiar imagen»). The app opens the file
/// panel on [`MediaSlot::on_pick`] (`Effects::pick_file`), validates the
/// file ([`validate_image_file`]) and passes the result back as
/// [`MediaSlot::current`] or [`MediaSlot::error`].
///
/// Parts: `<id>.preview`, `<id>.tint`, `<id>.hint` and `<id>.error`.
pub struct MediaSlot<Msg> {
    variant: MediaVariant,
    current: Option<ImageSource>,
    on_pick: Option<Msg>,
    on_drop: Option<Rc<dyn Fn(PathBuf) -> Msg>>,
    error: Option<String>,
    disabled: bool,
    test_id: Option<String>,
}

impl<Msg: Clone + 'static> MediaSlot<Msg> {
    pub fn new(variant: MediaVariant) -> Self {
        Self {
            variant,
            current: None,
            on_pick: None,
            on_drop: None,
            error: None,
            disabled: false,
            test_id: None,
        }
    }

    /// The image shown, if any.
    pub fn current(mut self, current: Option<ImageSource>) -> Self {
        self.current = current;
        self
    }

    /// Sent by a click, Space or Return on the preview.
    pub fn on_pick(mut self, msg: Msg) -> Self {
        self.on_pick = Some(msg);
        self
    }

    /// The first file dropped on the preview.
    pub fn on_drop(mut self, handler: impl Fn(PathBuf) -> Msg + 'static) -> Self {
        self.on_drop = Some(Rc::new(handler));
        self
    }

    /// Shown under the slot as an error alert.
    pub fn error(mut self, error: Option<String>) -> Self {
        self.error = error;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let has_media = self.current.is_some();
        let surface = "group relative flex items-center justify-center overflow-hidden shrink-0 \
                       focus-visible:ring-2 focus-visible:ring-primary/40";
        let interaction = if self.disabled {
            "cursor-not-allowed"
        } else {
            "cursor-pointer hover:border-choice drag:border-choice drag:ring-2 drag:ring-choice"
        };
        let (area_class, radius, image_class, placeholder) = match self.variant {
            MediaVariant::Avatar(size) => {
                let (circle, icon_size) = size.classes();
                (
                    format!("{circle} rounded-full border-[3px] border-secondary bg-muted/70"),
                    "rounded-full",
                    "object-cover",
                    icon(Icon::User).class(&format!("{icon_size} text-secondary")),
                )
            }
            MediaVariant::Banner => (
                "w-full max-w-[448px] aspect-video rounded-lg border border-border bg-muted/55"
                    .to_owned(),
                "rounded-lg",
                "object-cover",
                icon(Icon::ImagePlus).class("size-12 text-secondary"),
            ),
            MediaVariant::Logo => (
                "size-32 rounded-lg border border-border bg-muted/55".to_owned(),
                "rounded-lg",
                "object-contain",
                icon(Icon::ImagePlus).class("size-12 text-secondary"),
            ),
        };
        let label = if has_media {
            "Cambiar imagen"
        } else {
            "Agregar imagen"
        };
        let mut area = div()
            .class(&format!("{surface} {area_class} {interaction}"))
            .role(Role::Button)
            .label(label)
            .tooltip(label)
            .disabled(self.disabled)
            .part("preview")
            .child(placeholder);
        if let Some(source) = self.current {
            area = area.child(image(source).class(&format!(
                "absolute inset-0 size-full {radius} {image_class}"
            )));
        }

        if !self.disabled {
            // Over the image, so the tint shows on it; it lets the pointer
            // through to the preview.
            area = area.child(
                div()
                    .class(&format!(
                        "absolute inset-0 {radius} pointer-events-none bg-transparent \
                         group-hover:bg-choice/20"
                    ))
                    .part("tint"),
            );
            if has_media {
                area = area.child(
                    div()
                        .class(
                            "absolute inset-0 flex items-center justify-center \
                             pointer-events-none opacity-0 group-hover:opacity-100",
                        )
                        .part("hint")
                        .child(
                            div()
                                .class(
                                    "flex items-center justify-center size-8 rounded-full \
                                     bg-background/85 text-foreground shadow-sm",
                                )
                                .child(icon(Icon::Camera).class("size-4")),
                        ),
                );
            }
            if let Some(msg) = self.on_pick {
                area = area.on_click(msg);
            }
            if let Some(on_drop) = self.on_drop {
                area = area.on_drop_files(move |paths| {
                    on_drop(paths.into_iter().next().unwrap_or_default())
                });
            }
        }

        let mut root = div()
            .class(&format!(
                "flex flex-col items-center gap-3 w-full {}",
                if self.disabled { "opacity-60" } else { "" }
            ))
            .parts_root()
            .child(area);
        if let Some(error) = self.error {
            root = root.child(
                alert(AlertVariant::Error, error)
                    .role(Role::Alert)
                    .part("error"),
            );
        }
        root.test_id = self.test_id;
        root
    }
}

/// Check a file for a [`MediaSlot`]: an image extension
/// ([`IMAGE_FILE_TYPES`]) and at most `max_mb` megabytes, with the web
/// slot's messages.
pub fn validate_image_file(path: &Path, max_mb: u64) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    if !extension.is_some_and(|extension| IMAGE_FILE_TYPES.contains(&extension.as_str())) {
        return Err(format!(
            "Tipo no permitido: {}",
            IMAGE_FILE_TYPES.join(", ")
        ));
    }
    let size = std::fs::metadata(path)
        .map_err(|error| format!("No se pudo leer el archivo: {error}"))?
        .len();
    if size > max_mb * 1024 * 1024 {
        return Err(format!("Máximo {max_mb}MB"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::render::DrawOp;
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pick,
        Drop(PathBuf),
    }

    #[derive(Default)]
    struct Slots {
        current: Option<ImageSource>,
        error: Option<String>,
        disabled: bool,
    }

    impl SimpleApp for Slots {
        type Msg = Msg;
        fn title(&self) -> String {
            "Multimedia".into()
        }
        fn update(&mut self, _msg: Msg) {}
        fn view(&self) -> Element<Msg> {
            div().class("flex flex-col gap-4 p-4").children([
                Avatar::new("Ana López").test_id("ana").build(),
                Avatar::new("")
                    .size(AvatarSize::Sm)
                    .test_id("anonimo")
                    .build(),
                avatar(
                    "Bruno Díaz",
                    Some(ImageSource::rgba(1, 1, vec![9, 9, 9, 255])),
                ),
                MediaSlot::new(MediaVariant::Avatar(AvatarSize::Md))
                    .current(self.current.clone())
                    .on_pick(Msg::Pick)
                    .on_drop(Msg::Drop)
                    .error(self.error.clone())
                    .disabled(self.disabled)
                    .test_id("foto")
                    .build(),
            ])
        }
    }

    fn slots(slots: Slots) -> TestApp<Slots> {
        TestApp::new(slots)
    }

    #[test]
    fn avatars_are_images_named_after_the_person_with_the_user_icon() {
        let app = slots(Slots::default());
        let screen = app.screen();
        let ana = screen.get(By::role(Role::Image).name("Ana López"));
        assert_eq!(ana.test_id(), Some("ana"));
        assert_eq!(ana.bounds().width, 96.0, "medium is 6 rem");
        let placeholder = screen.get(By::test_id("ana.placeholder"));
        assert!(matches!(
            placeholder.node().content,
            crate::layout::Content::Icon(Icon::User)
        ));
        assert_eq!(placeholder.bounds().width, 40.0, "size-10");
        assert!(screen.query(By::text("AL")).is_none(), "no initials");
        let anonymous = screen.get(By::role(Role::Image).name("Usuario"));
        assert_eq!(anonymous.bounds().width, 64.0);
        assert!(screen.query(By::test_id("ana.image")).is_none());
    }

    #[test]
    fn an_avatar_with_an_image_draws_it_over_the_icon() {
        let app = slots(Slots::default());
        app.screen().get(By::role(Role::Image).name("Bruno Díaz"));
        let images = app
            .draw_list()
            .ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Image(image) => Some(image.radius),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(images, [48.0], "one circle, 96 points wide");
    }

    fn with_image() -> Slots {
        Slots {
            current: Some(ImageSource::rgba(1, 1, vec![0; 4])),
            ..Slots::default()
        }
    }

    #[test]
    fn the_image_is_the_only_control_named_after_its_action() {
        let app = slots(Slots::default());
        let screen = app.screen();
        let preview = screen.get(By::test_id("foto.preview"));
        assert_eq!(preview.role(), Role::Button);
        assert_eq!(preview.name().as_deref(), Some("Agregar imagen"));
        assert_eq!(
            screen.all(By::role(Role::Button)).len(),
            1,
            "no extra add or camera button"
        );
        assert!(screen.query(By::test_id("foto.hint")).is_none());

        let app = slots(with_image());
        app.screen()
            .get(By::role(Role::Button).name("Cambiar imagen"));
        assert!(app.screen().query(By::test_id("foto.hint")).is_some());
    }

    #[test]
    fn hovering_the_image_tints_it_with_the_choice_color() {
        let mut app = slots(with_image());
        let tint = |app: &TestApp<Slots>| {
            app.screen()
                .get(By::test_id("foto.tint"))
                .node()
                .style
                .background
        };
        let hint = |app: &TestApp<Slots>| app.screen().get(By::test_id("foto.hint")).node().opacity;
        assert_eq!(tint(&app).map(|color| color.a), Some(0), "clear at rest");
        assert_eq!(hint(&app), 0.0);

        app.hover(By::test_id("foto.preview"));
        let choice = app.session().theme().color("choice").unwrap();
        let hovered = tint(&app).expect("tinted");
        assert_eq!(
            (hovered.r, hovered.g, hovered.b),
            (choice.r, choice.g, choice.b)
        );
        assert!(hovered.a > 0 && hovered.a < 255, "a tint over the image");
        assert_eq!(hint(&app), 1.0, "the camera pill shows");
        let border = app
            .screen()
            .get(By::test_id("foto.preview"))
            .node()
            .style
            .border_color;
        assert_eq!(border, Some(choice));

        // The tint and the pill let the click through to the preview.
        let bounds = app.screen().get(By::test_id("foto.preview")).bounds();
        app.click_point(crate::geometry::Point {
            x: bounds.x + bounds.width / 2.0,
            y: bounds.y + bounds.height / 2.0,
        });
        assert_eq!(app.messages(), [Msg::Pick]);
        assert_eq!(app.session().unknown_classes(), Vec::<&str>::new());
    }

    #[test]
    fn clicks_and_drops_pick_but_not_when_disabled() {
        let mut app = slots(Slots::default());
        app.click(By::test_id("foto.preview"));
        app.drop_files(By::test_id("foto.preview"), ["/a.png", "/b.png"]);
        assert_eq!(app.messages(), [Msg::Pick, Msg::Drop("/a.png".into())]);

        let mut app = slots(Slots {
            disabled: true,
            ..Slots::default()
        });
        assert!(app.screen().query(By::test_id("foto.tint")).is_none());
        app.click(By::test_id("foto.preview"));
        app.hover_files();
        assert!(app.messages().is_empty());
    }

    #[test]
    fn the_error_shows_as_an_alert() {
        let app = slots(Slots {
            error: Some("Máximo 9MB".into()),
            ..Slots::default()
        });
        let error = app.screen().get(By::role(Role::Alert));
        assert_eq!(error.test_id(), Some("foto.error"));
        assert!(app.screen().query(By::text("Máximo 9MB")).is_some());
    }

    #[test]
    fn files_are_checked_by_extension_and_size() {
        let dir = std::env::temp_dir().join(format!("kai-media-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let small = dir.join("foto.PNG");
        std::fs::write(&small, [0u8; 16]).unwrap();
        assert_eq!(validate_image_file(&small, 9), Ok(()));
        let large = dir.join("grande.jpg");
        std::fs::write(&large, vec![0u8; 1024 * 1024 + 1]).unwrap();
        assert_eq!(validate_image_file(&large, 1), Err("Máximo 1MB".to_owned()));
        assert_eq!(
            validate_image_file(Path::new("/x/nota.txt"), 9),
            Err("Tipo no permitido: png, jpg, jpeg, webp".to_owned())
        );
        assert!(validate_image_file(&dir.join("falta.webp"), 9).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
