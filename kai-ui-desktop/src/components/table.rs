//! Long lists and tables (`NSTableView`, 12 Paso 20.8): rows are built
//! only while they are in view ([`Element::virtual_rows`]), so ten
//! thousand rows cost what a screenful does.

use std::rc::Rc;

use super::themed;
use crate::element::{Element, Role, div, icon, text};
use crate::icons::Icon;

/// Scrolling list of `count` rows of `row_height` points, built by
/// `render_row(index)` as they scroll into view. Rows without a role are
/// [`Role::ListItem`]s. Give it a height with `.class(..)`; add
/// [`Element::selecting_focus_group`] to make it one Tab stop whose
/// arrows choose.
pub fn virtual_list<Msg: 'static>(
    count: usize,
    row_height: f32,
    render_row: Rc<dyn Fn(usize) -> Element<Msg>>,
) -> Element<Msg> {
    let render: Rc<dyn Fn(usize) -> Element<Msg>> = Rc::new(move |index| {
        let row = render_row(index);
        if row.role.is_none() {
            row.role(Role::ListItem)
        } else {
            row
        }
    });
    div()
        .class("flex flex-col min-h-0 overflow-y-auto")
        .role(Role::List)
        .virtual_rows(count, row_height, None, render)
}

/// Order of a sorted column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// How wide a [`Column`] is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    /// Points.
    Fixed(f32),
    /// Share of the width the fixed columns leave.
    Flex(f32),
}

/// A column of a [`Table`]: title, width and whether it sorts.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub title: String,
    pub width: ColumnWidth,
    pub sortable: bool,
    /// Numbers: aligned to the end.
    pub numeric: bool,
}

impl Column {
    /// A `Flex(1)` column that does not sort.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: ColumnWidth::Flex(1.0),
            sortable: false,
            numeric: false,
        }
    }

    pub fn fixed(mut self, points: f32) -> Self {
        self.width = ColumnWidth::Fixed(points);
        self
    }

    pub fn flex(mut self, share: f32) -> Self {
        self.width = ColumnWidth::Flex(share);
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn numeric(mut self) -> Self {
        self.numeric = true;
        self
    }

    fn width_class(&self) -> String {
        match self.width {
            ColumnWidth::Fixed(points) => format!("w-[{points}px] shrink-0"),
            ColumnWidth::Flex(share) => format!("flex-1 grow-[{share}] min-w-0"),
        }
    }
}

/// Table over a [`virtual_list`]: a fixed header of [`Role::ColumnHeader`]
/// cells and `rows` [`Role::Row`]s of [`Role::Cell`]s whose text is
/// `cell(row, column)`. Rows are in display order: the app sorts and
/// filters its data and maps the index.
///
/// A sortable column's header sends `on_sort(column, direction)`, the
/// direction reversed when it is already the sorted one, and shows an
/// arrow. With `on_select` the rows are one Tab stop: a click, ↑/↓,
/// Home/End and PageUp/PageDown select one. AppKit rows alternate their
/// background.
///
/// Parts: `header`, `column.<i>`, `body` and `row.<i>`.
pub struct Table<Msg> {
    columns: Vec<Column>,
    rows: usize,
    cell: Rc<dyn Fn(usize, usize) -> String>,
    row_height: Option<f32>,
    sort: Option<(usize, SortDirection)>,
    on_sort: Option<Rc<dyn Fn(usize, SortDirection) -> Msg>>,
    selected: Option<usize>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    label: Option<String>,
    test_id: Option<String>,
    leading: Option<(usize, Leading<Msg>)>,
}

/// Element before the text of a cell, by row ([`Table::leading`]).
type Leading<Msg> = Rc<dyn Fn(usize) -> Element<Msg>>;

impl<Msg: Clone + 'static> Table<Msg> {
    pub fn new(
        columns: Vec<Column>,
        rows: usize,
        cell: impl Fn(usize, usize) -> String + 'static,
    ) -> Self {
        Self {
            columns,
            rows,
            cell: Rc::new(cell),
            row_height: None,
            sort: None,
            on_sort: None,
            selected: None,
            on_select: None,
            label: None,
            test_id: None,
            leading: None,
        }
    }

    /// Put `element(row)` before the text of `column`'s cells (an avatar
    /// next to a name). Built only for the visible rows.
    pub fn leading(
        mut self,
        column: usize,
        element: impl Fn(usize) -> Element<Msg> + 'static,
    ) -> Self {
        self.leading = Some((column, Rc::new(element)));
        self
    }

    /// Row height in points: 32 by default (24 is AppKit's).
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = Some(height);
        self
    }

    /// The column the rows are sorted by, and how.
    pub fn sort(mut self, column: usize, direction: SortDirection) -> Self {
        self.sort = Some((column, direction));
        self
    }

    pub fn on_sort(mut self, handler: impl Fn(usize, SortDirection) -> Msg + 'static) -> Self {
        self.on_sort = Some(Rc::new(handler));
        self
    }

    pub fn selected(mut self, row: Option<usize>) -> Self {
        self.selected = row;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Accessible name of the table.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<String>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    /// The table; give it a height with `.class(..)`.
    pub fn build(self) -> Element<Msg> {
        let row_height = self.row_height.unwrap_or(32.0);
        let header = div()
            .class(&themed(
                "flex items-stretch h-9 shrink-0 border-b border-border bg-muted/40",
                "flex items-stretch h-[24px] shrink-0 border-b border-separator bg-window",
            ))
            .role(Role::Row)
            .focus_group()
            .part("header")
            .children(
                self.columns
                    .iter()
                    .enumerate()
                    .map(|(index, column)| self.header_cell(index, column)),
            );
        let columns = Rc::new(self.columns);
        let cell = self.cell;
        let selected = self.selected;
        let on_select = self.on_select.clone();
        let leading = self.leading;
        let render: Rc<dyn Fn(usize) -> Element<Msg>> = Rc::new(move |index| {
            table_row(
                index,
                &columns,
                &cell,
                leading.as_ref(),
                selected,
                on_select.as_deref(),
            )
        });
        let mut body = div()
            .class("flex flex-col flex-1 min-h-0 overflow-y-auto")
            .part("body")
            .virtual_rows(self.rows, row_height, self.selected, render);
        body.row_count = None;
        if self.on_select.is_some() {
            body = body.selecting_focus_group();
        }
        let mut root = div()
            .class(&themed(
                "flex flex-col min-h-0 overflow-hidden rounded-lg border border-border \
                 bg-background",
                "flex flex-col min-h-0 overflow-hidden rounded-[6px] border-[0.5px] \
                 border-separator bg-control-background",
            ))
            .role(Role::Table)
            .row_count(self.rows)
            .parts_root()
            .children([header, body]);
        if let Some(label) = self.label {
            root = root.label(label);
        }
        root.test_id = self.test_id;
        root
    }

    fn header_cell(&self, index: usize, column: &Column) -> Element<Msg> {
        let sorted = self
            .sort
            .filter(|(sorted, _)| *sorted == index)
            .map(|(_, direction)| direction);
        let mut children = vec![
            text(column.title.as_str())
                .class("truncate")
                .part(format!("column.{index}.title")),
        ];
        if let Some(direction) = sorted {
            let arrow = match direction {
                SortDirection::Ascending => Icon::ArrowUp,
                SortDirection::Descending => Icon::ArrowDown,
            };
            children.push(icon(arrow).class("size-3.5 shrink-0"));
        }
        let justify = if column.numeric { "justify-end" } else { "" };
        let mut cell = div()
            .class(&format!(
                "flex items-center gap-1 {} {justify} {}",
                column.width_class(),
                themed(
                    "px-3 text-xs font-medium text-muted-foreground",
                    "px-1.5 text-footnote font-semibold text-secondary-label",
                )
            ))
            .role(Role::ColumnHeader)
            .label(column.title.as_str())
            .part(format!("column.{index}"))
            .children(children);
        if column.sortable
            && let Some(on_sort) = &self.on_sort
        {
            let next = sorted.map_or(SortDirection::Ascending, SortDirection::reversed);
            cell = cell
                .class(&themed(
                    "cursor-pointer hover:text-foreground hover:bg-muted \
                     focus-visible:ring-2 focus-visible:ring-primary/40",
                    "cursor-default hover:bg-fill",
                ))
                .on_click(on_sort(index, next))
                .state(match sorted {
                    Some(SortDirection::Ascending) => crate::element::ControlState::On,
                    Some(SortDirection::Descending) => crate::element::ControlState::Mixed,
                    None => crate::element::ControlState::Off,
                });
        }
        cell
    }
}

fn table_row<Msg: Clone>(
    index: usize,
    columns: &[Column],
    cell: &Rc<dyn Fn(usize, usize) -> String>,
    leading: Option<&(usize, Leading<Msg>)>,
    selected: Option<usize>,
    on_select: Option<&dyn Fn(usize) -> Msg>,
) -> Element<Msg> {
    let is_selected = selected == Some(index);
    let look = if is_selected {
        themed(
            "bg-info/20 text-foreground",
            "bg-selection text-on-accent window-inactive:bg-selection-inactive \
             window-inactive:text-label",
        )
    } else if index % 2 == 1 {
        themed("text-foreground hover:bg-info/10", "bg-label/4 text-label")
    } else {
        themed("text-foreground hover:bg-info/10", "text-label")
    };
    let cells = columns.iter().enumerate().map(|(column_index, column)| {
        let justify = if column.numeric { "justify-end" } else { "" };
        let before = leading
            .filter(|(leading_column, _)| *leading_column == column_index)
            .map(|(_, element)| element(index));
        div()
            .class(&format!(
                "flex items-center gap-2 {} {justify} {}",
                column.width_class(),
                themed("px-3", "px-1.5")
            ))
            .role(Role::Cell)
            .children(before)
            .child(text(cell(index, column_index)).class("truncate"))
    });
    let mut row = div()
        .class(&format!(
            "flex items-stretch {} {look}",
            themed(
                "text-sm border-b border-border/50 focus-visible:ring-2 \
                 focus-visible:ring-inset focus-visible:ring-primary/40",
                "text-body",
            )
        ))
        .role(Role::Row)
        .part(format!("row.{index}"))
        .children(cells);
    if let Some(on_select) = on_select {
        row = row
            .class(&themed("cursor-pointer", "cursor-default"))
            .pressed(is_selected)
            .on_click(on_select(index));
    }
    row
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{Column, SortDirection, Table, virtual_list};
    use crate::element::{ControlState, Element, Role, div, text};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    const ROWS: usize = 10_000;

    struct Files {
        /// Display order: indices into the data.
        order: Vec<usize>,
        sort: Option<(usize, SortDirection)>,
        selected: Option<usize>,
    }

    impl Default for Files {
        fn default() -> Self {
            Self {
                order: (0..ROWS).collect(),
                sort: None,
                selected: None,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Sort(usize, SortDirection),
        Select(usize),
    }

    fn size(file: usize) -> usize {
        (file * 7_919) % 10_007
    }

    impl SimpleApp for Files {
        type Msg = Msg;
        fn title(&self) -> String {
            "Archivos".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::Sort(column, direction) => {
                    self.sort = Some((column, direction));
                    self.order
                        .sort_by_key(|file| if column == 1 { size(*file) } else { *file });
                    if direction == SortDirection::Descending {
                        self.order.reverse();
                    }
                }
                Msg::Select(row) => self.selected = Some(row),
            }
        }
        fn view(&self) -> Element<Msg> {
            let order = Rc::new(self.order.clone());
            let mut table = Table::new(
                vec![
                    Column::new("Nombre").sortable(),
                    Column::new("Tamaño").fixed(120.0).sortable().numeric(),
                ],
                ROWS,
                move |row, column| {
                    let file = order[row];
                    match column {
                        0 => format!("archivo-{file}.txt"),
                        _ => format!("{} KB", size(file)),
                    }
                },
            )
            .selected(self.selected)
            .on_select(Msg::Select)
            .on_sort(Msg::Sort)
            .label("Archivos")
            .test_id("tabla");
            if let Some((column, direction)) = self.sort {
                table = table.sort(column, direction);
            }
            div()
                .class("flex flex-col w-full h-full p-4")
                .children([text("Archivos"), table.build().class("h-[400px]")])
        }
    }

    fn rows_built(app: &TestApp<Files>) -> usize {
        app.screen().all(By::role(Role::Row)).len() - 1
    }

    #[test]
    fn ten_thousand_rows_build_only_a_screenful() {
        let app = TestApp::new(Files::default());
        let table = app.screen().get(By::role(Role::Table).name("Archivos"));
        assert_eq!(table.node().row_count, Some(ROWS));
        let built = rows_built(&app);
        assert!((10..40).contains(&built), "{built} rows built");
        let row = app.screen().get(By::test_id("tabla.row.3"));
        assert_eq!(row.node().row_index, Some(3));
        assert_eq!(row.name().as_deref(), Some("archivo-3.txt 3743 KB"));
        assert!(app.screen().query(By::test_id("tabla.row.5000")).is_none());
    }

    #[test]
    fn keys_move_the_selection_by_index_and_reveal_it() {
        let mut app = TestApp::new(Files::default());
        app.click(By::test_id("tabla.row.2"));
        assert_eq!(app.app().selected, Some(2));
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(app.app().selected, Some(3));
        assert!(app.is_focused(By::test_id("tabla.row.3")));

        app.press(Key::End, Modifiers::NONE);
        assert_eq!(app.app().selected, Some(ROWS - 1));
        assert!(app.is_focused(By::test_id("tabla.row.9999")));
        let body = app.screen().get(By::test_id("tabla.body")).bounds();
        let last = app.screen().get(By::test_id("tabla.row.9999")).bounds();
        assert!(last.y + last.height <= body.y + body.height + 0.5);
        assert!(rows_built(&app) < 40);

        app.press(Key::PageUp, Modifiers::NONE);
        let page = app.app().selected.unwrap();
        assert!((ROWS - 14..ROWS - 9).contains(&page), "a page up: {page}");
        app.press(Key::Home, Modifiers::NONE);
        assert_eq!(app.app().selected, Some(0));
        assert!(app.is_focused(By::test_id("tabla.row.0")));
    }

    #[test]
    fn the_focused_row_stays_built_when_it_scrolls_away() {
        let mut app = TestApp::new(Files::default());
        app.click(By::test_id("tabla.row.1"));
        app.reveal_row(By::test_id("tabla.body"), 6_000);
        assert!(app.screen().query(By::test_id("tabla.row.6000")).is_some());
        assert!(app.is_focused(By::test_id("tabla.row.1")), "pinned");
        app.click(By::test_id("tabla.row.6000"));
        assert_eq!(app.app().selected, Some(6_000));
        let shift = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, Modifiers::NONE);
        app.press(Key::Tab, shift);
        assert!(
            app.is_focused(By::test_id("tabla.row.6000")),
            "Tab enters at the selection"
        );
    }

    #[test]
    fn headers_sort_and_show_the_direction() {
        let mut app = TestApp::new(Files::default());
        let size = By::role(Role::ColumnHeader).name("Tamaño");
        assert_eq!(
            app.screen().get(size.clone()).node().control_state,
            Some(ControlState::Off)
        );
        app.click(size.clone());
        assert_eq!(
            app.messages().last(),
            Some(&Msg::Sort(1, SortDirection::Ascending))
        );
        assert_eq!(
            app.screen().get(size.clone()).node().control_state,
            Some(ControlState::On)
        );
        app.screen()
            .get(By::role(Role::Row).name("archivo-0.txt 0 KB"));
        app.click(size.clone());
        assert_eq!(
            app.messages().last(),
            Some(&Msg::Sort(1, SortDirection::Descending))
        );
        assert_eq!(
            app.screen().get(size).node().control_state,
            Some(ControlState::Mixed)
        );
    }

    /// `cargo test table_capture -- --ignored` writes
    /// `target/snapshots/table-<theme>.png` (needs a GPU).
    #[test]
    #[ignore]
    fn table_capture() {
        use crate::theme::{AppKitGeneration, Appearance, Density, Theme};
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/snapshots");
        std::fs::create_dir_all(&dir).unwrap();
        let themes = [
            ("kai", Theme::kai(Density::Comfortable)),
            (
                "appkit",
                Theme::appkit_static(AppKitGeneration::Glass27, Appearance::Light),
            ),
        ];
        for (name, theme) in themes {
            let mut app = TestApp::builder(Files::default()).theme(theme).build();
            app.click(By::role(Role::ColumnHeader).name("Tamaño"));
            app.click(By::test_id("tabla.row.2"));
            let image = app.capture(By::test_id("tabla")).unwrap();
            crate::snapshot::write_png(&dir.join(format!("table-{name}.png")), &image).unwrap();
        }
    }

    /// Frames of a 10 000-row table scrolling with the wheel: view, layout
    /// with real text and paint. Run with
    /// `cargo test --release -- --ignored --nocapture bench_table_scroll`.
    #[test]
    #[ignore = "benchmark; run explicitly in release mode"]
    fn bench_table_scroll() {
        use crate::geometry::Size;
        use crate::session::{Measure, Session};
        use crate::testing::MemoryClipboard;
        use crate::theme::{Density, Theme};
        use std::time::{Duration, Instant};

        let mut session = Session::new(
            Files::default(),
            Theme::kai(Density::Comfortable),
            Size {
                width: 1024.0,
                height: 720.0,
            },
            Measure::Real,
            Box::new(MemoryClipboard::default()),
        );
        session.prepare_frame();
        let body = session
            .tree()
            .find_test_id("tabla.body")
            .map(|body| body.id)
            .unwrap();
        let bounds = session.layout().bounds(body).unwrap();
        session.pointer_moved(Some(crate::geometry::Point {
            x: bounds.x + 10.0,
            y: bounds.y + 10.0,
        }));
        let mut samples: Vec<Duration> = Vec::new();
        for frame in 0..400 {
            let delta = if frame < 200 { -37.0 } else { 37.0 };
            let start = Instant::now();
            session.scroll_by(delta);
            session.prepare_frame();
            let list = session.paint_list();
            samples.push(start.elapsed());
            assert!(!list.ops.is_empty());
        }
        samples.sort();
        let p50 = samples[samples.len() / 2];
        let p95 = samples[samples.len() * 95 / 100];
        println!("tabla de {ROWS} filas: p50 {p50:?}, p95 {p95:?} por frame");
    }

    #[test]
    fn the_wheel_builds_the_rows_it_brings_into_view() {
        let mut app = TestApp::new(Files::default());
        for _ in 0..10 {
            app.scroll_wheel(By::test_id("tabla.body"), -320.0);
        }
        assert!(app.screen().query(By::test_id("tabla.row.0")).is_none());
        let row = app.screen().get(By::test_id("tabla.row.100"));
        let body = app.screen().get(By::test_id("tabla.body")).bounds();
        assert!((row.bounds().y - body.y).abs() < 0.5, "row 100 at the top");
        assert!(rows_built(&app) < 40);
    }

    #[test]
    fn accessibility_reads_the_row_count_index_and_sort() {
        let mut app = TestApp::new(Files::default());
        app.click(By::role(Role::ColumnHeader).name("Nombre"));
        let tree = app.session().a11y_tree(1.0);
        let node = |by: By| {
            let id = crate::a11y::node_id(app.screen().get(by).id());
            tree.nodes
                .iter()
                .find(|(node, _)| *node == id)
                .map(|(_, node)| node.clone())
                .unwrap()
        };
        assert_eq!(node(By::role(Role::Table)).row_count(), Some(ROWS));
        assert_eq!(node(By::test_id("tabla.row.4")).row_index(), Some(4));
        assert_eq!(
            node(By::role(Role::ColumnHeader).name("Nombre")).sort_direction(),
            Some(accesskit::SortDirection::Ascending)
        );
    }

    #[test]
    fn a_virtual_list_marks_its_rows_as_items() {
        struct Names;
        impl SimpleApp for Names {
            type Msg = ();
            fn title(&self) -> String {
                "Nombres".into()
            }
            fn update(&mut self, (): ()) {}
            fn view(&self) -> Element<()> {
                div().child(
                    virtual_list(500, 24.0, Rc::new(|index| text(format!("Nombre {index}"))))
                        .class("h-[200px]")
                        .test_id("nombres"),
                )
            }
        }
        let app = TestApp::new(Names);
        let list = app.screen().get(By::role(Role::List));
        assert_eq!(list.node().row_count, Some(500));
        let items = app.screen().all(By::role(Role::ListItem));
        assert!((8..25).contains(&items.len()), "{} items", items.len());
        assert_eq!(items[0].name().as_deref(), Some("Nombre 0"));
    }
}
