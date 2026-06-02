#[cfg(feature = "sql")]
use crate::types::*;

#[cfg(feature = "sql")]
pub(crate) fn compute_cell_diffs(old: &SheetData, new: &SheetData) -> Vec<CellDiff> {
    let max_rows = old.rows.len().max(new.rows.len());
    let mut diffs = Vec::new();
    for r in 0..max_rows {
        let old_row = old.rows.get(r);
        let new_row = new.rows.get(r);
        let max_cols = old_row
            .map(|r| r.len())
            .unwrap_or(0)
            .max(new_row.map(|r| r.len()).unwrap_or(0));
        for c in 0..max_cols {
            let old_cell = old_row.and_then(|r| r.get(c));
            let new_cell = new_row.and_then(|r| r.get(c));
            let old_val = old_cell.and_then(|c| c.value.as_deref());
            let new_val = new_cell.and_then(|c| c.value.as_deref());
            let old_fml = old_cell.and_then(|c| c.formula.as_deref());
            let new_fml = new_cell.and_then(|c| c.formula.as_deref());
            if old_val != new_val || old_fml != new_fml {
                diffs.push(CellDiff {
                    row: r as u32,
                    col: c as u16,
                    cell_ref: format!("R{}C{}", r + 1, c + 1),
                    diff_type: if old_cell.is_none() {
                        DiffType::Add
                    } else if new_cell.is_none() {
                        DiffType::Delete
                    } else {
                        DiffType::Modify
                    },
                    old_value: old_val.map(String::from),
                    new_value: new_val.map(String::from),
                    old_formula: old_fml.map(String::from),
                    new_formula: new_fml.map(String::from),
                });
            }
        }
    }
    diffs
}

#[cfg(feature = "sql")]
pub(crate) fn make_diff_summary(diffs: &[CellDiff]) -> DiffSummary {
    let mut summary = DiffSummary {
        adds: 0,
        deletes: 0,
        modifies: 0,
        passives: 0,
        total_changes: diffs.len(),
    };
    for d in diffs {
        match d.diff_type {
            DiffType::Add => summary.adds += 1,
            DiffType::Delete => summary.deletes += 1,
            DiffType::Modify => summary.modifies += 1,
            DiffType::Passive => summary.passives += 1,
            DiffType::NoChange => {}
        }
    }
    summary
}