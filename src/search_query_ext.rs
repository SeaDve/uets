use crate::{date_time, date_time_range::DateTimeRange, search_query::SearchQueries};

pub trait SearchQueriesDateTimeRangeExt {
    fn dt_range(&self, start_iden: &str, end_iden: &str) -> DateTimeRange;
    fn set_dt_range(&mut self, start_iden: &str, end_iden: &str, dt_range: DateTimeRange);
}

impl SearchQueriesDateTimeRangeExt for SearchQueries {
    fn dt_range(&self, start_iden: &str, end_iden: &str) -> DateTimeRange {
        DateTimeRange {
            start: self
                .find_last(start_iden)
                .and_then(|dt_str| date_time::parse(dt_str).ok()),
            end: self
                .find_last(end_iden)
                .and_then(|dt_str| date_time::parse(dt_str).ok()),
        }
    }

    fn set_dt_range(&mut self, start_iden: &str, end_iden: &str, dt_range: DateTimeRange) {
        if let Some(end) = dt_range.end {
            self.replace_all_iden_or_insert(end_iden, &date_time::format::parseable(end));
        } else {
            self.remove_all_iden(end_iden);
        }

        if let Some(start) = dt_range.start {
            self.replace_all_iden_or_insert(start_iden, &date_time::format::parseable(start));
        } else {
            self.remove_all_iden(start_iden);
        }
    }
}
