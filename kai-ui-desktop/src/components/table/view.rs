//! What a [`Table`](super::Table) shows of its data: the global search,
//! the filter of each column and the sort, applied once in `update` and
//! read by the table and by the export.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use super::{Column, SortDirection};
use crate::fold::fold;

/// A calendar day, the value of a date column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

impl fmt::Display for Date {
    /// ISO 8601: `2026-10-20`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The value of a cell: what filters, the global search, the sort and the
/// export use, whatever the cell draws.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    Number(f64),
    Date(Date),
    Bool(bool),
}

impl fmt::Display for Value {
    /// The text a column without [`Column::format`] shows.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::Number(number) if number.fract() == 0.0 && number.abs() < 1e15 => {
                write!(f, "{number:.0}")
            }
            Self::Number(number) => write!(f, "{number}"),
            Self::Date(date) => date.fmt(f),
            Self::Bool(true) => f.write_str("Sí"),
            Self::Bool(false) => f.write_str("No"),
        }
    }
}

/// A change the table asks for; the app applies it with
/// [`TableView::apply`] and refreshes the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewChange {
    /// Sort by a column (its index in the table's columns).
    Sort(usize, SortDirection),
    /// The filter of a column, as typed; empty clears it.
    Filter(usize, String),
    /// Show or hide the filter fields; hiding them clears the filters.
    Filtering(bool),
    /// Hide (`true`) or show a column.
    Hide(usize, bool),
    /// Expand (`true`) or collapse a row, by its index in the data.
    Expand(usize, bool),
}

/// The state of a table the app keeps in its model: global search,
/// filters, sort, hidden columns and expanded rows, and the rows that
/// result. Columns are indices into the table's columns; rows are indices
/// into its data, so the selection and the expanded rows survive sorting
/// and filtering.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableView {
    /// Text searched in every searchable column, hidden ones included.
    pub query: String,
    pub filters: BTreeMap<usize, String>,
    /// The filter fields are shown in the header.
    pub filtering: bool,
    pub sort: Option<(usize, SortDirection)>,
    pub hidden: BTreeSet<usize>,
    pub expanded: BTreeSet<usize>,
    rows: Rc<[usize]>,
}

impl TableView {
    /// The view of `data` with nothing filtered, in data order.
    pub fn new<T, Msg>(data: &[T], columns: &[Column<T, Msg>]) -> Self {
        let mut view = Self::default();
        view.refresh(data, columns);
        view
    }

    /// Indices into the data of the rows shown, in display order.
    pub fn rows(&self) -> &Rc<[usize]> {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Where the row with data index `row` is shown, if it is.
    pub fn position(&self, row: usize) -> Option<usize> {
        self.rows.iter().position(|shown| *shown == row)
    }

    /// Some filter or the global search narrows the rows.
    pub fn is_filtered(&self) -> bool {
        !self.query.trim().is_empty() || self.has_filters()
    }

    /// Some column filter narrows the rows.
    pub fn has_filters(&self) -> bool {
        self.filters.values().any(|text| !text.trim().is_empty())
    }

    /// Apply `change`; call [`Self::refresh`] after it.
    pub fn apply(&mut self, change: ViewChange) {
        match change {
            ViewChange::Sort(column, direction) => self.sort = Some((column, direction)),
            ViewChange::Filter(column, text) => {
                if text.is_empty() {
                    self.filters.remove(&column);
                } else {
                    self.filters.insert(column, text);
                }
            }
            ViewChange::Filtering(on) => {
                self.filtering = on;
                if !on {
                    self.filters.clear();
                }
            }
            ViewChange::Hide(column, true) => {
                self.hidden.insert(column);
            }
            ViewChange::Hide(column, false) => {
                self.hidden.remove(&column);
            }
            ViewChange::Expand(row, true) => {
                self.expanded.insert(row);
            }
            ViewChange::Expand(row, false) => {
                self.expanded.remove(&row);
            }
        }
    }

    /// Recompute the rows of `data`: those that contain the global search
    /// in some searchable column and pass every filter, sorted (stable,
    /// ties in data order).
    pub fn refresh<T, Msg>(&mut self, data: &[T], columns: &[Column<T, Msg>]) {
        let query = fold(self.query.trim());
        let searchable: Vec<&Column<T, Msg>> = columns
            .iter()
            .filter(|column| column.is_searchable())
            .collect();
        let filters: Vec<(&Column<T, Msg>, &str)> = self
            .filters
            .iter()
            .filter_map(|(index, text)| Some((columns.get(*index)?, text.trim())))
            .filter(|(column, text)| column.has_value() && !text.is_empty())
            .collect();
        let mut rows: Vec<usize> = (0..data.len())
            .filter(|&index| {
                let row = &data[index];
                (query.is_empty()
                    || searchable.iter().any(|column| {
                        column
                            .cell_text(row)
                            .is_some_and(|(value, shown)| contains(&value, &shown, &query))
                    }))
                    && filters.iter().all(|(column, filter)| {
                        column
                            .cell_text(row)
                            .is_some_and(|(value, shown)| matches(&value, &shown, filter))
                    })
            })
            .collect();
        if let Some((index, direction)) = self.sort
            && let Some(column) = columns.get(index).filter(|column| column.has_value())
        {
            let keys: Vec<SortKey> = rows
                .iter()
                .map(|&row| SortKey::of(column.value_of(&data[row])))
                .collect();
            let mut order: Vec<usize> = (0..rows.len()).collect();
            order.sort_by(|&a, &b| {
                let by_value = keys[a].cmp(&keys[b]);
                match direction {
                    SortDirection::Ascending => by_value,
                    SortDirection::Descending => by_value.reverse(),
                }
                .then(rows[a].cmp(&rows[b]))
            });
            rows = order.into_iter().map(|i| rows[i]).collect();
        }
        self.rows = rows.into();
    }
}

/// `value` or its `shown` text contains `query` (already folded).
fn contains(value: &Value, shown: &str, query: &str) -> bool {
    let raw = match value {
        Value::Text(text) => fold(text),
        value => fold(&value.to_string()),
    };
    raw.contains(query) || fold(shown).contains(query)
}

/// Whether a cell passes the filter of its column. Text contains it;
/// numbers take `12`, `>12`, `>=12`, `<12`, `<=12` or `10..20`; dates a
/// prefix (`2026-10`) or a range (`2026-01-01..2026-03-31`); booleans
/// «sí» or «no». Anything else looks for the text in what the cell shows.
fn matches(value: &Value, shown: &str, filter: &str) -> bool {
    let folded = fold(filter);
    match value {
        Value::Number(number) => {
            number_filter(*number, filter).unwrap_or_else(|| contains(value, shown, &folded))
        }
        Value::Date(date) => {
            let iso = date.to_string();
            match filter.split_once("..") {
                Some((from, to)) => {
                    let (from, to) = (from.trim(), to.trim());
                    (from.is_empty() || iso.as_str() >= from)
                        && (to.is_empty() || iso.as_str() <= to || iso.starts_with(to))
                }
                None => iso.starts_with(filter) || contains(value, shown, &folded),
            }
        }
        Value::Bool(flag) => match folded.as_str() {
            "si" | "true" | "1" => *flag,
            "no" | "false" | "0" => !*flag,
            _ => contains(value, shown, &folded),
        },
        Value::Text(_) => contains(value, shown, &folded),
    }
}

/// `number` against a numeric filter, or `None` when `filter` is not one.
fn number_filter(number: f64, filter: &str) -> Option<bool> {
    let parse = |text: &str| text.trim().replace(',', ".").parse::<f64>().ok();
    let filter = filter.trim();
    if let Some((from, to)) = filter.split_once("..") {
        let from = if from.trim().is_empty() {
            f64::NEG_INFINITY
        } else {
            parse(from)?
        };
        let to = if to.trim().is_empty() {
            f64::INFINITY
        } else {
            parse(to)?
        };
        return Some(number >= from && number <= to);
    }
    for (operator, test) in [
        (">=", (|a: f64, b: f64| a >= b) as fn(f64, f64) -> bool),
        ("<=", |a, b| a <= b),
        (">", |a, b| a > b),
        ("<", |a, b| a < b),
        ("=", |a, b| a == b),
    ] {
        if let Some(rest) = filter.strip_prefix(operator) {
            return Some(test(number, parse(rest)?));
        }
    }
    parse(filter).map(|target| number == target)
}

/// What a sorted column compares: text folded, numbers by their total
/// order; rows without a value go last.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SortKey {
    Bool(bool),
    Date(Date),
    Number(Total),
    Text(String),
    None,
}

impl SortKey {
    fn of(value: Option<Value>) -> Self {
        match value {
            Some(Value::Text(text)) => Self::Text(fold(&text)),
            Some(Value::Number(number)) => Self::Number(Total(number)),
            Some(Value::Date(date)) => Self::Date(date),
            Some(Value::Bool(flag)) => Self::Bool(flag),
            None => Self::None,
        }
    }
}

/// An `f64` ordered by [`f64::total_cmp`].
struct Total(f64);

impl PartialEq for Total {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Total {}

impl PartialOrd for Total {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Total {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Date, TableView, Value, ViewChange, matches, number_filter};
    use crate::components::{Column, SortDirection};
    use crate::element::{Element, text};

    struct Cliente {
        nombre: &'static str,
        ciudad: &'static str,
        plan: &'static str,
        monto: f64,
        alta: Date,
        activo: bool,
    }

    const fn cliente(
        nombre: &'static str,
        ciudad: &'static str,
        plan: &'static str,
        monto: f64,
        alta: (i32, u8, u8),
        activo: bool,
    ) -> Cliente {
        Cliente {
            nombre,
            ciudad,
            plan,
            monto,
            alta: Date {
                year: alta.0,
                month: alta.1,
                day: alta.2,
            },
            activo,
        }
    }

    const DATOS: [Cliente; 6] = [
        cliente("Ana Rojas", "Temuco", "Pro", 1200.0, (2026, 1, 10), true),
        cliente(
            "Bruno Soto",
            "Valparaíso",
            "Básico",
            300.0,
            (2025, 11, 2),
            false,
        ),
        cliente(
            "Carla Vega",
            "Temuco",
            "Empresa",
            9800.0,
            (2026, 3, 5),
            true,
        ),
        cliente("Diego Muñoz", "Arica", "Pro", 450.5, (2026, 10, 1), true),
        cliente(
            "Elena Silva",
            "Temuco",
            "Básico",
            300.0,
            (2024, 6, 30),
            false,
        ),
        cliente(
            "Ángela Pro",
            "Santiago",
            "Básico",
            75.0,
            (2026, 10, 20),
            true,
        ),
    ];

    fn columnas() -> Vec<Column<Cliente, ()>> {
        vec![
            Column::text("Nombre", |c: &Cliente| c.nombre.to_owned()),
            Column::text("Ciudad", |c: &Cliente| c.ciudad.to_owned()).filterable(),
            Column::text("Plan", |c: &Cliente| c.plan.to_owned())
                .render(|c: &Cliente| text(format!("[{}]", c.plan)))
                .filterable(),
            Column::number("Monto", |c: &Cliente| c.monto)
                .format(|value| format!("$ {value}"))
                .filterable(),
            Column::date("Alta", |c: &Cliente| c.alta).filterable(),
            Column::boolean("Activo", |c: &Cliente| c.activo).filterable(),
            Column::actions(|_: &Cliente| -> Element<()> { text("Ver") }),
        ]
    }

    fn nombres(view: &TableView) -> Vec<&'static str> {
        view.rows().iter().map(|&row| DATOS[row].nombre).collect()
    }

    #[test]
    fn search_filters_and_sort_combine() {
        let columns = columnas();
        let mut view = TableView::new(&DATOS, &columns);
        assert_eq!(view.len(), 6);

        view.query = "temuco".into();
        view.apply(ViewChange::Filter(3, ">=300".into()));
        view.apply(ViewChange::Sort(3, SortDirection::Descending));
        view.refresh(&DATOS, &columns);
        assert_eq!(nombres(&view), ["Carla Vega", "Ana Rojas", "Elena Silva"]);

        view.apply(ViewChange::Filter(2, "basico".into()));
        view.refresh(&DATOS, &columns);
        assert_eq!(
            nombres(&view),
            ["Elena Silva"],
            "the badge filters by its value"
        );

        view.apply(ViewChange::Filtering(false));
        view.refresh(&DATOS, &columns);
        assert!(view.filters.is_empty());
        assert_eq!(view.len(), 3, "only the search is left");
    }

    #[test]
    fn the_search_ignores_case_and_accents_and_looks_in_hidden_columns() {
        let columns = columnas();
        let mut view = TableView::new(&DATOS, &columns);
        view.apply(ViewChange::Hide(1, true));
        view.query = "VALPARAISO".into();
        view.refresh(&DATOS, &columns);
        assert_eq!(nombres(&view), ["Bruno Soto"]);

        view.query = "angela".into();
        view.refresh(&DATOS, &columns);
        assert_eq!(nombres(&view), ["Ángela Pro"]);
    }

    #[test]
    fn the_search_only_looks_in_text_columns() {
        let columns = columnas();
        let mut view = TableView::new(&DATOS, &columns);
        view.query = "1200".into();
        view.refresh(&DATOS, &columns);
        assert!(view.is_empty(), "amounts are not searched by default");
    }

    #[test]
    fn sorting_is_stable_and_folds_text() {
        let columns = columnas();
        let mut view = TableView::new(&DATOS, &columns);
        view.apply(ViewChange::Sort(0, SortDirection::Ascending));
        view.refresh(&DATOS, &columns);
        assert_eq!(nombres(&view)[0], "Ana Rojas");
        assert_eq!(nombres(&view)[1], "Ángela Pro", "Á sorts as A");

        view.apply(ViewChange::Sort(3, SortDirection::Ascending));
        view.refresh(&DATOS, &columns);
        assert_eq!(&nombres(&view)[1..3], ["Bruno Soto", "Elena Silva"]);
        view.apply(ViewChange::Sort(3, SortDirection::Descending));
        view.refresh(&DATOS, &columns);
        assert_eq!(
            &nombres(&view)[3..5],
            ["Bruno Soto", "Elena Silva"],
            "ties keep the data order both ways"
        );
    }

    #[test]
    fn the_selection_by_data_index_survives_sorting() {
        let columns = columnas();
        let mut view = TableView::new(&DATOS, &columns);
        let carla = 2;
        assert_eq!(view.position(carla), Some(2));
        view.apply(ViewChange::Sort(3, SortDirection::Descending));
        view.refresh(&DATOS, &columns);
        assert_eq!(view.position(carla), Some(0));
        view.query = "arica".into();
        view.refresh(&DATOS, &columns);
        assert_eq!(view.position(carla), None);
    }

    #[test]
    fn filters_by_type() {
        let number = Value::Number(450.5);
        assert!(matches(&number, "$ 450.5", "450,5"));
        assert!(matches(&number, "$ 450.5", "400..500"));
        assert!(matches(&number, "$ 450.5", "..500"));
        assert!(!matches(&number, "$ 450.5", "<400"));
        assert!(
            matches(&number, "$ 450.5", "$ 45"),
            "not a number: the text"
        );
        assert_eq!(number_filter(3.0, "abc"), None);

        let date = Value::Date(Date::new(2026, 10, 1));
        assert!(matches(&date, "2026-10-01", "2026-10"));
        assert!(matches(&date, "2026-10-01", "2026-09-01..2026-10"));
        assert!(!matches(&date, "2026-10-01", "2026-01-01..2026-03-31"));

        let yes = Value::Bool(true);
        assert!(matches(&yes, "Sí", "sí"));
        assert!(!matches(&yes, "Sí", "no"));

        let columns = columnas();
        let mut view = TableView::new(&DATOS, &columns);
        view.apply(ViewChange::Filter(5, "no".into()));
        view.apply(ViewChange::Filter(4, "2024..2025-12-31".into()));
        view.refresh(&DATOS, &columns);
        assert_eq!(nombres(&view), ["Bruno Soto", "Elena Silva"]);
    }

    #[test]
    fn actions_have_no_value_and_ignore_filters_and_sort() {
        let columns = columnas();
        let acciones = &columns[6];
        assert!(!acciones.has_value());
        assert_eq!(acciones.value_of(&DATOS[0]), None);
        let mut view = TableView::new(&DATOS, &columns);
        view.apply(ViewChange::Filter(6, "ver".into()));
        view.apply(ViewChange::Sort(6, SortDirection::Descending));
        view.refresh(&DATOS, &columns);
        assert_eq!(view.len(), 6);
        assert_eq!(view.rows()[0], 0, "data order");
    }

    #[test]
    fn values_print_as_people_read_them() {
        assert_eq!(Value::Number(1200.0).to_string(), "1200");
        assert_eq!(Value::Number(450.5).to_string(), "450.5");
        assert_eq!(Value::Date(Date::new(2026, 3, 5)).to_string(), "2026-03-05");
        assert_eq!(Value::Bool(false).to_string(), "No");
    }
}
