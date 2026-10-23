//! Page templates on top of [`super::page`] (Fase C).
//!
//! Three layouts — master/detail, form (comfortable), data (compact) — plus
//! [`PageHeader`] / breadcrumbs in AppKit vocabulary. Window chrome stays
//! Sidebar + Toolbar; these are section content only.
//!
//! See [`docs/dev/22-page-templates.md`].

use crate::element::{Element, Role, div, icon, text};
use crate::icons::Icon;
use crate::theme::Density;

use super::page::{page_grid_item, tokens};
use super::{caption, themed};

/// Density intent for a page template (forms vs data surfaces).
///
/// Maps to [`Density`] when the app wants theme spacing to match the page.
/// Templates themselves only adjust gaps and control sizing hints in docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageDensity {
    /// Forms, settings, readable fields — prefer [`Density::Comfortable`].
    Comfortable,
    /// Tables, charts, dense lists — prefer [`Density::Compact`].
    Compact,
}

impl PageDensity {
    pub fn theme_density(self) -> Density {
        match self {
            Self::Comfortable => Density::Comfortable,
            Self::Compact => Density::Compact,
        }
    }

    /// Vertical rhythm between blocks inside the template body.
    ///
    /// Steps come from [`crate::theme::spacing`] (`FORM_STACK_STEPS` /
    /// `DATA_STACK_STEPS`) so the scale stays one source of truth.
    pub fn stack_gap_class(self) -> String {
        match self {
            Self::Comfortable => crate::theme::form_stack_gap_class(),
            Self::Compact => crate::theme::data_stack_gap_class(),
        }
    }
}

/// Typical form field span on the 12-col grid (half row).
pub const FORM_FIELD_SPAN: u8 = 6;
/// Full-width row on the 12-col grid.
pub const FORM_FULL_SPAN: u8 = 12;
/// Default master (list) column span; detail gets the remainder.
pub const MASTER_LIST_SPAN: u8 = 4;

/// AppKit-style page header: optional breadcrumbs, title, subtitle, actions.
pub struct PageHeader<Msg> {
    title: String,
    subtitle: Option<String>,
    breadcrumbs: Vec<String>,
    actions: Vec<Element<Msg>>,
    test_id: Option<String>,
}

impl<Msg> PageHeader<Msg> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            breadcrumbs: Vec::new(),
            actions: Vec::new(),
            test_id: None,
        }
    }

    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    /// Trail shown above the title (`Demo › Clientes`).
    pub fn breadcrumbs<I, S>(mut self, crumbs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.breadcrumbs = crumbs.into_iter().map(Into::into).collect();
        self
    }

    pub fn action(mut self, action: Element<Msg>) -> Self {
        self.actions.push(action);
        self
    }

    pub fn actions(mut self, actions: impl IntoIterator<Item = Element<Msg>>) -> Self {
        self.actions.extend(actions);
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg>
    where
        Msg: 'static,
    {
        let mut column: Vec<Element<Msg>> = Vec::new();
        if !self.breadcrumbs.is_empty() {
            column.push(breadcrumb_trail(&self.breadcrumbs).part("breadcrumbs"));
        }
        let title = text(self.title.clone())
            .class(&themed(
                "text-xl leading-tight font-semibold text-foreground",
                "text-title2 text-label",
            ))
            .role(Role::Heading)
            .part("title");
        let mut title_row = div()
            .class("flex flex-wrap items-start justify-between gap-3 w-full")
            .part("title_row")
            .child(title);
        if !self.actions.is_empty() {
            title_row = title_row.child(
                div()
                    .class("flex flex-wrap items-center gap-2 shrink-0")
                    .part("actions")
                    .children(self.actions),
            );
        }
        column.push(title_row);
        if let Some(subtitle) = self.subtitle {
            column.push(
                caption(subtitle)
                    .class(&themed(
                        "text-sm text-muted-foreground",
                        "text-subheadline text-secondary-label",
                    ))
                    .part("subtitle"),
            );
        }
        let mut root = div()
            .class("flex flex-col gap-2 w-full min-w-0")
            .children(column);
        if let Some(test_id) = self.test_id {
            root = root.test_id(test_id);
        }
        root
    }
}

/// Breadcrumb trail with chevrons (visual only; not a nav landmark).
pub fn breadcrumb_trail<Msg>(crumbs: &[String]) -> Element<Msg> {
    let mut parts: Vec<Element<Msg>> = Vec::new();
    for (index, crumb) in crumbs.iter().enumerate() {
        if index > 0 {
            parts.push(
                icon(Icon::ChevronRight)
                    .class(
                        "size-3.5 text-muted-foreground appkit:text-tertiary-label shrink-0",
                    )
                    .part(format!("sep.{index}")),
            );
        }
        let last = index + 1 == crumbs.len();
        parts.push(
            text(crumb.clone())
                .class(if last {
                    "text-sm font-medium text-foreground appkit:text-label"
                } else {
                    "text-sm text-muted-foreground appkit:text-secondary-label"
                })
                .part(if last {
                    "current".into()
                } else {
                    format!("crumb.{index}")
                }),
        );
    }
    div()
        .class("flex flex-wrap items-center gap-1 min-w-0")
        .children(parts)
}

/// Classes so a master pane stacks full-width below `md`, then takes `span`.
pub fn master_list_span_classes(span: u8) -> String {
    let span = span.clamp(1, tokens::COLS_MD - 1);
    format!("col-span-4 sm:col-span-8 md:col-span-{span} min-w-0")
}

/// Companion detail pane for [`master_list_span_classes`].
pub fn master_detail_span_classes(list_span: u8) -> String {
    let list_span = list_span.clamp(1, tokens::COLS_MD - 1);
    let detail = tokens::COLS_MD - list_span;
    format!("col-span-4 sm:col-span-8 md:col-span-{detail} min-w-0")
}

/// Lista + detalle (master/detail) over [`page_grid`].
pub struct MasterDetailPage<Msg> {
    header: Option<PageHeader<Msg>>,
    list: Option<Element<Msg>>,
    detail: Option<Element<Msg>>,
    list_span: u8,
    test_id: Option<String>,
}

impl<Msg> MasterDetailPage<Msg> {
    pub fn new(list: Element<Msg>, detail: Element<Msg>) -> Self {
        Self {
            header: None,
            list: Some(list),
            detail: Some(detail),
            list_span: MASTER_LIST_SPAN,
            test_id: None,
        }
    }

    pub fn header(mut self, header: PageHeader<Msg>) -> Self {
        self.header = Some(header);
        self
    }

    /// List column span on md+ (1–11); detail gets the rest. Default 4.
    pub fn list_span(mut self, span: u8) -> Self {
        self.list_span = span.clamp(1, tokens::COLS_MD - 1);
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn build(self) -> Element<Msg>
    where
        Msg: 'static,
    {
        let list = self.list.expect("list");
        let detail = self.detail.expect("detail");
        // Build the grid without `page_grid()`'s parts_root so list/detail
        // parts inherit this template's test_id.
        let panes = div()
            .class(super::page::PAGE_GRID_CLASS)
            .children([
                div()
                    .class(&master_list_span_classes(self.list_span))
                    .part("list")
                    .child(list),
                div()
                    .class(&master_detail_span_classes(self.list_span))
                    .part("detail")
                    .child(detail),
            ]);
        let mut children: Vec<Element<Msg>> = Vec::new();
        if let Some(header) = self.header {
            children.push(header.build());
        }
        children.push(panes);
        let mut root = div()
            .class("flex flex-col gap-4 w-full min-w-0")
            .parts_root()
            .children(children);
        if let Some(test_id) = self.test_id {
            root.test_id = Some(test_id);
        }
        root
    }
}

/// Formulario: [`PageDensity::Comfortable`] + PageGrid for fields.
pub struct FormPage<Msg> {
    header: Option<PageHeader<Msg>>,
    fields: Vec<Element<Msg>>,
    actions: Vec<Element<Msg>>,
    test_id: Option<String>,
}

impl<Msg> FormPage<Msg> {
    pub fn new() -> Self {
        Self {
            header: None,
            fields: Vec::new(),
            actions: Vec::new(),
            test_id: None,
        }
    }

    pub fn header(mut self, header: PageHeader<Msg>) -> Self {
        self.header = Some(header);
        self
    }

    /// Field cells (typically [`page_grid_item`] with span 6 or 12).
    pub fn fields(mut self, fields: impl IntoIterator<Item = Element<Msg>>) -> Self {
        self.fields.extend(fields);
        self
    }

    pub fn field(mut self, field: Element<Msg>) -> Self {
        self.fields.push(field);
        self
    }

    pub fn actions(mut self, actions: impl IntoIterator<Item = Element<Msg>>) -> Self {
        self.actions.extend(actions);
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn density(&self) -> PageDensity {
        PageDensity::Comfortable
    }

    pub fn build(self) -> Element<Msg>
    where
        Msg: 'static,
    {
        let density = PageDensity::Comfortable;
        let mut children: Vec<Element<Msg>> = Vec::new();
        if let Some(header) = self.header {
            children.push(header.build());
        }
        children.push(
            div()
                .class(super::page::PAGE_GRID_CLASS)
                .part("fields")
                .children(self.fields),
        );
        if !self.actions.is_empty() {
            children.push(
                div()
                    .class("flex flex-wrap items-center justify-end gap-2 w-full pt-1")
                    .part("actions")
                    .children(self.actions),
            );
        }
        let mut root = div()
            .class(&format!(
                "flex flex-col {} w-full min-w-0",
                density.stack_gap_class()
            ))
            .parts_root()
            .children(children);
        if let Some(test_id) = self.test_id {
            root.test_id = Some(test_id);
        }
        root
    }
}

impl<Msg> Default for FormPage<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

/// Vista de datos: [`PageDensity::Compact`] + dense body (table/charts).
pub struct DataPage<Msg> {
    header: Option<PageHeader<Msg>>,
    children: Vec<Element<Msg>>,
    test_id: Option<String>,
}

impl<Msg> DataPage<Msg> {
    pub fn new() -> Self {
        Self {
            header: None,
            children: Vec::new(),
            test_id: None,
        }
    }

    pub fn header(mut self, header: PageHeader<Msg>) -> Self {
        self.header = Some(header);
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

    pub fn density(&self) -> PageDensity {
        PageDensity::Compact
    }

    pub fn build(self) -> Element<Msg>
    where
        Msg: 'static,
    {
        let density = PageDensity::Compact;
        let mut column: Vec<Element<Msg>> = Vec::new();
        if let Some(header) = self.header {
            column.push(header.build());
        }
        column.extend(self.children);
        let mut root = div()
            .class(&format!(
                "flex flex-col {} w-full min-w-0",
                density.stack_gap_class()
            ))
            .parts_root()
            .children(column);
        if let Some(test_id) = self.test_id {
            root.test_id = Some(test_id);
        }
        root
    }
}

impl<Msg> Default for DataPage<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: half-width form field cell.
pub fn form_field_cell<Msg>(child: Element<Msg>) -> Element<Msg> {
    page_grid_item(FORM_FIELD_SPAN).child(child)
}

/// Convenience: full-width form field cell.
pub fn form_full_cell<Msg>(child: Element<Msg>) -> Element<Msg> {
    page_grid_item(FORM_FULL_SPAN).child(child)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::text;
    use crate::tw::{ClassList, State};
    use crate::theme::{Density, Theme};
    use super::super::page::col_span_classes;

    #[test]
    fn page_density_maps_to_theme() {
        assert_eq!(
            PageDensity::Comfortable.theme_density(),
            Density::Comfortable
        );
        assert_eq!(PageDensity::Compact.theme_density(), Density::Compact);
        assert_eq!(PageDensity::Comfortable.stack_gap_class(), "gap-5");
        assert_eq!(PageDensity::Compact.stack_gap_class(), "gap-3");
        assert_eq!(
            PageDensity::Comfortable.stack_gap_class(),
            crate::theme::form_stack_gap_class()
        );
        assert_eq!(
            PageDensity::Compact.stack_gap_class(),
            crate::theme::data_stack_gap_class()
        );
    }

    #[test]
    fn master_detail_spans_stack_below_md() {
        let list = master_list_span_classes(4);
        assert!(list.contains("col-span-4"), "{list}");
        assert!(list.contains("sm:col-span-8"), "{list}");
        assert!(list.contains("md:col-span-4"), "{list}");
        let detail = master_detail_span_classes(4);
        assert!(detail.contains("md:col-span-8"), "{detail}");
        let theme = Theme::kai(Density::Comfortable);
        let wide = ClassList::parse(&list, &theme).resolve(State {
            viewport_width: 900,
            ..State::default()
        });
        assert_eq!(wide.grid_column_span, 4);
        let narrow = ClassList::parse(&list, &theme).resolve(State {
            viewport_width: 400,
            ..State::default()
        });
        assert_eq!(narrow.grid_column_span, 4);
    }

    #[test]
    fn page_header_exposes_title_crumbs_and_actions() {
        let mut tree: Element<()> = PageHeader::new("Clientes")
            .subtitle("12 activos")
            .breadcrumbs(["Demo", "Clientes"])
            .action(text("Nuevo"))
            .test_id("hdr")
            .build();
        tree.assign_ids();
        assert_eq!(
            tree.find_test_id("hdr.title").map(|e| e.text_content()),
            Some("Clientes".into())
        );
        assert!(tree.find_test_id("hdr.breadcrumbs").is_some());
        assert!(tree.find_test_id("hdr.subtitle").is_some());
        assert!(tree.find_test_id("hdr.actions").is_some());
    }

    #[test]
    fn form_page_is_comfortable_and_grids_fields() {
        let page: Element<()> = FormPage::new()
            .header(PageHeader::new("Alta"))
            .field(form_field_cell(text("a")))
            .field(form_field_cell(text("b")))
            .test_id("form_tpl")
            .build();
        assert_eq!(FormPage::<()>::new().density(), PageDensity::Comfortable);
        let mut tree = page;
        tree.assign_ids();
        assert_eq!(tree.test_id.as_deref(), Some("form_tpl"));
        assert!(tree.class.contains("gap-5"), "{}", tree.class);
        assert!(tree.find_test_id("form_tpl.fields").is_some());
    }

    #[test]
    fn data_page_is_compact() {
        let mut tree: Element<()> = DataPage::new()
            .header(PageHeader::new("Métricas"))
            .child(text("tabla"))
            .test_id("data_tpl")
            .build();
        tree.assign_ids();
        assert_eq!(DataPage::<()>::new().density(), PageDensity::Compact);
        assert!(tree.class.contains("gap-3"), "{}", tree.class);
        assert!(tree.find_test_id("data_tpl.title").is_some());
    }

    #[test]
    fn master_detail_page_parts() {
        let mut tree: Element<()> = MasterDetailPage::new(text("lista"), text("detalle"))
            .header(PageHeader::new("Clientes"))
            .list_span(4)
            .test_id("md")
            .build();
        tree.assign_ids();
        assert!(tree.find_test_id("md.list").is_some());
        assert!(tree.find_test_id("md.detail").is_some());
        assert!(tree.find_test_id("md.title").is_some());
    }

    #[test]
    fn form_field_helpers_use_canonical_spans() {
        assert!(col_span_classes(FORM_FIELD_SPAN).contains("md:col-span-6"));
        assert!(col_span_classes(FORM_FULL_SPAN).contains("md:col-span-12"));
    }
}
