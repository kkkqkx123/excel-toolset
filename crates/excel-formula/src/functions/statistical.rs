//! Statistical functions.

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
    registry.insert(
        "AVEDEV".into(),
        Arc::new(|args, provider| stat_avedev(args)),
    );
    registry.insert(
        "STDEV.P".into(),
        Arc::new(|args, provider| stat_stdevp(args)),
    );
    registry.insert(
        "STDEV.S".into(),
        Arc::new(|args, provider| stat_stdevs(args)),
    );
    registry.insert("VAR.P".into(), Arc::new(|args, provider| stat_varp(args)));
    registry.insert("VAR.S".into(), Arc::new(|args, provider| stat_vars(args)));
    registry.insert(
        "MEDIAN".into(),
        Arc::new(|args, provider| stat_median(args)),
    );
    registry.insert("MODE".into(), Arc::new(|args, provider| stat_mode(args)));
    registry.insert(
        "MODE.SNGL".into(),
        Arc::new(|args, provider| stat_mode(args)),
    );
    registry.insert(
        "QUARTILE".into(),
        Arc::new(|args, provider| stat_quartile(args)),
    );
    registry.insert(
        "QUARTILE.INC".into(),
        Arc::new(|args, provider| stat_quartile(args)),
    );
    registry.insert(
        "QUARTILE.EXC".into(),
        Arc::new(|args, provider| stat_quartile_exc(args)),
    );
    registry.insert(
        "PERCENTILE".into(),
        Arc::new(|args, provider| stat_percentile(args)),
    );
    registry.insert(
        "PERCENTILE.INC".into(),
        Arc::new(|args, provider| stat_percentile(args)),
    );
    registry.insert(
        "NORM.DIST".into(),
        Arc::new(|args, provider| stat_norm_dist(args)),
    );
    registry.insert(
        "NORM.INV".into(),
        Arc::new(|args, provider| stat_norm_inv(args)),
    );
    registry.insert(
        "NORM.S.DIST".into(),
        Arc::new(|args, provider| stat_norm_s_dist(args)),
    );
    registry.insert(
        "NORM.S.INV".into(),
        Arc::new(|args, provider| stat_norm_s_inv(args)),
    );
    registry.insert(
        "T.DIST".into(),
        Arc::new(|args, provider| stat_t_dist(args)),
    );
    registry.insert("T.INV".into(), Arc::new(|args, provider| stat_t_inv(args)));
    registry.insert(
        "T.TEST".into(),
        Arc::new(|args, provider| stat_t_test(args)),
    );
    registry.insert(
        "CORREL".into(),
        Arc::new(|args, provider| stat_correl(args)),
    );
    registry.insert("COVAR".into(), Arc::new(|args, provider| stat_covar(args)));
    registry.insert(
        "CONFIDENCE.NORM".into(),
        Arc::new(|args, provider| stat_confidence_norm(args)),
    );
    registry.insert(
        "CONFIDENCE.T".into(),
        Arc::new(|args, provider| stat_confidence_t(args)),
    );
    registry.insert(
        "CHISQ.DIST".into(),
        Arc::new(|args, provider| stat_chisq_dist(args)),
    );
    registry.insert(
        "CHISQ.TEST".into(),
        Arc::new(|args, provider| stat_chisq_test(args)),
    );
    registry.insert(
        "LINEST".into(),
        Arc::new(|args, provider| stat_linest(args)),
    );
    registry.insert(
        "LOGEST".into(),
        Arc::new(|args, provider| stat_logest(args)),
    );
    registry.insert("TREND".into(), Arc::new(|args, provider| stat_trend(args)));
    registry.insert(
        "GROWTH".into(),
        Arc::new(|args, provider| stat_growth(args)),
    );
}

fn flatten_numbers(args: &[CellValue]) -> Vec<f64> {
    args.iter().filter_map(to_number).collect()
}

/// Extract numbers from args, handling range-marker format.
fn extract_numbers_from_range_args(args: &[CellValue]) -> Vec<f64> {
    if args.len() < 3 {
        return flatten_numbers(args);
    }
    // Check for range marker: sentinel = -(cols + 1_000_000.0)
    if let Some(sentinel) = args.first().and_then(to_number) {
        if sentinel < -999_999.0 && sentinel > -2_000_000.0 {
            // Range marker format: skip sentinel and row count
            return args[2..].iter().filter_map(to_number).collect();
        }
    }
    flatten_numbers(args)
}

/// Average absolute deviation
fn stat_avedev(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    if nums.is_empty() {
        return CellValue::Error("#DIV/0!".into());
    }
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    CellValue::Number(nums.iter().map(|n| (n - mean).abs()).sum::<f64>() / nums.len() as f64)
}

/// Population standard deviation
fn stat_stdevp(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    if nums.is_empty() {
        return CellValue::Error("#DIV/0!".into());
    }
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    let variance = nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / nums.len() as f64;
    CellValue::Number(variance.sqrt())
}

/// Sample standard deviation
fn stat_stdevs(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    if nums.len() < 2 {
        return CellValue::Error("#DIV/0!".into());
    }
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    let variance = nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / (nums.len() - 1) as f64;
    CellValue::Number(variance.sqrt())
}

/// Population variance
fn stat_varp(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    if nums.is_empty() {
        return CellValue::Error("#DIV/0!".into());
    }
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    CellValue::Number(nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / nums.len() as f64)
}

/// Sample variance
fn stat_vars(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    if nums.len() < 2 {
        return CellValue::Error("#DIV/0!".into());
    }
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    CellValue::Number(
        nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / (nums.len() - 1) as f64,
    )
}

/// Median
fn stat_median(args: &[CellValue]) -> CellValue {
    let mut nums = extract_numbers_from_range_args(args);
    if nums.is_empty() {
        return CellValue::Error("#NUM!".into());
    }
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = nums.len() / 2;
    if nums.len() % 2 == 0 {
        CellValue::Number((nums[mid - 1] + nums[mid]) / 2.0)
    } else {
        CellValue::Number(nums[mid])
    }
}

/// Mode (most frequent value)
fn stat_mode(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    if nums.is_empty() {
        return CellValue::Error("#NUM!".into());
    }
    let mut counts: HashMap<String, (f64, usize)> = HashMap::new();
    for n in nums {
        let key = format!("{:.10}", n);
        let entry = counts.entry(key).or_insert((n, 0));
        entry.1 += 1;
    }
    let max = counts.values().max_by_key(|(_, c)| *c);
    match max {
        Some((val, count)) if *count > 1 => CellValue::Number(*val),
        _ => CellValue::Error("#N/A".into()),
    }
}

/// Quartile (inclusive)
fn stat_quartile(args: &[CellValue]) -> CellValue {
    let quart = args.last().and_then(to_number).unwrap_or(0.0) as u32;
    let nums = &args[..args.len() - 1];
    let mut nums = extract_numbers_from_range_args(nums);
    if nums.is_empty() || quart > 4 {
        return CellValue::Error("#NUM!".into());
    }

    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = nums.len();

    match quart {
        0 => CellValue::Number(nums[0]),
        1 => {
            let idx = (n as f64 - 1.0) * 0.25;
            CellValue::Number(interpolate(&nums, idx))
        }
        2 => {
            let mid = n / 2;
            if n % 2 == 0 {
                CellValue::Number((nums[mid - 1] + nums[mid]) / 2.0)
            } else {
                CellValue::Number(nums[mid])
            }
        }
        3 => {
            let idx = (n as f64 - 1.0) * 0.75;
            CellValue::Number(interpolate(&nums, idx))
        }
        4 => CellValue::Number(nums[n - 1]),
        _ => CellValue::Error("#NUM!".into()),
    }
}

/// Quartile (exclusive)
fn stat_quartile_exc(args: &[CellValue]) -> CellValue {
    let quart = args.last().and_then(to_number).unwrap_or(0.0) as u32;
    let nums = &args[..args.len() - 1];
    let mut nums = extract_numbers_from_range_args(nums);
    if nums.len() < 3 || quart > 4 {
        return CellValue::Error("#NUM!".into());
    }

    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = nums.len();

    match quart {
        1 => {
            let idx = (n as f64 + 1.0) * 0.25 - 1.0;
            CellValue::Number(interpolate(&nums, idx))
        }
        2 => {
            let idx = (n as f64 + 1.0) * 0.5 - 1.0;
            CellValue::Number(interpolate(&nums, idx))
        }
        3 => {
            let idx = (n as f64 + 1.0) * 0.75 - 1.0;
            CellValue::Number(interpolate(&nums, idx))
        }
        _ => CellValue::Error("#NUM!".into()),
    }
}

/// Percentile (inclusive)
fn stat_percentile(args: &[CellValue]) -> CellValue {
    let k = args.last().and_then(to_number).unwrap_or(0.0);
    let nums = &args[..args.len() - 1];
    let mut nums = extract_numbers_from_range_args(nums);

    if nums.is_empty() || k < 0.0 || k > 1.0 {
        return CellValue::Error("#NUM!".into());
    }

    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = nums.len();
    let idx = k * (n as f64 - 1.0);
    CellValue::Number(interpolate(&nums, idx))
}

fn interpolate(sorted: &[f64], idx: f64) -> f64 {
    if idx < 0.0 {
        return sorted[0];
    }
    if idx >= (sorted.len() - 1) as f64 {
        return sorted[sorted.len() - 1];
    }

    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    let frac = idx - lower as f64;

    sorted[lower] * (1.0 - frac) + sorted[upper] * frac
}

// --- Normal distribution ---

/// NORM.DIST(x, mean, standard_dev, cumulative)
fn stat_norm_dist(args: &[CellValue]) -> CellValue {
    let x = args.first().and_then(to_number).unwrap_or(0.0);
    let mean = args.get(1).and_then(to_number).unwrap_or(0.0);
    let std_dev = args.get(2).and_then(to_number).unwrap_or(1.0);
    let cumulative = args.get(3).map_or(true, |v| match v {
        CellValue::Bool(true) => true,
        CellValue::Number(n) if *n != 0.0 => true,
        _ => false,
    });

    if std_dev <= 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    let z = (x - mean) / std_dev;

    if cumulative {
        CellValue::Number(norm_cdf(z))
    } else {
        CellValue::Number(norm_pdf(z) / std_dev)
    }
}

/// NORM.INV(probability, mean, standard_dev)
fn stat_norm_inv(args: &[CellValue]) -> CellValue {
    let p = args.first().and_then(to_number).unwrap_or(0.0);
    let mean = args.get(1).and_then(to_number).unwrap_or(0.0);
    let std_dev = args.get(2).and_then(to_number).unwrap_or(1.0);

    if p <= 0.0 || p >= 1.0 || std_dev <= 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    CellValue::Number(norm_inv(p) * std_dev + mean)
}

/// NORM.S.DIST(z, cumulative)
fn stat_norm_s_dist(args: &[CellValue]) -> CellValue {
    let z = args.first().and_then(to_number).unwrap_or(0.0);
    let cumulative = args.get(1).map_or(true, |v| match v {
        CellValue::Bool(true) => true,
        CellValue::Number(n) if *n != 0.0 => true,
        _ => false,
    });

    if cumulative {
        CellValue::Number(norm_cdf(z))
    } else {
        CellValue::Number(norm_pdf(z))
    }
}

/// NORM.S.INV(probability)
fn stat_norm_s_inv(args: &[CellValue]) -> CellValue {
    let p = args.first().and_then(to_number).unwrap_or(0.0);
    if p <= 0.0 || p >= 1.0 {
        return CellValue::Error("#NUM!".into());
    }
    CellValue::Number(norm_inv(p))
}

fn norm_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Standard normal CDF using Abramowitz and Stegun approximation.
fn norm_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - ((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t * (-x * x / 2.0).exp();

    0.5 * (1.0 + sign * y)
}

/// Inverse normal CDF using Moro's algorithm.
fn norm_inv(p: f64) -> f64 {
    let a0 = 2.50662823884;
    let a1 = -18.61500062529;
    let a2 = 41.39119773534;
    let a3 = -25.44106049637;
    let b1 = -8.47351093090;
    let b2 = 23.08336743743;
    let b3 = -21.06224101826;
    let b4 = 3.13082909833;
    let c0 = 0.3374754822726147;
    let c1 = 0.9761690190917186;
    let c2 = 0.1607979714918209;
    let c3 = 0.0276438810333863;
    let c4 = 0.0038405729373609;
    let c5 = 0.0003951896511919;
    let c6 = 0.0000321767881768;
    let c7 = 0.0000002888167364;
    let c8 = 0.0000003960315187;

    let q = p - 0.5;

    if q.abs() <= 0.42 {
        let r = q * q;
        q * (((a3 * r + a2) * r + a1) * r + a0) / ((((b4 * r + b3) * r + b2) * r + b1) * r + 1.0)
    } else {
        let r = if p < 0.5 { p } else { 1.0 - p };
        let r = (-r.ln()).sqrt();
        let sign = if q < 0.0 { -1.0 } else { 1.0 };
        let r1 = c0 + c1 * r;
        let r2 = c2 + c3 * r;
        let r3 = c4 + c5 * r;
        let r4 = c6 + c7 * r + c8 * r * r;
        sign * (r1 + r2 * r + r3 * r.powi(2) + r4 * r.powi(3))
            / (1.0 + r * (c1 + r * (c3 + r * c5 + r * (c7 + r * c8))))
    }
}

// --- T distribution ---

/// T.DIST(x, deg_freedom, cumulative)
fn stat_t_dist(args: &[CellValue]) -> CellValue {
    let x = args.first().and_then(to_number).unwrap_or(0.0);
    let df = args.get(1).and_then(to_number).unwrap_or(1.0);
    let cumulative = args.get(2).map_or(true, |v| match v {
        CellValue::Bool(true) => true,
        CellValue::Number(n) if *n != 0.0 => true,
        _ => false,
    });

    if df < 1.0 {
        return CellValue::Error("#NUM!".into());
    }

    if cumulative {
        CellValue::Number(t_cdf(x, df))
    } else {
        CellValue::Number(t_pdf(x, df))
    }
}

/// T.INV(probability, deg_freedom)
fn stat_t_inv(args: &[CellValue]) -> CellValue {
    let p = args.first().and_then(to_number).unwrap_or(0.0);
    let df = args.get(1).and_then(to_number).unwrap_or(1.0);

    if p <= 0.0 || p >= 1.0 || df < 1.0 {
        return CellValue::Error("#NUM!".into());
    }

    let mut x = norm_inv(p);
    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let f = t_cdf(x, df) - p;
        let df_val = t_pdf(x, df);
        if df_val.abs() < 1e-15 {
            break;
        }
        let new_x = x - f / df_val;
        if (new_x - x).abs() < tolerance {
            return CellValue::Number(new_x);
        }
        x = new_x;
    }

    CellValue::Number(x)
}

fn t_pdf(x: f64, df: f64) -> f64 {
    let num = gamma((df + 1.0) / 2.0);
    let den = (df * std::f64::consts::PI).sqrt() * gamma(df / 2.0);
    num / den * (1.0 + x * x / df).powf(-(df + 1.0) / 2.0)
}

/// Student's t CDF using regularized incomplete beta function approximation.
fn t_cdf(x: f64, df: f64) -> f64 {
    let a = df / 2.0;
    let b = 0.5;
    let s = x * x / (df + x * x);
    if x <= 0.0 {
        0.5 * reg_incomplete_beta(a, b, s)
    } else {
        1.0 - 0.5 * reg_incomplete_beta(a, b, s)
    }
}

/// Regularized incomplete beta function I_x(a, b) using continued fraction.
fn reg_incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let bt = (gamma(a + b) / (gamma(a) * gamma(b))) * x.powf(a) * (1.0 - x).powf(b);

    // Continued fraction for the incomplete beta function
    let mut fpm = 1.0;
    let mut fm = f64::MAX;
    let mut f = 1.0;
    let max_iter = 200;
    let eps = 3e-11;

    for m in 1..max_iter {
        let m_f = m as f64;
        let mm_f = (m - 1) as f64;

        // d = (2*m) * (a+b+m) * x^m  ... this is the continued fraction term
        let d = -(a + mm_f) * (a + b + mm_f) * x / ((a + 2.0 * mm_f) * (a + 2.0 * mm_f + 1.0));
        let mut e = m_f * (b - m_f) * x / ((a + 2.0 * mm_f - 1.0) * (a + 2.0 * mm_f));
        if m == 1 {
            e *= (a + 1.0) / a;
        }

        let d1 = 1.0 + d * fm;
        let d2 = 1.0 + e / d1;
        if d1.abs() < 1e-30 || d2.abs() < 1e-30 {
            break;
        }
        fm = 1.0 / d1;
        fpm = fm;
        f = fpm * d2;

        if (f - fm).abs() < eps {
            break;
        }
    }

    bt * f / a
}

// --- T.TEST ---

/// T.TEST(array1, array2, tails, type)
/// Welch's t-test (type=3) for comparing two sample means.
fn stat_t_test(args: &[CellValue]) -> CellValue {
    if args.len() < 4 {
        return CellValue::Error("#VALUE!".into());
    }

    // Extract array values, handling range markers
    let array1 = extract_numbers_from_range_args(&args[..args.len() - 2]);
    let array2 = extract_numbers_from_range_args(&args[args.len() - 2..]);

    let tails = args.get(args.len() - 2).and_then(to_number).unwrap_or(2.0) as i32;
    // type: 1=paired, 2=equal variance, 3=unequal variance (Welch's)
    let test_type = args.last().and_then(to_number).unwrap_or(3.0) as i32;

    if array1.len() < 2
        || array2.len() < 2
        || (tails != 1 && tails != 2)
        || test_type < 1
        || test_type > 3
    {
        return CellValue::Error("#NUM!".into());
    }

    let n1 = array1.len() as f64;
    let n2 = array2.len() as f64;

    let mean1 = array1.iter().sum::<f64>() / n1;
    let mean2 = array2.iter().sum::<f64>() / n2;

    let var1 = array1.iter().map(|x| (x - mean1).powi(2)).sum::<f64>() / (n1 - 1.0);
    let var2 = array2.iter().map(|x| (x - mean2).powi(2)).sum::<f64>() / (n2 - 1.0);

    let t_stat;
    let df;

    match test_type {
        1 => {
            // Paired t-test: assume arrays are same length, compute differences
            let min_len = array1.len().min(array2.len());
            let diffs: Vec<f64> = array1[..min_len]
                .iter()
                .zip(array2[..min_len].iter())
                .map(|(a, b)| a - b)
                .collect();
            let n = diffs.len() as f64;
            let mean_diff = diffs.iter().sum::<f64>() / n;
            let var_diff = diffs.iter().map(|d| (d - mean_diff).powi(2)).sum::<f64>() / (n - 1.0);
            t_stat = mean_diff / (var_diff / n).sqrt();
            df = n - 1.0;
        }
        2 => {
            // Equal variance (Student's t-test)
            let pooled_var = ((n1 - 1.0) * var1 + (n2 - 1.0) * var2) / (n1 + n2 - 2.0);
            let se = (pooled_var * (1.0 / n1 + 1.0 / n2)).sqrt();
            t_stat = (mean1 - mean2) / se;
            df = n1 + n2 - 2.0;
        }
        _ => {
            // Type 3: Welch's t-test (unequal variance)
            let se = (var1 / n1 + var2 / n2).sqrt();
            t_stat = (mean1 - mean2) / se;

            // Welch-Satterthwaite degrees of freedom
            let num = (var1 / n1 + var2 / n2).powi(2);
            let den = (var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0);
            df = num / den;
        }
    }

    if t_stat.is_nan() || df <= 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    // Two-tailed p-value = 2 * (1 - CDF(|t|, df))
    // One-tailed: 1 - CDF(|t|, df)
    let p_one_tail = 1.0 - t_cdf(t_stat.abs(), df);
    let p_value = if tails == 2 {
        2.0 * p_one_tail
    } else {
        p_one_tail
    };

    CellValue::Number(p_value.min(1.0).max(0.0))
}

// --- Correlation and Covariance ---

/// CORREL(array1, array2) -- Pearson correlation coefficient
fn stat_correl(args: &[CellValue]) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let array1 = extract_numbers_from_range_args(args);
    // Find midpoint: assume roughly equal split
    let mid = array1.len() / 2;
    let mid = mid.min(args.len() / 2);

    let nums1: Vec<f64> = array1[..mid].to_vec();
    let nums2: Vec<f64> = array1[mid..].to_vec();

    if nums1.len() < 2 || nums2.len() < 2 || nums1.len() != nums2.len() {
        return CellValue::Error("#N/A".into());
    }

    let n = nums1.len() as f64;
    let mean1 = nums1.iter().sum::<f64>() / n;
    let mean2 = nums2.iter().sum::<f64>() / n;

    let mut num = 0.0;
    let mut den1 = 0.0;
    let mut den2 = 0.0;

    for i in 0..nums1.len() {
        let d1 = nums1[i] - mean1;
        let d2 = nums2[i] - mean2;
        num += d1 * d2;
        den1 += d1 * d1;
        den2 += d2 * d2;
    }

    let denom = (den1 * den2).sqrt();
    if denom < 1e-15 {
        CellValue::Error("#DIV/0!".into())
    } else {
        CellValue::Number(num / denom)
    }
}

fn stat_covar(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    let mid = nums.len() / 2;
    let nums1 = &nums[..mid];
    let nums2 = &nums[mid..];

    if nums1.len() < 2 || nums2.len() < 2 || nums1.len() != nums2.len() {
        return CellValue::Error("#N/A".into());
    }

    let n = nums1.len() as f64;
    let mean1 = nums1.iter().sum::<f64>() / n;
    let mean2 = nums2.iter().sum::<f64>() / n;

    let cov = nums1
        .iter()
        .zip(nums2.iter())
        .map(|(a, b)| (a - mean1) * (b - mean2))
        .sum::<f64>()
        / n;
    CellValue::Number(cov)
}

// --- Confidence intervals ---

fn stat_confidence_norm(args: &[CellValue]) -> CellValue {
    let alpha = args.first().and_then(to_number).unwrap_or(0.05);
    let std_dev = args.get(1).and_then(to_number).unwrap_or(1.0);
    let size = args.get(2).and_then(to_number).unwrap_or(1.0);

    if alpha <= 0.0 || alpha >= 1.0 || std_dev <= 0.0 || size < 1.0 {
        return CellValue::Error("#NUM!".into());
    }

    let z = norm_inv(1.0 - alpha / 2.0);
    CellValue::Number(z * std_dev / size.sqrt())
}

/// CONFIDENCE.T(alpha, stddev, size) -- Confidence interval using t-distribution.
fn stat_confidence_t(args: &[CellValue]) -> CellValue {
    let alpha = args.first().and_then(to_number).unwrap_or(0.05);
    let std_dev = args.get(1).and_then(to_number).unwrap_or(1.0);
    let size = args.get(2).and_then(to_number).unwrap_or(1.0);

    if alpha <= 0.0 || alpha >= 1.0 || std_dev <= 0.0 || size < 2.0 {
        return CellValue::Error("#NUM!".into());
    }

    let df = size - 1.0;
    let p = 1.0 - alpha / 2.0;
    let t_val = stat_t_inv(&[CellValue::Number(p), CellValue::Number(df)]);
    let t_val = to_number(&t_val).unwrap_or(0.0);

    CellValue::Number(t_val * std_dev / size.sqrt())
}

// --- Chi-squared distribution ---

fn stat_chisq_dist(args: &[CellValue]) -> CellValue {
    let x = args.first().and_then(to_number).unwrap_or(0.0);
    let df = args.get(1).and_then(to_number).unwrap_or(1.0);
    let cumulative = args.get(2).map_or(true, |v| match v {
        CellValue::Bool(true) => true,
        CellValue::Number(n) if *n != 0.0 => true,
        _ => false,
    });

    if x < 0.0 || df < 1.0 {
        return CellValue::Error("#NUM!".into());
    }

    if cumulative {
        // Chi-squared CDF = P(k/2, x/2) where P is the regularized lower incomplete gamma function
        CellValue::Number(chisq_cdf(x, df))
    } else {
        // Chi-squared PDF
        let a = df / 2.0;
        CellValue::Number((x.powf(a - 1.0) * (-x / 2.0).exp()) / (2.0f64.powf(a) * gamma(a)))
    }
}

/// Chi-squared CDF using regularized incomplete gamma function.
fn chisq_cdf(x: f64, k: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    reg_lower_incomplete_gamma(k / 2.0, x / 2.0)
}

/// Regularized lower incomplete gamma function P(a, x) = gamma(a, x) / Gamma(a)
/// using series expansion.
fn reg_lower_incomplete_gamma(a: f64, x: f64) -> f64 {
    if x < 0.0 || a <= 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }

    let mut sum = 1.0 / a;
    let mut term = 1.0 / a;
    let max_iter = 200;

    for n in 1..max_iter {
        term *= x / (a + n as f64);
        let prev_sum = sum;
        sum += term;
        if (sum - prev_sum).abs() < 1e-15 {
            break;
        }
    }

    sum * x.powf(a) * (-x).exp() / gamma(a)
}

/// CHISQ.TEST(actual, expected) -- Chi-square goodness-of-fit test.
fn stat_chisq_test(args: &[CellValue]) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let nums = extract_numbers_from_range_args(args);
    let mid = nums.len() / 2;
    let observed = &nums[..mid];
    let expected = &nums[mid..];

    let min_len = observed.len().min(expected.len());
    if min_len < 2 {
        return CellValue::Error("#NUM!".into());
    }

    let mut chi_sq = 0.0;
    for i in 0..min_len {
        let o = observed[i];
        let e = expected[i];
        if e > 0.0 {
            chi_sq += (o - e).powi(2) / e;
        }
    }

    let df = (min_len - 1) as f64;
    let p_value = 1.0 - chisq_cdf(chi_sq, df);
    CellValue::Number(p_value.max(0.0).min(1.0))
}

// --- LINEST ---

/// LINEST(known_y, known_x, const, stats)
/// Simple linear regression using least squares. Returns slope and intercept.
fn stat_linest(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);

    // Simple case: assume args are split, first half = y, second half = x
    let mid = nums.len() / 2;
    let y = &nums[..mid];
    let x = &nums[mid..];
    let n = y.len().min(x.len());

    if n < 2 {
        return CellValue::Error("#NUM!".into());
    }

    let mean_x = x[..n].iter().sum::<f64>() / n as f64;
    let mean_y = y[..n].iter().sum::<f64>() / n as f64;

    let mut num = 0.0;
    let mut den = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        num += dx * (y[i] - mean_y);
        den += dx * dx;
    }

    if den.abs() < 1e-15 {
        return CellValue::Error("#DIV/0!".into());
    }

    let slope = num / den;
    let intercept = mean_y - slope * mean_x;

    // Return slope as single cell value; intercept would need array output
    CellValue::Number(slope)
}

/// LOGEST(known_y, known_x)
/// Exponential regression: ln-transform y, do LINEST, exponentiate results.
fn stat_logest(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);
    let mid = nums.len() / 2;
    let y = &nums[..mid];
    let x = &nums[mid..];
    let n = y.len().min(x.len());

    if n < 2 {
        return CellValue::Error("#NUM!".into());
    }

    // Ln-transform y
    let ln_y: Vec<f64> = y
        .iter()
        .map(|v| if *v > 0.0 { v.ln() } else { 0.0 })
        .collect();

    let mean_x = x[..n].iter().sum::<f64>() / n as f64;
    let mean_ly = ln_y.iter().sum::<f64>() / n as f64;

    let mut num = 0.0;
    let mut den = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        num += dx * (ln_y[i] - mean_ly);
        den += dx * dx;
    }

    if den.abs() < 1e-15 {
        return CellValue::Error("#DIV/0!".into());
    }

    let b_ln = num / den;
    let a_ln = mean_ly - b_ln * mean_x;

    // b (growth rate) and a (coefficient)
    CellValue::Number(b_ln.exp())
}

/// TREND(known_y, known_x, new_x, const)
/// Predict y values from linear regression.
fn stat_trend(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);

    // Split into thirds: y, x, new_x
    let third = nums.len() / 3;
    if third < 1 {
        return CellValue::Error("#NUM!".into());
    }

    let y = &nums[..third];
    let x = &nums[third..2 * third];
    let new_x = &nums[2 * third..];

    let n = y.len().min(x.len());
    if n < 2 {
        return CellValue::Error("#NUM!".into());
    }

    let mean_x = x[..n].iter().sum::<f64>() / n as f64;
    let mean_y = y[..n].iter().sum::<f64>() / n as f64;

    let mut num = 0.0;
    let mut den = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        num += dx * (y[i] - mean_y);
        den += dx * dx;
    }

    if den.abs() < 1e-15 {
        return CellValue::Error("#DIV/0!".into());
    }

    let slope = num / den;
    let intercept = mean_y - slope * mean_x;

    // Predict for new_x[0]
    let pred = intercept + slope * new_x.first().copied().unwrap_or(0.0);
    CellValue::Number(pred)
}

/// GROWTH(known_y, known_x, new_x)
/// Predict y values from exponential regression.
fn stat_growth(args: &[CellValue]) -> CellValue {
    let nums = extract_numbers_from_range_args(args);

    let third = nums.len() / 3;
    if third < 1 {
        return CellValue::Error("#NUM!".into());
    }

    let y = &nums[..third];
    let x = &nums[third..2 * third];
    let new_x = &nums[2 * third..];

    let n = y.len().min(x.len());
    if n < 2 {
        return CellValue::Error("#NUM!".into());
    }

    // Ln-transform y
    let ln_y: Vec<f64> = y
        .iter()
        .map(|v| if *v > 0.0 { v.ln() } else { 0.0 })
        .collect();

    let mean_x = x[..n].iter().sum::<f64>() / n as f64;
    let mean_ly = ln_y.iter().sum::<f64>() / n as f64;

    let mut num = 0.0;
    let mut den = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        num += dx * (ln_y[i] - mean_ly);
        den += dx * dx;
    }

    if den.abs() < 1e-15 {
        return CellValue::Error("#DIV/0!".into());
    }

    let b_ln = num / den;
    let a_ln = mean_ly - b_ln * mean_x;

    // Predict: a * b^new_x = exp(ln_a + ln_b * new_x)
    let pred_ln = a_ln + b_ln * new_x.first().copied().unwrap_or(0.0);
    CellValue::Number(pred_ln.exp())
}

// --- Helper: Gamma function (Stirling's approximation) ---

fn gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::INFINITY;
    }
    if x < 0.5 {
        std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x))
    } else {
        let x = x - 1.0;
        let coef = [
            1.0,
            1.0 / 12.0,
            1.0 / 288.0,
            -139.0 / 51840.0,
            -571.0 / 2488320.0,
        ];
        let lg = (x + 0.5) * (x + 7.0 / 6.0).ln() - (x + 7.0 / 6.0)
            + 0.5 * (2.0 * std::f64::consts::PI).ln();
        let mut series = coef[0];
        let mut z = x + 1.0;
        for &c in &coef[1..] {
            series += c / z;
            z *= x + 1.0;
        }
        lg.exp() * series
    }
}
