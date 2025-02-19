use std::{fmt, str::FromStr};

use anyhow::{anyhow, Error, Result};
use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike,
    Utc, Weekday,
};
use gtk::{glib, pango};
use serde::{Deserialize, Serialize};

use crate::date_time;

const MIN_TIME: NaiveTime = NaiveTime::MIN;

#[allow(deprecated)]
const MAX_TIME: NaiveTime = NaiveTime::from_hms(23, 59, 59);

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, glib::Boxed)]
#[boxed_type(name = "UetsDateTimeRange")]
pub struct DateTimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

impl fmt::Display for DateTimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (_, stripped, _) =
            pango::parse_markup(&self.label_markup(), 0 as char).map_err(|_| fmt::Error)?;
        write!(f, "{}", stripped)
    }
}

impl FromStr for DateTimeRange {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.to_lowercase();
        let s = s.trim();

        if s == "all time" || s.is_empty() {
            return Ok(Self::all_time());
        }

        if s.starts_with("until") {
            let end = date_time::parse(s.trim_start_matches("until").trim())?;
            return Ok(Self {
                start: None,
                end: Some(end),
            });
        }

        if let Some((raw_start, raw_end)) = s.split_once("to") {
            let start = date_time::parse(raw_start.trim())?;
            let end = date_time::parse(raw_end.trim())?;
            return Ok(Self {
                start: Some(start),
                end: Some(end),
            });
        }

        if s.ends_with("onwards") {
            let start = date_time::parse(s.trim_end_matches("onwards").trim())?;
            return Ok(Self {
                start: Some(start),
                end: None,
            });
        }

        Err(anyhow!("`{}` did not match any formats", s))
    }
}

impl DateTimeRange {
    pub fn all_time() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    pub fn today() -> Self {
        let now = Local::now().naive_local();
        Self::from_naive_local(
            NaiveDateTime::new(now.date(), MIN_TIME),
            NaiveDateTime::new(now.date(), MAX_TIME),
        )
    }

    pub fn yesterday() -> Self {
        let now = Local::now().naive_local();
        let yesterday = now.date().pred_opt().unwrap();
        Self::from_naive_local(
            NaiveDateTime::new(yesterday, MIN_TIME),
            NaiveDateTime::new(yesterday, MAX_TIME),
        )
    }

    pub fn this_week() -> Self {
        let now = Local::now().naive_local();
        let today = now.date();

        let weekday = today.weekday();
        let start_of_week = if weekday == Weekday::Sun {
            today
        } else {
            today - Duration::days(weekday.num_days_from_sunday() as i64)
        };

        let end_of_week = start_of_week + Duration::days(6);

        Self::from_naive_local(
            NaiveDateTime::new(start_of_week, MIN_TIME),
            NaiveDateTime::new(end_of_week, MAX_TIME),
        )
    }

    pub fn this_month() -> Self {
        let now = Local::now().naive_local();
        let start_of_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
        let end_of_month = NaiveDate::from_ymd_opt(
            now.year(),
            now.month(),
            days_in_month(now.year(), now.month()),
        )
        .unwrap();

        Self::from_naive_local(
            NaiveDateTime::new(start_of_month, MIN_TIME),
            NaiveDateTime::new(end_of_month, MAX_TIME),
        )
    }

    pub fn this_year() -> Self {
        let now = Local::now().naive_local();
        let start_of_year = NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap();
        let end_of_year = NaiveDate::from_ymd_opt(now.year(), 12, 31).unwrap();

        Self::from_naive_local(
            NaiveDateTime::new(start_of_year, MIN_TIME),
            NaiveDateTime::new(end_of_year, MAX_TIME),
        )
    }

    pub fn is_all_time(&self) -> bool {
        self.eq_ignore_subsec(&Self::all_time())
    }

    pub fn is_today(&self) -> bool {
        self.eq_ignore_subsec(&Self::today())
    }

    pub fn is_yesterday(&self) -> bool {
        self.eq_ignore_subsec(&Self::yesterday())
    }

    pub fn is_this_week(&self) -> bool {
        self.eq_ignore_subsec(&Self::this_week())
    }

    pub fn is_this_month(&self) -> bool {
        self.eq_ignore_subsec(&Self::this_month())
    }

    pub fn is_this_year(&self) -> bool {
        self.eq_ignore_subsec(&Self::this_year())
    }

    pub fn is_empty(&self) -> bool {
        match (self.start, self.end) {
            (Some(start), Some(end)) => start > end,
            _ => false,
        }
    }

    pub fn contains<Tz: TimeZone>(&self, dt: DateTime<Tz>) -> bool {
        match (self.start, self.end) {
            (Some(s), Some(e)) => s <= dt && dt <= e,
            (Some(s), None) => s <= dt,
            (None, Some(e)) => dt <= e,
            (None, None) => true,
        }
    }

    pub fn label_markup(&self) -> String {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                format!(
                    "<b>{}</b> to <b>{}</b>",
                    glib::markup_escape_text(&date_time::format::human_readable(start)),
                    glib::markup_escape_text(&date_time::format::human_readable(end)),
                )
            }
            (Some(start), None) => {
                format!(
                    "<b>{}</b> Onwards",
                    glib::markup_escape_text(&date_time::format::human_readable(start)),
                )
            }
            (None, Some(end)) => {
                format!(
                    "Until <b>{}</b>",
                    glib::markup_escape_text(&date_time::format::human_readable(end)),
                )
            }
            (None, None) => "<b>All Time</b>".to_string(),
        }
    }

    pub fn short_label_markup(&self) -> String {
        if self.is_today() {
            return "<b>Today</b>".to_string();
        }

        if self.is_yesterday() {
            return "<b>Yesterday</b>".to_string();
        }

        if self.is_this_week() {
            return "<b>This Week</b>".to_string();
        }

        if self.is_this_month() {
            return "<b>This Month</b>".to_string();
        }

        if self.is_this_year() {
            return "<b>This Year</b>".to_string();
        }

        self.label_markup()
    }

    fn eq_ignore_subsec(&self, other: &Self) -> bool {
        match (self.start, self.end, other.start, other.end) {
            (Some(start), Some(end), Some(other_start), Some(other_end)) => {
                is_eq_ignore_subsec(start, other_start) && is_eq_ignore_subsec(end, other_end)
            }
            (Some(start), None, Some(other_start), None) => is_eq_ignore_subsec(start, other_start),
            (None, Some(end), None, Some(other_end)) => is_eq_ignore_subsec(end, other_end),
            (None, None, None, None) => true,
            _ => false,
        }
    }

    fn from_naive_local(start: NaiveDateTime, end: NaiveDateTime) -> Self {
        Self {
            start: Some(start.and_local_timezone(Local).single().unwrap().to_utc()),
            end: Some(end.and_local_timezone(Local).single().unwrap().to_utc()),
        }
    }
}

fn is_eq_ignore_subsec(a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
    a.date_naive() == b.date_naive()
        && a.hour() == b.hour()
        && a.minute() == b.minute()
        && a.second() == b.second()
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let start_dt = DateTime::parse_from_rfc3339("2024-11-03T01:21:16Z")
            .unwrap()
            .to_utc();
        let end_dt = DateTime::parse_from_rfc3339("2024-11-04T02:22:17Z")
            .unwrap()
            .to_utc();

        assert_eq!(
            DateTimeRange {
                start: Some(start_dt),
                end: Some(end_dt),
            },
            " 2024-11-03 01:21:16Z   to  2024-11-04 02:22:17Z "
                .parse()
                .unwrap(),
        );
        assert_eq!(
            DateTimeRange {
                start: None,
                end: Some(end_dt),
            },
            " Until 2024-11-04 02:22:17Z  ".parse().unwrap(),
        );
        assert_eq!(
            DateTimeRange {
                start: Some(start_dt),
                end: None,
            },
            "  2024-11-03 01:21:16Z  Onwards  ".parse().unwrap(),
        );
        assert_eq!(
            DateTimeRange {
                start: None,
                end: None,
            },
            "All Time".parse().unwrap(),
        );
    }
}
