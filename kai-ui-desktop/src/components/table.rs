//! Long lists and tables (`NSTableView`, 12 Paso 20.8): rows are built
//! only while they are in view ([`Element::virtual_rows`]), so ten
//! thousand rows cost what a screenful does.

mod view;

use std::rc::Rc;

pub use view::{Date, TableView, Value, ViewChange};

use super::themed;
use crate::element::{ControlState, Element, Role, div, icon, text};
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

type ValueFn<T> = Rc<dyn Fn(&T) -> Value>;
type FormatFn = Rc<dyn Fn(&Value) -> String>;
type RenderFn<T, Msg> = Rc<dyn Fn(&T) -> Element<Msg>>;

/// A column of a [`Table`] over rows of type `T`. Every column but
/// [`Column::actions`] has a typed [`Value`] per row: what the filters, the
/// global search, the sort and the export use, also when
/// [`Column::render`] draws a component in the cell.
pub struct Column<T, Msg> {
    pub title: String,
    pub width: ColumnWidth,
    value: Option<ValueFn<T>>,
    format: Option<FormatFn>,
    render: Option<RenderFn<T, Msg>>,
    numeric: bool,
    sortable: bool,
    filterable: bool,
    searchable: bool,
    hideable: bool,
}

impl<T, Msg> Clone for Column<T, Msg> {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            width: self.width,
            value: self.value.clone(),
            format: self.format.clone(),
            render: self.render.clone(),
            numeric: self.numeric,
            sortable: self.sortable,
            filterable: self.filterable,
            searchable: self.searchable,
            hideable: self.hideable,
        }
    }
}

impl<T: 'static, Msg: 'static> Column<T, Msg> {
    fn with_value(
        title: impl Into<String>,
        value: impl Fn(&T) -> Value + 'static,
        numeric: bool,
        searchable: bool,
    ) -> Self {
        Self {
            title: title.into(),
            width: ColumnWidth::Flex(1.0),
            value: Some(Rc::new(value)),
            format: None,
            render: None,
            numeric,
            sortable: false,
            filterable: false,
            searchable,
            hideable: false,
        }
    }

    /// A text column; the global search looks in it.
    pub fn text(title: impl Into<String>, value: impl Fn(&T) -> String + 'static) -> Self {
        Self::with_value(title, move |row| Value::Text(value(row)), false, true)
    }

    /// A number column, aligned to the end.
    pub fn number(title: impl Into<String>, value: impl Fn(&T) -> f64 + 'static) -> Self {
        Self::with_value(title, move |row| Value::Number(value(row)), true, false)
    }

    pub fn date(title: impl Into<String>, value: impl Fn(&T) -> Date + 'static) -> Self {
        Self::with_value(title, move |row| Value::Date(value(row)), false, false)
    }

    /// A yes/no column, shown «Sí» or «No» unless [`Self::format`] says.
    pub fn boolean(title: impl Into<String>, value: impl Fn(&T) -> bool + 'static) -> Self {
        Self::with_value(title, move |row| Value::Bool(value(row)), false, false)
    }

    /// Buttons for each row, without a value: never sorted, filtered,
    /// searched, hidden or exported. Its header is empty; assistive
    /// technology names it «Acciones».
    pub fn actions(render: impl Fn(&T) -> Element<Msg> + 'static) -> Self {
        Self {
            title: "Acciones".into(),
            width: ColumnWidth::Fixed(80.0),
            value: None,
            format: None,
            render: Some(Rc::new(render)),
            numeric: false,
            sortable: false,
            filterable: false,
            searchable: false,
            hideable: false,
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

    /// Its header sorts the rows.
    pub fn sortable(mut self) -> Self {
        self.sortable = self.value.is_some();
        self
    }

    /// It gets a field in the header's filter row.
    pub fn filterable(mut self) -> Self {
        self.filterable = self.value.is_some();
        self
    }

    /// Whether the global search looks in it (text columns do).
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable && self.value.is_some();
        self
    }

    /// People can hide it.
    pub fn hideable(mut self) -> Self {
        self.hideable = self.value.is_some();
        self
    }

    /// The text the cell shows (and the search reads) for its value.
    pub fn format(mut self, format: impl Fn(&Value) -> String + 'static) -> Self {
        self.format = Some(Rc::new(format));
        self
    }

    /// Draw `render(row)` in the cell instead of its text. The value still
    /// filters, sorts and exports, and names the cell for assistive
    /// technology. Built only for the rows in view.
    pub fn render(mut self, render: impl Fn(&T) -> Element<Msg> + 'static) -> Self {
        self.render = Some(Rc::new(render));
        self
    }
}

impl<T, Msg> Column<T, Msg> {
    pub fn is_actions(&self) -> bool {
        self.value.is_none()
    }

    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    pub fn is_sortable(&self) -> bool {
        self.sortable
    }

    pub fn is_filterable(&self) -> bool {
        self.filterable
    }

    pub fn is_searchable(&self) -> bool {
        self.searchable
    }

    pub fn is_hideable(&self) -> bool {
        self.hideable
    }

    /// The value of `row` in this column; `None` for actions.
    pub fn value_of(&self, row: &T) -> Option<Value> {
        self.value.as_ref().map(|value| value(row))
    }

    /// The text `value` shows in this column.
    pub fn shown(&self, value: &Value) -> String {
        match &self.format {
            Some(format) => format(value),
            None => value.to_string(),
        }
    }

    /// The value of `row` and the text it shows; `None` for actions.
    pub(crate) fn cell_text(&self, row: &T) -> Option<(Value, String)> {
        let value = self.value_of(row)?;
        let shown = self.shown(&value);
        Some((value, shown))
    }

    fn width_class(&self) -> String {
        match self.width {
            ColumnWidth::Fixed(points) => format!("w-[{points}px] shrink-0"),
            ColumnWidth::Flex(share) => format!("flex-1 grow-[{share}] min-w-0"),
        }
    }
}

/// Table over a [`virtual_list`]: a fixed header of [`Role::ColumnHeader`]
/// cells and a [`Role::Row`] of [`Role::Cell`]s for each row of
/// `view.rows()`, the data the [`TableView`] keeps after its search,
/// filters and sort. The app keeps the view and applies the
/// [`ViewChange`]s the table sends with `on_view`: a sortable header
/// sends `Sort(column, direction)`, reversed when it is already the
/// sorted one, and shows an arrow.
///
/// Rows are indices into the data: `selected` and `on_select` take them,
/// so the selection survives sorting. With `on_select` the rows are one
/// Tab stop: a click, ↑/↓, Home/End and PageUp/PageDown select one.
/// Hidden columns are left out. AppKit rows alternate their background.
///
/// Parts: `header`, `column.<i>`, `body` and `row.<i>` (`i` is the
/// column's index, and the row's place on screen).
pub struct Table<T, Msg> {
    data: Rc<[T]>,
    columns: Rc<[Column<T, Msg>]>,
    view: TableView,
    row_height: Option<f32>,
    on_view: Option<Rc<dyn Fn(ViewChange) -> Msg>>,
    selected: Option<usize>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    label: Option<String>,
    test_id: Option<String>,
}

impl<T: 'static, Msg: Clone + 'static> Table<T, Msg> {
    pub fn new(data: Rc<[T]>, columns: Vec<Column<T, Msg>>, view: &TableView) -> Self {
        Self {
            data,
            columns: columns.into(),
            view: view.clone(),
            row_height: None,
            on_view: None,
            selected: None,
            on_select: None,
            label: None,
            test_id: None,
        }
    }

    /// Row height in points: 32 by default (24 is AppKit's).
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = Some(height);
        self
    }

    /// What the table asks to change of its [`TableView`].
    pub fn on_view(mut self, handler: impl Fn(ViewChange) -> Msg + 'static) -> Self {
        self.on_view = Some(Rc::new(handler));
        self
    }

    /// The selected row, by its index in the data.
    pub fn selected(mut self, row: Option<usize>) -> Self {
        self.selected = row;
        self
    }

    /// Called with the index in the data of the row people choose.
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

    /// Indices of the columns shown: all but the hidden ones.
    fn visible(&self) -> Rc<[usize]> {
        (0..self.columns.len())
            .filter(|index| self.columns[*index].is_actions() || !self.view.hidden.contains(index))
            .collect()
    }

    /// The table; give it a height with `.class(..)`.
    pub fn build(self) -> Element<Msg> {
        let row_height = self.row_height.unwrap_or(32.0);
        let visible = self.visible();
        let header = div()
            .class(&themed(
                "flex items-stretch h-9 shrink-0 border-b border-border bg-muted/40",
                "flex items-stretch h-[24px] shrink-0 border-b border-separator bg-window",
            ))
            .role(Role::Row)
            .focus_group()
            .part("header")
            .children(visible.iter().map(|&index| self.header_cell(index)));
        let rows = Rc::clone(self.view.rows());
        let count = rows.len();
        let selected = self.selected.and_then(|row| self.view.position(row));
        let body_rows = BodyRows {
            data: Rc::clone(&self.data),
            columns: Rc::clone(&self.columns),
            visible,
            rows,
            selected: self.selected,
            on_select: self.on_select.clone(),
        };
        let render: Rc<dyn Fn(usize) -> Element<Msg>> = Rc::new(move |index| body_rows.row(index));
        let mut body = div()
            .class("flex flex-col flex-1 min-h-0 overflow-y-auto")
            .part("body")
            .virtual_rows(count, row_height, selected, render);
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
            .row_count(count)
            .parts_root()
            .children([header, body]);
        if let Some(label) = self.label {
            root = root.label(label);
        }
        root.test_id = self.test_id;
        root
    }

    fn header_cell(&self, index: usize) -> Element<Msg> {
        let column = &self.columns[index];
        let sorted = self
            .view
            .sort
            .filter(|(sorted, _)| *sorted == index)
            .map(|(_, direction)| direction);
        let mut children = Vec::new();
        if !column.is_actions() {
            children.push(
                text(column.title.as_str())
                    .class("truncate")
                    .part(format!("column.{index}.title")),
            );
        }
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
            && let Some(on_view) = &self.on_view
        {
            let next = sorted.map_or(SortDirection::Ascending, SortDirection::reversed);
            cell = cell
                .class(&themed(
                    "cursor-pointer hover:text-foreground hover:bg-muted \
                     focus-visible:ring-2 focus-visible:ring-primary/40",
                    "cursor-default hover:bg-fill",
                ))
                .on_click(on_view(ViewChange::Sort(index, next)))
                .state(match sorted {
                    Some(SortDirection::Ascending) => ControlState::On,
                    Some(SortDirection::Descending) => ControlState::Mixed,
                    None => ControlState::Off,
                });
        }
        cell
    }
}

/// What builds the rows of a [`Table`] as they scroll into view.
struct BodyRows<T, Msg> {
    data: Rc<[T]>,
    columns: Rc<[Column<T, Msg>]>,
    visible: Rc<[usize]>,
    /// Indices into `data`, in display order.
    rows: Rc<[usize]>,
    /// Selected row, by its index in `data`.
    selected: Option<usize>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
}

impl<T, Msg: Clone> BodyRows<T, Msg> {
    /// The row shown at `index`.
    fn row(&self, index: usize) -> Element<Msg> {
        let Some(&data_index) = self.rows.get(index) else {
            return div().role(Role::Row);
        };
        let item = &self.data[data_index];
        let is_selected = self.selected == Some(data_index);
        // Rows touch: the choice tint without its border, which would draw
        // over the row dividers.
        let look = if is_selected {
            themed(
                "bg-choice/20 hover:bg-choice/25 text-foreground",
                "bg-choice/20 text-label",
            )
        } else if index % 2 == 1 {
            themed(
                "text-foreground hover:bg-choice/10",
                "bg-label/4 text-label",
            )
        } else {
            themed("text-foreground hover:bg-choice/10", "text-label")
        };
        let cells = self.visible.iter().map(|&column_index| {
            let column = &self.columns[column_index];
            let justify = if column.numeric { "justify-end" } else { "" };
            let cell = div()
                .class(&format!(
                    "flex items-center gap-2 {} {justify} {}",
                    column.width_class(),
                    themed("px-3", "px-1.5")
                ))
                .role(Role::Cell);
            let shown = column.cell_text(item).map(|(_, shown)| shown);
            match (&column.render, shown) {
                (Some(render), Some(shown)) => cell.label(shown).child(render(item)),
                (Some(render), None) => cell.child(render(item)),
                (None, Some(shown)) => cell.child(text(shown).class("truncate")),
                (None, None) => cell,
            }
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
        if let Some(on_select) = &self.on_select {
            row = row
                .class(&themed("cursor-pointer", "cursor-default"))
                .pressed(is_selected)
                .on_click(on_select(data_index));
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{Column, SortDirection, Table, TableView, ViewChange, virtual_list};
    use crate::components::{BadgeVariant, ButtonVariant, badge, button};
    use crate::element::{ControlState, Element, Role, div, text};
    use crate::runtime::SimpleApp;
    use crate::testing::{By, TestApp};
    use crate::ui_engine::{Key, Modifiers};

    const ROWS: usize = 10_000;

    struct File {
        name: String,
        size: usize,
    }

    struct Files {
        data: Rc<[File]>,
        view: TableView,
        selected: Option<usize>,
        opened: Option<usize>,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        View(ViewChange),
        Select(usize),
        Open(usize),
    }

    fn columns() -> Vec<Column<File, Msg>> {
        vec![
            Column::text("Nombre", |file: &File| file.name.clone()).sortable(),
            Column::number("Tamaño", |file: &File| file.size as f64)
                .format(|size| format!("{size} KB"))
                .fixed(120.0)
                .sortable(),
            Column::text("Tipo", |file: &File| {
                if file.size.is_multiple_of(2) {
                    "Par"
                } else {
                    "Impar"
                }
                .to_owned()
            })
            .render(|file: &File| {
                let (label, variant) = if file.size.is_multiple_of(2) {
                    ("par", BadgeVariant::Info)
                } else {
                    ("impar", BadgeVariant::Warning)
                };
                badge(label, variant)
            })
            .fixed(96.0),
            Column::actions(|file: &File| {
                button("Abrir", ButtonVariant::Text)
                    .on_click(Msg::Open(file.size))
                    .test_id(format!("abrir.{}", file.name))
            }),
        ]
    }

    impl Default for Files {
        fn default() -> Self {
            let data: Rc<[File]> = (0..ROWS)
                .map(|index| File {
                    name: format!("archivo-{index}.txt"),
                    size: (index * 7_919) % 10_007,
                })
                .collect();
            let view = TableView::new(&data, &columns());
            Self {
                data,
                view,
                selected: None,
                opened: None,
            }
        }
    }

    impl SimpleApp for Files {
        type Msg = Msg;
        fn title(&self) -> String {
            "Archivos".into()
        }
        fn update(&mut self, msg: Msg) {
            match msg {
                Msg::View(change) => {
                    self.view.apply(change);
                    self.view.refresh(&self.data, &columns());
                }
                Msg::Select(row) => self.selected = Some(row),
                Msg::Open(size) => self.opened = Some(size),
            }
        }
        fn view(&self) -> Element<Msg> {
            let table = Table::new(Rc::clone(&self.data), columns(), &self.view)
                .selected(self.selected)
                .on_select(Msg::Select)
                .on_view(Msg::View)
                .label("Archivos")
                .test_id("tabla");
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
        assert_eq!(
            row.name().as_deref(),
            Some("archivo-3.txt 3743 KB impar Abrir")
        );
        assert!(app.screen().query(By::test_id("tabla.row.5000")).is_none());
    }

    #[test]
    fn a_rendered_cell_is_named_by_its_value_and_actions_are_buttons() {
        let mut app = TestApp::new(Files::default());
        let screen = app.screen();
        let row = screen.within(screen.get_by_test_id("tabla.row.0"));
        let names: Vec<_> = row
            .all(By::role(Role::Cell))
            .iter()
            .map(|cell| cell.name())
            .collect();
        assert_eq!(names[2].as_deref(), Some("Par"), "the value, not the badge");
        row.get(By::role(Role::Status));
        row.get(By::text("par"));
        let actions = screen.get(By::role(Role::ColumnHeader).name("Acciones"));
        assert!(
            screen.within(actions).query(By::text("Acciones")).is_none(),
            "the actions header shows no title"
        );

        app.click(By::test_id("abrir.archivo-3.txt"));
        assert_eq!(app.app().opened, Some(3743));
        assert_eq!(
            app.app().selected,
            None,
            "the button does not select the row"
        );
    }

    #[test]
    fn keys_move_the_selection_and_reveal_it() {
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
    fn the_selection_follows_its_row_through_sorting() {
        let mut app = TestApp::new(Files::default());
        app.click(By::test_id("tabla.row.1"));
        assert_eq!(app.app().selected, Some(1));
        app.click(By::role(Role::ColumnHeader).name("Tamaño"));
        let shown = app.app().view.position(1).unwrap();
        let row = app.screen().get(By::test_id(format!("tabla.row.{shown}")));
        assert_eq!(row.node().control_state, Some(ControlState::On));
        assert!(row.name().unwrap().starts_with("archivo-1.txt"));
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
            Some(&Msg::View(ViewChange::Sort(1, SortDirection::Ascending)))
        );
        assert_eq!(
            app.screen().get(size.clone()).node().control_state,
            Some(ControlState::On)
        );
        app.screen()
            .get(By::role(Role::Row).name("archivo-0.txt 0 KB par Abrir"));
        app.click(size.clone());
        assert_eq!(
            app.messages().last(),
            Some(&Msg::View(ViewChange::Sort(1, SortDirection::Descending)))
        );
        assert_eq!(
            app.screen().get(size).node().control_state,
            Some(ControlState::Mixed)
        );
        assert!(
            app.screen()
                .query(By::role(Role::ColumnHeader).name("Tipo"))
                .is_some_and(|header| header.node().control_state.is_none()),
            "a column that does not sort is not a control"
        );
    }

    #[test]
    fn hidden_columns_are_left_out_but_actions_stay() {
        let mut files = Files::default();
        files.view.apply(ViewChange::Hide(1, true));
        files.view.apply(ViewChange::Hide(3, true));
        let app = TestApp::new(files);
        let headers: Vec<_> = app
            .screen()
            .all(By::role(Role::ColumnHeader))
            .iter()
            .filter_map(|header| header.name())
            .collect();
        assert_eq!(headers, ["Nombre", "Tipo", "Acciones"]);
        assert!(app.screen().query(By::test_id("tabla.column.1")).is_none());
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
