//! Deterministic logical layout for the current desktop prototype.
//!
//! This is the first L4 slice: it computes the demo's boxes from a logical
//! viewport. It deliberately recalculates the complete result and has no cache.

use crate::geometry::{Point, Rect, Size};
use crate::grid_layout::{GridItemSpec, compute_explicit_row_grid};

const OUTER_MARGIN: f32 = 40.0;
const CONTENT_INSET: f32 = 8.0;
const MIN_PANEL_SIZE: Size = Size {
    width: 340.0,
    height: 260.0,
};
const PANEL_REFERENCE_SIZE: Size = Size {
    width: 540.0,
    height: 330.0,
};
const BUTTON_SIZE: Size = Size {
    width: 220.0,
    height: 48.0,
};
const INPUT_SIZE: Size = Size {
    width: 480.0,
    height: 48.0,
};
const REFERENCE_GAPS: [f32; 4] = [16.0, 32.0, 24.0, 46.0];
const REFERENCE_BOTTOM_INSET: f32 = 22.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBlockHeights {
    pub heading: f32,
    pub description: f32,
    pub status: f32,
}

impl Default for TextBlockHeights {
    fn default() -> Self {
        Self {
            heading: 36.0,
            description: 22.0,
            status: 20.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowRootLayout {
    pub bounds: Rect,
    pub main_content: MainContentLayout,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MainContentLayout {
    pub bounds: Rect,
    pub scroll: ScrollViewportLayout,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollViewportLayout {
    pub bounds: Rect,
    pub content_size: Size,
    pub scroll_range: f32,
    pub children: ContentChildrenLayout,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContentChildrenLayout {
    pub heading_box: Rect,
    pub description_box: Rect,
    pub button: Rect,
    pub button_label_box: Rect,
    pub status_box: Rect,
    pub input: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DemoLayout {
    pub root: WindowRootLayout,
    pub heading_origin: Point,
    pub description_origin: Point,
    pub button_label_origin: Point,
    pub status_origin: Point,
    pub input_text_origin: Point,
    pub input_caret_origin: Point,
}

impl DemoLayout {
    /// Smallest logical window supported by the prototype. Content may still
    /// overflow this viewport and be reached through vertical scrolling.
    pub const MINIMUM_VIEWPORT: Size = Size {
        width: OUTER_MARGIN * 2.0 + MIN_PANEL_SIZE.width,
        height: OUTER_MARGIN * 2.0 + MIN_PANEL_SIZE.height,
    };

    pub fn content_width_for_viewport(viewport: Size) -> f32 {
        let panel_width = PANEL_REFERENCE_SIZE
            .width
            .min((viewport.width - OUTER_MARGIN * 2.0).max(0.0));
        (panel_width - CONTENT_INSET * 2.0).max(0.0)
    }

    /// Compute all prototype bounds in logical window units.
    ///
    /// The layout is anchored at the original 40pt margin, preserving the
    /// 1024x720 prototype appearance. Panel and control widths shrink when the
    /// available viewport is narrower than the reference size.
    pub fn compute(viewport: Size) -> Self {
        Self::compute_with_text_heights(viewport, TextBlockHeights::default())
    }

    /// Compute a content-driven vertical flow. Wrapped text increases its box
    /// height and moves following children down; short viewports expose a
    /// scroll range instead of shrinking controls or vertical gaps.
    pub fn compute_with_text_heights(viewport: Size, text: TextBlockHeights) -> Self {
        let available_width = (viewport.width - OUTER_MARGIN * 2.0).max(0.0);
        let available_height = (viewport.height - OUTER_MARGIN * 2.0).max(0.0);
        let text = TextBlockHeights {
            heading: text.heading.max(0.0),
            description: text.description.max(0.0),
            status: text.status.max(0.0),
        };
        let panel_width = PANEL_REFERENCE_SIZE.width.min(available_width);
        let content_width = (panel_width - CONTENT_INSET * 2.0).max(0.0);
        let row_heights = [
            text.heading,
            REFERENCE_GAPS[0],
            text.description,
            REFERENCE_GAPS[1],
            BUTTON_SIZE.height,
            REFERENCE_GAPS[2],
            text.status,
            REFERENCE_GAPS[3],
            INPUT_SIZE.height,
            REFERENCE_BOTTOM_INSET,
        ];
        let grid = compute_explicit_row_grid(
            content_width,
            &row_heights,
            &[
                GridItemSpec {
                    row: 0,
                    preferred_width: content_width,
                },
                GridItemSpec {
                    row: 2,
                    preferred_width: content_width,
                },
                GridItemSpec {
                    row: 4,
                    preferred_width: BUTTON_SIZE.width,
                },
                GridItemSpec {
                    row: 6,
                    preferred_width: content_width,
                },
                GridItemSpec {
                    row: 8,
                    preferred_width: INPUT_SIZE.width,
                },
            ],
        )
        .expect("demo grid dimensions and tracks are valid");
        let content_height = grid.content_size.height;
        let preferred_panel_height =
            (CONTENT_INSET * 2.0 + content_height).max(MIN_PANEL_SIZE.height);
        let panel = Rect {
            x: OUTER_MARGIN,
            y: OUTER_MARGIN,
            width: panel_width,
            height: preferred_panel_height.min(available_height),
        };
        let content_x = panel.x + CONTENT_INSET;
        let content_viewport = Rect {
            x: content_x,
            y: panel.y + CONTENT_INSET,
            width: content_width,
            height: (panel.height - CONTENT_INSET * 2.0).max(0.0),
        };

        let translate_content = |rect: Rect| {
            rect.translated(Point {
                x: content_x,
                y: content_viewport.y,
            })
        };
        let heading_box = translate_content(grid.items[0]);
        let description_box = translate_content(grid.items[1]);
        let button = translate_content(grid.items[2]);
        let status_box = translate_content(grid.items[3]);
        let input = translate_content(grid.items[4]);
        let button_label_box = Rect {
            x: button.x + 20.0,
            y: button.y + 14.0,
            width: (button.width - 40.0).max(1.0),
            height: 20.0,
        };
        let scroll_range = (content_height - content_viewport.height).max(0.0);

        Self {
            root: WindowRootLayout {
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: viewport.width,
                    height: viewport.height,
                },
                main_content: MainContentLayout {
                    bounds: panel,
                    scroll: ScrollViewportLayout {
                        bounds: content_viewport,
                        content_size: Size {
                            width: content_width,
                            height: content_height,
                        },
                        scroll_range,
                        children: ContentChildrenLayout {
                            heading_box,
                            description_box,
                            button,
                            button_label_box,
                            status_box,
                            input,
                        },
                    },
                },
            },
            heading_origin: Point {
                x: heading_box.x,
                y: heading_box.y,
            },
            description_origin: Point {
                x: description_box.x,
                y: description_box.y,
            },
            button_label_origin: Point {
                x: button.x + 20.0,
                y: button.y + 14.0,
            },
            status_origin: Point {
                x: status_box.x,
                y: status_box.y,
            },
            input_text_origin: Point {
                x: input.x + 12.0,
                y: input.y + 13.0,
            },
            input_caret_origin: Point {
                x: input.x + 12.0,
                y: input.y + 10.0,
            },
        }
    }

    pub fn input_content_width(self) -> f32 {
        (self.root.main_content.scroll.children.input.width - 24.0).max(1.0)
    }

    pub fn window_size(self) -> Size {
        Size {
            width: self.root.bounds.width,
            height: self.root.bounds.height,
        }
    }

    pub fn panel(self) -> Rect {
        self.root.main_content.bounds
    }

    pub fn content_viewport(self) -> Rect {
        self.root.main_content.scroll.bounds
    }

    pub fn content_height(self) -> f32 {
        self.root.main_content.scroll.content_size.height
    }

    pub fn scroll_range(self) -> f32 {
        self.root.main_content.scroll.scroll_range
    }

    pub fn heading_box(self) -> Rect {
        self.root.main_content.scroll.children.heading_box
    }

    pub fn description_box(self) -> Rect {
        self.root.main_content.scroll.children.description_box
    }

    pub fn button(self) -> Rect {
        self.root.main_content.scroll.children.button
    }

    pub fn button_label_box(self) -> Rect {
        self.root.main_content.scroll.children.button_label_box
    }

    pub fn status_box(self) -> Rect {
        self.root.main_content.scroll.children.status_box
    }

    pub fn input(self) -> Rect {
        self.root.main_content.scroll.children.input
    }

    pub fn input_local_position(self, point: Point) -> Point {
        Point {
            x: point.x - self.input_caret_origin.x,
            y: point.y - self.input_caret_origin.y,
        }
    }

    pub fn viewport_to_content(self, point: Point, scroll_y: f32) -> Option<Point> {
        self.content_viewport().contains(point).then_some(Point {
            x: point.x,
            y: point.y + scroll_y,
        })
    }

    pub fn content_to_viewport(self, rect: Rect, scroll_y: f32) -> Option<Rect> {
        rect.translated(Point {
            x: 0.0,
            y: -scroll_y,
        })
        .intersection(self.content_viewport())
    }

    /// Update a logical vertical scroll offset. Positive `delta_y` means the
    /// user scrolls toward the content's start (wheel-up).
    pub fn scroll_by(self, current: f32, delta_y: f32) -> f32 {
        (current - delta_y).clamp(0.0, self.scroll_range())
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::{Point, Rect, Size};

    use super::DemoLayout;

    #[test]
    fn reference_viewport_preserves_prototype_layout() {
        let layout = DemoLayout::compute(Size {
            width: 1024.0,
            height: 720.0,
        });

        assert_eq!(
            layout.panel(),
            Rect {
                x: 40.0,
                y: 40.0,
                width: 540.0,
                height: 330.0,
            }
        );
        assert_eq!(
            layout.button(),
            Rect {
                x: 48.0,
                y: 154.0,
                width: 220.0,
                height: 48.0,
            }
        );
        assert_eq!(
            layout.input(),
            Rect {
                x: 48.0,
                y: 292.0,
                width: 480.0,
                height: 48.0,
            }
        );
        assert_eq!(layout.heading_origin, Point { x: 48.0, y: 48.0 });
        assert_eq!(layout.description_origin, Point { x: 48.0, y: 100.0 });
        assert_eq!(layout.status_origin, Point { x: 48.0, y: 226.0 });
        assert_eq!(layout.input_content_width(), 456.0);
    }

    #[test]
    fn narrower_viewport_shrinks_panel_and_control_widths_deterministically() {
        let layout = DemoLayout::compute(Size {
            width: 400.0,
            height: 600.0,
        });

        assert_eq!(layout.panel().width, 320.0);
        assert_eq!(layout.button().width, 220.0);
        assert_eq!(layout.input().width, 304.0);
        assert!(layout.panel().x + layout.panel().width <= layout.window_size().width);
        assert!(
            layout.button().x + layout.button().width <= layout.panel().x + layout.panel().width
        );
        assert!(layout.input().x + layout.input().width <= layout.panel().x + layout.panel().width);
    }

    #[test]
    fn resized_viewport_updates_full_layout_and_input_local_coordinates() {
        let layout = DemoLayout::compute(Size {
            width: 800.0,
            height: 600.0,
        });

        assert_eq!(layout.panel().width, 540.0);
        assert_eq!(layout.panel().height, 330.0);
        assert_eq!(
            layout.input_local_position(Point { x: 70.0, y: 315.0 }),
            Point { x: 10.0, y: 13.0 }
        );
    }

    #[test]
    fn minimum_viewport_preserves_component_sizes_and_exposes_scroll_range() {
        let layout = DemoLayout::compute(DemoLayout::MINIMUM_VIEWPORT);

        assert_eq!(layout.button().height, super::BUTTON_SIZE.height);
        assert_eq!(layout.input().height, super::INPUT_SIZE.height);
        assert_eq!(
            layout.description_box().y,
            layout.heading_box().y + layout.heading_box().height + super::REFERENCE_GAPS[0]
        );
        assert_eq!(
            layout.button().y,
            layout.description_box().y + layout.description_box().height + super::REFERENCE_GAPS[1]
        );
        assert!(layout.scroll_range() > 0.0);
        assert!(
            layout.input().x + layout.input().width
                <= layout.panel().x + layout.panel().width - 8.0
        );
    }

    #[test]
    fn shorter_viewport_scrolls_instead_of_compressing_controls_or_gaps() {
        let layout = DemoLayout::compute(Size {
            width: 800.0,
            height: 350.0,
        });

        assert!(layout.panel().height < super::PANEL_REFERENCE_SIZE.height);
        assert_eq!(layout.description_box().y, 100.0);
        assert_eq!(layout.button().y, 154.0);
        assert_eq!(layout.button().height, super::BUTTON_SIZE.height);
        assert_eq!(layout.input().height, super::INPUT_SIZE.height);
        assert!(layout.scroll_range() > 0.0);
    }

    #[test]
    fn wrapped_description_pushes_following_children_down_by_added_height() {
        let viewport = Size {
            width: 1024.0,
            height: 720.0,
        };
        let normal = DemoLayout::compute(viewport);
        let wrapped = DemoLayout::compute_with_text_heights(
            viewport,
            super::TextBlockHeights {
                description: 44.0,
                ..super::TextBlockHeights::default()
            },
        );

        assert_eq!(wrapped.description_box().height, 44.0);
        assert_eq!(wrapped.button().y, normal.button().y + 22.0);
        assert_eq!(wrapped.status_box().y, normal.status_box().y + 22.0);
        assert_eq!(wrapped.input().y, normal.input().y + 22.0);
        assert_eq!(wrapped.panel().height, normal.panel().height + 22.0);
        assert_eq!(wrapped.scroll_range(), 0.0);
    }

    #[test]
    fn scrolling_maps_viewport_points_back_into_content_coordinates() {
        let layout = DemoLayout::compute(DemoLayout::MINIMUM_VIEWPORT);
        let viewport_point = Point {
            x: layout.content_viewport().x + 12.0,
            y: layout.content_viewport().y + 10.0,
        };
        let content_point = layout.viewport_to_content(viewport_point, 40.0).unwrap();

        assert_eq!(content_point.y, viewport_point.y + 40.0);
        assert!(
            layout
                .viewport_to_content(Point { x: 1.0, y: 1.0 }, 40.0)
                .is_none()
        );
    }

    #[test]
    fn scroll_offset_is_clamped_and_wheel_direction_is_consistent() {
        let layout = DemoLayout::compute(DemoLayout::MINIMUM_VIEWPORT);
        assert!(layout.scroll_range() > 0.0);
        assert_eq!(layout.scroll_by(0.0, 30.0), 0.0);
        assert_eq!(layout.scroll_by(0.0, -30.0), 30.0);
        assert_eq!(
            layout.scroll_by(layout.scroll_range(), -30.0),
            layout.scroll_range()
        );
    }

    #[test]
    fn window_root_contains_main_content_and_scroll_viewport_in_nested_bounds() {
        let layout = DemoLayout::compute(Size {
            width: 1024.0,
            height: 720.0,
        });
        let root = layout.root;
        let main = root.main_content;
        let scroll = main.scroll;

        assert_eq!(
            root.bounds,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 1024.0,
                height: 720.0
            }
        );
        assert!(root.bounds.contains(Point {
            x: main.bounds.x,
            y: main.bounds.y
        }));
        assert!(main.bounds.contains(Point {
            x: scroll.bounds.x,
            y: scroll.bounds.y
        }));
        assert!(scroll.bounds.contains(Point {
            x: scroll.children.button.x,
            y: scroll.children.button.y,
        }));
        assert!(scroll.content_size.height >= scroll.bounds.height);
        assert_eq!(
            layout.window_size(),
            Size {
                width: 1024.0,
                height: 720.0
            }
        );
    }
}
