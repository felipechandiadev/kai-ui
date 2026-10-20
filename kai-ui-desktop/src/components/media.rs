//! Avatars and the image slot of a form (`MultimediaSingleSlot` in
//! `kai-ui-web/src/components/Multimedia`). Showing ([`Avatar`]) is apart
//! from editing ([`MediaSlot`]): the app keeps the [`ImageSource`] in its
//! model and the slot only sends messages (pick, drop).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::{AlertVariant, IconButtonSize, IconButtonVariant, alert, icon_button_sized};
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

/// Where the action button of an avatar [`MediaSlot`] goes: under the
/// circle, or as a badge on its edge. Banners and logos always have it
/// below.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionPlacement {
    #[default]
    Below,
    Edge,
}

/// The image field of a form: a preview area (click to choose a file,
/// or drop one from Finder) and an action button, «Agregar foto» (`Plus`)
/// while empty and «Cambiar foto» (`Camera`) with an image. The app opens
/// the file panel on [`MediaSlot::on_pick`] (`Effects::pick_file`),
/// validates the file ([`validate_image_file`]) and passes the result
/// back as [`MediaSlot::current`] or [`MediaSlot::error`].
///
/// Parts: `<id>.preview`, `<id>.action` and `<id>.error`.
pub struct MediaSlot<Msg> {
    variant: MediaVariant,
    current: Option<ImageSource>,
    placement: ActionPlacement,
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
            placement: ActionPlacement::default(),
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

    pub fn action_placement(mut self, placement: ActionPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sent by the button and by a click on the preview.
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
        let edge = matches!(self.variant, MediaVariant::Avatar(_))
            && self.placement == ActionPlacement::Edge;
        let surface = "relative flex items-center justify-center overflow-hidden shrink-0 \
                       hover:border-primary drag:ring-2 drag:ring-primary \
                       focus-visible:ring-2 focus-visible:ring-primary/40";
        let cursor = if self.disabled {
            "cursor-not-allowed"
        } else {
            "cursor-pointer"
        };
        let (area_class, image_class, placeholder) = match self.variant {
            MediaVariant::Avatar(size) => {
                let (circle, icon_size) = size.classes();
                (
                    format!("{circle} rounded-full border-[3px] border-secondary bg-muted/70"),
                    "rounded-full object-cover",
                    icon(Icon::User).class(&format!("{icon_size} text-secondary")),
                )
            }
            MediaVariant::Banner => (
                "w-full max-w-[448px] aspect-video rounded-lg border border-border bg-muted/55"
                    .to_owned(),
                "rounded-lg object-cover",
                icon(Icon::ImagePlus).class("size-12 text-secondary"),
            ),
            MediaVariant::Logo => (
                "size-32 rounded-lg border border-border bg-muted/55".to_owned(),
                "rounded-lg object-contain",
                icon(Icon::ImagePlus).class("size-12 text-secondary"),
            ),
        };
        let mut area = div()
            .class(&format!("{surface} {area_class} {cursor}"))
            .role(Role::Button)
            .label(if has_media {
                "Actualizar imagen"
            } else {
                "Seleccionar imagen"
            })
            .disabled(self.disabled)
            .part("preview")
            .child(placeholder);
        if let Some(source) = self.current {
            area = area
                .child(image(source).class(&format!("absolute inset-0 size-full {image_class}")));
        }

        let (icon_kind, label) = if has_media {
            (Icon::Camera, "Cambiar foto")
        } else {
            (Icon::Plus, "Agregar foto")
        };
        let mut action = icon_button_sized(
            icon_kind,
            label,
            IconButtonVariant::Secondary,
            if edge {
                IconButtonSize::Sm
            } else {
                IconButtonSize::Md
            },
        )
        .disabled(self.disabled)
        .part("action");
        if edge {
            let badge = match self.variant {
                MediaVariant::Avatar(AvatarSize::Lg) => "",
                _ => "size-6",
            };
            action = action.class(&format!(
                "{badge} rounded-full border-2 border-background shadow-sm"
            ));
        }

        if !self.disabled {
            if let Some(msg) = self.on_pick {
                area = area.on_click(msg.clone());
                action = action.on_click(msg);
            }
            if let Some(on_drop) = self.on_drop {
                area = area.on_drop_files(move |paths| {
                    on_drop(paths.into_iter().next().unwrap_or_default())
                });
            }
        }

        let body = if edge {
            let offset = match self.variant {
                MediaVariant::Avatar(AvatarSize::Lg) => "bottom-[5.5px] right-[5.5px]",
                MediaVariant::Avatar(AvatarSize::Md) => "bottom-[4.5px] right-[4.5px]",
                _ => "bottom-[3px] right-[3px]",
            };
            vec![
                div()
                    .class("relative inline-flex shrink-0 p-0.5")
                    .children([
                        area,
                        div()
                            .class(&format!("absolute z-10 {offset}"))
                            .child(action),
                    ]),
            ]
        } else {
            vec![area, action]
        };
        let mut root = div()
            .class(&format!(
                "flex flex-col items-center w-full {} {}",
                if edge { "gap-1" } else { "gap-3" },
                if self.disabled { "opacity-60" } else { "" }
            ))
            .parts_root()
            .children(body);
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

    struct Slots {
        current: Option<ImageSource>,
        placement: ActionPlacement,
        error: Option<String>,
        disabled: bool,
    }

    impl Default for Slots {
        fn default() -> Self {
            Self {
                current: None,
                placement: ActionPlacement::Below,
                error: None,
                disabled: false,
            }
        }
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
                    .action_placement(self.placement)
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

    #[test]
    fn the_action_says_add_without_an_image_and_change_with_one() {
        let app = slots(Slots::default());
        let action = app.screen().get(By::test_id("foto.action"));
        assert_eq!(action.name().as_deref(), Some("Agregar foto"));
        assert_eq!(
            app.screen()
                .get(By::test_id("foto.preview"))
                .name()
                .as_deref(),
            Some("Seleccionar imagen")
        );

        let app = slots(Slots {
            current: Some(ImageSource::rgba(1, 1, vec![0; 4])),
            ..Slots::default()
        });
        app.screen()
            .get(By::role(Role::Button).name("Cambiar foto"));
        app.screen()
            .get(By::role(Role::Button).name("Actualizar imagen"));
    }

    #[test]
    fn edge_puts_the_action_on_the_circle_and_below_under_it() {
        let below = slots(Slots::default());
        let preview = below.screen().get(By::test_id("foto.preview")).bounds();
        let action = below.screen().get(By::test_id("foto.action")).bounds();
        assert!(action.y >= preview.y + preview.height, "below the circle");

        let edge = slots(Slots {
            placement: ActionPlacement::Edge,
            ..Slots::default()
        });
        let preview = edge.screen().get(By::test_id("foto.preview")).bounds();
        let action = edge.screen().get(By::test_id("foto.action")).bounds();
        assert_eq!(action.width, 24.0, "a small badge");
        assert!(action.y < preview.y + preview.height, "over the circle");
        assert!(action.x + action.width > preview.x + preview.width - 10.0);
    }

    #[test]
    fn clicks_and_drops_pick_but_not_when_disabled() {
        let mut app = slots(Slots::default());
        app.click(By::test_id("foto.action"));
        app.click(By::test_id("foto.preview"));
        app.drop_files(By::test_id("foto.preview"), ["/a.png", "/b.png"]);
        assert_eq!(
            app.messages(),
            [Msg::Pick, Msg::Pick, Msg::Drop("/a.png".into())]
        );

        let mut app = slots(Slots {
            disabled: true,
            ..Slots::default()
        });
        app.click(By::test_id("foto.action"));
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
