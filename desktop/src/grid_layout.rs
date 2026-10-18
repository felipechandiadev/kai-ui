//! Uniform CSS Grid helper over Taffy for Kai's logical desktop coordinate space.
//!
//! The widget tree (`layout::TreeLayout`) builds its own Taffy tree; this
//! helper and its tests pin the Grid behavior (`fr` tracks, gaps, auto
//! placement) that a future `Grid` widget will rely on.

use crate::geometry::Rect;
use taffy::prelude::{AvailableSpace, Dimension, Display, Size, Style, TaffyTree, fr, length};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridCell {
    pub row: u16,
    pub column: u16,
    pub bounds: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridLayoutError {
    InvalidGrid,
    EngineFailure,
}

/// Compute an explicitly sized, uniform CSS Grid in logical window units.
///
/// Grid items are auto-placed in row order and stretch to fill their `fr` tracks,
/// as ordinary CSS Grid items do.
pub fn compute_uniform_grid(
    width: f32,
    height: f32,
    columns: u16,
    rows: u16,
    gap_x: f32,
    gap_y: f32,
) -> Result<Vec<GridCell>, GridLayoutError> {
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || columns == 0
        || rows == 0
        || !gap_x.is_finite()
        || !gap_y.is_finite()
        || gap_x < 0.0
        || gap_y < 0.0
    {
        return Err(GridLayoutError::InvalidGrid);
    }

    let mut tree: TaffyTree<()> = TaffyTree::new();
    let cell_count = usize::from(columns) * usize::from(rows);
    let children = (0..cell_count)
        .map(|_| {
            tree.new_leaf(Style::default())
                .map_err(|_| GridLayoutError::EngineFailure)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let root_style = Style {
        display: Display::Grid,
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        grid_template_columns: vec![fr(1.0); usize::from(columns)],
        grid_template_rows: vec![fr(1.0); usize::from(rows)],
        gap: Size {
            width: length(gap_x),
            height: length(gap_y),
        },
        ..Style::default()
    };
    let root = tree
        .new_with_children(root_style, &children)
        .map_err(|_| GridLayoutError::EngineFailure)?;
    tree.compute_layout(
        root,
        Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        },
    )
    .map_err(|_| GridLayoutError::EngineFailure)?;

    children
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            let layout = tree
                .layout(node)
                .map_err(|_| GridLayoutError::EngineFailure)?;
            Ok(GridCell {
                row: (index / usize::from(columns)) as u16,
                column: (index % usize::from(columns)) as u16,
                bounds: Rect {
                    x: layout.location.x,
                    y: layout.location.y,
                    width: layout.size.width,
                    height: layout.size.height,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::geometry::Rect;

    use super::{GridLayoutError, compute_uniform_grid};

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
    }

    #[test]
    fn fr_tracks_fill_container_and_auto_place_in_row_order() {
        let cells = compute_uniform_grid(320.0, 180.0, 2, 2, 16.0, 20.0).unwrap();

        assert_eq!(cells.len(), 4);
        assert_eq!((cells[0].row, cells[0].column), (0, 0));
        assert_eq!((cells[1].row, cells[1].column), (0, 1));
        assert_eq!((cells[2].row, cells[2].column), (1, 0));
        assert_eq!((cells[3].row, cells[3].column), (1, 1));
        for cell in &cells {
            close(cell.bounds.width, 152.0);
            close(cell.bounds.height, 80.0);
        }
        assert_eq!(
            cells[0].bounds,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 152.0,
                height: 80.0
            }
        );
        close(cells[1].bounds.x, 168.0);
        close(cells[2].bounds.y, 100.0);
    }

    #[test]
    fn grid_reflows_to_new_available_dimensions_without_overlap() {
        let cells = compute_uniform_grid(500.0, 240.0, 3, 2, 10.0, 12.0).unwrap();

        for row in 0u16..2 {
            for column in 0u16..3 {
                let cell = cells[usize::from(row) * 3 + usize::from(column)].bounds;
                close(cell.width, 160.0);
                close(cell.height, 114.0);
                if column > 0 {
                    let previous = cells[usize::from(row) * 3 + usize::from(column - 1)].bounds;
                    assert!(previous.x + previous.width <= cell.x);
                }
                if row > 0 {
                    let above = cells[usize::from(row - 1) * 3 + usize::from(column)].bounds;
                    assert!(above.y + above.height <= cell.y);
                }
            }
        }
    }

    #[test]
    fn invalid_dimensions_tracks_and_gaps_are_rejected() {
        assert_eq!(
            compute_uniform_grid(0.0, 100.0, 1, 1, 0.0, 0.0),
            Err(GridLayoutError::InvalidGrid)
        );
        assert_eq!(
            compute_uniform_grid(100.0, 100.0, 0, 1, 0.0, 0.0),
            Err(GridLayoutError::InvalidGrid)
        );
        assert_eq!(
            compute_uniform_grid(100.0, 100.0, 1, 1, -1.0, 0.0),
            Err(GridLayoutError::InvalidGrid)
        );
    }
}
