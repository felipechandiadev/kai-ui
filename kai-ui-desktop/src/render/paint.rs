//! Pure paint pass: computed layout -> [`DrawList`].
//!
//! Nodes paint in tree order with siblings stably sorted by `z-index`
//! (painter's algorithm), each one as CSS would: shadow, ring (Tailwind
//! rings are box shadows, so they sit outside the border box and under the
//! node), background and border, content, children, and finally the overlay
//! scrollbar of `overflow-auto` nodes. Every op is clipped to the node's
//! inherited overflow clip only, so borders and rings of unclipped nodes are
//! never cut.

use super::{DrawList, Mask, Quad, TRANSPARENT, TextId};
use crate::element::WidgetId;
use crate::geometry::{Point, Rect};
use crate::layout::{Content, Layout, LayoutNode};
use crate::render::Color;
use crate::text_field::TextFieldDecorations;
use crate::theme::Theme;

const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_INSET: f32 = 2.0;
const SCROLL_THUMB_MIN: f32 = 24.0;

pub struct PaintInput<'a> {
    pub layout: &'a Layout,
    pub theme: &'a Theme,
    pub focused: Option<WidgetId>,
    /// Caret, selection and preedit per input; missing entries paint as an
    /// unfocused empty field.
    pub text_fields: &'a [(WidgetId, &'a TextFieldDecorations)],
}

struct Painter<'a> {
    input: &'a PaintInput<'a>,
    list: DrawList,
    clip: Option<Rect>,
}

pub fn paint(input: &PaintInput<'_>) -> DrawList {
    let clear = input
        .layout
        .root()
        .and_then(|root| root.style.background)
        .unwrap_or(input.theme.colors.background);
    let mut painter = Painter {
        input,
        list: DrawList::new(clear),
        clip: None,
    };
    if input.layout.root().is_some() {
        painter.node(0);
    }
    painter.set_clip(None);
    painter.list
}

fn faded(color: Color, opacity: f32) -> Color {
    if opacity >= 1.0 {
        return color;
    }
    color.with_alpha((f32::from(color.a) * opacity.max(0.0)).round() as u8)
}

fn grow(rect: Rect, amount: f32) -> Rect {
    Rect {
        x: rect.x - amount,
        y: rect.y - amount,
        width: (rect.width + amount * 2.0).max(0.0),
        height: (rect.height + amount * 2.0).max(0.0),
    }
}

fn origin(rect: Rect) -> Point {
    Point {
        x: rect.x,
        y: rect.y,
    }
}

impl Painter<'_> {
    fn set_clip(&mut self, clip: Option<Rect>) {
        if clip == self.clip {
            return;
        }
        if self.clip.is_some() {
            self.list.pop_clip();
        }
        if let Some(rect) = clip {
            self.list.push_clip(rect);
        }
        self.clip = clip;
    }

    fn node(&mut self, index: usize) {
        let layout = self.input.layout;
        let node = layout.get(index);
        if node.hidden {
            return;
        }
        self.set_clip(node.clip);
        self.decorations(node);
        match &node.content {
            Content::Box => {}
            Content::Text(_) => {
                if let Some(text_box) = node.text_box {
                    self.list.text(
                        TextId::content(node.id),
                        origin(text_box),
                        faded(node.text.color, node.opacity),
                    );
                }
            }
            Content::Input { .. } => self.input_contents(node),
            Content::Icon(icon) => {
                let area = node.content_box;
                let side = area.width.min(area.height);
                if side > 0.0 {
                    self.list.quad(Quad::icon(
                        Rect {
                            x: area.x + (area.width - side) / 2.0,
                            y: area.y + (area.height - side) / 2.0,
                            width: side,
                            height: side,
                        },
                        faded(node.text.color, node.opacity),
                        Mask {
                            icon: *icon,
                            stroke_width: node.text.stroke_width,
                        },
                    ));
                }
            }
        }
        let mut children = node.children.clone();
        children.sort_by_key(|child| layout.get(*child).style.z_index);
        for child in children {
            self.node(child);
        }
        self.scrollbar(node);
    }

    fn decorations(&mut self, node: &LayoutNode) {
        let style = &node.style;
        let colors = &self.input.theme.colors;
        let opacity = node.opacity;
        let bounds = node.bounds;
        if let Some(shadow) = style.shadow {
            self.list.quad(Quad {
                blur: shadow.blur * 0.75,
                radius: style.radius + shadow.spread.max(0.0),
                ..Quad::fill(
                    grow(bounds, shadow.spread).translated(Point {
                        x: 0.0,
                        y: shadow.offset_y,
                    }),
                    faded(shadow.color, opacity),
                )
            });
        }
        let ring = style.ring;
        if ring.width > 0.0 {
            let spread = ring.offset + ring.width;
            self.list.quad(Quad {
                rect: grow(bounds, spread),
                fill: TRANSPARENT,
                border: faded(
                    ring.color.unwrap_or(colors.primary.with_alpha(128)),
                    opacity,
                ),
                border_width: ring.width,
                radius: if style.radius > 0.0 {
                    style.radius + spread
                } else {
                    0.0
                },
                blur: 0.0,
                mask: None,
            });
        }
        let fill = faded(style.background.unwrap_or(TRANSPARENT), opacity);
        let border_color = faded(style.border_color.unwrap_or(colors.border), opacity);
        let widths = style.border_width;
        if widths.is_uniform() {
            self.list.quad(Quad {
                rect: bounds,
                fill,
                border: border_color,
                border_width: widths.top,
                radius: style.radius,
                blur: 0.0,
                mask: None,
            });
        } else {
            self.list.quad(Quad {
                radius: style.radius,
                ..Quad::fill(bounds, fill)
            });
            let sides = [
                Rect {
                    height: widths.top,
                    ..bounds
                },
                Rect {
                    y: bounds.y + bounds.height - widths.bottom,
                    height: widths.bottom,
                    ..bounds
                },
                Rect {
                    width: widths.left,
                    ..bounds
                },
                Rect {
                    x: bounds.x + bounds.width - widths.right,
                    width: widths.right,
                    ..bounds
                },
            ];
            for side in sides {
                if side.width > 0.0 && side.height > 0.0 {
                    self.list.fill_rect(side, border_color);
                }
            }
        }
    }

    fn input_contents(&mut self, node: &LayoutNode) {
        let Some(line) = node.text_box else {
            return;
        };
        let theme = self.input.theme;
        let opacity = node.opacity;
        let focused = self.input.focused == Some(node.id);
        let empty = TextFieldDecorations::default();
        let field = self
            .input
            .text_fields
            .iter()
            .find(|(id, _)| *id == node.id)
            .map_or(&empty, |(_, decorations)| *decorations);
        let interior = Rect {
            x: line.x,
            y: node.padding_box.y,
            width: line.width,
            height: node.padding_box.height,
        };
        let clip = match node.clip {
            None => interior,
            Some(clip) => clip.intersection(interior).unwrap_or(Rect {
                width: 0.0,
                height: 0.0,
                ..interior
            }),
        };
        self.set_clip(Some(clip));
        let line_origin = origin(line);
        let text_color = faded(node.text.color, opacity);
        for rect in &field.selection {
            self.list.fill_rect(
                rect.translated(line_origin),
                faded(theme.colors.primary.with_alpha(71), opacity),
            );
        }
        for rect in &field.preedit_underline {
            self.list
                .fill_rect(rect.translated(line_origin), text_color);
        }
        if focused && let Some(caret) = field.caret {
            self.list
                .fill_rect(caret.translated(line_origin), text_color);
        }
        if field.has_content {
            self.list.text(
                TextId::value(node.id),
                Point {
                    x: line.x - field.scroll_x,
                    y: line.y,
                },
                text_color,
            );
        } else {
            let placeholder = node
                .style
                .placeholder_color
                .unwrap_or(theme.colors.muted_foreground);
            self.list.text(
                TextId::placeholder(node.id),
                line_origin,
                faded(placeholder, opacity),
            );
        }
        self.set_clip(node.clip);
    }

    fn scrollbar(&mut self, node: &LayoutNode) {
        let Some(scroll) = node.scroll.filter(|scroll| scroll.max > 0.0) else {
            return;
        };
        self.set_clip(node.clip);
        let view = node.padding_box;
        let track_height = (view.height - SCROLLBAR_INSET * 2.0).max(0.0);
        let content_height = view.height + scroll.max;
        let thumb_height = (track_height * view.height / content_height)
            .max(SCROLL_THUMB_MIN)
            .min(track_height);
        let travel = (track_height - thumb_height).max(0.0);
        let progress = (scroll.offset / scroll.max).clamp(0.0, 1.0);
        self.list.quad(Quad {
            radius: SCROLLBAR_WIDTH / 2.0,
            ..Quad::fill(
                Rect {
                    x: view.x + view.width - SCROLLBAR_WIDTH - SCROLLBAR_INSET,
                    y: view.y + SCROLLBAR_INSET + travel * progress,
                    width: SCROLLBAR_WIDTH,
                    height: thumb_height,
                },
                faded(
                    self.input.theme.colors.muted_foreground.with_alpha(153),
                    node.opacity,
                ),
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{PaintInput, paint};
    use crate::element::sample::{self, Msg};
    use crate::element::{Element, WidgetId, div, icon, text};
    use crate::geometry::{Point, Rect, Size};
    use crate::icons::Icon;
    use crate::layout::testing::FixedMeasure;
    use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
    use crate::render::{Batch, DrawList, DrawOp, Mask, Quad, TextId, resolve};
    use crate::text_field::TextFieldDecorations;
    use crate::theme::{Density, Theme};

    const WINDOW: Size = Size {
        width: 800.0,
        height: 600.0,
    };

    fn theme() -> Theme {
        Theme::kai(Density::Comfortable)
    }

    fn layout(root: &Element<Msg>, viewport: Size, interaction: &Interaction) -> Layout {
        Layout::compute(
            root,
            &LayoutInput {
                viewport,
                theme: &theme(),
                interaction,
                scroll: &ScrollOffsets::new(),
            },
            &mut StyleCache::default(),
            &mut FixedMeasure,
        )
    }

    fn draw(
        root: &Element<Msg>,
        viewport: Size,
        interaction: &Interaction,
        fields: &[(WidgetId, &TextFieldDecorations)],
    ) -> DrawList {
        let layout = layout(root, viewport, interaction);
        paint(&PaintInput {
            layout: &layout,
            theme: &theme(),
            focused: interaction.focused,
            text_fields: fields,
        })
    }

    fn quads(list: &DrawList) -> Vec<Quad> {
        list.ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Quad(quad) => Some(*quad),
                _ => None,
            })
            .collect()
    }

    fn text_index(list: &DrawList, id: TextId) -> usize {
        list.ops
            .iter()
            .position(|op| matches!(op, DrawOp::Text { id: text, .. } if *text == id))
            .unwrap()
    }

    #[test]
    fn nodes_paint_in_tree_order_so_later_boxes_cover_earlier_text() {
        let tree = sample::tree();
        let list = draw(&tree, WINDOW, &Interaction::default(), &[]);
        let theme = theme();
        assert_eq!(list.clear, theme.colors.neutral, "root background clears");
        let label = text_index(&list, TextId::content(sample::button_label()));
        let button_bg = list
            .ops
            .iter()
            .position(|op| matches!(op, DrawOp::Quad(q) if q.fill == theme.colors.primary))
            .unwrap();
        let field = text_index(&list, TextId::placeholder(sample::field()));
        assert!(button_bg < label && label < field);
        assert!(resolve(&list).len() >= 4, "quads and texts interleave");

        let pushes = list
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::PushClip(_)))
            .count();
        let pops = list
            .ops
            .iter()
            .filter(|op| matches!(op, DrawOp::PopClip))
            .count();
        assert_eq!(pushes, pops);
    }

    #[test]
    fn focus_ring_and_border_are_drawn_outside_and_inside_the_border_box() {
        let mut button: Element<Msg> = div()
            .class("p-4")
            .child(div().class("w-20 h-10 rounded-md border border-primary ring-2 ring-offset-2"));
        button.assign_ids();
        let list = draw(&button, WINDOW, &Interaction::default(), &[]);
        let quads = quads(&list);
        let ring = quads
            .iter()
            .find(|q| q.fill.a == 0 && q.border_width == 2.0)
            .unwrap();
        assert_eq!(
            ring.rect,
            Rect {
                x: 12.0,
                y: 12.0,
                width: 88.0,
                height: 48.0
            }
        );
        assert_eq!(ring.radius, 6.0 + 4.0);
        let body = quads.iter().find(|q| q.border_width == 1.0).unwrap();
        assert_eq!(
            body.rect,
            Rect {
                x: 16.0,
                y: 16.0,
                width: 80.0,
                height: 40.0
            }
        );
        assert!(!list.ops.iter().any(|op| matches!(op, DrawOp::PushClip(_))));
    }

    #[test]
    fn interaction_variants_change_paint_and_disabled_fades() {
        let tree = sample::tree();
        let theme = theme();
        let field_focused = Interaction {
            focused: Some(sample::field()),
            ..Interaction::default()
        };
        let decorations = TextFieldDecorations {
            has_content: true,
            caret: Some(Rect {
                x: 10.0,
                y: 0.0,
                width: 1.5,
                height: 24.0,
            }),
            ..TextFieldDecorations::default()
        };
        let list = draw(
            &tree,
            WINDOW,
            &field_focused,
            &[(sample::field(), &decorations)],
        );
        let layout = layout(&tree, WINDOW, &field_focused);
        let line = layout.text_box(sample::field()).unwrap();
        let quads = quads(&list);
        assert!(
            quads
                .iter()
                .any(|q| q.border == theme.colors.primary && q.border_width == 1.0)
        );
        assert!(quads.iter().any(|q| q.rect
            == Rect {
                x: line.x + 10.0,
                y: line.y,
                width: 1.5,
                height: 24.0
            }));

        let disabled = layout.node(sample::disabled_button()).unwrap();
        assert_eq!(disabled.opacity, 0.5);
        let label = layout.get(disabled.children[0]);
        assert_eq!(label.opacity, 0.5, "opacity multiplies down the tree");
    }

    #[test]
    fn icons_paint_as_masked_quads_with_inherited_color_and_stroke() {
        let theme = theme();
        let mut root: Element<Msg> = div()
            .class("flex gap-2 p-2 text-secondary stroke-[2.25]")
            .children([
                icon(Icon::Search).class("size-5"),
                text("Buscar").class("text-sm"),
                icon(Icon::Eye).class("text-primary stroke-1 opacity-50"),
            ]);
        root.assign_ids();
        let list = draw(&root, WINDOW, &Interaction::default(), &[]);
        let icons: Vec<Quad> = quads(&list)
            .into_iter()
            .filter(|quad| quad.mask.is_some())
            .collect();
        assert_eq!(icons.len(), 2);
        assert_eq!(
            icons[0].rect,
            Rect {
                x: 8.0,
                y: 8.0,
                width: 20.0,
                height: 20.0
            }
        );
        assert_eq!(icons[0].fill, theme.colors.secondary);
        assert_eq!(
            icons[0].mask,
            Some(Mask {
                icon: Icon::Search,
                stroke_width: 2.25
            })
        );
        assert_eq!(icons[1].rect.width, 16.0, "1em of the inherited font size");
        assert_eq!(icons[1].fill, theme.colors.primary.with_alpha(128));
        assert_eq!(icons[1].mask.unwrap().stroke_width, 1.0);
        let text = text_index(&list, TextId::content(root.children[1].id));
        let eye = list
            .ops
            .iter()
            .position(
                |op| matches!(op, DrawOp::Quad(q) if q.mask.is_some_and(|m| m.icon == Icon::Eye)),
            )
            .unwrap();
        assert!(text < eye, "icons keep painter's order with text");
    }

    #[test]
    fn empty_input_shows_placeholder_and_overflow_draws_a_scrollbar() {
        let tree = sample::tree();
        let short = Size {
            width: 420.0,
            height: 200.0,
        };
        let list = draw(&tree, short, &Interaction::default(), &[]);
        assert!(list.ops.iter().any(|op| matches!(
            op,
            DrawOp::Text { id, .. } if *id == TextId::placeholder(sample::field())
        )));
        let thumb = quads(&list).into_iter().last().unwrap();
        assert_eq!(thumb.rect.width, 4.0);
        assert_eq!(thumb.rect.x, 420.0 - 4.0 - 2.0);

        let mut clipped: Element<Msg> = div().class("w-full h-full").child(
            div()
                .class("w-20 h-20 overflow-hidden")
                .child(text("muy largo").class("w-[400px] whitespace-nowrap")),
        );
        clipped.assign_ids();
        let list = draw(&clipped, WINDOW, &Interaction::default(), &[]);
        let batches = resolve(&list);
        let Some(Batch::Texts(texts)) = batches.last() else {
            panic!("text batch");
        };
        assert_eq!(
            texts[0].clip,
            Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 80.0,
                height: 80.0
            })
        );
        assert_eq!(texts[0].origin, Point { x: 0.0, y: 0.0 });
    }
}
