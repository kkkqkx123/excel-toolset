//! Logical functions.

use std::collections::HashMap;
use std::sync::Arc;

use excel_types::CellValue;

use crate::engine::DataProvider;

pub fn register(
    registry: &mut HashMap<
        String,
        Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
    >,
) {
    registry.insert("IF".into(), Arc::new(|args, provider| logical_if(args)));
    registry.insert("AND".into(), Arc::new(|args, provider| logical_and(args)));
    registry.insert("OR".into(), Arc::new(|args, provider| logical_or(args)));
    registry.insert("NOT".into(), Arc::new(|args, provider| logical_not(args)));
    registry.insert(
        "IFERROR".into(),
        Arc::new(|args, provider| logical_iferror(args)),
    );
    registry.insert("IFNA".into(), Arc::new(|args, provider| logical_ifna(args)));
    registry.insert(
        "ISBLANK".into(),
        Arc::new(|args, provider| logical_isblank(args)),
    );
    registry.insert(
        "ISERROR".into(),
        Arc::new(|args, provider| logical_iserror(args)),
    );
    registry.insert(
        "ISNUMBER".into(),
        Arc::new(|args, provider| logical_isnumber(args)),
    );
    registry.insert(
        "ISTEXT".into(),
        Arc::new(|args, provider| logical_istext(args)),
    );
    registry.insert(
        "ISLOGICAL".into(),
        Arc::new(|args, provider| logical_islogical(args)),
    );
    registry.insert("ISNA".into(), Arc::new(|args, provider| logical_isna(args)));
    registry.insert(
        "TRUE".into(),
        Arc::new(|_args, _provider| CellValue::Bool(true)),
    );
    registry.insert(
        "FALSE".into(),
        Arc::new(|_args, _provider| CellValue::Bool(false)),
    );
    registry.insert("XOR".into(), Arc::new(|args, provider| logical_xor(args)));
    registry.insert(
        "SWITCH".into(),
        Arc::new(|args, provider| logical_switch(args)),
    );
    registry.insert("IFS".into(), Arc::new(|args, provider| logical_ifs(args)));
}

/// Convert a CellValue to a boolean (Excel truthiness rules).
/// 0 = FALSE, empty string = FALSE, error = FALSE, anything else = TRUE
fn to_bool(val: &CellValue) -> bool {
    match val {
        CellValue::Bool(b) => *b,
        CellValue::Number(n) => *n != 0.0,
        CellValue::String(s) => !s.is_empty() && s.to_uppercase() != "FALSE",
        CellValue::Empty => false,
        CellValue::Error(_) => false,
        CellValue::DateTime(_) => true,
    }
}

fn logical_if(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }
    let condition = to_bool(&args[0]);
    if condition {
        args.get(1).cloned().unwrap_or(CellValue::Bool(true))
    } else {
        args.get(2).cloned().unwrap_or(CellValue::Bool(false))
    }
}

fn logical_and(args: &[CellValue]) -> CellValue {
    let all_true = args.iter().all(to_bool);
    CellValue::Bool(all_true && !args.is_empty())
}

fn logical_or(args: &[CellValue]) -> CellValue {
    let any_true = args.iter().any(to_bool);
    CellValue::Bool(any_true)
}

fn logical_not(args: &[CellValue]) -> CellValue {
    match args.first() {
        Some(v) => CellValue::Bool(!to_bool(v)),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn logical_iferror(args: &[CellValue]) -> CellValue {
    match args.first() {
        Some(CellValue::Error(_)) => args.get(1).cloned().unwrap_or(CellValue::Empty),
        Some(val) => val.clone(),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn logical_ifna(args: &[CellValue]) -> CellValue {
    match args.first() {
        Some(CellValue::Error(e)) if e == "#N/A" => {
            args.get(1).cloned().unwrap_or(CellValue::Empty)
        }
        Some(val) => val.clone(),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn logical_isblank(args: &[CellValue]) -> CellValue {
    CellValue::Bool(args.first().map_or(true, |v| matches!(v, CellValue::Empty)))
}

fn logical_iserror(args: &[CellValue]) -> CellValue {
    CellValue::Bool(
        args.first()
            .map_or(false, |v| matches!(v, CellValue::Error(_))),
    )
}

fn logical_isnumber(args: &[CellValue]) -> CellValue {
    CellValue::Bool(
        args.first()
            .map_or(false, |v| matches!(v, CellValue::Number(_))),
    )
}

fn logical_istext(args: &[CellValue]) -> CellValue {
    CellValue::Bool(
        args.first()
            .map_or(false, |v| matches!(v, CellValue::String(_))),
    )
}

fn logical_islogical(args: &[CellValue]) -> CellValue {
    CellValue::Bool(
        args.first()
            .map_or(false, |v| matches!(v, CellValue::Bool(_))),
    )
}

fn logical_isna(args: &[CellValue]) -> CellValue {
    CellValue::Bool(
        args.first()
            .map_or(false, |v| matches!(v, CellValue::Error(e) if e == "#N/A")),
    )
}

fn logical_xor(args: &[CellValue]) -> CellValue {
    let count = args.iter().filter(|v| to_bool(v)).count();
    CellValue::Bool(count % 2 == 1)
}

fn logical_switch(args: &[CellValue]) -> CellValue {
    // SWITCH(expression, value1, result1, [value2, result2, ...], [default])
    if args.len() < 3 {
        return CellValue::Error("#VALUE!".into());
    }

    let expr = &args[0];
    let pairs_count = (args.len() - 1) / 2;

    for i in 0..pairs_count {
        let val_idx = 1 + i * 2;
        let result_idx = val_idx + 1;
        if &args[val_idx] == expr {
            return args[result_idx].clone();
        }
    }

    // Check for default value (odd number of remaining args)
    if (args.len() - 1) % 2 == 1 {
        args.last()
            .cloned()
            .unwrap_or(CellValue::Error("#N/A".into()))
    } else {
        CellValue::Error("#N/A".into())
    }
}

fn logical_ifs(args: &[CellValue]) -> CellValue {
    // IFS(logical_test1, value1, [logical_test2, value2], ...)
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    for i in (0..args.len()).step_by(2) {
        if i + 1 < args.len() {
            if to_bool(&args[i]) {
                return args[i + 1].clone();
            }
        }
    }

    CellValue::Error("#N/A".into())
}
