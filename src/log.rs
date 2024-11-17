use std::collections::BTreeMap;

use crate::date_time::DateTime;

#[derive(Default)]
pub struct Log<T> {
    map: BTreeMap<DateTime, T>,
}

impl<T: Copy> Log<T> {
    pub fn latest(&self) -> Option<&T> {
        self.map.last_key_value().map(|(_, v)| v)
    }

    pub fn latest_dt(&self) -> Option<DateTime> {
        self.map.last_key_value().map(|(dt, _)| *dt)
    }

    pub fn for_dt(&self, dt: DateTime) -> Option<&T> {
        self.map.range(..=dt).next_back().map(|(_, v)| v)
    }

    pub fn insert(&mut self, dt: DateTime, value: T) {
        self.map.insert(dt, value);
    }
}
