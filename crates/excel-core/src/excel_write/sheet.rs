use std::collections::HashMap;

use crate::types::*;

pub fn add(data: &mut HashMap<String, SheetData>, name: &str) -> Result<()> {
    if data.contains_key(name) {
        return Err(AppError::Custom(format!("Sheet '{}' already exists", name)));
    }
    data.insert(
        name.to_string(),
        SheetData {
            name: name.to_string(),
            rows: Vec::new(),
        },
    );
    Ok(())
}

pub fn delete(data: &mut HashMap<String, SheetData>, name: &str) -> Result<()> {
    if !data.contains_key(name) {
        return Err(AppError::Custom(format!("Sheet '{}' not found", name)));
    }
    data.remove(name);
    Ok(())
}

pub fn rename(data: &mut HashMap<String, SheetData>, old_name: &str, new_name: &str) -> Result<()> {
    if !data.contains_key(old_name) {
        return Err(AppError::Custom(format!("Sheet '{}' not found", old_name)));
    }
    if data.contains_key(new_name) {
        return Err(AppError::Custom(format!(
            "Sheet '{}' already exists",
            new_name
        )));
    }
    if let Some(mut sd) = data.remove(old_name) {
        sd.name = new_name.to_string();
        data.insert(new_name.to_string(), sd);
    }
    Ok(())
}

pub fn sort(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    columns: &[SortColumn],
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;
    if sd.rows.len() > 1 {
        let header = sd.rows[0].clone();
        let mut body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();
        body.sort_by(|a, b| {
            for sc in columns {
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
    Ok(())
}

pub fn dedup(data: &mut HashMap<String, SheetData>, sheet: &str, columns: &[u16]) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;
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
    Ok(())
}
