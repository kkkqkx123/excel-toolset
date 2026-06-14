use csv::ReaderBuilder;

use crate::types::*;

pub fn read_csv_to_cell_values(csv_path: &str) -> Result<Vec<Vec<CellValue>>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_path(csv_path)
        .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

    let mut grid = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| AppError::Io(std::io::Error::other(e)))?;
        let row: Vec<CellValue> = record
            .iter()
            .map(|field| {
                if let Ok(n) = field.parse::<f64>() {
                    CellValue::Number(n)
                } else {
                    CellValue::String(field.to_string())
                }
            })
            .collect();
        grid.push(row);
    }
    Ok(grid)
}

pub fn write_range_from_csv(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    target_range: &str,
    csv_path: &str,
) -> Result<WriteResult> {
    let data = read_csv_to_cell_values(csv_path)?;
    super::operations::write_range(path, params, sheet, target_range, &data)
}
