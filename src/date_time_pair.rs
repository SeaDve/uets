use chrono::TimeDelta;

use crate::date_time::DateTime;

#[derive(Debug, Clone)]
pub struct DateTimePair {
    pub entry: DateTime,
    pub exit: Option<DateTime>,
}

impl DateTimePair {
    pub fn inside_duration(&self) -> Option<TimeDelta> {
        self.exit
            .as_ref()
            .map(|exit| exit.inner() - self.entry.inner())
    }
}
