//! Mathematical and trigonometric functions.

use std::collections::HashMap;
use std::sync::Arc;

use excel_types::CellValue;

use crate::engine::DataProvider;
use crate::evaluator::to_number;

pub fn register(
    registry: &mut HashMap<
        String,
        Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
    >,
) {
    // Basic math
    registry.insert("ABS".into(), Arc::new(|args, provider| math_abs(args)));
    registry.insert("SUM".into(), Arc::new(|args, provider| math_sum(args)));
    registry.insert(
        "AVERAGE".into(),
        Arc::new(|args, provider| math_average(args)),
    );
    registry.insert("COUNT".into(), Arc::new(|args, provider| math_count(args)));
    registry.insert(
        "COUNTA".into(),
        Arc::new(|args, provider| math_counta(args)),
    );
    registry.insert("MIN".into(), Arc::new(|args, provider| math_min(args)));
    registry.insert("MAX".into(), Arc::new(|args, provider| math_max(args)));
    registry.insert(
        "PRODUCT".into(),
        Arc::new(|args, provider| math_product(args)),
    );
    registry.insert("ROUND".into(), Arc::new(|args, provider| math_round(args)));
    registry.insert(
        "ROUNDUP".into(),
        Arc::new(|args, provider| math_roundup(args)),
    );
    registry.insert(
        "ROUNDDOWN".into(),
        Arc::new(|args, provider| math_rounddown(args)),
    );
    registry.insert("SQRT".into(), Arc::new(|args, provider| math_sqrt(args)));
    registry.insert("POWER".into(), Arc::new(|args, provider| math_power(args)));
    registry.insert("MOD".into(), Arc::new(|args, provider| math_mod(args)));
    registry.insert("INT".into(), Arc::new(|args, provider| math_int(args)));
    registry.insert("TRUNC".into(), Arc::new(|args, provider| math_trunc(args)));

    // Aggregate with criteria
    registry.insert("SUMIF".into(), Arc::new(|args, provider| math_sumif(args)));
    registry.insert(
        "SUMIFS".into(),
        Arc::new(|args, provider| math_sumifs(args)),
    );
    registry.insert(
        "COUNTIF".into(),
        Arc::new(|args, provider| math_countif(args)),
    );
    registry.insert(
        "COUNTIFS".into(),
        Arc::new(|args, provider| math_countifs(args)),
    );
    registry.insert(
        "AVERAGEIF".into(),
        Arc::new(|args, provider| math_averageif(args)),
    );

    // Rounding
    registry.insert(
        "CEILING".into(),
        Arc::new(|args, provider| math_ceiling(args)),
    );
    registry.insert("FLOOR".into(), Arc::new(|args, provider| math_floor(args)));
    registry.insert("EVEN".into(), Arc::new(|args, provider| math_even(args)));
    registry.insert("ODD".into(), Arc::new(|args, provider| math_odd(args)));

    // Trigonometry
    registry.insert(
        "PI".into(),
        Arc::new(|_args, _provider| CellValue::Number(std::f64::consts::PI)),
    );
    registry.insert("SIN".into(), Arc::new(|args, provider| math_sin(args)));
    registry.insert("COS".into(), Arc::new(|args, provider| math_cos(args)));
    registry.insert("TAN".into(), Arc::new(|args, provider| math_tan(args)));
    registry.insert("ASIN".into(), Arc::new(|args, provider| math_asin(args)));
    registry.insert("ACOS".into(), Arc::new(|args, provider| math_acos(args)));
    registry.insert("ATAN".into(), Arc::new(|args, provider| math_atan(args)));
    registry.insert("ATAN2".into(), Arc::new(|args, provider| math_atan2(args)));
    registry.insert(
        "DEGREES".into(),
        Arc::new(|args, provider| math_degrees(args)),
    );
    registry.insert(
        "RADIANS".into(),
        Arc::new(|args, provider| math_radians(args)),
    );

    // Random
    registry.insert(
        "RAND".into(),
        Arc::new(|_args, _provider| CellValue::Number(rand::random::<f64>())),
    );
    registry.insert(
        "RANDBETWEEN".into(),
        Arc::new(|args, provider| math_randbetween(args)),
    );

    // Log/Exp
    registry.insert("EXP".into(), Arc::new(|args, provider| math_exp(args)));
    registry.insert("LN".into(), Arc::new(|args, provider| math_ln(args)));
    registry.insert("LOG".into(), Arc::new(|args, provider| math_log(args)));
    registry.insert("LOG10".into(), Arc::new(|args, provider| math_log10(args)));

    // GCD/LCM
    registry.insert("GCD".into(), Arc::new(|args, provider| math_gcd(args)));
    registry.insert("LCM".into(), Arc::new(|args, provider| math_lcm(args)));

    // SUBTOTAL
    registry.insert(
        "SUBTOTAL".into(),
        Arc::new(|args, provider| math_subtotal(args)),
    );
}

fn flatten_numbers(args: &[CellValue]) -> Vec<f64> {
    let mut numbers = Vec::new();
    for arg in args {
        match arg {
            CellValue::Number(n) => numbers.push(*n),
            CellValue::String(s) => {
                if let Ok(n) = s.parse::<f64>() {
                    numbers.push(n);
                }
            }
            CellValue::Bool(true) => numbers.push(1.0),
            CellValue::Bool(false) => numbers.push(0.0),
            _ => {}
        }
    }
    numbers
}

fn math_abs(args: &[CellValue]) -> CellValue {
    if let Some(n) = args.first().and_then(to_number) {
        CellValue::Number(n.abs())
    } else {
        CellValue::Error("#VALUE!".into())
    }
}

fn math_sum(args: &[CellValue]) -> CellValue {
    let nums = flatten_numbers(args);
    CellValue::Number(nums.iter().sum())
}

fn math_average(args: &[CellValue]) -> CellValue {
    let nums = flatten_numbers(args);
    if nums.is_empty() {
        CellValue::Error("#DIV/0!".into())
    } else {
        CellValue::Number(nums.iter().sum::<f64>() / nums.len() as f64)
    }
}

fn math_count(args: &[CellValue]) -> CellValue {
    let count = args
        .iter()
        .filter(|a| matches!(a, CellValue::Number(_) | CellValue::DateTime(_)))
        .count();
    CellValue::Number(count as f64)
}

fn math_counta(args: &[CellValue]) -> CellValue {
    let count = args
        .iter()
        .filter(|a| !matches!(a, CellValue::Empty))
        .count();
    CellValue::Number(count as f64)
}

fn math_min(args: &[CellValue]) -> CellValue {
    let nums = flatten_numbers(args);
    if nums.is_empty() {
        CellValue::Number(0.0)
    } else {
        CellValue::Number(nums.iter().copied().fold(f64::INFINITY, f64::min))
    }
}

fn math_max(args: &[CellValue]) -> CellValue {
    let nums = flatten_numbers(args);
    if nums.is_empty() {
        CellValue::Number(0.0)
    } else {
        CellValue::Number(nums.iter().copied().fold(f64::NEG_INFINITY, f64::max))
    }
}

fn math_product(args: &[CellValue]) -> CellValue {
    let nums = flatten_numbers(args);
    if nums.is_empty() {
        CellValue::Number(0.0)
    } else {
        CellValue::Number(nums.iter().product())
    }
}

fn math_round(args: &[CellValue]) -> CellValue {
    let n = args.first().and_then(to_number);
    let digits = args.get(1).and_then(to_number).unwrap_or(0.0) as i32;
    match n {
        Some(val) => {
            let factor = 10_f64.powi(digits);
            CellValue::Number((val * factor).round() / factor)
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_roundup(args: &[CellValue]) -> CellValue {
    let n = args.first().and_then(to_number);
    let digits = args.get(1).and_then(to_number).unwrap_or(0.0) as i32;
    match n {
        Some(val) => {
            let factor = 10_f64.powi(digits);
            let scaled = val * factor;
            CellValue::Number(scaled.ceil() / factor)
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_rounddown(args: &[CellValue]) -> CellValue {
    let n = args.first().and_then(to_number);
    let digits = args.get(1).and_then(to_number).unwrap_or(0.0) as i32;
    match n {
        Some(val) => {
            let factor = 10_f64.powi(digits);
            let scaled = val * factor;
            CellValue::Number(scaled.floor() / factor)
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_sqrt(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) if n >= 0.0 => CellValue::Number(n.sqrt()),
        Some(_) => CellValue::Error("#NUM!".into()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_power(args: &[CellValue]) -> CellValue {
    let base = args.first().and_then(to_number);
    let exp = args.get(1).and_then(to_number);
    match (base, exp) {
        (Some(b), Some(e)) => CellValue::Number(b.powf(e)),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn math_int(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.floor()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_trunc(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.trunc()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_mod(args: &[CellValue]) -> CellValue {
    let a = args.first().and_then(to_number);
    let b = args.get(1).and_then(to_number);
    match (a, b) {
        (Some(x), Some(y)) if y != 0.0 => CellValue::Number(x - (x / y).floor() * y),
        (Some(_), Some(_)) => CellValue::Error("#DIV/0!".into()),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn math_ceiling(args: &[CellValue]) -> CellValue {
    let n = args.first().and_then(to_number);
    let significance = args.get(1).and_then(to_number).unwrap_or(1.0);
    match n {
        Some(val) if significance != 0.0 => {
            CellValue::Number((val / significance).ceil() * significance)
        }
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn math_floor(args: &[CellValue]) -> CellValue {
    let n = args.first().and_then(to_number);
    let significance = args.get(1).and_then(to_number).unwrap_or(1.0);
    match n {
        Some(val) if significance != 0.0 => {
            CellValue::Number((val / significance).floor() * significance)
        }
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn math_even(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => {
            let sign = if n >= 0.0 { 1.0 } else { -1.0 };
            CellValue::Number(sign * (n.abs() / 2.0).ceil() * 2.0)
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_odd(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => {
            let sign = if n >= 0.0 { 1.0 } else { -1.0 };
            let result = sign * (n.abs() / 2.0).ceil() * 2.0;
            if result % 2.0 == 0.0 {
                CellValue::Number(result + sign)
            } else {
                CellValue::Number(result)
            }
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

// --- Trig functions ---

fn math_sin(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.sin()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_cos(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.cos()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_tan(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.tan()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_asin(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) if n >= -1.0 && n <= 1.0 => CellValue::Number(n.asin()),
        Some(_) => CellValue::Error("#NUM!".into()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_acos(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) if n >= -1.0 && n <= 1.0 => CellValue::Number(n.acos()),
        Some(_) => CellValue::Error("#NUM!".into()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_atan(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.atan()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_atan2(args: &[CellValue]) -> CellValue {
    let x = args.first().and_then(to_number);
    let y = args.get(1).and_then(to_number);
    match (x, y) {
        (Some(xv), Some(yv)) => CellValue::Number(yv.atan2(xv)),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn math_degrees(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.to_degrees()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_radians(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.to_radians()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

// --- Log/Exp ---

fn math_exp(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) => CellValue::Number(n.exp()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_ln(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) if n > 0.0 => CellValue::Number(n.ln()),
        Some(_) => CellValue::Error("#NUM!".into()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn math_log(args: &[CellValue]) -> CellValue {
    let n = args.first().and_then(to_number);
    let base = args.get(1).and_then(to_number).unwrap_or(10.0);
    match (n, base) {
        (Some(nv), b) if nv > 0.0 && b > 0.0 && (b - 1.0).abs() > 1e-10 => {
            CellValue::Number(nv.log(b))
        }
        (Some(_), _) => CellValue::Error("#NUM!".into()),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn math_log10(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_number) {
        Some(n) if n > 0.0 => CellValue::Number(n.log10()),
        Some(_) => CellValue::Error("#NUM!".into()),
        None => CellValue::Error("#VALUE!".into()),
    }
}

// --- Conditional aggregates ---

fn math_sumif(args: &[CellValue]) -> CellValue {
    // SUMIF(range, criteria, [sum_range])
    // Range is passed as inline marker format
    if args.len() < 3 {
        return CellValue::Number(0.0);
    }

    let values = extract_range_values(args);
    let criteria = args.last().cloned().unwrap_or(CellValue::Empty);

    let sum: f64 = values
        .iter()
        .filter(|v| matches_criteria(v, &criteria))
        .filter_map(to_number)
        .sum();

    CellValue::Number(sum)
}

fn math_sumifs(args: &[CellValue]) -> CellValue {
    // SUMIFS(sum_range, criteria_range1, criteria1, [criteria_range2, criteria2], ...)
    // First arg is sum_range, then pairs of (criteria_range, criteria)
    if args.len() < 4 {
        return CellValue::Number(0.0);
    }

    // Parse sum_range and pairs using range-marker format
    let mut pairs: Vec<(Vec<CellValue>, CellValue)> = Vec::new();
    let mut sum_values: Vec<CellValue> = Vec::new();

    let mut i = 0;
    let args_len = args.len();

    // Parse first range (sum_range)
    if let Some(sentinel) = args.get(i).and_then(to_number) {
        if sentinel < -999_999.0 && sentinel > -2_000_000.0 {
            let n_cols = (-(sentinel as f64 + 1_000_000.0)) as usize;
            if let Some(n_rows) = args.get(i + 1).and_then(to_number) {
                let n_rows = n_rows as usize;
                let total = n_cols * n_rows;
                let range_end = i + 2 + total;
                for j in (i + 2)..range_end.min(args_len) {
                    sum_values.push(args[j].clone());
                }
                i = range_end;
            } else {
                i += 1;
            }
        } else {
            sum_values.push(args[i].clone());
            i += 1;
        }
    } else {
        sum_values.push(args[i].clone());
        i += 1;
    }

    // Parse criteria_range/criteria pairs
    while i + 1 < args_len {
        let range_start = i;
        if let Some(sentinel) = args.get(range_start).and_then(to_number) {
            if sentinel < -999_999.0 && sentinel > -2_000_000.0 {
                let n_cols = (-(sentinel as f64 + 1_000_000.0)) as usize;
                if let Some(n_rows) = args.get(range_start + 1).and_then(to_number) {
                    let n_rows = n_rows as usize;
                    let total = n_cols * n_rows;
                    let range_end = range_start + 2 + total;
                    let mut range_values: Vec<CellValue> = Vec::new();
                    for j in (range_start + 2)..range_end.min(args_len) {
                        range_values.push(args[j].clone());
                    }
                    if range_end < args_len {
                        pairs.push((range_values, args[range_end].clone()));
                        i = range_end + 1;
                    } else {
                        break;
                    }
                } else {
                    i += 1;
                }
            } else {
                let crit = args.get(i + 1).cloned().unwrap_or(CellValue::Empty);
                pairs.push((vec![args[i].clone()], crit));
                i += 2;
            }
        } else {
            let crit = args.get(i + 1).cloned().unwrap_or(CellValue::Empty);
            pairs.push((vec![args[i].clone()], crit));
            i += 2;
        }
    }

    if pairs.is_empty() || sum_values.is_empty() {
        return CellValue::Number(0.0);
    }

    let first_range = &pairs[0].0;
    let mut sum = 0.0;

    for (idx, _) in first_range.iter().enumerate() {
        let mut all_match = true;
        for (range_vals, criteria) in &pairs {
            let check_val = range_vals.get(idx).unwrap_or(&CellValue::Empty);
            if !matches_criteria(check_val, criteria) {
                all_match = false;
                break;
            }
        }
        if all_match {
            if let Some(val) = sum_values.get(idx) {
                sum += to_number(val).unwrap_or(0.0);
            }
        }
    }

    CellValue::Number(sum)
}

fn math_countif(args: &[CellValue]) -> CellValue {
    // COUNTIF(range, criteria)
    // Range is passed as inline marker format: [sentinel, rows, data...]
    if args.len() < 3 {
        return CellValue::Number(0.0);
    }

    let values = extract_range_values(args);
    let criteria = args.last().cloned().unwrap_or(CellValue::Empty);

    let count = values
        .iter()
        .filter(|v| matches_criteria(v, &criteria))
        .count();
    CellValue::Number(count as f64)
}

fn math_countifs(args: &[CellValue]) -> CellValue {
    // COUNTIFS(criteria_range1, criteria1, [criteria_range2, criteria2], ...)
    // Each criteria_range/criteria pair uses the range-marker format.
    // We iterate over all pairs and count cells where ALL criteria match.
    if args.len() < 3 {
        return CellValue::Number(0.0);
    }

    // Collect all (range_values, criteria) pairs
    let mut pairs: Vec<(Vec<CellValue>, CellValue)> = Vec::new();
    let mut i = 0;
    let args_len = args.len();

    while i + 1 < args_len {
        let range_start = i;

        // Find the end of this range's inline data
        // Range format: [sentinel: -(cols+1M), rows, n_cols*n_rows cells]
        if let Some(sentinel) = args.get(range_start).and_then(to_number) {
            if sentinel < -999_999.0 && sentinel > -2_000_000.0 {
                let n_cols = (-(sentinel as f64 + 1_000_000.0)) as usize;
                if let Some(n_rows) = args.get(range_start + 1).and_then(to_number) {
                    let n_rows = n_rows as usize;
                    let total = n_cols * n_rows;
                    let range_end = range_start + 2 + total;

                    // Extract the range values
                    let mut range_values: Vec<CellValue> = Vec::new();
                    for j in (range_start + 2)..range_end.min(args_len) {
                        range_values.push(args[j].clone());
                    }

                    // The criteria is the value right after the range data
                    if range_end < args_len {
                        pairs.push((range_values, args[range_end].clone()));
                        i = range_end + 1;
                    } else {
                        // No criteria found, break
                        break;
                    }
                } else {
                    i += 1;
                }
            } else {
                // Non-marker range, treat this arg as a single value range
                // and the next as criteria
                let val = args[i].clone();
                let crit = args.get(i + 1).cloned().unwrap_or(CellValue::Empty);
                pairs.push((vec![val], crit));
                i += 2;
            }
        } else {
            // Non-marker arg pair
            let val = args[i].clone();
            let crit = args.get(i + 1).cloned().unwrap_or(CellValue::Empty);
            pairs.push((vec![val], crit));
            i += 2;
        }
    }

    if pairs.is_empty() {
        return CellValue::Number(0.0);
    }

    // Get the first range's values
    let first_values = &pairs[0].0;
    let mut count = 0;

    // For each position in the first range, check if ALL criteria match
    for (idx, val) in first_values.iter().enumerate() {
        let mut all_match = true;
        for (range_vals, criteria) in &pairs {
            let check_val = range_vals.get(idx).unwrap_or(&CellValue::Empty);
            if !matches_criteria(check_val, criteria) {
                all_match = false;
                break;
            }
        }
        if all_match {
            count += 1;
        }
    }

    CellValue::Number(count as f64)
}

fn math_averageif(args: &[CellValue]) -> CellValue {
    // AVERAGEIF(range, criteria, [average_range])
    if args.len() < 3 {
        return CellValue::Error("#DIV/0!".into());
    }

    let criteria = args.last().cloned().unwrap_or(CellValue::Empty);
    let values = extract_range_values(args);

    let matching: Vec<f64> = values
        .iter()
        .filter(|v| matches_criteria(v, &criteria))
        .filter_map(to_number)
        .collect();

    if matching.is_empty() {
        CellValue::Error("#DIV/0!".into())
    } else {
        CellValue::Number(matching.iter().sum::<f64>() / matching.len() as f64)
    }
}

// --- GCD/LCM ---

fn math_gcd(args: &[CellValue]) -> CellValue {
    let nums = flatten_numbers(args);
    if nums.is_empty() {
        return CellValue::Number(0.0);
    }
    let mut result = nums[0].abs() as u64;
    for n in nums.iter().skip(1) {
        result = gcd(result, n.abs() as u64);
    }
    CellValue::Number(result as f64)
}

fn math_lcm(args: &[CellValue]) -> CellValue {
    let nums = flatten_numbers(args);
    if nums.is_empty() {
        return CellValue::Number(0.0);
    }
    let mut result = nums[0].abs() as u64;
    for n in nums.iter().skip(1) {
        let b = n.abs() as u64;
        result = result / gcd(result, b) * b;
    }
    CellValue::Number(result as f64)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

// --- RAND ---

fn math_randbetween(args: &[CellValue]) -> CellValue {
    let bottom = args.first().and_then(to_number);
    let top = args.get(1).and_then(to_number);
    match (bottom, top) {
        (Some(lo), Some(hi)) if lo <= hi => {
            CellValue::Number(lo + rand::random::<f64>() * (hi - lo + 1.0).floor())
        }
        _ => CellValue::Error("#VALUE!".into()),
    }
}

// --- SUBTOTAL ---

fn math_subtotal(args: &[CellValue]) -> CellValue {
    // SUBTOTAL(function_num, ref1, ref2, ...)
    // function_num: 1-AVERAGE, 2-COUNT, 3-COUNTA, 4-MAX, 5-MIN, 6-PRODUCT, 7-STDEV, 8-STDEVP, 9-SUM, 10-VAR, 11-VARP
    // Adding 100 ignores hidden rows (not yet supported)
    let func_num = args.first().and_then(to_number).unwrap_or(0.0) as u32;
    let data = &args[1..];

    match func_num % 100 {
        1 => math_average(data),
        2 => math_count(data),
        3 => math_counta(data),
        4 => math_max(data),
        5 => math_min(data),
        6 => math_product(data),
        9 => math_sum(data),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

// --- Helper functions for criteria-based functions ---

/// Check if a CellValue matches a given criteria.
/// Supports comparison operators: ">N", ">=N", "<N", "<=N", "<>N", "=N", just "N", and string equality.
fn matches_criteria(val: &CellValue, criteria: &CellValue) -> bool {
    let criteria_str = match criteria {
        CellValue::String(s) => s.clone(),
        CellValue::Number(n) => format!("{}", n),
        CellValue::Bool(true) => "TRUE".to_string(),
        CellValue::Bool(false) => "FALSE".to_string(),
        _ => return false,
    };
    let criteria_str = criteria_str.trim();
    let val_num = to_number(val);

    if let Some(rest) = criteria_str.strip_prefix(">=") {
        val_num.map_or(false, |v| v >= rest.parse::<f64>().unwrap_or(0.0))
    } else if let Some(rest) = criteria_str.strip_prefix("<=") {
        val_num.map_or(false, |v| v <= rest.parse::<f64>().unwrap_or(0.0))
    } else if let Some(rest) = criteria_str.strip_prefix("<>") {
        if let Some(v) = val_num {
            v != rest.parse::<f64>().unwrap_or(f64::NAN)
        } else {
            format_val_string(val) != rest
        }
    } else if let Some(rest) = criteria_str.strip_prefix('>') {
        val_num.map_or(false, |v| v > rest.parse::<f64>().unwrap_or(0.0))
    } else if let Some(rest) = criteria_str.strip_prefix('<') {
        val_num.map_or(false, |v| v < rest.parse::<f64>().unwrap_or(0.0))
    } else if let Some(rest) = criteria_str.strip_prefix('=') {
        if let Some(v) = val_num {
            (v - rest.parse::<f64>().unwrap_or(f64::NAN)).abs() < 1e-12
        } else {
            format_val_string(val) == rest
        }
    } else {
        if let Some(v) = val_num {
            if let Ok(crit_num) = criteria_str.parse::<f64>() {
                (v - crit_num).abs() < 1e-12
            } else {
                format_val_string(val).to_uppercase() == criteria_str.to_uppercase()
            }
        } else {
            format_val_string(val).to_uppercase() == criteria_str.to_uppercase()
        }
    }
}

fn format_val_string(val: &CellValue) -> String {
    match val {
        CellValue::String(s) => s.clone(),
        CellValue::Number(n) => format!("{}", n),
        CellValue::Bool(true) => "TRUE".to_string(),
        CellValue::Bool(false) => "FALSE".to_string(),
        CellValue::Empty => "".to_string(),
        CellValue::Error(e) => e.clone(),
        CellValue::DateTime(_) => "".to_string(),
    }
}

/// Extract values from range-marker format args (skipping sentinel and row count markers).
fn extract_range_values(args: &[CellValue]) -> Vec<CellValue> {
    if args.len() < 3 {
        return args.to_vec();
    }
    if let Some(sentinel) = args.first().and_then(to_number) {
        if sentinel < -999_999.0 && sentinel > -2_000_000.0 {
            // Range marker format: [sentinel, rows, data...]
            // Skip sentinel and row count, return only the data
            if let Some(n_rows) = args.get(1).and_then(to_number) {
                let n_cols = (-(sentinel as f64 + 1_000_000.0)) as usize;
                let total = n_cols * (n_rows as usize);
                let end = (2 + total).min(args.len() - 1); // -1 for the criteria arg at the end
                if end > 2 {
                    return args[2..end].to_vec();
                }
            }
            return args[2..].to_vec();
        }
    }
    // Non-marker: return all args except the last one (criteria)
    if args.len() > 1 {
        args[..args.len() - 1].to_vec()
    } else {
        args.to_vec()
    }
}
