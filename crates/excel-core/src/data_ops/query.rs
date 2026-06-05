use crate::excel_read::read_sheet_all;
use crate::types::*;

use super::modify::modify_data_file;

pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    let data = read_sheet_all(path, sheet)?;
    let header = data.rows.first().cloned().unwrap_or_default();
    let mut results = vec![header];

    for row in data.rows.iter().skip(1) {
        if matches_all(row, conditions) {
            results.push(row.clone());
        }
    }
    Ok(results)
}

pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        if sd.rows.len() > 1 {
            let header = sd.rows[0].clone();
            let mut body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();

            body.sort_by(|a, b| {
                for sc in sort_columns {
                    let ca = a
                        .get(sc.column as usize)
                        .and_then(|c| c.value.as_deref())
                        .unwrap_or("");
                    let cb = b
                        .get(sc.column as usize)
                        .and_then(|c| c.value.as_deref())
                        .unwrap_or("");
                    let cmp = ca.to_lowercase().cmp(&cb.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        return if sc.descending { cmp.reverse() } else { cmp };
                    }
                }
                std::cmp::Ordering::Equal
            });

            sd.rows.push(header);
            sd.rows.extend(body);
        }
        Ok(new_data)
    })
}

pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        if sd.rows.len() > 1 {
            let header = sd.rows[0].clone();
            let body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();
            let mut seen = std::collections::HashSet::new();
            let cols: Vec<usize> = if columns.is_empty() {
                (0..body.iter().map(|r| r.len()).max().unwrap_or(0)).collect()
            } else {
                columns.iter().map(|c| *c as usize).collect()
            };

            for row in body {
                let key: Vec<String> = cols
                    .iter()
                    .map(|&ci| {
                        row.get(ci)
                            .and_then(|c| c.value.as_deref())
                            .unwrap_or("")
                            .to_string()
                    })
                    .collect();
                if seen.insert(key) {
                    sd.rows.push(row);
                }
            }
            sd.rows.insert(0, header);
        }
        Ok(new_data)
    })
}

fn matches_all(row: &[CellData], conditions: &[FilterCondition]) -> bool {
    conditions.iter().all(|c| matches_one(row, c))
}

fn matches_one(row: &[CellData], cond: &FilterCondition) -> bool {
    let cell_val = row
        .get(cond.column as usize)
        .and_then(|c| c.value.as_deref())
        .unwrap_or("");
    let lower_val = cell_val.to_lowercase();
    let lower_cond = cond.value.to_lowercase();

    match cond.operator {
        FilterOp::Eq => lower_val == lower_cond,
        FilterOp::Ne => lower_val != lower_cond,
        FilterOp::Gt => lower_val > lower_cond,
        FilterOp::Lt => lower_val < lower_cond,
        FilterOp::Ge => lower_val >= lower_cond,
        FilterOp::Le => lower_val <= lower_cond,
        FilterOp::Contains => lower_val.contains(&lower_cond),
        FilterOp::StartsWith => lower_val.starts_with(&lower_cond),
        FilterOp::EndsWith => lower_val.ends_with(&lower_cond),
    }
}