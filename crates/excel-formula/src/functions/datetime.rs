//! Date and time functions.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

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
        "TODAY".into(),
        Arc::new(|_args, _provider| datetime_today()),
    );
    registry.insert("NOW".into(), Arc::new(|_args, _provider| datetime_now()));
    registry.insert(
        "DATE".into(),
        Arc::new(|args, provider| datetime_date(args)),
    );
    registry.insert(
        "TIME".into(),
        Arc::new(|args, provider| datetime_time(args)),
    );
    registry.insert(
        "YEAR".into(),
        Arc::new(|args, provider| datetime_year(args)),
    );
    registry.insert(
        "MONTH".into(),
        Arc::new(|args, provider| datetime_month(args)),
    );
    registry.insert("DAY".into(), Arc::new(|args, provider| datetime_day(args)));
    registry.insert(
        "HOUR".into(),
        Arc::new(|args, provider| datetime_hour(args)),
    );
    registry.insert(
        "MINUTE".into(),
        Arc::new(|args, provider| datetime_minute(args)),
    );
    registry.insert(
        "SECOND".into(),
        Arc::new(|args, provider| datetime_second(args)),
    );
    registry.insert(
        "WEEKDAY".into(),
        Arc::new(|args, provider| datetime_weekday(args)),
    );
    registry.insert(
        "WEEKNUM".into(),
        Arc::new(|args, provider| datetime_weeknum(args)),
    );
    registry.insert(
        "EDATE".into(),
        Arc::new(|args, provider| datetime_edate(args)),
    );
    registry.insert(
        "EOMONTH".into(),
        Arc::new(|args, provider| datetime_eomonth(args)),
    );
    registry.insert(
        "DAYS".into(),
        Arc::new(|args, provider| datetime_days(args)),
    );
    registry.insert(
        "DAYS360".into(),
        Arc::new(|args, provider| datetime_days360(args)),
    );
    registry.insert(
        "NETWORKDAYS".into(),
        Arc::new(|args, provider| datetime_networkdays(args)),
    );
    registry.insert(
        "DATEDIF".into(),
        Arc::new(|args, provider| datetime_datedif(args)),
    );
}

/// Convert a date-like cell value to a chrono NaiveDate.
fn to_date(val: &CellValue) -> Option<NaiveDate> {
    match val {
        CellValue::DateTime(dt) => Some(dt.date()),
        CellValue::String(s) => {
            // Try common date formats
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .or_else(|_| NaiveDate::parse_from_str(s, "%m/%d/%Y"))
                .or_else(|_| NaiveDate::parse_from_str(s, "%d/%m/%Y"))
                .ok()
        }
        _ => None,
    }
}

fn datetime_today() -> CellValue {
    let today = chrono::Local::now().date_naive();
    let dt = today.and_hms_opt(0, 0, 0).expect("valid datetime");
    CellValue::DateTime(dt)
}

fn datetime_now() -> CellValue {
    CellValue::DateTime(chrono::Local::now().naive_local())
}

fn datetime_date(args: &[CellValue]) -> CellValue {
    let year = args.first().and_then(to_number).unwrap_or(1900.0) as i32;
    let month = args.get(1).and_then(to_number).unwrap_or(1.0) as u32;
    let day = args.get(2).and_then(to_number).unwrap_or(1.0) as u32;

    match NaiveDate::from_ymd_opt(year, month, day) {
        Some(d) => {
            let dt = d.and_hms_opt(0, 0, 0).expect("valid datetime");
            CellValue::DateTime(dt)
        }
        None => CellValue::Error("#NUM!".into()),
    }
}

fn datetime_time(args: &[CellValue]) -> CellValue {
    let hour = args.first().and_then(to_number).unwrap_or(0.0) as u32;
    let minute = args.get(1).and_then(to_number).unwrap_or(0.0) as u32;
    let second = args.get(2).and_then(to_number).unwrap_or(0.0) as u32;

    match NaiveTime::from_hms_opt(hour, minute, second) {
        Some(t) => {
            let d = NaiveDate::from_ymd_opt(1899, 12, 30).expect("valid date");
            CellValue::DateTime(NaiveDateTime::new(d, t))
        }
        None => CellValue::Error("#NUM!".into()),
    }
}

fn datetime_year(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_date) {
        Some(d) => CellValue::Number(d.year() as f64),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_month(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_date) {
        Some(d) => CellValue::Number(d.month() as f64),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_day(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_date) {
        Some(d) => CellValue::Number(d.day() as f64),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_hour(args: &[CellValue]) -> CellValue {
    match args.first() {
        Some(CellValue::DateTime(dt)) => CellValue::Number(dt.hour() as f64),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_minute(args: &[CellValue]) -> CellValue {
    match args.first() {
        Some(CellValue::DateTime(dt)) => CellValue::Number(dt.minute() as f64),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_second(args: &[CellValue]) -> CellValue {
    match args.first() {
        Some(CellValue::DateTime(dt)) => CellValue::Number(dt.second() as f64),
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_weekday(args: &[CellValue]) -> CellValue {
    let return_type = args.get(1).and_then(to_number).unwrap_or(1.0) as u32;
    match args.first().and_then(to_date) {
        Some(d) => {
            let num = d.weekday().number_from_monday(); // 1=Mon, 7=Sun
            let result = match return_type {
                1 => {
                    // 1=Sun, 7=Sat
                    if num == 7 { 1.0 } else { (num + 1) as f64 }
                }
                2 => {
                    // 1=Mon, 7=Sun
                    num as f64
                }
                3 => {
                    // 0=Mon, 6=Sun
                    (num - 1) as f64
                }
                _ => num as f64,
            };
            CellValue::Number(result)
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_weeknum(args: &[CellValue]) -> CellValue {
    match args.first().and_then(to_date) {
        Some(d) => CellValue::Number(d.iso_week().week() as f64),
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_edate(args: &[CellValue]) -> CellValue {
    let date = args.first().and_then(to_date);
    let months = args.get(1).and_then(to_number).unwrap_or(0.0) as i32;

    match date {
        Some(d) => {
            let total_months = d.year() * 12 + d.month() as i32 - 1 + months;
            let new_year = total_months / 12;
            let new_month = ((total_months % 12) + 12) % 12 + 1;
            let new_day = d.day().min(
                NaiveDate::from_ymd_opt(new_year, new_month as u32, 1)
                    .and_then(|first| first.checked_add_signed(chrono::TimeDelta::days(31)))
                    .map(|last| last.day())
                    .unwrap_or(28),
            );
            match NaiveDate::from_ymd_opt(new_year, new_month as u32, new_day) {
                Some(nd) => {
                    let dt = nd.and_hms_opt(0, 0, 0).expect("valid datetime");
                    CellValue::DateTime(dt)
                }
                None => CellValue::Error("#NUM!".into()),
            }
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_eomonth(args: &[CellValue]) -> CellValue {
    // EOMONTH(start_date, months): last day of month after offset
    let date = args.first().and_then(to_date);
    let months = args.get(1).and_then(to_number).unwrap_or(0.0) as i32;

    match date {
        Some(d) => {
            let total_months = d.year() * 12 + d.month() as i32 - 1 + months + 1;
            let new_year = total_months / 12;
            let new_month = ((total_months % 12) + 12) % 12 + 1;

            // First day of target month, then go back one day
            let first_of_target = NaiveDate::from_ymd_opt(new_year, new_month as u32, 1);
            match first_of_target {
                Some(first) => {
                    let last_day = first.pred_opt().unwrap_or(first);
                    let dt = last_day.and_hms_opt(0, 0, 0).expect("valid datetime");
                    CellValue::DateTime(dt)
                }
                None => CellValue::Error("#NUM!".into()),
            }
        }
        None => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_days(args: &[CellValue]) -> CellValue {
    let start = args.first().and_then(to_date);
    let end = args.get(1).and_then(to_date);

    match (start, end) {
        (Some(s), Some(e)) => {
            let diff = e.signed_duration_since(s).num_days();
            CellValue::Number(diff as f64)
        }
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_days360(args: &[CellValue]) -> CellValue {
    // 360-day year (NASD method)
    let start = args.first().and_then(to_date);
    let end = args.get(1).and_then(to_date);

    match (start, end) {
        (Some(s), Some(e)) => {
            let (sy, sm, sd) = (s.year(), s.month() as i32, s.day() as i32);
            let (ey, em, ed) = (e.year(), e.month() as i32, e.day() as i32);

            // Adjust last day of February
            let sd_adj = if sm == 2 && sd >= 28 { 30 } else { sd };
            let ed_adj = if em == 2 && ed >= 28 { 30 } else { ed };

            let result = (ey - sy) * 360 + (em - sm) * 30 + (ed_adj - sd_adj);
            CellValue::Number(result as f64)
        }
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_networkdays(args: &[CellValue]) -> CellValue {
    // Simple implementation without holidays
    let start = args.first().and_then(to_date);
    let end = args.get(1).and_then(to_date);

    match (start, end) {
        (Some(s), Some(e)) => {
            let mut count: i64 = 0;
            let mut current = s;
            while current <= e {
                let wd = current.weekday().number_from_monday();
                if wd <= 5 {
                    // Mon-Fri
                    count += 1;
                }
                current = current
                    .checked_add_signed(chrono::TimeDelta::days(1))
                    .unwrap_or(current);
            }
            CellValue::Number(count as f64)
        }
        _ => CellValue::Error("#VALUE!".into()),
    }
}

fn datetime_datedif(args: &[CellValue]) -> CellValue {
    // DATEDIF(start_date, end_date, unit)
    let start = args.first().and_then(to_date);
    let end = args.get(1).and_then(to_date);
    let unit = args
        .get(2)
        .map(|v| match v {
            CellValue::String(s) => s.to_uppercase(),
            _ => String::new(),
        })
        .unwrap_or_default();

    match (start, end) {
        (Some(s), Some(e)) => {
            let diff = e.signed_duration_since(s);
            let result = match unit.as_str() {
                "Y" => e.years_since(s).unwrap_or(0) as f64,
                "M" => {
                    let y_diff = e.year() - s.year();
                    let m_diff = e.month() as i32 - s.month() as i32;
                    (y_diff * 12 + m_diff) as f64
                }
                "D" => diff.num_days() as f64,
                "YD" => {
                    // Days excluding years
                    let e_this_year = NaiveDate::from_ymd_opt(s.year(), e.month(), e.day());
                    match e_this_year {
                        Some(ed) => {
                            let d = ed.signed_duration_since(s);
                            if d.num_days() < 0 {
                                (d.num_days() + 365) as f64
                            } else {
                                d.num_days() as f64
                            }
                        }
                        None => CellValue::Error("#NUM!".into()).into_num(),
                    }
                }
                "MD" => {
                    // Days excluding months
                    let e_this_month = NaiveDate::from_ymd_opt(e.year(), e.month(), s.day());
                    match e_this_month {
                        Some(ed) => {
                            let d = ed.signed_duration_since(e);
                            d.num_days().abs() as f64
                        }
                        None => CellValue::Error("#NUM!".into()).into_num(),
                    }
                }
                _ => f64::NAN,
            };

            if result.is_nan() {
                CellValue::Error("#VALUE!".into())
            } else {
                CellValue::Number(result)
            }
        }
        _ => CellValue::Error("#VALUE!".into()),
    }
}

// Helper trait for converting CellValue to f64 for arithmetic
trait IntoNum {
    fn into_num(self) -> f64;
}

impl IntoNum for CellValue {
    fn into_num(self) -> f64 {
        match self {
            CellValue::Number(n) => n,
            _ => f64::NAN,
        }
    }
}
