//! Long lists and tables (`NSTableView`, 12 Paso 20.8): rows are built
//! only while they are in view ([`Element::virtual_rows`]), so ten
//! thousand rows cost what a screenful does.

mod view;

use std::collections::BTreeSet;
use std::rc::Rc;

pub use view::{Date, Sheet, TableView, Value, ViewChange};

use super::{search_field, themed};
use crate::element::{ControlSize, ControlState, Element, Role, div, icon, text};
use crate::icons::Icon;
use crate::responder::Action;

/// Width of the column of expand buttons ([`Table::expandable`]).
const TOGGLE_WIDTH: &str = "w-8 shrink-0";
const TOGGLE_POINTS: f32 = 32.0;

/// Narrowest a [`ColumnWidth::Flex`] column gets without
/// [`Column::min_width`].
const FLEX_MIN_WIDTH: f32 = 80.0;

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
    min_width: Option<f32>,
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
            min_width: self.min_width,
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
            min_width: None,
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
            min_width: None,
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

    /// Narrowest a flex column gets (80 points by default): when the
    /// columns do not fit, the table scrolls across instead.
    pub fn min_width(mut self, points: f32) -> Self {
        self.min_width = Some(points.max(0.0));
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

    /// The narrowest it gets, in points.
    fn min_points(&self) -> f32 {
        match self.width {
            ColumnWidth::Fixed(points) => points,
            ColumnWidth::Flex(_) => self.min_width.unwrap_or(FLEX_MIN_WIDTH),
        }
    }

    fn width_class(&self) -> String {
        match self.width {
            ColumnWidth::Fixed(points) => format!("w-[{points}px] shrink-0"),
            ColumnWidth::Flex(share) => {
                format!("flex-1 grow-[{share}] min-w-[{}px]", self.min_points())
            }
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
/// While [`TableView::filtering`], each [`Column::filterable`] gets a field
/// under its header («Filtrar <título>», Escape clears it) that sends
/// [`ViewChange::Filter`] on every edit: the app may wait for a pause in
/// the typing before it refreshes (`Effects::after(..).keyed(..)`).
///
/// A right click (or Ctrl+click) on the header opens the [`column_menu`].
///
/// With [`Table::expandable`], rows open a panel below them.
///
/// Columns keep their width ([`Column::fixed`]) or minimum width
/// ([`Column::min_width`]): when they do not fit, the header and the rows
/// scroll across together (trackpad, Shift+wheel, or the focus moving to
/// a cell out of view).
///
/// Parts: `scroller`, `header`, `column.<i>`, `filters`, `filter.<i>`,
/// `body`, `row.<r>`, `row.<r>.toggle` and `detail.<r>` (`i` is the
/// column's index, `r` the row's place on screen).
pub struct Table<T, Msg> {
    data: Rc<[T]>,
    columns: Rc<[Column<T, Msg>]>,
    view: TableView,
    row_height: Option<f32>,
    on_view: Option<Rc<dyn Fn(ViewChange) -> Msg>>,
    selected: Option<usize>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    expand: Option<(RenderFn<T, Msg>, f32)>,
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
            expand: None,
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

    /// Rows open a panel `height` points tall below them, built by
    /// `detail(row)`: a chevron at the start of each row opens and closes
    /// it, and so do → and ← on a focused row, sending
    /// [`ViewChange::Expand`]; [`TableView::expanded`] says which are open
    /// (several at once). The row reads as expanded or collapsed; the panel
    /// is a [`Role::Region`] named after the row's values, and its controls
    /// are Tab stops. Panels are not exported.
    pub fn expandable(
        mut self,
        detail: impl Fn(&T) -> Element<Msg> + 'static,
        height: f32,
    ) -> Self {
        self.expand = Some((Rc::new(detail), height.max(0.0)));
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
        let toggle = if self.expand.is_some() {
            TOGGLE_POINTS
        } else {
            0.0
        };
        let min_width = toggle
            + visible
                .iter()
                .map(|&index| self.columns[index].min_points())
                .sum::<f32>();
        let mut header = div()
            .class(&themed(
                "flex items-stretch h-9 shrink-0 border-b border-border bg-muted/40",
                "flex items-stretch h-[24px] shrink-0 border-b border-separator bg-window",
            ))
            .role(Role::Row)
            .focus_group()
            .part("header")
            .children(self.expand.is_some().then(|| {
                div()
                    .class(TOGGLE_WIDTH)
                    .role(Role::ColumnHeader)
                    .label("Detalle")
            }))
            .children(visible.iter().map(|&index| self.header_cell(index)));
        if let Some(on_view) = &self.on_view {
            let on_view = Rc::clone(on_view);
            header = column_menu(header, &self.columns, &self.view, move |change| {
                on_view(change)
            })
            .context_menu();
        }
        let filters = self.filter_row(&visible);
        let rows = Rc::clone(self.view.rows());
        let count = rows.len();
        let selected = self.selected.and_then(|row| self.view.position(row));
        let extras: Vec<(usize, f32)> = match &self.expand {
            Some((_, height)) if !self.view.expanded.is_empty() => rows
                .iter()
                .enumerate()
                .filter(|(_, row)| self.view.expanded.contains(row))
                .map(|(position, _)| (position, *height))
                .collect(),
            _ => Vec::new(),
        };
        let body_rows = BodyRows {
            data: Rc::clone(&self.data),
            columns: Rc::clone(&self.columns),
            visible,
            rows,
            row_height,
            selected: self.selected,
            on_select: self.on_select.clone(),
            expand: self.expand.clone(),
            expanded: Rc::new(self.view.expanded.clone()),
            on_view: self.on_view.clone(),
        };
        let render: Rc<dyn Fn(usize) -> Element<Msg>> = Rc::new(move |index| body_rows.row(index));
        let mut body = div()
            .class("flex flex-col flex-1 min-h-0 overflow-y-auto")
            .part("body")
            .virtual_rows(count, row_height, selected, render)
            .row_extras(extras);
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
            .child(
                div()
                    .class("flex flex-col flex-1 min-h-0 overflow-x-auto")
                    .part("scroller")
                    .child(
                        div()
                            .class(&format!(
                                "flex flex-col flex-1 min-h-0 min-w-[{min_width}px]"
                            ))
                            .child(header)
                            .children(filters)
                            .child(body),
                    ),
            );
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

    /// While [`TableView::filtering`], a row of search fields under the
    /// header, one per filterable column; each is its own Tab stop.
    fn filter_row(&self, visible: &[usize]) -> Option<Element<Msg>> {
        if !self.view.filtering {
            return None;
        }
        let on_view = self.on_view.as_ref()?;
        let cells = visible.iter().map(|&index| {
            let column = &self.columns[index];
            let cell = div()
                .class(&format!(
                    "flex items-center {} {}",
                    column.width_class(),
                    themed("px-1.5", "px-1")
                ))
                .role(Role::Cell);
            if !column.filterable {
                return cell;
            }
            let on_view = Rc::clone(on_view);
            let value = self.view.filters.get(&index).cloned().unwrap_or_default();
            let mut field =
                search_field(value, move |text| on_view(ViewChange::Filter(index, text)))
                    .placeholder("Filtrar")
                    .label(format!("Filtrar {}", column.title))
                    .control_size(ControlSize::Small);
            if let Some(id) = &self.test_id {
                field = field.test_id(format!("{id}.filter.{index}"));
            }
            cell.child(field.build().class("min-w-0"))
        });
        Some(
            div()
                .class(&themed(
                    "flex items-center h-10 shrink-0 border-b border-border",
                    "flex items-center h-[28px] shrink-0 border-b border-separator",
                ))
                .role(Role::Row)
                .label("Filtros")
                .part("filters")
                .children(
                    self.expand
                        .is_some()
                        .then(|| div().class(TOGGLE_WIDTH).role(Role::Cell)),
                )
                .children(cells),
        )
    }
}

/// Make `target` open the menu of the [`Column::hideable`] columns: a
/// check item each, checked while it is shown, that sends
/// [`ViewChange::Hide`] and leaves the menu open. The last column shown
/// cannot be unchecked. The header of a [`Table`] with `on_view` opens the
/// same menu with a right click; give it to a «Columnas» button too.
pub fn column_menu<T: 'static, Msg: 'static>(
    target: Element<Msg>,
    columns: &[Column<T, Msg>],
    view: &TableView,
    on_view: impl Fn(ViewChange) -> Msg + 'static,
) -> Element<Msg> {
    let hideable: Rc<[usize]> = (0..columns.len())
        .filter(|index| columns[*index].is_hideable())
        .collect();
    if hideable.is_empty() {
        return target;
    }
    let items = hideable
        .iter()
        .map(|&index| columns[index].title.clone())
        .collect();
    let shown: Rc<[bool]> = hideable
        .iter()
        .map(|index| !view.hidden.contains(index))
        .collect();
    let disabled = hideable
        .iter()
        .zip(shown.iter())
        .map(|(&index, &shown)| shown && !view.can_hide(index))
        .collect();
    target
        .check_options(items, shown.to_vec())
        .disabled_options(disabled)
        .on_select(move |pick| on_view(ViewChange::Hide(hideable[pick], shown[pick])))
}

/// What builds the rows of a [`Table`] as they scroll into view.
struct BodyRows<T, Msg> {
    data: Rc<[T]>,
    columns: Rc<[Column<T, Msg>]>,
    visible: Rc<[usize]>,
    /// Indices into `data`, in display order.
    rows: Rc<[usize]>,
    row_height: f32,
    /// Selected row, by its index in `data`.
    selected: Option<usize>,
    on_select: Option<Rc<dyn Fn(usize) -> Msg>>,
    expand: Option<(RenderFn<T, Msg>, f32)>,
    /// Expanded rows, by index in `data`.
    expanded: Rc<BTreeSet<usize>>,
    on_view: Option<Rc<dyn Fn(ViewChange) -> Msg>>,
}

impl<T: 'static, Msg: Clone + 'static> BodyRows<T, Msg> {
    /// The row shown at `index`. In an expandable table it is wrapped
    /// (open or not, so its id stays) with the panel below when open.
    fn row(&self, index: usize) -> Element<Msg> {
        let Some(&data_index) = self.rows.get(index) else {
            return div().role(Role::Row);
        };
        let open = self
            .expand
            .is_some()
            .then(|| self.expanded.contains(&data_index));
        let row = self.cells(index, data_index, open);
        let Some((detail, height)) = &self.expand else {
            return row;
        };
        let wrapper = div()
            .class("flex flex-col")
            .child(row.class(&format!("shrink-0 h-[{}px]", self.row_height)));
        if open != Some(true) {
            return wrapper;
        }
        let item = &self.data[data_index];
        let name: Vec<String> = self
            .visible
            .iter()
            .filter_map(|&column| self.columns[column].cell_text(item))
            .map(|(_, shown)| shown)
            .collect();
        let panel = div()
            .class(&format!(
                "flex flex-col shrink-0 h-[{height}px] overflow-hidden {}",
                themed(
                    "px-3 py-2 border-b border-border/50 bg-muted/30",
                    "px-1.5 py-1 bg-label/4",
                )
            ))
            .role(Role::Region)
            .label(name.join(" "))
            .part(format!("detail.{index}"))
            .own_tab_stops()
            .child(detail(item));
        wrapper.child(panel)
    }

    /// The chevron that opens or closes the row at `index`.
    fn toggle(&self, index: usize, data_index: usize, open: bool) -> Element<Msg> {
        let chevron = icon(if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        })
        .class("size-4");
        let mut button = div()
            .class(&format!(
                "flex items-center justify-center size-6 rounded-md {}",
                themed(
                    "text-muted-foreground hover:text-foreground hover:bg-muted",
                    "text-secondary-label",
                )
            ))
            .child(chevron);
        if let Some(on_view) = &self.on_view {
            button = button
                .label(if open { "Contraer" } else { "Expandir" })
                .part(format!("row.{index}.toggle"))
                .focus_on_press(false)
                .on_click(on_view(ViewChange::Expand(data_index, !open)));
        }
        div()
            .class(&format!("flex items-center justify-center {TOGGLE_WIDTH}"))
            .role(Role::Cell)
            .child(button)
    }

    /// The [`Role::Row`] of cells shown at `index`; `open` says whether it
    /// is expanded, when the table is expandable.
    fn cells(&self, index: usize, data_index: usize, open: Option<bool>) -> Element<Msg> {
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
            .children(open.map(|open| self.toggle(index, data_index, open)))
            .children(cells);
        if let Some(on_select) = &self.on_select {
            row = row
                .class(&themed("cursor-pointer", "cursor-default"))
                .pressed(is_selected)
                .on_click(on_select(data_index));
        }
        if let Some(open) = open {
            row = row.expanded(open).row_index(index);
            if let Some(on_view) = &self.on_view {
                let on_view = Rc::clone(on_view);
                row = row.on_action(move |action| match action {
                    Action::MoveRight if !open => {
                        Some(on_view(ViewChange::Expand(data_index, true)))
                    }
                    Action::MoveLeft if open => {
                        Some(on_view(ViewChange::Expand(data_index, false)))
                    }
                    _ => None,
                });
            }
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
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
        expandable: bool,
        /// The table is 260 points wide: its columns need more.
        narrow: bool,
    }

    impl Files {
        fn expandable() -> Self {
            Self {
                expandable: true,
                ..Self::default()
            }
        }

        fn narrow() -> Self {
            Self {
                narrow: true,
                ..Self::default()
            }
        }
    }

    const DETAIL: f32 = 120.0;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        View(ViewChange),
        Select(usize),
        Open(usize),
    }

    fn columns() -> Vec<Column<File, Msg>> {
        vec![
            Column::text("Nombre", |file: &File| file.name.clone())
                .min_width(200.0)
                .sortable()
                .filterable(),
            Column::number("Tamaño", |file: &File| file.size as f64)
                .format(|size| format!("{size} KB"))
                .fixed(120.0)
                .sortable()
                .filterable()
                .hideable(),
            Column::text("Tipo", |file: &File| {
                if file.size.is_multiple_of(2) {
                    "Par"
                } else {
                    "Impar"
                }
                .to_owned()
            })
            .hideable()
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
                expandable: false,
                narrow: false,
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
            let mut table = Table::new(Rc::clone(&self.data), columns(), &self.view)
                .selected(self.selected)
                .on_select(Msg::Select)
                .on_view(Msg::View)
                .label("Archivos")
                .test_id("tabla");
            if self.expandable {
                table = table.expandable(
                    |file: &File| {
                        div().class("flex items-center gap-2").children([
                            text(format!("Detalle de {}", file.name)),
                            button("Abrir", ButtonVariant::Text)
                                .on_click(Msg::Open(file.size))
                                .test_id(format!("detalle.{}", file.name)),
                        ])
                    },
                    DETAIL,
                );
            }
            let width = if self.narrow { "w-[260px]" } else { "" };
            div().class("flex flex-col w-full h-full p-4").children([
                text("Archivos"),
                table.build().class(&format!("h-[400px] {width}")),
            ])
        }
    }

    fn rows_built(app: &TestApp<Files>) -> usize {
        app.screen().all(By::role(Role::Row)).len() - 1
    }

    #[test]
    fn the_filter_fields_narrow_each_column_with_the_global_search() {
        let mut files = Files::default();
        files.view.query = "9".into();
        files.view.apply(ViewChange::Filtering(true));
        files.view.refresh(&files.data, &columns());
        let mut app = TestApp::new(files);
        let screen = app.screen();
        let filters = screen.get(By::role(Role::Row).name("Filtros"));
        let fields = screen.within(filters).all(By::role(Role::SearchInput));
        let names: Vec<_> = fields.iter().filter_map(|field| field.name()).collect();
        assert_eq!(
            names,
            ["Filtrar Nombre", "Filtrar Tamaño"],
            "only the filterable columns"
        );

        app.type_text(By::test_id("tabla.filter.0"), "archivo-1");
        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("tabla.filter.0.clear")));
        app.press(Key::Tab, Modifiers::NONE);
        assert!(
            app.is_focused(By::test_id("tabla.filter.1")),
            "each field is a Tab stop"
        );
        app.type_text(By::test_id("tabla.filter.1"), ">=9000");
        let expected = app
            .app()
            .data
            .iter()
            .filter(|file| {
                file.name.contains('9') && file.name.contains("archivo-1") && file.size >= 9_000
            })
            .count();
        assert!(expected > 0);
        let count = |app: &TestApp<Files>| app.screen().get_by_test_id("tabla").node().row_count;
        assert_eq!(count(&app), Some(expected));
        assert_eq!(
            app.app().view.filters.get(&1).map(String::as_str),
            Some(">=9000")
        );

        app.press(Key::Escape, Modifiers::NONE);
        assert!(!app.app().view.filters.contains_key(&1), "Escape clears it");
        let by_name = app
            .app()
            .data
            .iter()
            .filter(|file| file.name.contains('9') && file.name.contains("archivo-1"))
            .count();
        assert_eq!(count(&app), Some(by_name));
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

    fn headers(app: &TestApp<Files>) -> Vec<String> {
        app.screen()
            .all(By::role(Role::ColumnHeader))
            .iter()
            .filter_map(|header| header.name())
            .collect()
    }

    fn menu_items(app: &TestApp<Files>) -> Vec<(String, Option<ControlState>)> {
        app.screen()
            .all(By::role(Role::MenuItemCheckBox))
            .iter()
            .map(|item| (item.name().unwrap(), item.node().control_state))
            .collect()
    }

    #[test]
    fn a_right_click_on_the_header_hides_and_shows_columns() {
        let mut app = TestApp::new(Files::default());
        app.right_click(By::role(Role::ColumnHeader).name("Tamaño"));
        assert_eq!(
            menu_items(&app),
            [
                ("Tamaño".to_owned(), Some(ControlState::On)),
                ("Tipo".to_owned(), Some(ControlState::On)),
            ],
            "only the hideable columns"
        );

        app.click(By::test_id("kai.menu.0"));
        assert_eq!(headers(&app), ["Nombre", "Tipo", "Acciones"]);
        assert_eq!(
            menu_items(&app)[0],
            ("Tamaño".to_owned(), Some(ControlState::Off)),
            "the menu stays open, unchecked"
        );

        // Keys: ↓ and Space hide Tipo, ↑ and Return show Tamaño again.
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.press(Key::Space, Modifiers::NONE);
        assert_eq!(headers(&app), ["Nombre", "Acciones"]);
        app.press(Key::ArrowUp, Modifiers::NONE);
        app.press(Key::Enter, Modifiers::NONE);
        assert_eq!(headers(&app), ["Nombre", "Tamaño", "Acciones"]);
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.screen().query(By::test_id("kai.menu")).is_none());
        assert_eq!(app.app().view.sort, None, "the right click did not sort");
    }

    #[test]
    fn ctrl_click_and_assistive_technology_open_the_column_menu() {
        let mut app = TestApp::new(Files::default());
        app.hold_modifiers(Modifiers {
            control: true,
            ..Modifiers::NONE
        });
        app.click(By::role(Role::ColumnHeader).name("Nombre"));
        app.hold_modifiers(Modifiers::NONE);
        assert_eq!(menu_items(&app).len(), 2);
        assert_eq!(app.app().view.sort, None, "Ctrl+click does not sort");
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.screen().query(By::test_id("kai.menu")).is_none());

        let header = app.screen().get_by_test_id("tabla.header").id();
        let update = app.session().a11y_tree(1.0);
        let node = update
            .nodes
            .iter()
            .find(|(id, _)| *id == crate::a11y::node_id(header))
            .map(|(_, node)| node)
            .unwrap();
        assert!(node.supports_action(accesskit::Action::ShowContextMenu));
        assert!(
            !node.supports_action(accesskit::Action::Click),
            "not a control"
        );
        app.accessibility_action(&accesskit::ActionRequest {
            action: accesskit::Action::ShowContextMenu,
            target_tree: accesskit::TreeId::ROOT,
            target_node: crate::a11y::node_id(header),
            data: None,
        });
        assert_eq!(menu_items(&app).len(), 2);
        let update = app.session().a11y_tree(1.0);
        let checked = update
            .nodes
            .iter()
            .find(|(_, node)| node.role() == accesskit::Role::MenuItemCheckBox)
            .and_then(|(_, node)| node.toggled());
        assert_eq!(checked, Some(accesskit::Toggled::True));
    }

    fn bounds(app: &TestApp<Files>, test_id: &str) -> crate::geometry::Rect {
        app.screen().get(By::test_id(test_id)).bounds()
    }

    /// Name of the panel of data row `index`: its values.
    fn detail_name(app: &TestApp<Files>, index: usize) -> String {
        let file = &app.app().data[index];
        let kind = if file.size.is_multiple_of(2) {
            "Par"
        } else {
            "Impar"
        };
        format!("{} {} KB {kind}", file.name, file.size)
    }

    #[test]
    fn chevrons_open_several_panels_below_their_rows() {
        let mut app = TestApp::new(Files::expandable());
        app.screen()
            .get(By::role(Role::ColumnHeader).name("Detalle"));
        app.click(By::test_id("tabla.row.2.toggle"));
        app.click(By::test_id("tabla.row.5.toggle"));
        assert_eq!(app.app().view.expanded, BTreeSet::from([2, 5]));
        assert_eq!(app.app().selected, None, "the chevron does not select");

        let regions: Vec<_> = app
            .screen()
            .all(By::role(Role::Region))
            .iter()
            .filter_map(|region| region.name())
            .collect();
        assert_eq!(regions, [detail_name(&app, 2), detail_name(&app, 5)]);
        let (row, panel) = (bounds(&app, "tabla.row.2"), bounds(&app, "tabla.detail.2"));
        assert!(
            (panel.y - (row.y + row.height)).abs() < 0.5,
            "under its row"
        );
        assert!((panel.height - DETAIL).abs() < 0.5);
        assert!((bounds(&app, "tabla.row.3").y - (panel.y + DETAIL)).abs() < 0.5);
        let gap = bounds(&app, "tabla.row.6").y - bounds(&app, "tabla.row.5").y;
        assert!((gap - (row.height + DETAIL)).abs() < 0.5, "{gap}");

        app.click(By::test_id("detalle.archivo-2.txt"));
        assert_eq!(app.app().opened, Some(app.app().data[2].size));
        app.click(By::test_id("tabla.row.2.toggle"));
        assert_eq!(app.app().view.expanded, BTreeSet::from([5]));
        assert_eq!(app.screen().all(By::role(Role::Region)).len(), 1);
        assert!(app.screen().query(By::test_id("tabla.detail.2")).is_none());
    }

    #[test]
    fn arrows_open_and_close_the_focused_row_and_tab_enters_its_panel() {
        let mut app = TestApp::new(Files::expandable());
        app.click(By::test_id("tabla.row.1"));
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(app.app().view.expanded, BTreeSet::from([1]));
        assert!(
            app.is_focused(By::test_id("tabla.row.1")),
            "the row keeps the focus"
        );
        app.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(app.app().view.expanded, BTreeSet::from([1]));

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("detalle.archivo-1.txt")));
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(app.app().selected, Some(1), "the panel keeps its arrows");
        assert!(app.is_focused(By::test_id("detalle.archivo-1.txt")));
        let shift = Modifiers {
            shift: true,
            ..Modifiers::NONE
        };
        app.press(Key::Tab, shift);
        assert!(app.is_focused(By::test_id("tabla.row.1")));

        app.press(Key::ArrowLeft, Modifiers::NONE);
        assert!(app.app().view.expanded.is_empty());
        app.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(app.app().selected, Some(2));
        assert!(app.is_focused(By::test_id("tabla.row.2")));
    }

    #[test]
    fn the_keys_reveal_rows_below_open_panels() {
        let mut files = Files::expandable();
        files.view.expanded = (0..5).collect();
        let mut app = TestApp::new(files);
        let body = bounds(&app, "tabla.body");
        let in_view = |row: crate::geometry::Rect| {
            row.y >= body.y - 0.5 && row.y + row.height <= body.y + body.height + 0.5
        };
        app.click(By::test_id("tabla.row.0"));
        app.press(Key::PageDown, Modifiers::NONE);
        let page = app.app().selected.unwrap();
        assert!(
            (1..4).contains(&page),
            "the panels make a page shorter: {page}"
        );
        assert!(in_view(bounds(&app, &format!("tabla.row.{page}"))));

        app.press(Key::End, Modifiers::NONE);
        assert_eq!(app.app().selected, Some(ROWS - 1));
        assert!(in_view(bounds(&app, "tabla.row.9999")));
        app.press(Key::Home, Modifiers::NONE);
        assert!(in_view(bounds(&app, "tabla.row.0")));

        app.reveal_row(By::test_id("tabla.body"), 20);
        let row = bounds(&app, "tabla.row.20");
        assert!(
            (row.y + row.height - (body.y + body.height)).abs() < 0.5,
            "row 20 at the bottom, five panels above it"
        );
        assert!(rows_built(&app) < 40);
    }

    #[test]
    fn assistive_technology_reads_expanded_rows_and_their_panels() {
        let mut files = Files::expandable();
        files.view.expanded.insert(1);
        let app = TestApp::new(files);
        let tree = app.session().a11y_tree(1.0);
        let node = |test_id: &str| {
            let id = crate::a11y::node_id(app.screen().get(By::test_id(test_id)).id());
            tree.nodes
                .iter()
                .find(|(node, _)| *node == id)
                .map(|(_, node)| node.clone())
                .unwrap()
        };
        assert_eq!(node("tabla.row.1").is_expanded(), Some(true));
        assert_eq!(node("tabla.row.0").is_expanded(), Some(false));
        assert_eq!(node("tabla.row.1").row_index(), Some(1));
        let panel = node("tabla.detail.1");
        assert_eq!(panel.role(), accesskit::Role::Region);
        assert_eq!(panel.label(), Some(detail_name(&app, 1).as_str()));
        let toggle = node("tabla.row.1.toggle");
        assert_eq!(toggle.role(), accesskit::Role::Button);
        assert_eq!(toggle.label(), Some("Contraer"));
        assert_eq!(node("tabla.row.0.toggle").label(), Some("Expandir"));
    }

    fn scroller(app: &TestApp<Files>) -> crate::layout::ScrollState {
        app.screen()
            .get(By::test_id("tabla.scroller"))
            .node()
            .scroll
            .unwrap()
    }

    #[test]
    fn a_narrow_table_scrolls_its_header_and_rows_across_together() {
        let mut app = TestApp::new(Files::narrow());
        let over = By::test_id("tabla.scroller");
        let max = scroller(&app).max_x;
        assert!((230.0..=240.0).contains(&max), "496 points in 258: {max}");
        let header = bounds(&app, "tabla.column.1").x;
        let button = bounds(&app, "abrir.archivo-0.txt").x;
        let row = bounds(&app, "tabla.row.0").y;

        app.scroll_wheel_xy(over.clone(), -60.0, 0.0);
        assert_eq!(scroller(&app).offset_x, 60.0);
        assert_eq!(bounds(&app, "tabla.column.1").x, header - 60.0);
        assert_eq!(bounds(&app, "abrir.archivo-0.txt").x, button - 60.0);
        assert_eq!(bounds(&app, "tabla.row.0").y, row, "only across");

        app.scroll_wheel_xy(over.clone(), -10_000.0, 0.0);
        assert_eq!(scroller(&app).offset_x, max, "clamped at the end");
        let view = bounds(&app, "tabla.scroller");
        let last = bounds(&app, "tabla.column.3");
        assert!((last.x + last.width - (view.x + view.width)).abs() < 0.5);

        app.hold_modifiers(Modifiers {
            shift: true,
            ..Modifiers::NONE
        });
        app.scroll_wheel_xy(over.clone(), 0.0, 10_000.0);
        app.hold_modifiers(Modifiers::NONE);
        assert_eq!(scroller(&app).offset_x, 0.0, "Shift turns the wheel across");
        assert_eq!(bounds(&app, "tabla.row.0").y, row);

        app.scroll_wheel(over, -320.0);
        assert_eq!(scroller(&app).offset_x, 0.0);
        let body = app.screen().get(By::test_id("tabla.body")).node().scroll;
        assert_eq!(body.map(|scroll| scroll.offset), Some(320.0));
    }

    #[test]
    fn focus_and_clicks_reach_cells_scrolled_out_of_view() {
        let mut files = Files::narrow();
        files.view.apply(ViewChange::Filtering(true));
        let mut app = TestApp::new(files);
        let view = bounds(&app, "tabla.scroller");
        let inside = |rect: crate::geometry::Rect| {
            rect.x >= view.x - 0.5 && rect.x + rect.width / 2.0 <= view.x + view.width
        };
        assert!(!inside(bounds(&app, "abrir.archivo-2.txt")));
        app.click(By::test_id("abrir.archivo-2.txt"));
        assert_eq!(app.app().opened, Some(app.app().data[2].size));
        assert!(
            inside(bounds(&app, "abrir.archivo-2.txt")),
            "scrolled to it"
        );
        let offset = scroller(&app).offset_x;
        assert!(offset > 0.0);

        app.click(By::test_id("tabla.row.3"));
        assert_eq!(app.app().selected, Some(3));
        assert_eq!(
            scroller(&app).offset_x,
            offset,
            "a wide row keeps the scroll"
        );

        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::role(Role::ColumnHeader).name("Nombre")));
        assert_eq!(scroller(&app).offset_x, 0.0, "the focus brings it back");
        app.press(Key::Tab, Modifiers::NONE);
        app.press(Key::Tab, Modifiers::NONE);
        assert!(app.is_focused(By::test_id("tabla.filter.1")));
        assert!(scroller(&app).offset_x > 0.0);
        assert!(inside(bounds(&app, "tabla.filter.1")));
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
