use excel_core::types::{DiffSummary, DiffType, SheetDiff};

pub(crate) fn summarize(sheet_diffs: &[SheetDiff]) -> DiffSummary {
    let mut adds = 0;
    let mut deletes = 0;
    let mut modifies = 0;
    let mut passives = 0;

    for sd in sheet_diffs {
        for cd in &sd.cell_diffs {
            match cd.diff_type {
                DiffType::Add => adds += 1,
                DiffType::Delete => deletes += 1,
                DiffType::Modify => modifies += 1,
                DiffType::Passive => passives += 1,
                DiffType::NoChange => {}
            }
        }
    }

    let total_changes = adds + deletes + modifies + passives;

    DiffSummary {
        adds,
        deletes,
        modifies,
        passives,
        total_changes,
    }
}
