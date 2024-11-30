use anyhow::Result;
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Local, NaiveTime, Utc,
};

pub fn parse(input: &str) -> Result<DateTime<Utc>> {
    dateparser::parse_with(input, &Local, NaiveTime::MIN)
}

pub fn parseable_format(dt: &DateTime<Utc>) -> DelayedFormat<StrftimeItems<'_>> {
    let dt = dt.with_timezone(&Local).naive_local();

    if dt.time() == NaiveTime::MIN {
        return dt.format("%Y-%m-%d");
    }

    dt.format("%Y-%m-%d %H:%M:%S")
}
