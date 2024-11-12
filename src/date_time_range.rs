use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use gtk::glib;

const MIN_TIME: NaiveTime = NaiveTime::MIN;

#[allow(deprecated)]
const MAX_TIME: NaiveTime = NaiveTime::from_hms(23, 59, 59);

const DT_FORMAT: &str = "%b %-d %Y %r";

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, glib::Boxed)]
#[boxed_type(name = "UetsDateTimeRange")]
pub struct DateTimeRange {
    pub start: Option<NaiveDateTime>,
    pub end: Option<NaiveDateTime>,
}

impl DateTimeRange {
    pub fn all_time() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    pub fn onwards(start: NaiveDateTime) -> Self {
        Self {
            start: Some(start),
            end: None,
        }
    }

    pub fn until(end: NaiveDateTime) -> Self {
        Self {
            start: None,
            end: Some(end),
        }
    }

    pub fn today() -> Self {
        let now = Local::now().naive_local();
        Self::custom(
            NaiveDateTime::new(now.date(), MIN_TIME),
            NaiveDateTime::new(now.date(), MAX_TIME),
        )
    }

    pub fn yesterday() -> Self {
        let now = Local::now().naive_local();
        let yesterday = now.date().pred_opt().unwrap();
        Self::custom(
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

        Self::custom(
            NaiveDateTime::new(start_of_week, MIN_TIME),
            NaiveDateTime::new(end_of_week, MAX_TIME),
        )
    }

    pub fn this_month() -> Self {
        let now = Local::now().naive_local();
        let start_of_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
        let end_of_month =
            NaiveDate::from_ymd_opt(now.year(), now.month(), now.date().days_in_month()).unwrap();

        Self::custom(
            NaiveDateTime::new(start_of_month, MIN_TIME),
            NaiveDateTime::new(end_of_month, MAX_TIME),
        )
    }

    pub fn this_year() -> Self {
        let now = Local::now().naive_local();
        let start_of_month = NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap();
        let end_of_month =
            start_of_month + chrono::Duration::days(start_of_month.days_in_month() as i64 - 1);

        Self::custom(
            NaiveDateTime::new(start_of_month, MIN_TIME),
            NaiveDateTime::new(end_of_month, MAX_TIME),
        )
    }

    pub fn is_all_time(&self) -> bool {
        self.eq_ignore_subsec(&Self::all_time())
    }

    pub fn is_empty(&self) -> bool {
        match (self.start, self.end) {
            (Some(start), Some(end)) => start > end,
            _ => false,
        }
    }

    pub fn contains(&self, date_time: NaiveDateTime) -> bool {
        match (self.start, self.end) {
            (Some(start), Some(end)) => start <= date_time && date_time <= end,
            (Some(start), None) => start <= date_time,
            (None, Some(end)) => date_time <= end,
            (None, None) => true,
        }
    }

    pub fn eq_ignore_subsec(&self, other: &Self) -> bool {
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

    pub fn label_markup(&self) -> String {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                format!(
                    "<b>{}</b> to <b>{}</b>",
                    glib::markup_escape_text(&start.format(DT_FORMAT).to_string()),
                    glib::markup_escape_text(&end.format(DT_FORMAT).to_string()),
                )
            }
            (Some(start), None) => {
                format!(
                    "<b>{}</b> Onwards",
                    glib::markup_escape_text(&start.format(DT_FORMAT).to_string()),
                )
            }
            (None, Some(end)) => {
                format!(
                    "Until <b>{}</b>",
                    glib::markup_escape_text(&end.format(DT_FORMAT).to_string()),
                )
            }
            (None, None) => "All Time".to_string(),
        }
    }

    fn custom(start: NaiveDateTime, end: NaiveDateTime) -> Self {
        Self {
            start: Some(start),
            end: Some(end),
        }
    }
}

fn is_eq_ignore_subsec(a: NaiveDateTime, b: NaiveDateTime) -> bool {
    a.date() == b.date()
        && a.hour() == b.hour()
        && a.minute() == b.minute()
        && a.second() == b.second()
}

trait NaiveDateExt {
    fn days_in_month(&self) -> u32;
    fn is_leap_year(&self) -> bool;
}

impl NaiveDateExt for NaiveDate {
    fn days_in_month(&self) -> u32 {
        let month = self.month();
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.is_leap_year() {
                    29
                } else {
                    28
                }
            }
            _ => unreachable!(),
        }
    }

    fn is_leap_year(&self) -> bool {
        let year = self.year();
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }
}
