//! Compiles and decompiles board settings code and creates default config based on unicode
use chrono::{Datelike, NaiveDate};

#[derive(Debug)]
pub struct LocaleDateFormat {
    pub format: String,  // e.g. "%d-%m-%y"
    pub separator: char, // '-', '/', '.'
}

fn detect_locale_date_format() -> LocaleDateFormat {
    // Pick a date with distinct values
    let sample = NaiveDate::from_ymd_opt(2026, 1, 16).unwrap();

    // %x is locale-dependent
    let localized = sample.format("%x").to_string();

    // Detect separator
    let separator = localized
        .chars()
        .find(|c| !c.is_ascii_digit())
        .unwrap_or('/');

    // Build inferred format by position
    let parts: Vec<&str> = localized.split(separator).collect();

    let format = match parts.as_slice() {
        // dd-mm-yy
        [d, m, y] if d.len() == 2 && m.len() == 2 => {
            if y.len() == 2 {
                format!("%d{separator}%m{separator}%y")
            } else {
                format!("%d{separator}%m{separator}%Y")
            }
        }
        // mm-dd-yy
        [_m, _d, y] => {
            if y.len() == 2 {
                format!("%m{separator}%d{separator}%y")
            } else {
                format!("%m{separator}%d{separator}%Y")
            }
        }
        _ => "%x".to_string(),
    };

    LocaleDateFormat { format, separator }
}

pub fn format_date(day: u8, month: u8, year: i32) -> String {
    let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32).expect("Invalid date");

    let locale_fmt = detect_locale_date_format();
    date.format(&locale_fmt.format).to_string()
}

pub fn parse_date(s: &str) -> Result<(u8, u8, i32), chrono::ParseError> {
    let locale_fmt = detect_locale_date_format();

    let date = NaiveDate::parse_from_str(s, &locale_fmt.format)?;

    Ok((date.day() as u8, date.month() as u8, date.year()))
}
