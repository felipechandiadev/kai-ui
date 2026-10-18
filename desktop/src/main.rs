pub mod a11y;
pub mod components;
pub mod element;
pub mod geometry;
pub mod grid_layout;
pub mod layout;
pub mod platform_adapter;
pub mod render;
pub mod runtime;
pub mod signal;
pub mod style;
pub mod text_field;
pub mod theme;
pub mod tw;
pub mod ui_engine;

use components::{
    ButtonVariant, button, caption, card, card_actions, card_content, card_header, paragraph, stat,
    text_field, text_input,
};
use element::{Element, Role, div, text};
use runtime::App;
use theme::{Density, Theme};

/// Demo model: everything the screen shows derives from it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Demo {
    clicks: u32,
    name: String,
    email: String,
    submissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Msg {
    Increment,
    Reset,
    NameChanged(String),
    EmailChanged(String),
    Submit,
    Clear,
}

impl Demo {
    fn can_submit(&self) -> bool {
        !self.name.trim().is_empty()
    }
}

impl App for Demo {
    type Msg = Msg;

    fn title(&self) -> String {
        if self.clicks == 0 {
            "Kai UI Desktop".to_owned()
        } else {
            format!("Kai UI Desktop — {} clics", self.clicks)
        }
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Increment => self.clicks += 1,
            Msg::Reset => self.clicks = 0,
            Msg::NameChanged(name) => self.name = name,
            Msg::EmailChanged(email) => self.email = email,
            Msg::Submit => {
                if self.can_submit() {
                    self.submissions.push(self.name.trim().to_owned());
                    self.name.clear();
                    self.email.clear();
                }
            }
            Msg::Clear => {
                self.name.clear();
                self.email.clear();
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        let header = div()
            .class(
                "flex items-center justify-between gap-4 h-14 px-6 bg-background \
                 border-b border-border/70 shadow-md z-10",
            )
            .role(Role::Header)
            .children([
                text("Kai UI Desktop")
                    .class("text-lg font-semibold text-primary whitespace-nowrap"),
                caption("Elm · clases Tailwind · wgpu").class("truncate"),
            ]);

        let counter = card().class("w-full max-w-3xl").children([
            card_header(
                "Contador reactivo",
                Some("El estado vive en el modelo; cada mensaje vuelve a ejecutar la vista"),
            ),
            card_content().children([
                div().class("flex items-end gap-2").children([
                    text(self.clicks.to_string())
                        .class("text-3xl font-bold text-foreground")
                        .role(Role::Status),
                    caption(if self.clicks == 1 { "clic" } else { "clics" }).class("pb-1"),
                ]),
                paragraph(
                    "Los botones usan las mismas clases que Kai web: hover, foco con anillo, \
                     estado activo y deshabilitado.",
                ),
            ]),
            card_actions().children([
                button("Reiniciar", ButtonVariant::Outlined)
                    .on_click(Msg::Reset)
                    .disabled(self.clicks == 0),
                button("Incrementar", ButtonVariant::Primary).on_click(Msg::Increment),
            ]),
        ]);

        let greeting = if self.name.trim().is_empty() {
            caption("Escribe tu nombre para ver la vista reaccionar.")
        } else {
            text(format!("Hola, {} 👋", self.name.trim()))
                .class("text-base font-medium text-primary")
        };
        let form = card().class("w-full max-w-3xl").children([
            card_header(
                "Formulario controlado",
                Some("Cada tecla emite un mensaje con el nuevo valor"),
            ),
            card_content().class("gap-4").children([
                div().class("grid grid-cols-2 gap-4").children([
                    text_field(
                        "Nombre",
                        text_input(&self.name, "")
                            .on_input(Msg::NameChanged)
                            .on_submit(Msg::Submit),
                    ),
                    text_field(
                        "Correo",
                        text_input(&self.email, "")
                            .on_input(Msg::EmailChanged)
                            .on_submit(Msg::Submit),
                    ),
                ]),
                greeting.role(Role::Status),
            ]),
            card_actions().children([
                button("Limpiar", ButtonVariant::Text).on_click(Msg::Clear),
                button("Enviar", ButtonVariant::Primary)
                    .on_click(Msg::Submit)
                    .disabled(!self.can_submit()),
            ]),
        ]);

        let last = self
            .submissions
            .last()
            .map_or("Sin envíos todavía".to_owned(), |name| {
                format!("Último: {name}")
            });
        let stats = div()
            .class("grid grid-cols-3 gap-4 w-full max-w-3xl")
            .children([
                stat("Clics", self.clicks.to_string(), None),
                stat(
                    "Caracteres",
                    self.name.chars().count().to_string(),
                    Some("en el nombre"),
                ),
                stat("Envíos", self.submissions.len().to_string(), Some(&last)),
            ]);

        let variants = card().class("w-full max-w-3xl").children([
            card_header("Variantes de botón", None),
            card_content().child(
                div().class("flex flex-wrap items-center gap-3").children([
                    button("Primary", ButtonVariant::Primary).on_click(Msg::Increment),
                    button("Secondary", ButtonVariant::Secondary).on_click(Msg::Increment),
                    button("Outlined", ButtonVariant::Outlined).on_click(Msg::Increment),
                    button("Text", ButtonVariant::Text).on_click(Msg::Increment),
                    button("Danger", ButtonVariant::Danger).on_click(Msg::Reset),
                    button("Deshabilitado", ButtonVariant::Primary)
                        .on_click(Msg::Increment)
                        .disabled(true),
                ]),
            ),
        ]);

        div()
            .class("w-full h-full flex flex-col bg-neutral")
            .role(Role::Main)
            .children([
                header,
                div().class("flex-1 overflow-y-auto").child(
                    div()
                        .class("flex flex-col items-center gap-6 p-6")
                        .children([counter, form, stats, variants]),
                ),
            ])
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(target_os = "macos") {
        return Err("Kai UI Desktop comienza como prototipo para macOS".into());
    }
    runtime::run(Demo::default(), Theme::kai(Density::Comfortable))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use cosmic_text::{
        Action, Attrs, Buffer, Edit, Editor, FontSystem, Metrics, Motion, Selection, Shaping,
    };

    use super::{Demo, Msg};
    use crate::element::{FocusKind, FocusOrder, Kind, WidgetId};
    use crate::geometry::Size;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache, text_style};
    use crate::render::TextId;
    use crate::render::paint::{PaintInput, paint};
    use crate::render::text::TextStore;
    use crate::render::wgpu_backend::WgpuRenderer;
    use crate::runtime::{App, build_view};
    use crate::text_field::{TextFieldDecorations, TextFieldState};
    use crate::theme::{Density, Theme};

    fn layout(demo: &Demo, viewport: Size, styles: &mut StyleCache) -> Layout {
        Layout::compute(
            &build_view(demo),
            &LayoutInput {
                viewport,
                theme: &Theme::kai(Density::Comfortable),
                interaction: &Interaction::default(),
                scroll: &ScrollOffsets::new(),
            },
            styles,
            &mut FixedMeasure,
        )
    }

    #[test]
    fn update_follows_messages_and_view_reflects_the_model() {
        let mut demo = Demo::default();
        demo.update(Msg::Increment);
        demo.update(Msg::Increment);
        assert_eq!(demo.clicks, 2);
        assert_eq!(demo.title(), "Kai UI Desktop — 2 clics");
        demo.update(Msg::Submit);
        assert!(demo.submissions.is_empty(), "empty names are not submitted");
        demo.update(Msg::NameChanged("  Ana ".to_owned()));
        let view = build_view(&demo);
        let mut texts = Vec::new();
        view.walk(&mut |element| {
            if let Kind::Text(text) = &element.kind {
                texts.push(text.clone());
            }
        });
        assert!(texts.contains(&"Hola, Ana 👋".to_owned()));
        assert!(texts.contains(&"2".to_owned()));
        demo.update(Msg::Submit);
        assert_eq!(demo.submissions, vec!["Ana".to_owned()]);
        assert!(demo.name.is_empty());
        demo.update(Msg::Reset);
        assert_eq!(demo.clicks, 0);
    }

    #[test]
    fn demo_focus_order_follows_document_order_and_skips_disabled() {
        let demo = Demo::default();
        let order = FocusOrder::from_tree(&build_view(&demo));
        let kinds: Vec<FocusKind> = order.ids().map(|id| order.kind(id).unwrap()).collect();
        assert_eq!(
            kinds[0],
            FocusKind::Button,
            "Incrementar; Reiniciar starts disabled"
        );
        assert_eq!(kinds[1], FocusKind::TextField);
        assert_eq!(kinds[2], FocusKind::TextField);
        assert_eq!(kinds.len(), 1 + 2 + 1 + 5, "Enviar starts disabled");
    }

    #[test]
    fn demo_uses_only_supported_classes_and_fills_the_window() {
        let mut styles = StyleCache::default();
        let mut demo = Demo::default();
        demo.update(Msg::NameChanged("Ana".to_owned()));
        let layout = layout(
            &demo,
            Size {
                width: 420.0,
                height: 340.0,
            },
            &mut styles,
        );
        assert!(styles.unknown().is_empty(), "{:?}", styles.unknown());
        let root = layout.root().unwrap();
        assert_eq!((root.bounds.width, root.bounds.height), (420.0, 340.0));
        let content = layout.get(root.children[1]);
        let scroll = content.scroll.expect("content scrolls");
        assert!(scroll.max > 0.0, "the small window overflows");
        let header = layout.get(root.children[0]);
        assert_eq!(header.bounds.y, 0.0, "header stays fixed");
        assert_eq!(content.clip, None);
    }

    /// Full layout + paint of the demo view with real cosmic-text
    /// measurement and no cache; widths alternate so text re-wraps every
    /// frame. Run with:
    /// `cargo test --release -- --ignored --nocapture bench_layout_and_paint`
    #[test]
    #[ignore = "benchmark; run explicitly in release mode"]
    fn bench_layout_and_paint() {
        const ITERATIONS: usize = 400;
        const WIDTHS: [f32; 4] = [1024.0, 800.0, 600.0, 420.0];
        let theme = Theme::kai(Density::Comfortable);
        let mut demo = Demo::default();
        demo.update(Msg::NameChanged("Ana".to_owned()));
        let mut text = TextStore::new(FontSystem::new());
        let mut styles = StyleCache::default();

        let mut samples: Vec<Duration> = Vec::with_capacity(ITERATIONS);
        let mut ops = 0;
        for index in 0..ITERATIONS {
            let viewport = Size {
                width: WIDTHS[index % WIDTHS.len()],
                height: 720.0,
            };
            let start = Instant::now();
            let tree = build_view(&demo);
            let layout = Layout::compute(
                &tree,
                &LayoutInput {
                    viewport,
                    theme: &theme,
                    interaction: &Interaction::default(),
                    scroll: &ScrollOffsets::new(),
                },
                &mut styles,
                &mut text,
            );
            let list = paint(&PaintInput {
                layout: &layout,
                theme: &theme,
                focused: None,
                text_fields: &[],
            });
            samples.push(start.elapsed());
            ops += list.ops.len();
        }
        samples.sort();
        let total: Duration = samples.iter().sum();
        eprintln!(
            "view+layout+paint: {ITERATIONS} iteraciones, media {:?}, p50 {:?}, p95 {:?}, máx {:?} ({} ops/frame)",
            total / ITERATIONS as u32,
            samples[ITERATIONS / 2],
            samples[ITERATIONS * 95 / 100],
            samples[ITERATIONS - 1],
            ops / ITERATIONS,
        );
        assert!(ops > 0);
    }

    fn element_with_text(demo: &Demo, label: &str) -> WidgetId {
        let tree = build_view(demo);
        let mut found = None;
        tree.walk(&mut |element| {
            if matches!(&element.kind, Kind::Text(text) if text == label) {
                found = Some(element.id);
            }
        });
        let path = tree.path_to(found.expect("label exists")).unwrap();
        path[path.len() - 2]
    }

    fn first_input(demo: &Demo) -> WidgetId {
        let tree = build_view(demo);
        let mut found = None;
        tree.walk(&mut |element| {
            if found.is_none() && matches!(element.kind, Kind::Input { .. }) {
                found = Some(element.id);
            }
        });
        found.unwrap()
    }

    /// Render the demo offscreen exactly as the runtime does and save it as
    /// a BMP under `target/snapshots/`.
    fn snapshot(
        name: &str,
        demo: &Demo,
        logical: Size,
        scale: f32,
        interaction: &Interaction,
        scroll: &ScrollOffsets,
    ) {
        let theme = Theme::kai(Density::Comfortable);
        let tree = build_view(demo);
        let mut text = TextStore::new(FontSystem::new());
        let mut styles = StyleCache::default();
        let layout = Layout::compute(
            &tree,
            &LayoutInput {
                viewport: logical,
                theme: &theme,
                interaction,
                scroll,
            },
            &mut styles,
            &mut text,
        );
        let mut decorations = Vec::new();
        tree.walk(&mut |element| {
            if let Kind::Input { value, .. } = &element.kind
                && let Some(node) = layout.node(element.id)
            {
                let fonts = &mut text.font_system;
                let mut field = TextFieldState::new(
                    fonts,
                    Metrics::new(node.text.font_size, node.text.line_height),
                );
                field.set_text(fonts, value);
                field.set_width(fonts, node.text_box.map_or(1.0, |line| line.width));
                let view = field.view(fonts);
                decorations.push((element.id, view.decorations));
                text.set_buffer(
                    TextId::value(element.id),
                    view.buffer,
                    text_style(&node.text),
                );
            }
        });
        let fields: Vec<(WidgetId, &TextFieldDecorations)> =
            decorations.iter().map(|(id, d)| (*id, d)).collect();
        let list = paint(&PaintInput {
            layout: &layout,
            theme: &theme,
            focused: interaction.focused,
            text_fields: &fields,
        });
        let (width, height) = (
            (logical.width * scale) as u32,
            (logical.height * scale) as u32,
        );
        let mut renderer =
            pollster::block_on(WgpuRenderer::new_offscreen(width, height)).expect("GPU");
        let pixels = renderer
            .render_to_rgba(&list, &mut text, scale)
            .expect("snapshot render");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.bmp")), bmp(width, height, &pixels)).unwrap();
    }

    /// 24-bit bottom-up BMP from top-down RGBA8.
    fn bmp(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let row = (width * 3).div_ceil(4) * 4;
        let size = 54 + row * height;
        let mut out = Vec::with_capacity(size as usize);
        out.extend_from_slice(b"BM");
        for value in [size, 0, 54, 40, width, height] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        for value in [0u32, row * height, 2835, 2835, 0, 0] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for y in (0..height).rev() {
            let start = out.len();
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                out.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i]]);
            }
            out.resize(start + row as usize, 0);
        }
        out
    }

    /// Writes `target/snapshots/*.bmp` for visual review. Needs a GPU:
    /// `cargo test -- --ignored snapshot_demo`
    #[test]
    #[ignore = "needs a GPU; writes images for visual review"]
    fn snapshot_demo() {
        let wide = Size {
            width: 1024.0,
            height: 720.0,
        };
        let idle = Demo::default();
        let none = ScrollOffsets::new();
        snapshot(
            "demo-idle",
            &idle,
            wide,
            2.0,
            &Interaction::default(),
            &none,
        );

        let mut typed = Demo {
            clicks: 3,
            ..Demo::default()
        };
        typed.update(Msg::NameChanged("Ana".to_owned()));
        let hovered_button = element_with_text(&typed, "Incrementar");
        let field = first_input(&typed);
        let tree = build_view(&typed);
        snapshot(
            "demo-interaction",
            &typed,
            wide,
            2.0,
            &Interaction {
                hovered: tree.path_to(hovered_button).unwrap(),
                focused: Some(field),
                pressed: None,
            },
            &none,
        );
        snapshot(
            "demo-narrow",
            &typed,
            Size {
                width: 420.0,
                height: 560.0,
            },
            2.0,
            &Interaction::default(),
            &none,
        );

        let content = tree.children[1].id;
        let bottom = ScrollOffsets::from([(content, 10_000.0)]);
        snapshot(
            "demo-variants-focus",
            &typed,
            wide,
            2.0,
            &Interaction {
                focused: Some(element_with_text(&typed, "Secondary")),
                ..Interaction::default()
            },
            &bottom,
        );
    }

    #[test]
    fn editor_inserts_unicode_and_selects_text() {
        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(15.0, 22.0));
        buffer.set_text("", &Attrs::new(), Shaping::Advanced, None);
        let mut editor = Editor::new(buffer);
        editor.insert_string("Kai 😀", None);
        editor.shape_as_needed(&mut font_system, false);
        editor.action(&mut font_system, Action::Motion(Motion::BufferStart));
        let anchor = editor.cursor();
        editor.set_selection(Selection::Normal(anchor));
        editor.action(&mut font_system, Action::Motion(Motion::BufferEnd));
        assert_eq!(editor.copy_selection().as_deref(), Some("Kai 😀"));
        editor.insert_string("Kai UI", None);
        let value = editor.with_buffer(|buffer| buffer.lines[0].text().to_owned());
        assert_eq!(value, "Kai UI");
    }
}
