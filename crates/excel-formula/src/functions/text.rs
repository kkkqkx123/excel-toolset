//! Text functions.

use std::collections::HashMap;
use std::sync::Arc;

use excel_types::CellValue;

use crate::engine::DataProvider;
use crate::evaluator::{cell_value_to_string, to_number};

pub fn register(
    registry: &mut HashMap<
        String,
        Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
    >,
) {
    registry.insert("LEN".into(), Arc::new(|args, provider| text_len(args)));
    registry.insert("LEFT".into(), Arc::new(|args, provider| text_left(args)));
    registry.insert("RIGHT".into(), Arc::new(|args, provider| text_right(args)));
    registry.insert("MID".into(), Arc::new(|args, provider| text_mid(args)));
    registry.insert("UPPER".into(), Arc::new(|args, provider| text_upper(args)));
    registry.insert("LOWER".into(), Arc::new(|args, provider| text_lower(args)));
    registry.insert("TRIM".into(), Arc::new(|args, provider| text_trim(args)));
    registry.insert(
        "CONCATENATE".into(),
        Arc::new(|args, provider| text_concat(args)),
    );
    registry.insert(
        "CONCAT".into(),
        Arc::new(|args, provider| text_concat(args)),
    );
    registry.insert("FIND".into(), Arc::new(|args, provider| text_find(args)));
    registry.insert(
        "SEARCH".into(),
        Arc::new(|args, provider| text_search(args)),
    );
    registry.insert(
        "REPLACE".into(),
        Arc::new(|args, provider| text_replace(args)),
    );
    registry.insert(
        "SUBSTITUTE".into(),
        Arc::new(|args, provider| text_substitute(args)),
    );
    registry.insert("TEXT".into(), Arc::new(|args, provider| text_text(args)));
    registry.insert("VALUE".into(), Arc::new(|args, provider| text_value(args)));
    registry.insert("REPT".into(), Arc::new(|args, provider| text_rept(args)));
}

fn text_len(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    CellValue::Number(s.chars().count() as f64)
}

fn text_left(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    let n = args.get(1).and_then(to_number).unwrap_or(1.0) as usize;
    let result: String = s.chars().take(n).collect();
    CellValue::String(result)
}

fn text_right(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    let n = args.get(1).and_then(to_number).unwrap_or(1.0) as usize;
    let len = s.chars().count();
    let result: String = s.chars().skip(len.saturating_sub(n)).collect();
    CellValue::String(result)
}

fn text_mid(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    let start = args.get(1).and_then(to_number).unwrap_or(1.0) as usize;
    let n = args.get(2).and_then(to_number).unwrap_or(1.0) as usize;

    if start == 0 {
        return CellValue::Error("#VALUE!".into());
    }

    let result: String = s.chars().skip(start.saturating_sub(1)).take(n).collect();
    CellValue::String(result)
}

fn text_upper(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    CellValue::String(s.to_uppercase())
}

fn text_lower(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    CellValue::String(s.to_lowercase())
}

fn text_trim(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    // Trim leading/trailing whitespace and collapse internal multi-spaces
    let words: Vec<&str> = s.split_whitespace().collect();
    CellValue::String(words.join(" "))
}

fn text_concat(args: &[CellValue]) -> CellValue {
    let result: String = args.iter().map(cell_value_to_string).collect();
    CellValue::String(result)
}

fn text_find(args: &[CellValue]) -> CellValue {
    // FIND(find_text, within_text, [start_num]) -- case-sensitive
    let find = args.first().map(cell_value_to_string).unwrap_or_default();
    let within = args.get(1).map(cell_value_to_string).unwrap_or_default();
    let start = args.get(2).and_then(to_number).unwrap_or(1.0) as usize;

    if find.is_empty() {
        return CellValue::Number(1.0);
    }

    let start_idx = start.saturating_sub(1);
    if let Some(pos) = within[start_idx..].find(&find) {
        CellValue::Number((start_idx + pos + 1) as f64)
    } else {
        CellValue::Error("#VALUE!".into())
    }
}

fn text_search(args: &[CellValue]) -> CellValue {
    // SEARCH(find_text, within_text, [start_num]) -- case-insensitive
    let find = args
        .first()
        .map(cell_value_to_string)
        .unwrap_or_default()
        .to_lowercase();
    let within = args
        .get(1)
        .map(cell_value_to_string)
        .unwrap_or_default()
        .to_lowercase();
    let start = args.get(2).and_then(to_number).unwrap_or(1.0) as usize;

    if find.is_empty() {
        return CellValue::Number(1.0);
    }

    let start_idx = start.saturating_sub(1);
    if let Some(pos) = within[start_idx..].find(&find) {
        CellValue::Number((start_idx + pos + 1) as f64)
    } else {
        CellValue::Error("#VALUE!".into())
    }
}

fn text_replace(args: &[CellValue]) -> CellValue {
    // REPLACE(old_text, start_num, num_chars, new_text)
    let old_text = args.first().map(cell_value_to_string).unwrap_or_default();
    let start = args.get(1).and_then(to_number).unwrap_or(1.0) as usize;
    let num_chars = args.get(2).and_then(to_number).unwrap_or(0.0) as usize;
    let new_text = args.get(3).map(cell_value_to_string).unwrap_or_default();

    if start == 0 {
        return CellValue::Error("#VALUE!".into());
    }

    let start_idx = start.saturating_sub(1);
    let chars: Vec<char> = old_text.chars().collect();
    let end_idx = (start_idx + num_chars).min(chars.len());

    let mut result = String::new();
    result.extend(&chars[..start_idx]);
    result.push_str(&new_text);
    result.extend(&chars[end_idx..]);

    CellValue::String(result)
}

fn text_substitute(args: &[CellValue]) -> CellValue {
    // SUBSTITUTE(text, old_text, new_text, [instance_num])
    let text = args.first().map(cell_value_to_string).unwrap_or_default();
    let old = args.get(1).map(cell_value_to_string).unwrap_or_default();
    let new = args.get(2).map(cell_value_to_string).unwrap_or_default();
    let instance = args.get(3).and_then(to_number).map(|n| n as usize);

    match instance {
        Some(nth) => {
            // Replace only the nth occurrence
            let mut result = String::new();
            let mut found = 0;
            let mut i = 0;
            while i < text.len() {
                if text[i..].starts_with(&old) {
                    found += 1;
                    if found == nth {
                        result.push_str(&new);
                        i += old.len();
                    } else {
                        result.push_str(&old);
                        i += old.len();
                    }
                } else {
                    result.push(text.chars().nth(i).unwrap_or(' '));
                    i += 1;
                }
            }
            CellValue::String(result)
        }
        None => {
            // Replace all
            CellValue::String(text.replace(&old, &new))
        }
    }
}

fn text_text(args: &[CellValue]) -> CellValue {
    // TEXT(value, format_text) -- simplified implementation
    let val = args.first();
    let fmt = args.get(1).map(cell_value_to_string).unwrap_or_default();

    match val {
        Some(CellValue::Number(n)) => {
            // Simple numeric formatting
            let result = if fmt.contains("0.00") {
                format!("{:.2}", n)
            } else if fmt.contains("0.000") {
                format!("{:.3}", n)
            } else if fmt.contains('#') || fmt.contains('0') {
                if fmt.starts_with('$') || fmt.starts_with('¥') {
                    let prefix = fmt.chars().next().unwrap();
                    format!("{}{}", prefix, n)
                } else {
                    n.to_string()
                }
            } else {
                n.to_string()
            };
            CellValue::String(result)
        }
        _ => CellValue::String(cell_value_to_string(val.unwrap_or(&CellValue::Empty))),
    }
}

fn text_value(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    match s.trim().parse::<f64>() {
        Ok(n) => CellValue::Number(n),
        Err(_) => CellValue::Error("#VALUE!".into()),
    }
}

fn text_rept(args: &[CellValue]) -> CellValue {
    let s = args.first().map(cell_value_to_string).unwrap_or_default();
    let n = args.get(1).and_then(to_number).unwrap_or(0.0) as usize;
    CellValue::String(s.repeat(n.min(32767)))
}
