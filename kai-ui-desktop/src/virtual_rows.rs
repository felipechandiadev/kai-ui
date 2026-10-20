//! Building the rows of [`Element::virtual_rows`] lists: only the ones in
//! view plus [`OVERSCAN`] on each side, between spacers as tall as the
//! rows left out, so the list scrolls as if every row were there.

use std::ops::Range;

use crate::element::{Element, VirtualRows, WidgetId};

/// Rows built beyond each edge of the visible ones.
pub const OVERSCAN: usize = 6;

/// Rows built while the height of the list is unknown (before its first
/// layout, or in a capture without a session).
pub const INITIAL_ROWS: usize = 30;

/// What the session knows of a list when it builds its rows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    /// Scroll offset of the list.
    pub offset: f32,
    /// Height of its padding box; `None` before its first layout.
    pub height: Option<f32>,
    /// A row to build wherever it is (the focused one, or one the keys
    /// are moving to).
    pub pinned: Option<usize>,
}

/// Rows of a list of `count` rows of `row_height` to build for `viewport`.
pub fn visible_range(count: usize, row_height: f32, viewport: Viewport) -> Range<usize> {
    let first = (viewport.offset.max(0.0) / row_height).floor() as usize;
    let visible = viewport.height.map_or(INITIAL_ROWS, |height| {
        (height / row_height).ceil() as usize + 1
    });
    let start = first.saturating_sub(OVERSCAN).min(count);
    let end = (first + visible + OVERSCAN).min(count);
    start..end
}

/// Replace the children of every virtual list in `tree` with its rows for
/// `viewport(list id)`. Returns the range built for each list, so the
/// session can tell when a new layout needs other rows. Ids must be
/// assigned before (lists are found by id) and again after.
pub fn materialize<Msg>(
    tree: &mut Element<Msg>,
    viewport: &impl Fn(WidgetId) -> Viewport,
) -> Vec<(WidgetId, Range<usize>)> {
    let mut built = Vec::new();
    visit(tree, viewport, &mut built);
    built
}

fn visit<Msg>(
    element: &mut Element<Msg>,
    viewport: &impl Fn(WidgetId) -> Viewport,
    built: &mut Vec<(WidgetId, Range<usize>)>,
) {
    if let Some(rows) = &element.virtual_rows {
        let viewport = viewport(element.id);
        let range = visible_range(rows.count, rows.row_height, viewport);
        element.children = children(rows, range.clone(), viewport.pinned);
        built.push((element.id, range));
        return;
    }
    for child in &mut element.children {
        visit(child, viewport, built);
    }
}

fn children<Msg>(
    rows: &VirtualRows<Msg>,
    range: Range<usize>,
    pinned: Option<usize>,
) -> Vec<Element<Msg>> {
    let mut indices: Vec<usize> = range.collect();
    for extra in [pinned, rows.selected].into_iter().flatten() {
        if extra < rows.count && !indices.contains(&extra) {
            indices.push(extra);
        }
    }
    indices.sort_unstable();
    let height = rows.row_height;
    let spacer = |from: usize, to: usize| {
        crate::element::div()
            .class(&format!("shrink-0 h-[{}px]", (to - from) as f32 * height))
            .key(format!("kai.hueco.{from}"))
    };
    let mut children = Vec::with_capacity(indices.len() + 2);
    let mut next = 0;
    for index in indices {
        if index > next {
            children.push(spacer(next, index));
        }
        children.push(
            (rows.render)(index)
                .class(&format!("shrink-0 h-[{height}px]"))
                .key(index.to_string())
                .row_index(index),
        );
        next = index + 1;
    }
    if rows.count > next {
        children.push(spacer(next, rows.count));
    }
    children
}

/// The virtual list on `path` (root first) and the index of its row on
/// that path: where a focused element sits.
pub fn row_on_path<Msg>(
    tree: &Element<Msg>,
    path: &[WidgetId],
) -> Option<(WidgetId, usize, usize, f32)> {
    let mut found = None;
    let mut element = tree;
    for (depth, id) in path.iter().enumerate() {
        if depth > 0 {
            element = element.children.iter().find(|child| child.id == *id)?;
        }
        if let Some(rows) = &element.virtual_rows {
            let row = path
                .get(depth + 1)
                .and_then(|row| element.children.iter().find(|child| child.id == *row))
                .and_then(|row| row.row_index);
            if let Some(index) = row {
                found = Some((element.id, index, rows.count, rows.row_height));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{INITIAL_ROWS, OVERSCAN, Viewport, materialize, visible_range};
    use crate::element::{Element, Kind, div, text};

    fn list(count: usize) -> Element<()> {
        let mut tree = div().child(div().class("h-[100px] overflow-y-auto").virtual_rows(
            count,
            20.0,
            None,
            Rc::new(|index| text(format!("{index}"))),
        ));
        tree.assign_ids();
        tree
    }

    fn row_indices(list: &Element<()>) -> Vec<usize> {
        list.children
            .iter()
            .filter_map(|child| child.row_index)
            .collect()
    }

    #[test]
    fn only_the_rows_in_view_and_a_margin_are_built() {
        assert_eq!(
            visible_range(10_000, 20.0, Viewport::default()),
            0..INITIAL_ROWS + OVERSCAN
        );
        let viewport = Viewport {
            offset: 2_000.0,
            height: Some(100.0),
            pinned: None,
        };
        assert_eq!(
            visible_range(10_000, 20.0, viewport),
            100 - OVERSCAN..100 + 6 + OVERSCAN
        );
        assert_eq!(visible_range(3, 20.0, viewport), 3..3);
    }

    #[test]
    fn spacers_stand_for_the_rows_left_out_and_pins_stay() {
        let mut tree = list(1_000);
        let built = materialize(&mut tree, &|_| Viewport {
            offset: 2_000.0,
            height: Some(100.0),
            pinned: Some(3),
        });
        tree.assign_ids();
        let list = &tree.children[0];
        assert_eq!(built, vec![(list.id, 94..112)]);
        let rows = row_indices(list);
        assert_eq!(rows[0], 3);
        assert_eq!(rows[1..], (94..112).collect::<Vec<_>>());
        let classes: Vec<&str> = list
            .children
            .iter()
            .filter(|child| child.row_index.is_none())
            .map(|child| child.class.as_str())
            .collect();
        assert_eq!(
            classes,
            [
                "shrink-0 h-[60px]",
                "shrink-0 h-[1800px]",
                "shrink-0 h-[17760px]"
            ]
        );
        let row = list
            .children
            .iter()
            .find(|child| child.row_index == Some(100));
        assert_eq!(row.unwrap().kind, Kind::Text("100".into()));
        assert_eq!(list.row_count, Some(1_000));
    }

    #[test]
    fn a_row_keeps_its_id_wherever_the_window_is() {
        let id_of = |offset: f32| {
            let mut tree = list(1_000);
            materialize(&mut tree, &|_| Viewport {
                offset,
                height: Some(100.0),
                pinned: None,
            });
            tree.assign_ids();
            tree.children[0]
                .children
                .iter()
                .find(|child| child.row_index == Some(100))
                .map(|row| row.id)
        };
        assert_eq!(id_of(2_000.0), id_of(1_950.0));
    }
}
