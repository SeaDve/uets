use anyhow::Result;
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Local, NaiveDateTime, NaiveTime, Utc,
};

use crate::{date_time_range::DateTimeRange, search_query::SearchQueries};

pub trait SearchQueriesDateTimeRangeExt {
    fn get_dt_range(&self, start_iden: &str, end_iden: &str) -> DateTimeRange;
    fn set_dt_range(&mut self, start_iden: &str, end_iden: &str, dt_range: DateTimeRange);
}

impl SearchQueriesDateTimeRangeExt for SearchQueries {
    fn get_dt_range(&self, start_iden: &str, end_iden: &str) -> DateTimeRange {
        DateTimeRange {
            start: self
                .find_last(start_iden)
                .and_then(|dt_str| parse_dt(dt_str).ok())
                .map(|dt| dt.with_timezone(&Local).naive_local()),
            end: self
                .find_last(end_iden)
                .and_then(|dt_str| parse_dt(dt_str).ok())
                .map(|dt| dt.with_timezone(&Local).naive_local()),
        }
    }

    fn set_dt_range(&mut self, start_iden: &str, end_iden: &str, dt_range: DateTimeRange) {
        if let Some(end) = dt_range.end {
            self.replace_all_iden_or_insert(end_iden, &parseable_dt_fmt(&end).to_string());
        } else {
            self.remove_all_iden(end_iden);
        }

        if let Some(start) = dt_range.start {
            self.replace_all_iden_or_insert(start_iden, &parseable_dt_fmt(&start).to_string());
        } else {
            self.remove_all_iden(start_iden);
        }
    }
}

fn parse_dt(input: &str) -> Result<DateTime<Utc>> {
    dateparser::parse_with(input, &Local, NaiveTime::MIN)
}

fn parseable_dt_fmt(dt: &NaiveDateTime) -> DelayedFormat<StrftimeItems<'_>> {
    if dt.time() == NaiveTime::MIN {
        return dt.format("%Y-%m-%d");
    }

    dt.format("%Y-%m-%d %H:%M:%S")
}
