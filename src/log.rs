use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

pub struct Log<T> {
    map: BTreeMap<DateTime<Utc>, T>,
}

impl<T> Default for Log<T> {
    fn default() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

impl<T: Copy> Log<T> {
    pub fn latest(&self) -> Option<&T> {
        self.map.last_key_value().map(|(_, v)| v)
    }

    pub fn latest_dt(&self) -> Option<DateTime<Utc>> {
        self.map.last_key_value().map(|(dt, _)| *dt)
    }

    pub fn for_dt(&self, dt: DateTime<Utc>) -> Option<&T> {
        self.map.range(..=dt).next_back().map(|(_, v)| v)
    }

    pub fn insert(&mut self, dt: DateTime<Utc>, value: T) {
        self.map.insert(dt, value);
    }
}
