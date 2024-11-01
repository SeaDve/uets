use std::{
    collections::{HashSet, VecDeque},
    fmt,
};

#[derive(Debug, PartialEq, Eq)]
pub enum SearchQuery {
    IdenValue(String, String),
    Standalone(String),
}

impl SearchQuery {
    fn parse(part: &str) -> Self {
        debug_assert!(!part.contains(char::is_whitespace));

        if let Some((iden, value)) = part.split_once(':') {
            SearchQuery::IdenValue(iden.to_string(), value.to_string())
        } else {
            SearchQuery::Standalone(part.to_string())
        }
    }
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchQuery::IdenValue(iden, value) => write!(f, "{}:{}", iden, value),
            SearchQuery::Standalone(part) => write!(f, "{}", part),
        }
    }
}

#[derive(Debug)]
pub struct SearchQueries(VecDeque<SearchQuery>);

impl fmt::Display for SearchQueries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();

        if let Some(first) = iter.next() {
            write!(f, "{}", first)?;

            for query in iter {
                write!(f, " {}", query)?;
            }
        }

        Ok(())
    }
}

impl SearchQueries {
    pub fn parse(text: &str) -> Self {
        Self(text.split_whitespace().map(SearchQuery::parse).collect())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the last query that matches any of the given `parts`.
    pub fn find_last_match(&self, parts: &[&str]) -> Option<&SearchQuery> {
        self.0
            .iter()
            .rev()
            .find(|query| parts.iter().any(|part| SearchQuery::parse(part) == **query))
    }

    /// Returns all values for the given `iden`.
    pub fn all_values(&self, iden: &str) -> HashSet<&str> {
        self.0
            .iter()
            .filter_map(|query| match query {
                SearchQuery::IdenValue(i, v) if i == iden => Some(v.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Inserts a new query with the given `iden` and `value` if it doesn't already exist.
    pub fn insert(&mut self, iden: &str, value: &str) {
        for query in &mut self.0 {
            if let SearchQuery::IdenValue(i, v) = query {
                if i == iden && v == value {
                    return;
                }
            }
        }

        self.0
            .push_front(SearchQuery::IdenValue(iden.to_string(), value.to_string()));
    }

    /// Removes all queries with the given `iden` and `value`.
    pub fn remove(&mut self, iden: &str, value: &str) {
        self.0.retain(|query| {
            if let SearchQuery::IdenValue(i, v) = query {
                i != iden || v != value
            } else {
                true
            }
        });
    }

    /// Removes all queries with the given `iden`.
    pub fn remove_iden(&mut self, iden: &str) {
        self.0.retain(|query| {
            if let SearchQuery::IdenValue(i, _) = query {
                i != iden
            } else {
                true
            }
        });
    }
}
