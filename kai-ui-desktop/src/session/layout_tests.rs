//! Form layout across themes: two-field rows that stack in a narrow
//! window and fields that line up with buttons (docs/dev/27).

use crate::components::{
    ButtonVariant, FieldDensity, FieldKind, TextField, button, card, card_content, page, page_body,
    page_grid, page_grid_item,
};
use crate::element::{ControlSize, Element, div};
use crate::geometry::{Rect, Size};
use crate::layout::testing::FixedMeasure;
use crate::layout::{Interaction, Layout, LayoutInput, ScrollOffsets, StyleCache};
use crate::testing::{By, Screen};
use crate::theme::{AppKitGeneration, Appearance, Density, MixStyle, Theme};

/// A page with a card of two two-field rows, as forms have them.
fn form() -> Element<()> {
    let field = |label: &str, kind| {
        page_grid_item(6).child(TextField::new(label, "").field_icon(kind).build())
    };
    let mut tree = page().child(page_body().child(card().child(card_content().child(
        page_grid().children([
            field("Nombre", FieldKind::Name),
            field("Correo", FieldKind::Email),
            field("Buscar", FieldKind::Search),
            field("Contraseña", FieldKind::Password),
        ]),
    ))));
    tree.assign_ids();
    tree
}

/// Bounds of the shells (the 180 pt minimum grid cells) of two fields.
fn row_bounds(theme: &Theme, (a, b): (&str, &str), width: f32) -> (Rect, Rect) {
    let tree = form();
    let layout = Layout::compute(
        &tree,
        &LayoutInput {
            viewport: Size {
                width,
                height: 700.0,
            },
            theme,
            interaction: &Interaction::default(),
            scroll: &ScrollOffsets::new(),
        },
        &mut StyleCache::default(),
        &mut FixedMeasure,
    );
    let bounds = |label| {
        let input = Screen::new(&layout).get(By::label(label)).id();
        // The field's shell: the input's grid cell ancestor.
        let path = tree.path_to(input).unwrap();
        let cell = path.iter().rev().find(|id| {
            layout
                .node(**id)
                .is_some_and(|n| n.style.min_width == crate::style::Length::Px(180.0))
        });
        layout.bounds(*cell.unwrap()).unwrap()
    };
    (bounds(a), bounds(b))
}

/// Two-field rows on PageGrid span 6: side by side from `md` (and half
/// of the 8-col `sm` grid), stacked full-width below 640 pt; never wider
/// than the window (the fields keep their 180 pt minimum).
#[test]
fn field_rows_stack_in_a_narrow_window() {
    let rows = [("Nombre", "Correo"), ("Buscar", "Contraseña")];
    let themes = [
        Theme::kai(Density::Comfortable),
        Theme::appkit_static(AppKitGeneration::Aqua11, Appearance::Light).with_mix(MixStyle::C),
        Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Dark),
    ];
    for theme in themes {
        let at = |width: f32| rows.map(|pair| row_bounds(&theme, pair, width));
        for (first, second) in at(1024.0) {
            assert_eq!(first.y, second.y, "{:?}", theme.kind);
            assert!(second.x >= first.x + first.width);
        }
        for width in [639.0, 420.0, 400.0] {
            for (first, second) in at(width) {
                assert_eq!(first.x, second.x, "{width} {:?}", theme.kind);
                assert!(second.y >= first.y + first.height);
                assert!(second.x + second.width <= width, "{width} {:?}", theme.kind);
            }
        }
    }
}

/// A field and a button in an `items-end` row (mix C, `near` label
/// above the field): same height, same bottom, for regular fields with
/// regular buttons and compact fields with small ones, on Aqua11 and
/// Glass27, light and dark.
#[test]
fn mix_c_buttons_line_up_with_fields_in_an_items_end_row() {
    for generation in [AppKitGeneration::Aqua11, AppKitGeneration::Glass27] {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let theme = Theme::appkit_static(generation, appearance);
            assert_eq!(theme.mix, MixStyle::AppKit);
            let theme = theme.with_mix(MixStyle::C);
            for (density, size, height) in [
                (FieldDensity::Default, ControlSize::Regular, 36.0),
                (FieldDensity::Compact, ControlSize::Small, 32.0),
            ] {
                let mut tree: Element<()> = div().class("w-full h-full p-4").child(
                    div().class("flex items-end gap-2").children([
                        TextField::new("Campo", "x")
                            .test_id("campo")
                            .density(density)
                            .build(),
                        button("Enviar", ButtonVariant::Primary)
                            .control_size(size)
                            .test_id("submit"),
                    ]),
                );
                tree.assign_ids();
                let layout = Layout::compute(
                    &tree,
                    &LayoutInput {
                        viewport: Size {
                            width: 600.0,
                            height: 200.0,
                        },
                        theme: &theme,
                        interaction: &Interaction::default(),
                        scroll: &ScrollOffsets::new(),
                    },
                    &mut StyleCache::default(),
                    &mut FixedMeasure,
                );
                let screen = Screen::new(&layout);
                let field = screen.get_by_test_id("campo.shell").bounds();
                let button = screen.get_by_test_id("submit").bounds();
                let at = format!("{generation:?} {appearance:?} {size:?}");
                assert_eq!(field.height, height, "{at}");
                assert_eq!(button.height, height, "{at}");
                assert_eq!(button.y + button.height, field.y + field.height, "{at}");
            }
        }
    }
}
