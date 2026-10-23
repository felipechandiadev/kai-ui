//! Page shell and 12-column [`PageGrid`] (AppKit-canonical; not Carbon 16).
//!
//! Hierarchy: [`page`] → [`page_body`] → [`page_grid`] / [`page_grid_item`].
//! Window chrome stays Sidebar + Toolbar AppKit; this module only lays out
//! the section content beside that chrome.
//!
//! See [`docs/dev/21-page-layout.md`].

use crate::element::{Element, div};

/// Layout tokens in points. Prefer the utility classes these map to
/// (`gap-4`, `px-4` / `sm:px-6` / `md:px-8`, `max-w-6xl`).
pub mod tokens {
    /// Grid gutter (`gap-4` with Kai `spacing_unit` 4 → 16 pt).
    pub const GUTTER_PT: f32 = 16.0;
    /// Horizontal page margin below `sm` (`px-4`).
    pub const MARGIN_BASE_PT: f32 = 16.0;
    /// Horizontal page margin from `sm` (`sm:px-6`).
    pub const MARGIN_SM_PT: f32 = 24.0;
    /// Horizontal page margin from `md` (`md:px-8`).
    pub const MARGIN_MD_PT: f32 = 32.0;
    /// Optional content max-width (`max-w-6xl` = 72 rem ≈ 1152 pt at 16).
    pub const MAX_CONTENT_PT: f32 = 1152.0;
    /// Columns below `sm`.
    pub const COLS_BASE: u8 = 4;
    /// Columns from `sm` (≥ 640 pt).
    pub const COLS_SM: u8 = 8;
    /// Columns from `md` (≥ 768 pt) — the canonical 12-col grid.
    pub const COLS_MD: u8 = 12;
}

/// Classes for [`page`]: scroll shell for section content (not the window).
/// Margins and the column live on [`PAGE_BODY_CLASS`] (two nodes, as before Fase B).
pub const PAGE_CLASS: &str = "flex-1 min-h-0 min-w-0 overflow-y-auto";

/// Classes for [`page_body`]: scrollable area with responsive margins.
pub const PAGE_BODY_CLASS: &str = "w-full flex flex-col items-center gap-6 \
     px-4 sm:px-6 md:px-8 py-6";

/// Classes for [`page_body`] when `max_content` is on (inner width cap).
pub const PAGE_BODY_MAX_CONTENT_CLASS: &str = "w-full max-w-6xl flex flex-col items-center gap-6";

/// Classes for [`page_grid`]: 4 / 8 / 12 columns + 16 pt gutter.
pub const PAGE_GRID_CLASS: &str = "grid grid-cols-4 sm:grid-cols-8 md:grid-cols-12 gap-4 w-full";

/// Map a canonical 12-column span onto a breakpoint column count.
///
/// Half-or-wider spans (`≥ 6` on the 12-col grid) stack full-width below
/// `sm` (4 cols) and take half of the 8-col `sm` grid — same stacking
/// behaviour as the former `grid-cols-1 sm:grid-cols-2` form rows.
pub fn span_at(span12: u8, cols: u8) -> u8 {
    let span12 = span12.clamp(1, tokens::COLS_MD);
    let cols = cols.max(1);
    if cols < tokens::COLS_MD && span12 * 2 >= tokens::COLS_MD {
        // Full-row span stays full width at every breakpoint.
        if span12 >= tokens::COLS_MD {
            return cols;
        }
        return if cols <= tokens::COLS_BASE {
            cols // half+ stacks full-width below sm
        } else {
            cols / 2 // half of sm (8 → 4)
        };
    }
    let n = (u16::from(span12) * u16::from(cols)).div_ceil(u16::from(tokens::COLS_MD));
    (n as u8).clamp(1, cols)
}

/// Responsive `col-span-*` classes for a canonical 12-col span.
pub fn col_span_classes(span12: u8) -> String {
    let span12 = span12.clamp(1, tokens::COLS_MD);
    format!(
        "col-span-{} sm:col-span-{} md:col-span-{} min-w-0",
        span_at(span12, tokens::COLS_BASE),
        span_at(span12, tokens::COLS_SM),
        span12
    )
}

/// Section content container beside Sidebar/Toolbar (not the window shell).
pub fn page<Msg>() -> Element<Msg> {
    // No Role::Region: section demos already use Region (e.g. table row
    // detail); the scroll shell stays anonymous like the pre–Fase B div.
    div().class(PAGE_CLASS).parts_root()
}

/// Scrollable page body with margins that grow at `sm` / `md`.
pub fn page_body<Msg>() -> Element<Msg> {
    div().class(PAGE_BODY_CLASS).parts_root()
}

/// [`page_body`] with an optional inner max-width (`max-w-6xl`).
pub struct PageBody<Msg> {
    max_content: bool,
    class: String,
    children: Vec<Element<Msg>>,
    test_id: Option<String>,
    key: Option<String>,
}

impl<Msg> Default for PageBody<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg> PageBody<Msg> {
    pub fn new() -> Self {
        Self {
            max_content: false,
            class: String::new(),
            children: Vec::new(),
            test_id: None,
            key: None,
        }
    }

    /// Cap the content column at [`tokens::MAX_CONTENT_PT`] (`max-w-6xl`).
    pub fn max_content(mut self, on: bool) -> Self {
        self.max_content = on;
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn child(mut self, child: Element<Msg>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Element<Msg>>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let inner_children = if self.max_content {
            vec![
                div()
                    .class(PAGE_BODY_MAX_CONTENT_CLASS)
                    .children(self.children),
            ]
        } else {
            self.children
        };
        let mut root = page_body().children(inner_children);
        if !self.class.is_empty() {
            root = root.class(&self.class);
        }
        if let Some(test_id) = self.test_id {
            root = root.test_id(test_id);
        }
        if let Some(key) = self.key {
            root = root.key(key);
        }
        root
    }
}

/// CSS grid: 4 cols (base) / 8 (`sm`) / 12 (`md+`) with 16 pt gutter.
pub fn page_grid<Msg>() -> Element<Msg> {
    div().class(PAGE_GRID_CLASS).parts_root()
}

/// One cell of [`page_grid`]; `span` is the canonical 12-col span (1–12).
pub struct PageGridItem<Msg> {
    span: u8,
    class: String,
    children: Vec<Element<Msg>>,
    test_id: Option<String>,
}

impl<Msg> PageGridItem<Msg> {
    /// `span` on the md+ 12-column grid (clamped to 1–12).
    pub fn new(span: u8) -> Self {
        Self {
            span: span.clamp(1, tokens::COLS_MD),
            class: String::new(),
            children: Vec::new(),
            test_id: None,
        }
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn child(mut self, child: Element<Msg>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Element<Msg>>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg> {
        let mut root = div()
            .class(&col_span_classes(self.span))
            .parts_root()
            .children(self.children);
        if !self.class.is_empty() {
            root = root.class(&self.class);
        }
        if let Some(test_id) = self.test_id {
            root.test_id = Some(test_id);
        }
        root
    }
}

/// Shorthand for [`PageGridItem::new`]`(span).build()` with one child.
pub fn page_grid_item<Msg>(span: u8) -> Element<Msg> {
    PageGridItem::new(span).build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::text;
    use crate::style::Display;
    use crate::theme::{Density, Theme};
    use crate::tw::{ClassList, State};

    #[test]
    fn span_at_maps_proportionally() {
        assert_eq!(span_at(6, 4), 4, "half stacks full-width below sm");
        assert_eq!(span_at(6, 8), 4, "half of sm grid");
        assert_eq!(span_at(6, 12), 6);
        assert_eq!(span_at(12, 4), 4);
        assert_eq!(span_at(12, 8), 8);
        assert_eq!(span_at(1, 4), 1);
        assert_eq!(span_at(0, 12), 1, "zero clamps to 1");
        assert_eq!(span_at(99, 12), 12, "oversize clamps to 12");
    }

    #[test]
    fn col_span_classes_are_responsive() {
        let classes = col_span_classes(6);
        assert!(classes.contains("col-span-4"), "{classes}");
        assert!(classes.contains("sm:col-span-4"), "{classes}");
        assert!(classes.contains("md:col-span-6"), "{classes}");
    }

    #[test]
    fn page_grid_declares_twelve_columns_at_md() {
        let theme = Theme::kai(Density::Comfortable);
        let list = ClassList::parse(PAGE_GRID_CLASS, &theme);
        let narrow = list.resolve(State {
            viewport_width: 400,
            ..State::default()
        });
        assert_eq!(narrow.display, Display::Grid);
        assert_eq!(narrow.grid_columns, 4);
        assert_eq!(narrow.gap_x, tokens::GUTTER_PT);
        let mid = list.resolve(State {
            viewport_width: 700,
            ..State::default()
        });
        assert_eq!(mid.grid_columns, 8);
        let wide = list.resolve(State {
            viewport_width: 900,
            ..State::default()
        });
        assert_eq!(wide.grid_columns, 12);
    }

    #[test]
    fn page_grid_item_span_builds_col_span_classes() {
        let mut tree: Element<()> = PageGridItem::new(6)
            .test_id("cell")
            .child(text("hola"))
            .build();
        tree.assign_ids();
        assert_eq!(tree.test_id.as_deref(), Some("cell"));
        let theme = Theme::kai(Density::Comfortable);
        let list = ClassList::parse(&tree.class, &theme);
        let wide = list.resolve(State {
            viewport_width: 900,
            ..State::default()
        });
        assert_eq!(wide.grid_column_span, 6);
        let narrow = list.resolve(State {
            viewport_width: 400,
            ..State::default()
        });
        assert_eq!(narrow.grid_column_span, 4);
    }

    #[test]
    fn page_body_max_content_wraps_inner() {
        let mut tree: Element<()> = PageBody::new()
            .max_content(true)
            .test_id("body")
            .child(text("x"))
            .build();
        tree.assign_ids();
        assert_eq!(tree.test_id.as_deref(), Some("body"));
        assert!(tree.class.contains("px-4"), "{}", tree.class);
        assert_eq!(tree.children.len(), 1);
        assert!(
            tree.children[0].class.contains("max-w-6xl"),
            "{}",
            tree.children[0].class
        );
    }

    #[test]
    fn page_is_the_scroll_shell() {
        let tree = page::<()>();
        assert_eq!(tree.role, None);
        assert!(tree.class.contains("flex-1"), "{}", tree.class);
        assert!(tree.class.contains("overflow-y-auto"), "{}", tree.class);
    }
}
