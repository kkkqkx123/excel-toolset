//! Financial functions.

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
    registry.insert("PMT".into(), Arc::new(|args, provider| fin_pmt(args)));
    registry.insert("IPMT".into(), Arc::new(|args, provider| fin_ipmt(args)));
    registry.insert("PPMT".into(), Arc::new(|args, provider| fin_ppmt(args)));
    registry.insert("FV".into(), Arc::new(|args, provider| fin_fv(args)));
    registry.insert("PV".into(), Arc::new(|args, provider| fin_pv(args)));
    registry.insert("NPER".into(), Arc::new(|args, provider| fin_nper(args)));
    registry.insert("RATE".into(), Arc::new(|args, provider| fin_rate(args)));
    registry.insert("NPV".into(), Arc::new(|args, provider| fin_npv(args)));
    registry.insert("IRR".into(), Arc::new(|args, provider| fin_irr(args)));
    registry.insert("XIRR".into(), Arc::new(|args, provider| fin_xirr(args)));
    registry.insert("XNPV".into(), Arc::new(|args, provider| fin_xnpv(args)));
    registry.insert("SLN".into(), Arc::new(|args, provider| fin_sln(args)));
    registry.insert("DB".into(), Arc::new(|args, provider| fin_db(args)));
    registry.insert("DDB".into(), Arc::new(|args, provider| fin_ddb(args)));
    registry.insert("EFFECT".into(), Arc::new(|args, provider| fin_effect(args)));
    registry.insert(
        "NOMINAL".into(),
        Arc::new(|args, provider| fin_nominal(args)),
    );
    registry.insert("PRICE".into(), Arc::new(|args, provider| fin_price(args)));
    registry.insert("YIELD".into(), Arc::new(|args, provider| fin_yield(args)));
    registry.insert(
        "DURATION".into(),
        Arc::new(|args, provider| fin_duration(args)),
    );
    registry.insert(
        "MDURATION".into(),
        Arc::new(|args, provider| fin_mduration(args)),
    );
    registry.insert(
        "COUPNUM".into(),
        Arc::new(|args, provider| fin_coupnum(args)),
    );
    registry.insert(
        "COUPDAYS".into(),
        Arc::new(|args, provider| fin_coupdays(args)),
    );
    registry.insert(
        "COUPDAYBS".into(),
        Arc::new(|args, provider| fin_coupdaybs(args)),
    );
    registry.insert(
        "COUPDAYSNC".into(),
        Arc::new(|args, provider| fin_coupdaysnc(args)),
    );
}

/// PMT(rate, nper, pv, [fv], [type])
/// Calculates the payment for a loan based on constant payments and a constant interest rate.
fn fin_pmt(args: &[CellValue]) -> CellValue {
    let rate = args.first().and_then(to_number).unwrap_or(0.0);
    let nper = args.get(1).and_then(to_number).unwrap_or(0.0);
    let pv = args.get(2).and_then(to_number).unwrap_or(0.0);
    let fv = args.get(3).and_then(to_number).unwrap_or(0.0);
    let pmt_type = args.get(4).and_then(to_number).unwrap_or(0.0);

    if rate == 0.0 {
        CellValue::Number(-(pv + fv) / nper)
    } else {
        let factor = (1.0 + rate).powf(nper);
        let pmt = -(pv * factor + fv) * rate / ((factor - 1.0) * (1.0 + rate * pmt_type));
        CellValue::Number(pmt)
    }
}

/// IPMT(rate, per, nper, pv, [fv], [type])
fn fin_ipmt(args: &[CellValue]) -> CellValue {
    let rate = args.first().and_then(to_number).unwrap_or(0.0);
    let per = args.get(1).and_then(to_number).unwrap_or(1.0);
    let nper = args.get(2).and_then(to_number).unwrap_or(0.0);
    let pv = args.get(3).and_then(to_number).unwrap_or(0.0);
    let fv = args.get(4).and_then(to_number).unwrap_or(0.0);
    let pmt_type = args.get(5).and_then(to_number).unwrap_or(0.0);

    let pmt = match to_number(&fin_pmt(&[
        CellValue::Number(rate),
        CellValue::Number(nper),
        CellValue::Number(pv),
        CellValue::Number(fv),
        CellValue::Number(pmt_type),
    ])) {
        Some(p) => p,
        None => return CellValue::Error("#VALUE!".into()),
    };

    let fv_before = fin_fv_value(rate, per - 1.0, pmt, pv, pmt_type);
    CellValue::Number(fv_before * rate)
}

/// PPMT(rate, per, nper, pv, [fv], [type])
fn fin_ppmt(args: &[CellValue]) -> CellValue {
    let pmt = match to_number(&fin_pmt(args)) {
        Some(p) => p,
        None => return CellValue::Error("#VALUE!".into()),
    };
    let ipmt = match to_number(&fin_ipmt(args)) {
        Some(i) => i,
        None => return CellValue::Error("#VALUE!".into()),
    };
    CellValue::Number(pmt - ipmt)
}

fn fin_fv_value(rate: f64, nper: f64, pmt: f64, pv: f64, pmt_type: f64) -> f64 {
    if rate == 0.0 {
        -(pv + pmt * nper)
    } else {
        let factor = (1.0 + rate).powf(nper);
        -(pv * factor + pmt * (1.0 + rate * pmt_type) * (factor - 1.0) / rate)
    }
}

/// FV(rate, nper, pmt, [pv], [type])
fn fin_fv(args: &[CellValue]) -> CellValue {
    let rate = args.first().and_then(to_number).unwrap_or(0.0);
    let nper = args.get(1).and_then(to_number).unwrap_or(0.0);
    let pmt = args.get(2).and_then(to_number).unwrap_or(0.0);
    let pv = args.get(3).and_then(to_number).unwrap_or(0.0);
    let pmt_type = args.get(4).and_then(to_number).unwrap_or(0.0);

    CellValue::Number(fin_fv_value(rate, nper, pmt, pv, pmt_type))
}

/// PV(rate, nper, pmt, [fv], [type])
fn fin_pv(args: &[CellValue]) -> CellValue {
    let rate = args.first().and_then(to_number).unwrap_or(0.0);
    let nper = args.get(1).and_then(to_number).unwrap_or(0.0);
    let pmt = args.get(2).and_then(to_number).unwrap_or(0.0);
    let fv = args.get(3).and_then(to_number).unwrap_or(0.0);
    let pmt_type = args.get(4).and_then(to_number).unwrap_or(0.0);

    if rate == 0.0 {
        CellValue::Number(-(fv + pmt * nper))
    } else {
        let factor = (1.0 + rate).powf(nper);
        CellValue::Number(-(fv + pmt * (1.0 + rate * pmt_type) * (factor - 1.0) / rate) / factor)
    }
}

/// NPER(rate, pmt, pv, [fv], [type])
fn fin_nper(args: &[CellValue]) -> CellValue {
    let rate = args.first().and_then(to_number).unwrap_or(0.0);
    let pmt = args.get(1).and_then(to_number).unwrap_or(0.0);
    let pv = args.get(2).and_then(to_number).unwrap_or(0.0);
    let fv = args.get(3).and_then(to_number).unwrap_or(0.0);
    let pmt_type = args.get(4).and_then(to_number).unwrap_or(0.0);

    if rate == 0.0 {
        if pmt == 0.0 {
            return CellValue::Error("#NUM!".into());
        }
        CellValue::Number(-(pv + fv) / pmt)
    } else {
        let z = pmt * (1.0 + rate * pmt_type) / rate;
        let nper = ((z - fv) / (z + pv)).ln() / (1.0 + rate).ln();
        if nper.is_nan() || nper.is_infinite() {
            CellValue::Error("#NUM!".into())
        } else {
            CellValue::Number(nper)
        }
    }
}

/// RATE(nper, pmt, pv, [fv], [type], [guess])
fn fin_rate(args: &[CellValue]) -> CellValue {
    let nper = args.first().and_then(to_number).unwrap_or(0.0);
    let pmt = args.get(1).and_then(to_number).unwrap_or(0.0);
    let pv = args.get(2).and_then(to_number).unwrap_or(0.0);
    let fv = args.get(3).and_then(to_number).unwrap_or(0.0);
    let pmt_type = args.get(4).and_then(to_number).unwrap_or(0.0);
    let mut guess = args.get(5).and_then(to_number).unwrap_or(0.1);

    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let factor = (1.0 + guess).powf(nper);
        let f = pv * factor + pmt * (1.0 + guess * pmt_type) * (factor - 1.0) / guess + fv;
        let df = nper * pv * (1.0 + guess).powf(nper - 1.0)
            + pmt
                * (1.0 + guess * pmt_type)
                * (nper * guess * (1.0 + guess).powf(nper - 1.0) - (factor - 1.0))
                / (guess * guess);

        if df.abs() < 1e-15 {
            return CellValue::Error("#NUM!".into());
        }

        let new_guess = guess - f / df;
        if (new_guess - guess).abs() < tolerance {
            return CellValue::Number(new_guess);
        }
        guess = new_guess;
    }

    CellValue::Error("#NUM!".into())
}

/// NPV(rate, value1, [value2], ...)
fn fin_npv(args: &[CellValue]) -> CellValue {
    let rate = args.first().and_then(to_number).unwrap_or(0.0);
    let values = &args[1..];

    let mut npv = 0.0;
    for (i, val) in values.iter().enumerate() {
        let n = to_number(val).unwrap_or(0.0);
        npv += n / (1.0 + rate).powf((i + 1) as f64);
    }

    CellValue::Number(npv)
}

/// IRR(values, [guess])
fn fin_irr(args: &[CellValue]) -> CellValue {
    let values = &args;
    let mut guess = args.last().and_then(to_number).unwrap_or(0.1);

    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let mut npv = 0.0;
        let mut dnpv = 0.0;

        for (i, val) in values.iter().enumerate() {
            let n = to_number(val).unwrap_or(0.0);
            let t = i as f64;
            let factor = (1.0 + guess).powf(t);
            npv += n / factor;
            dnpv -= t * n / ((1.0 + guess).powf(t + 1.0));
        }

        if dnpv.abs() < 1e-15 {
            return CellValue::Number(guess);
        }

        let new_guess = guess - npv / dnpv;
        if (new_guess - guess).abs() < tolerance {
            return CellValue::Number(new_guess);
        }
        guess = new_guess;
    }

    CellValue::Error("#NUM!".into())
}

/// XIRR(values, dates, [guess])
fn fin_xirr(args: &[CellValue]) -> CellValue {
    // XIRR requires date information and is complex
    // Returns #NUM! as a stub; full implementation needs cashflow+date pair processing
    CellValue::Error("#NUM!".into())
}

/// XNPV(rate, values, dates)
/// Net present value for irregular cash flows.
fn fin_xnpv(args: &[CellValue]) -> CellValue {
    if args.len() < 3 {
        return CellValue::Error("#VALUE!".into());
    }

    let rate = args.first().and_then(to_number).unwrap_or(0.0);

    // For now, values and dates are interleaved or in range-marker format.
    // Simple case: args[1..] are alternating value/date pairs
    let pair_args = &args[1..];
    let mut npv = 0.0;
    let mut first_date = None;

    for chunk in pair_args.chunks(2) {
        if chunk.len() < 2 {
            break;
        }
        let value = to_number(&chunk[0]).unwrap_or(0.0);
        let date = to_number(&chunk[1]).unwrap_or(0.0);

        if first_date.is_none() {
            first_date = Some(date);
        }

        let days = date - first_date.unwrap_or(date);
        let years = days / 365.0;
        npv += value / (1.0 + rate).powf(years);
    }

    CellValue::Number(npv)
}

/// SLN(cost, salvage, life) -- Straight-line depreciation
fn fin_sln(args: &[CellValue]) -> CellValue {
    let cost = args.first().and_then(to_number).unwrap_or(0.0);
    let salvage = args.get(1).and_then(to_number).unwrap_or(0.0);
    let life = args.get(2).and_then(to_number).unwrap_or(1.0);

    if life == 0.0 {
        CellValue::Error("#DIV/0!".into())
    } else {
        CellValue::Number((cost - salvage) / life)
    }
}

/// DB(cost, salvage, life, period, [month])
fn fin_db(args: &[CellValue]) -> CellValue {
    let cost = args.first().and_then(to_number).unwrap_or(0.0);
    let salvage = args.get(1).and_then(to_number).unwrap_or(0.0);
    let life = args.get(2).and_then(to_number).unwrap_or(1.0);
    let period = args.get(3).and_then(to_number).unwrap_or(1.0);
    let month = args.get(4).and_then(to_number).unwrap_or(12.0);

    if cost < 0.0 || salvage < 0.0 || life <= 0.0 || period <= 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    let rate = 1.0 - (salvage / cost).powf(1.0 / life);
    let rate = (rate * 1000.0).round() / 1000.0;

    let first_period_dep = cost * rate * month / 12.0;
    if (period - 1.0).abs() < 1e-10 {
        return CellValue::Number(first_period_dep);
    }

    if ((period - life).abs() < 1e-10) || (period > life) {
        let mut total_dep = first_period_dep;
        let mut book_value = cost - first_period_dep;
        for _ in 2..(period as usize) {
            let dep = book_value * rate;
            total_dep += dep;
            book_value -= dep;
        }
        CellValue::Number(cost - total_dep - salvage)
    } else {
        let mut book_value = cost - first_period_dep;
        for _ in 2..(period as usize) {
            let dep = book_value * rate;
            book_value -= dep;
        }
        CellValue::Number(book_value * rate)
    }
}

/// DDB(cost, salvage, life, period, [factor])
fn fin_ddb(args: &[CellValue]) -> CellValue {
    let cost = args.first().and_then(to_number).unwrap_or(0.0);
    let salvage = args.get(1).and_then(to_number).unwrap_or(0.0);
    let life = args.get(2).and_then(to_number).unwrap_or(1.0);
    let period = args.get(3).and_then(to_number).unwrap_or(1.0);
    let factor = args.get(4).and_then(to_number).unwrap_or(2.0);

    if cost < 0.0 || life <= 0.0 || period <= 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    let mut book_value = cost;
    let mut total_dep = 0.0;

    for p in 1..=(period as usize) {
        let dep = (book_value * factor / life)
            .min(book_value - salvage)
            .max(0.0);
        total_dep += dep;
        book_value -= dep;

        if (p as f64 - period).abs() < 1e-10 {
            return CellValue::Number(dep);
        }
    }

    CellValue::Number(0.0)
}

/// EFFECT(nominal_rate, npery)
fn fin_effect(args: &[CellValue]) -> CellValue {
    let nominal = args.first().and_then(to_number).unwrap_or(0.0);
    let npery = args.get(1).and_then(to_number).unwrap_or(1.0);

    if nominal <= 0.0 || npery < 1.0 {
        return CellValue::Error("#NUM!".into());
    }

    CellValue::Number((1.0 + nominal / npery).powf(npery) - 1.0)
}

/// NOMINAL(effect_rate, npery)
fn fin_nominal(args: &[CellValue]) -> CellValue {
    let effect = args.first().and_then(to_number).unwrap_or(0.0);
    let npery = args.get(1).and_then(to_number).unwrap_or(1.0);

    if effect <= 0.0 || npery < 1.0 {
        return CellValue::Error("#NUM!".into());
    }

    CellValue::Number(npery * ((1.0 + effect).powf(1.0 / npery) - 1.0))
}

// --- Bond functions ---

/// Convert Excel serial date (days since 1900-01-01 with the Lotus 1-2-3 bug) to
/// (year, month, day) components. Excel serial number 1 = Jan 1, 1900.
/// Note: Excel incorrectly treats 1900 as a leap year (day 60 = Feb 29, 1900).
fn excel_date_to_ymd(serial: f64) -> (i32, u32, u32) {
    if serial <= 0.0 {
        return (1900, 1, 1);
    }

    let mut days = serial as i32;

    // Handle the Lotus 1-2-3 1900 leap year bug: day 60 should be Feb 29, 1900
    if days > 60 {
        days -= 1; // Compensate for the non-existent Feb 29, 1900
    }

    // Use a known baseline: 1900-01-01 = day 1 in Excel (after bug compensation, it's day 0)
    // 1970-01-01 = day 25569 in Excel
    let excel_epoch = 1; // Excel serial number for 1900-01-01

    let total_days = serial as i32 - excel_epoch;

    // Algorithm: start from 1900-01-01, iterate years
    let mut year = 1900i32;
    let mut remaining = total_days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let month_days_normal = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let month_days_leap = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let month_days = if is_leap_year(year) {
        &month_days_leap[..]
    } else {
        &month_days_normal[..]
    };

    let mut month = 1u32;
    for &md in month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
    }

    let day = (remaining + 1) as u32;
    (year, month, day.min(31))
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// 30/360 day count between two dates (basis=0).
/// Days between dates d1 and d2 (Excel serial numbers).
fn days_360(d1: f64, d2: f64) -> f64 {
    let (y1, m1, d1d) = excel_date_to_ymd(d1);
    let (y2, m2, d2d) = excel_date_to_ymd(d2);

    let mut dy = y2 as f64 - y1 as f64;
    let mut dm = m2 as f64 - m1 as f64;
    let mut dd = d2d as f64 - d1d as f64;

    if dd < 0.0 {
        dm -= 1.0;
        dd += 30.0;
    }
    if dm < 0.0 {
        dy -= 1.0;
        dm += 12.0;
    }

    dy * 360.0 + dm * 30.0 + dd
}

/// Actual days between two Excel serial dates.
fn days_between(d1: f64, d2: f64) -> f64 {
    d2 - d1
}

/// PRICE(settlement, maturity, rate, yld, redemption, frequency, [basis])
/// Bond price per 100 face value.
fn fin_price(args: &[CellValue]) -> CellValue {
    let settlement = args.first().and_then(to_number).unwrap_or(0.0);
    let maturity = args.get(1).and_then(to_number).unwrap_or(0.0);
    let rate = args.get(2).and_then(to_number).unwrap_or(0.0);
    let yld = args.get(3).and_then(to_number).unwrap_or(0.0);
    let redemption = args.get(4).and_then(to_number).unwrap_or(100.0);
    let frequency = args.get(5).and_then(to_number).unwrap_or(2.0) as i32;
    let _basis = args.get(6).and_then(to_number).unwrap_or(0.0) as i32;

    if settlement >= maturity
        || rate < 0.0
        || yld < 0.0
        || redemption <= 0.0
        || frequency < 1
        || frequency > 4
    {
        return CellValue::Error("#NUM!".into());
    }

    let freq = frequency as f64;

    // Use 30/360 day count for simplicity (basis=0)
    let dsr = days_360(settlement, settlement); // 0

    // Find coupon dates working backwards from maturity
    let months_per_period = 12.0 / freq;
    let (mat_y, mat_m, mat_d) = excel_date_to_ymd(maturity);

    // Number of coupons remaining
    let n = coupnum(settlement, maturity, frequency);
    if n == 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    // Days in current coupon period
    let coup_days = 360.0 / freq;

    // Days from settlement to next coupon date
    let mut prev_coupon_date = maturity;
    for _ in 0..(n as i32) {
        let (y, m, d) = excel_date_to_ymd(prev_coupon_date);
        let new_m = m as f64 - months_per_period;
        let (ny, nm) = if new_m <= 0.0 {
            (y as f64 - 1.0, new_m + 12.0)
        } else {
            (y as f64, new_m)
        };
        prev_coupon_date = date_to_excel_serial(ny as i32, nm as u32, d.min(30u32));
    }
    let next_coupon_date = maturity;

    // Days from settlement to next coupon (using actual day count)
    let dsc = days_between(settlement, next_coupon_date).max(1.0);
    // Days from beginning of period to settlement
    let e = coup_days;
    let a = e - dsc;

    // Present value of coupons
    let coupon = 100.0 * rate / freq;
    let mut pv_coupons = 0.0;
    for k in 0..(n as i32) {
        let exponent = (k as f64 + dsc / e);
        pv_coupons += coupon / (1.0 + yld / freq).powf(exponent);
    }

    // Present value of redemption
    let pv_redemption = redemption / (1.0 + yld / freq).powf((n as f64 - 1.0) + dsc / e);

    // Accrued interest
    let accrued = coupon * a / e;

    CellValue::Number(pv_coupons + pv_redemption - accrued)
}

/// YIELD(settlement, maturity, rate, pr, redemption, frequency, [basis])
/// Bond yield. Uses Newton's method.
fn fin_yield(args: &[CellValue]) -> CellValue {
    let settlement = args.first().and_then(to_number).unwrap_or(0.0);
    let maturity = args.get(1).and_then(to_number).unwrap_or(0.0);
    let rate = args.get(2).and_then(to_number).unwrap_or(0.0);
    let pr = args.get(3).and_then(to_number).unwrap_or(0.0);
    let redemption = args.get(4).and_then(to_number).unwrap_or(100.0);
    let frequency = args.get(5).and_then(to_number).unwrap_or(2.0) as i32;
    let _basis = args.get(6).and_then(to_number).unwrap_or(0.0) as i32;

    if settlement >= maturity
        || rate < 0.0
        || pr <= 0.0
        || redemption <= 0.0
        || frequency < 1
        || frequency > 4
    {
        return CellValue::Error("#NUM!".into());
    }

    let freq = frequency as f64;
    let n = coupnum(settlement, maturity, frequency);
    if n == 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    // Days from settlement to next coupon
    let next_coupon_date = maturity;
    let dsc = days_between(settlement, next_coupon_date).max(1.0);
    let e = 360.0 / freq;
    let a = e - dsc;

    let coupon = 100.0 * rate / freq;

    // Newton's method starting from rate as initial guess
    let mut guess = rate;
    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let y = guess;
        let yf = y / freq;

        // Price at current yield
        let mut price = 0.0;
        for k in 0..(n as i32) {
            let exponent = k as f64 + dsc / e;
            price += coupon / (1.0 + yf).powf(exponent);
        }
        price += redemption / (1.0 + yf).powf((n as f64 - 1.0) + dsc / e);
        price -= coupon * a / e; // subtract accrued interest

        let f_val = price - pr;

        // Derivative of price with respect to yield
        let mut deriv = 0.0;
        for k in 0..(n as i32) {
            let exponent = k as f64 + dsc / e;
            deriv -= exponent * coupon / (1.0 + yf).powf(exponent + 1.0) / freq;
        }
        deriv -= ((n as f64 - 1.0) + dsc / e) * redemption
            / (1.0 + yf).powf((n as f64 - 1.0) + dsc / e + 1.0)
            / freq;

        if deriv.abs() < 1e-15 {
            break;
        }

        let new_guess = guess - f_val / deriv;
        if (new_guess - guess).abs() < tolerance {
            return CellValue::Number(new_guess);
        }
        guess = new_guess;
    }

    CellValue::Error("#NUM!".into())
}

/// DURATION(settlement, maturity, coupon, yld, frequency, [basis])
/// Macaulay duration of a bond.
fn fin_duration(args: &[CellValue]) -> CellValue {
    let settlement = args.first().and_then(to_number).unwrap_or(0.0);
    let maturity = args.get(1).and_then(to_number).unwrap_or(0.0);
    let coupon = args.get(2).and_then(to_number).unwrap_or(0.0);
    let yld = args.get(3).and_then(to_number).unwrap_or(0.0);
    let frequency = args.get(4).and_then(to_number).unwrap_or(2.0) as i32;
    let _basis = args.get(5).and_then(to_number).unwrap_or(0.0) as i32;

    if settlement >= maturity || coupon < 0.0 || yld < 0.0 || frequency < 1 || frequency > 4 {
        return CellValue::Error("#NUM!".into());
    }

    let freq = frequency as f64;
    let n = coupnum(settlement, maturity, frequency);
    if n == 0.0 {
        return CellValue::Error("#NUM!".into());
    }

    let dsc = days_between(settlement, maturity).max(1.0);
    let e = 360.0 / freq;
    let yf = yld / freq;
    let c = coupon / freq;

    let mut weighted_sum = 0.0;
    let mut price = 0.0;

    for k in 0..(n as i32) {
        let t = k as f64 + dsc / e;
        let pv = c / (1.0 + yf).powf(t);
        weighted_sum += t * pv;
        price += pv;
    }

    // Add redemption (face value = 100)
    let t_redemption = (n as f64 - 1.0) + dsc / e;
    let pv_redemption = 100.0 / (1.0 + yf).powf(t_redemption);
    weighted_sum += t_redemption * pv_redemption;
    price += pv_redemption;

    if price.abs() < 1e-15 {
        return CellValue::Error("#NUM!".into());
    }

    CellValue::Number(weighted_sum / price / freq)
}

/// MDURATION(settlement, maturity, coupon, yld, frequency, [basis])
/// Modified duration = Macaulay duration / (1 + yield/frequency).
fn fin_mduration(args: &[CellValue]) -> CellValue {
    let yld = args.get(3).and_then(to_number).unwrap_or(0.0);
    let frequency = args.get(4).and_then(to_number).unwrap_or(2.0) as i32;

    let duration = match to_number(&fin_duration(args)) {
        Some(d) => d,
        None => return CellValue::Error("#NUM!".into()),
    };

    CellValue::Number(duration / (1.0 + yld / frequency as f64))
}

/// COUPNUM(settlement, maturity, frequency, [basis])
/// Number of coupons between settlement and maturity.
fn fin_coupnum(args: &[CellValue]) -> CellValue {
    let settlement = args.first().and_then(to_number).unwrap_or(0.0);
    let maturity = args.get(1).and_then(to_number).unwrap_or(0.0);
    let frequency = args.get(2).and_then(to_number).unwrap_or(2.0) as i32;
    let _basis = args.get(3).and_then(to_number).unwrap_or(0.0) as i32;

    if settlement >= maturity || frequency < 1 || frequency > 4 {
        return CellValue::Error("#NUM!".into());
    }

    CellValue::Number(coupnum(settlement, maturity, frequency))
}

/// Helper: compute number of coupons remaining.
fn coupnum(settlement: f64, maturity: f64, frequency: i32) -> f64 {
    let days = days_360(settlement, maturity);
    let period_days = 360.0 / frequency as f64;
    (days / period_days).ceil().max(1.0)
}

/// COUPDAYS(settlement, maturity, frequency, [basis])
/// Days in the coupon period containing settlement.
fn fin_coupdays(args: &[CellValue]) -> CellValue {
    let frequency = args.get(2).and_then(to_number).unwrap_or(2.0) as i32;

    if frequency < 1 || frequency > 4 {
        return CellValue::Error("#NUM!".into());
    }

    CellValue::Number(360.0 / frequency as f64)
}

/// COUPDAYBS(settlement, maturity, frequency, [basis])
/// Days from beginning of coupon period to settlement.
fn fin_coupdaybs(args: &[CellValue]) -> CellValue {
    let settlement = args.first().and_then(to_number).unwrap_or(0.0);
    let maturity = args.get(1).and_then(to_number).unwrap_or(0.0);
    let frequency = args.get(2).and_then(to_number).unwrap_or(2.0) as i32;
    let _basis = args.get(3).and_then(to_number).unwrap_or(0.0) as i32;

    if settlement >= maturity || frequency < 1 || frequency > 4 {
        return CellValue::Error("#NUM!".into());
    }

    let period_days = 360.0 / frequency as f64;
    let total_days = days_360(settlement, maturity);
    let dsc = total_days % period_days;
    let dsc = if dsc == 0.0 { period_days } else { dsc };

    CellValue::Number(period_days - dsc)
}

/// COUPDAYSNC(settlement, maturity, frequency, [basis])
/// Days from settlement to next coupon date.
fn fin_coupdaysnc(args: &[CellValue]) -> CellValue {
    let settlement = args.first().and_then(to_number).unwrap_or(0.0);
    let maturity = args.get(1).and_then(to_number).unwrap_or(0.0);
    let frequency = args.get(2).and_then(to_number).unwrap_or(2.0) as i32;
    let _basis = args.get(3).and_then(to_number).unwrap_or(0.0) as i32;

    if settlement >= maturity || frequency < 1 || frequency > 4 {
        return CellValue::Error("#NUM!".into());
    }

    let period_days = 360.0 / frequency as f64;
    let total_days = days_360(settlement, maturity);
    let dsc = total_days % period_days;
    let dsc = if dsc == 0.0 { period_days } else { dsc };

    CellValue::Number(dsc)
}

/// Convert (year, month, day) back to Excel serial number.
fn date_to_excel_serial(year: i32, month: u32, day: u32) -> f64 {
    let mut days = 0;
    for y in 1900..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for m in 0..(month as usize - 1) {
        days += month_days[m];
    }
    (days + day as i32 + 1) as f64 // +1 because Excel serial 1 = Jan 1, 1900
}
