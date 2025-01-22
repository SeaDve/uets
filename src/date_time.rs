use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime, Utc};

pub fn parse(input: &str) -> Result<DateTime<Utc>> {
    dateparser::parse_with(input, &Local, NaiveTime::MIN)
        .or_else(|_| parse_from_str(input, "%b %d %Y"))
        .or_else(|_| parse_from_str(input, "%b %d %Y %r"))
}

fn parse_from_str(input: &str, format: &str) -> Result<DateTime<Utc>> {
    Ok(NaiveDate::parse_from_str(input, format)?
        .and_time(NaiveTime::MIN)
        .and_local_timezone(Local)
        .single()
        .context("Invalid date")?
        .to_utc())
}

pub mod format {
    use super::*;

    pub fn parseable(dt: DateTime<Utc>) -> String {
        let dt = dt.with_timezone(&Local).naive_local();

        if dt.time() == NaiveTime::MIN {
            dt.format("%Y-%m-%d").to_string() // 2024-11-03
        } else {
            dt.format("%Y-%m-%d %H:%M:%S").to_string() // 2024-11-03 01:21:16
        }
    }

    pub fn human_readable(dt: DateTime<Utc>) -> String {
        let dt = dt.with_timezone(&Local).naive_local();

        if dt.time() == NaiveTime::MIN {
            dt.format("%b %-d %Y").to_string() // Nov 3 2024
        } else {
            dt.format("%b %-d %Y %r").to_string() // Nov 3 2024 1:21:16 AM
        }
    }

    pub fn human_readable_date(dt: DateTime<Utc>) -> String {
        let dt = dt.with_timezone(&Local);

        dt.format("%b %-d %Y").to_string()
    }

    pub fn fuzzy(dt: DateTime<Utc>) -> String {
        let dt = dt.with_timezone(&Local);
        let now = Local::now();

        if dt.year() == now.year() && dt.month() == now.month() && dt.day() == now.day() {
            dt.format("today at %r").to_string() // today at 13:21:16 AM
        } else if (now - dt).num_hours() <= 30 {
            dt.format("yesterday at %r").to_string() // yesterday at 13:21:16 AM
        } else {
            dt.format("%a, %-d %b %Y at %r").to_string() // Sun, 3 Nov 2024 at 13:21:16 AM
        }
    }
}
