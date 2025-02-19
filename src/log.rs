use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Log<T> {
    map: BTreeMap<DateTime<Utc>, T>,
}

impl<T> Default for Log<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Log<T> {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn latest_dt(&self) -> Option<DateTime<Utc>> {
        self.map.last_key_value().map(|(dt, _)| *dt)
    }

    pub fn latest(&self) -> Option<&T> {
        self.latest_full().map(|(_, v)| v)
    }

    pub fn latest_full(&self) -> Option<(DateTime<Utc>, &T)> {
        self.map.last_key_value().map(|(dt, v)| (*dt, v))
    }

    pub fn for_dt(&self, dt: DateTime<Utc>) -> Option<&T> {
        self.for_dt_full(dt).map(|(_, v)| v)
    }

    pub fn for_dt_full(&self, dt: DateTime<Utc>) -> Option<(DateTime<Utc>, &T)> {
        self.map.range(..=dt).next_back().map(|(dt, v)| (*dt, v))
    }

    pub fn insert(&mut self, dt: DateTime<Utc>, value: T) {
        self.map.insert(dt, value);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}
