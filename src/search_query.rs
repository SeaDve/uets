use std::{
    collections::{HashSet, VecDeque},
    fmt,
};

#[derive(Debug, PartialEq, Eq)]
pub enum SearchQuery {
    IdenValue(String, String),
    Standalone(String),
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchQuery::IdenValue(iden, value) => write!(f, "{}:{}", iden, value),
            SearchQuery::Standalone(standalone) => write!(f, "{}", standalone),
        }
    }
}

impl SearchQuery {
    pub fn parse(query: &str) -> Self {
        debug_assert!(!query.contains(char::is_whitespace));

        if let Some((iden, value)) = query.split_once(':') {
            SearchQuery::IdenValue(iden.to_string(), value.to_string())
        } else {
            SearchQuery::Standalone(query.to_string())
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

        if let Some(SearchQuery::IdenValue(_, _)) = self.0.back() {
            write!(f, " ")?;
        }

        Ok(())
    }
}

impl SearchQueries {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    pub fn parse(text: &str) -> Self {
        Self(text.split_whitespace().map(SearchQuery::parse).collect())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the last query that matches any of the given needles.
    pub fn find_last_match(&self, iden: &str, values: &[&str]) -> Option<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(values
            .iter()
            .all(|needle| !needle.contains(char::is_whitespace)));

        self.0.iter().rev().find_map(|query| match query {
            SearchQuery::IdenValue(i, v) if i == iden && values.contains(&v.as_str()) => {
                Some(v.as_str())
            }
            _ => None,
        })
    }

    /// Returns all unique standalone queries.
    pub fn all_standalones(&self) -> HashSet<&str> {
        self.0
            .iter()
            .filter_map(|query| match query {
                SearchQuery::Standalone(s) => Some(s.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn remove_all_standlones(&mut self) {
        self.0
            .retain(|query| !matches!(query, SearchQuery::Standalone(_)));
    }

    /// Returns all unique values without for the given `iden`.
    pub fn all_values(&self, iden: &str) -> HashSet<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));

        self.0
            .iter()
            .filter_map(|query| match query {
                SearchQuery::IdenValue(i, v) if i == iden => Some(v.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Inserts or replaces all queries with the given `iden` and `old_value` with the `new_value`,
    /// and deduplicate those queries, leaving only the first occurrence.
    ///
    /// If there are iden with already the `new_value`, it will remain and the other succeeding
    /// queries with `new_value` and `old_value` will be removed.
    pub fn replace_all_or_insert(&mut self, iden: &str, old_value: &str, new_value: &str) {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(!old_value.contains(char::is_whitespace));
        debug_assert!(!new_value.contains(char::is_whitespace));
        debug_assert_ne!(old_value, new_value);

        let mut is_inserted = false;
        self.0.retain_mut(|query| match query {
            SearchQuery::IdenValue(i, v) if i == iden && v == old_value => {
                let retain = !is_inserted;

                if !is_inserted {
                    *v = new_value.to_string();
                    is_inserted = true;
                }

                retain
            }
            SearchQuery::IdenValue(i, v) if i == iden && v == new_value => {
                let retain = !is_inserted;

                if !is_inserted {
                    is_inserted = true;
                }

                retain
            }
            _ => true,
        });

        if !is_inserted {
            self.0.push_front(SearchQuery::IdenValue(
                iden.to_string(),
                new_value.to_string(),
            ));
        }
    }

    /// Inserts or replaces all queries with the given `iden` values with the `new_value`,
    /// and deduplicate those queries, leaving only the first occurrence.
    ///
    /// If there are iden with already the `new_value`, it will remain and the other succeeding
    /// queries with `new_value` and `old_value` will be removed.
    pub fn replace_all_iden_or_insert(&mut self, iden: &str, new_value: &str) {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(!new_value.contains(char::is_whitespace));

        let mut is_inserted = false;
        self.0.retain_mut(|query| match query {
            SearchQuery::IdenValue(i, v) if i == iden => {
                let retain = !is_inserted;

                if !is_inserted {
                    *v = new_value.to_string();
                    is_inserted = true;
                }

                retain
            }
            _ => true,
        });

        if !is_inserted {
            self.0.push_front(SearchQuery::IdenValue(
                iden.to_string(),
                new_value.to_string(),
            ));
        }
    }

    /// Removes all queries with the given `iden` and `value`.
    pub fn remove_all(&mut self, iden: &str, value: &str) {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(!value.contains(char::is_whitespace));

        self.0.retain(|query| {
            if let SearchQuery::IdenValue(i, v) = query {
                i != iden || v != value
            } else {
                true
            }
        });
    }
}
