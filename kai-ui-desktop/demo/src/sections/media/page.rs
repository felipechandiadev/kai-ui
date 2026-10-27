//! /media: avatars, images, an image loaded from a URL and image fields
//! that take a picked or dropped file.

use std::path::PathBuf;

use kai::commands::{CommandRegistry, PageAction};
use kai::components::{
    Avatar, AvatarSize, ButtonVariant, MediaSlot, MediaVariant, TextField, avatar, button,
    caption, card, card_content, card_header, page, page_body, validate_image_file,
};
use kai::element::{Element, div, image};
use kai::media::{ImageSource, sample_landscape, sample_portrait};
use kai::router::{Context, HasPage, Page};
use kai::runtime::{Effects, PickFile};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::State;

/// Largest image the fields take.
const MAX_IMAGE_MB: u64 = 5;

/// An image field: what it shows and the last file refused.
#[derive(Clone, Debug, Default, PartialEq)]
struct ImageSlot {
    actual: Option<ImageSource>,
    error: Option<String>,
}

impl ImageSlot {
    fn with(source: ImageSource) -> Self {
        Self {
            actual: Some(source),
            error: None,
        }
    }

    /// Show `path` if it is an image of at most [`MAX_IMAGE_MB`].
    fn set(&mut self, path: PathBuf) {
        match validate_image_file(&path, MAX_IMAGE_MB) {
            Ok(()) => *self = Self::with(ImageSource::Path(path)),
            Err(error) => self.error = Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageField {
    Photo,
    Cover,
    Logo,
}

pub struct Screen {
    photo: ImageSlot,
    cover: ImageSlot,
    logo: ImageSlot,
    url: String,
    avatar_url: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Url(String),
    LoadUrl(String),
    Pick(ImageField),
    Picked(ImageField, Option<PathBuf>),
}

impl Screen {
    fn slot(&mut self, field: ImageField) -> &mut ImageSlot {
        match field {
            ImageField::Photo => &mut self.photo,
            ImageField::Cover => &mut self.cover,
            ImageField::Logo => &mut self.logo,
        }
    }
}

impl Page for Screen {
    type Msg = Msg;
    type Params = ();
    type State = State;

    fn init(_: &(), _: &State) -> (Self, Effects<Msg>) {
        let screen = Self {
            photo: ImageSlot::with(sample_portrait()),
            cover: ImageSlot::default(),
            logo: ImageSlot::with(sample_landscape()),
            url: String::new(),
            avatar_url: None,
        };
        (screen, Effects::none())
    }

    fn update(&mut self, msg: Msg, _: &mut Context<'_, State>) -> Effects<Msg> {
        match msg {
            Msg::Url(url) => self.url = url,
            Msg::LoadUrl(url) => {
                let url = url.trim().to_owned();
                self.avatar_url = (!url.is_empty()).then(|| url.clone());
                self.url = url;
            }
            Msg::Pick(field) => {
                let title = match field {
                    ImageField::Photo => "Elegir foto de perfil",
                    ImageField::Cover => "Elegir portada",
                    ImageField::Logo => "Elegir logo",
                };
                return Effects::pick_file(PickFile::images(title), move |path| {
                    Msg::Picked(field, path)
                });
            }
            Msg::Picked(field, Some(path)) => self.slot(field).set(path),
            Msg::Picked(_, None) => {}
        }
        Effects::none()
    }

    fn view(&self, _: &State) -> Element<Msg> {
        page().child(page_body().children([self.avatars(), self.image_fields()]))
    }

    fn title(&self, _: &State) -> String {
        "Multimedia".into()
    }

    fn commands<A: HasPage<Self>>(registry: &mut CommandRegistry<A>) {
        registry.page_action::<LoadUrl>().expose();
    }
}

impl Screen {
    fn avatars(&self) -> Element<Msg> {
        let sizes = [
            AvatarSize::Xs,
            AvatarSize::Sm,
            AvatarSize::Md,
            AvatarSize::Lg,
        ];
        let row = |label: &str, avatars: Vec<Element<Msg>>| {
            div()
                .class("flex flex-wrap items-center gap-3")
                .child(caption(label).class("w-24"))
                .children(avatars)
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("media.avatars")
            .children([
                card_header(
                    "Avatares",
                    Some(
                        "El ícono User de Lucide mientras no hay imagen; la imagen se recorta \
                         al círculo",
                    ),
                ),
                card_content().class("gap-4").children([
                    row(
                        "Sin imagen:",
                        ["Ana Rojas", "Bruno Soto", "Carla Vega", "Diego Silva"]
                            .into_iter()
                            .zip(sizes)
                            .map(|(name, size)| Avatar::new(name).size(size).build())
                            .collect(),
                    ),
                    row(
                        "Con imagen:",
                        sizes
                            .into_iter()
                            .map(|size| {
                                Avatar::new("Felipe Muñoz")
                                    .image(sample_portrait())
                                    .size(size)
                                    .build()
                            })
                            .collect(),
                    ),
                    div().class("flex items-center gap-4").children([
                        avatar(
                            "Imagen remota",
                            self.avatar_url.clone().map(ImageSource::Url),
                        )
                        .test_id("media.avatar_url"),
                        div().class("flex flex-1 flex-col gap-2 min-w-0").children([
                            caption("Desde una URL (media.load_url):"),
                            div().class("flex items-center gap-3").children([
                                TextField::new("URL de la imagen", &self.url)
                                    .placeholder("https://…/foto.png")
                                    .on_input(Msg::Url)
                                    .on_submit(Msg::LoadUrl(self.url.clone()))
                                    .test_id("media.url")
                                    .build()
                                    .class("flex-1"),
                                button("Cargar", ButtonVariant::Outlined)
                                    .on_page_command(LoadUrl {
                                        url: self.url.clone(),
                                    })
                                    .test_id("media.load"),
                            ]),
                        ]),
                    ]),
                    row(
                        "image(src):",
                        vec![
                            image(sample_landscape())
                                .alt("Paisaje")
                                .class("w-48 rounded-md")
                                .test_id("media.landscape"),
                            image(sample_landscape())
                                .alt("Paisaje recortado")
                                .class("size-24 rounded-md object-cover"),
                        ],
                    ),
                ]),
            ])
    }

    fn image_fields(&self) -> Element<Msg> {
        let field = |field: ImageField, variant: MediaVariant, slot: &ImageSlot, id: &str| {
            MediaSlot::new(variant)
                .current(slot.actual.clone())
                .on_pick(Msg::Pick(field))
                .on_drop(move |path| Msg::Picked(field, Some(path)))
                .error(slot.error.clone())
                .test_id(id)
                .build()
        };
        let labeled = |label: &str, slot: Element<Msg>| {
            div()
                .class("flex flex-col items-center gap-2 min-w-0")
                .children([caption(label), slot])
        };
        card()
            .class("w-full max-w-3xl")
            .test_id("media.fields")
            .children([
                card_header(
                    "Campos de imagen",
                    Some(
                        format!(
                            "Clic para elegir o arrastra desde Finder; png, jpg o webp de \
                             hasta {MAX_IMAGE_MB} MB"
                        )
                        .as_str(),
                    ),
                ),
                card_content().class("gap-6").children([
                    div().class("grid grid-cols-2 gap-6").children([
                        labeled(
                            "Foto de perfil",
                            field(
                                ImageField::Photo,
                                MediaVariant::Avatar(AvatarSize::Md),
                                &self.photo,
                                "media.photo",
                            ),
                        ),
                        labeled(
                            "Logo",
                            field(
                                ImageField::Logo,
                                MediaVariant::Logo,
                                &self.logo,
                                "media.logo",
                            ),
                        ),
                    ]),
                    labeled(
                        "Portada",
                        field(
                            ImageField::Cover,
                            MediaVariant::Banner,
                            &self.cover,
                            "media.cover",
                        ),
                    ),
                ]),
            ])
    }
}

/// `media.load_url`: download an image and show it in the avatar.
#[derive(Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoadUrl {
    #[schemars(
        description = "Dirección http(s) de una imagen png, jpg o webp; vacía, vuelve al ícono"
    )]
    url: String,
}

impl PageAction for LoadUrl {
    type Page = Screen;
    const ID: &'static str = "media.load_url";
    const TITLE: &'static str = "Cargar imagen desde URL";
    fn description() -> &'static str {
        "Descarga una imagen y la muestra en el avatar de la sección Multimedia"
    }
    fn into_msg(self) -> Msg {
        Msg::LoadUrl(self.url)
    }
}
