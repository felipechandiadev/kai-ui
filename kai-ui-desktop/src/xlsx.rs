//! Excel files (`.xlsx`) of a [`Sheet`], such as the rows a table shows
//! ([`TableView::export`]): bold titles in a frozen first row with an
//! autofilter, and numbers, dates and booleans with their Excel type.
//!
//! [`TableView::export`]: crate::components::TableView::export

use std::path::Path;

use rust_xlsxwriter::{ExcelDateTime, Format, Workbook, XlsxError};

use crate::components::{Sheet, Value};

/// Excel takes at most this many rows, the titles included.
const MAX_ROWS: usize = 1_048_576;

/// Write `sheet` to `path` as a one-sheet workbook named `name`, replacing
/// the file. Slow for large sheets: call it from `Effects::perform`.
pub fn write(sheet: &Sheet, name: &str, path: &Path) -> Result<(), String> {
    if sheet.rows.len() >= MAX_ROWS {
        return Err(format!(
            "Excel admite hasta {} filas; hay {}",
            MAX_ROWS - 1,
            sheet.rows.len()
        ));
    }
    workbook(sheet, name)
        .and_then(|mut workbook| workbook.save(path))
        .map_err(|error| format!("No se pudo escribir el Excel: {error}"))
}

fn workbook(sheet: &Sheet, name: &str) -> Result<Workbook, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(name)?;
    let title = Format::new().set_bold();
    let date = Format::new().set_num_format("yyyy-mm-dd");
    for (column, heading) in (0u16..).zip(&sheet.columns) {
        worksheet.write_string_with_format(0, column, heading, &title)?;
    }
    for (row, values) in (1u32..).zip(&sheet.rows) {
        for (column, value) in (0u16..).zip(values) {
            match value {
                Value::Text(text) => worksheet.write_string(row, column, text)?,
                Value::Number(number) => worksheet.write_number(row, column, *number)?,
                Value::Bool(on) => worksheet.write_boolean(row, column, *on)?,
                Value::Date(day) => {
                    let when = u16::try_from(day.year)
                        .ok()
                        .and_then(|year| ExcelDateTime::from_ymd(year, day.month, day.day).ok());
                    match when {
                        Some(when) => {
                            worksheet.write_datetime_with_format(row, column, when, &date)?
                        }
                        None => worksheet.write_string(row, column, day.to_string())?,
                    }
                }
            };
        }
    }
    if let Some(last) = sheet.columns.len().checked_sub(1) {
        let last = u16::try_from(last).unwrap_or(u16::MAX);
        let rows = u32::try_from(sheet.rows.len()).unwrap_or(u32::MAX);
        worksheet.set_freeze_panes(1, 0)?;
        worksheet.autofilter(0, 0, rows, last)?;
        worksheet.autofit();
    }
    Ok(workbook)
}

#[cfg(test)]
mod tests {
    use super::write;
    use crate::components::{Date, Sheet, Value};

    #[test]
    fn a_sheet_is_saved_as_an_xlsx_file() {
        let sheet = Sheet {
            columns: vec![
                "Nombre".into(),
                "Monto".into(),
                "Alta".into(),
                "Activo".into(),
            ],
            rows: vec![vec![
                Value::Text("Ana".into()),
                Value::Number(1200.5),
                Value::Date(Date::new(2026, 10, 20)),
                Value::Bool(true),
            ]],
        };
        let dir = std::env::temp_dir().join(format!("kai-xlsx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hoja.xlsx");
        write(&sheet, "Clientes", &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"PK"), "a zip file");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_bad_sheet_name_is_an_error() {
        let path = std::env::temp_dir().join("kai-xlsx-no.xlsx");
        let error = write(&Sheet::default(), "a/b", &path).unwrap_err();
        assert!(error.starts_with("No se pudo escribir el Excel"), "{error}");
        assert!(!path.exists());
    }
}
