use std::{
    collections::{HashSet, VecDeque},
    fmt,
};

use gtk::pango;

#[derive(Debug, PartialEq, Eq)]
pub enum SearchQuery {
    IdenValue(String, String),
    Standalone(String),
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchQuery::IdenValue(iden, value) => {
                if value.contains(char::is_whitespace) {
                    write!(f, "{}:\"{}\"", iden, value)
                } else {
                    write!(f, "{}:{}", iden, value)
                }
            }
            SearchQuery::Standalone(standalone) => write!(f, "{}", standalone),
        }
    }
}

impl SearchQuery {
    fn from_raw(iden: Option<&str>, value: &str) -> Self {
        let value = if is_value_in_quotes(value) {
            &value[1..value.len() - 1]
        } else {
            value
        };
        if let Some(iden) = iden {
            SearchQuery::IdenValue(iden.to_string(), value.to_string())
        } else {
            SearchQuery::Standalone(value.to_string())
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
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
        let mut queries = VecDeque::new();

        parse_raw(text, |_, iden, value| {
            queries.push_back(SearchQuery::from_raw(iden, value));
        });

        Self(queries)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the last query that matches any of the given values.
    pub fn find_last_match(&self, iden: &str, values: &[&str]) -> Option<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(values.iter().all(|v| !v.contains(is_quote)));

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

    pub fn remove_all_standalones(&mut self) {
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
    pub fn replace_all_or_insert(&mut self, iden: &str, old_values: &[&str], new_value: &str) {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(old_values.iter().all(|v| !v.contains(is_quote)));
        debug_assert!(!new_value.contains(is_quote));

        let mut is_inserted = false;
        self.0.retain_mut(|query| match query {
            SearchQuery::IdenValue(i, v) if i == iden && v == new_value => {
                let retain = !is_inserted;

                if !is_inserted {
                    is_inserted = true;
                }

                retain
            }
            SearchQuery::IdenValue(i, v) if i == iden && old_values.contains(&v.as_str()) => {
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

    /// Inserts or replaces all queries with the given `iden` values with the `new_value`,
    /// and deduplicate those queries, leaving only the first occurrence.
    ///
    /// If there are iden with already the `new_value`, it will remain and the other succeeding
    /// queries with `new_value` and `old_value` will be removed.
    pub fn replace_all_iden_or_insert(&mut self, iden: &str, new_value: &str) {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(!new_value.contains(is_quote));

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
        debug_assert!(!value.contains(is_quote));

        self.0.retain(|query| {
            if let SearchQuery::IdenValue(i, v) = query {
                i != iden || v != value
            } else {
                true
            }
        });
    }
}

pub fn attr_list_for(text: &str) -> pango::AttrList {
    let attrs = pango::AttrList::new();

    parse_raw(text, |index, iden, value| {
        if let Some(iden) = iden {
            let start_index = index as u32;
            let end_index = (index + iden.len() + 1 + value.len()) as u32;

            let is_value_in_quotes = is_value_in_quotes(value);
            let value_start_index = if is_value_in_quotes {
                (index + iden.len() + 2) as u32
            } else {
                (index + iden.len() + 1) as u32
            };
            let value_end_index = if is_value_in_quotes {
                end_index - 1
            } else {
                end_index
            };

            let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
            attr.set_start_index(start_index);
            attr.set_end_index((index + iden.len()) as u32);
            attrs.insert(attr);

            let mut attr = pango::AttrInt::new_weight(pango::Weight::Bold);
            attr.set_start_index(value_start_index);
            attr.set_end_index(value_end_index);
            attrs.insert(attr);

            let mut attr = pango::AttrInt::new_foreground_alpha((0.40 * u16::MAX as f32) as u16);
            attr.set_start_index(start_index);
            attr.set_end_index(end_index);
            attrs.insert(attr);
        }
    });

    attrs
}

fn is_value_in_quotes(value: &str) -> bool {
    value.starts_with('"') && value.ends_with('"') && value.len() > 1
}

fn is_quote(c: char) -> bool {
    c == '"'
}

fn parse_raw(text: &str, mut cb: impl FnMut(usize, Option<&str>, &str)) {
    let mut start_index = 0;
    let mut end_index = 0;
    let mut in_quotes = false;
    let mut iden: Option<(usize, &str)> = None;

    for (i, c) in text.char_indices() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                if !in_quotes {
                    if start_index < end_index {
                        if let Some((iden_start_index, iden)) = iden.take() {
                            cb(iden_start_index, Some(iden), &text[start_index..end_index]);
                        } else {
                            cb(start_index, None, &text[start_index..end_index]);
                        }
                    }
                    start_index = i + 1;
                    end_index = start_index;
                }
            }
            ':' if !in_quotes => {
                if start_index < end_index {
                    iden = Some((start_index, &text[start_index..end_index]));
                } else {
                    iden = Some((start_index, ""));
                }
                start_index = i + 1;
                end_index = start_index;
            }
            ' ' if !in_quotes => {
                if start_index < end_index {
                    if let Some((iden_start_index, iden)) = iden.take() {
                        cb(iden_start_index, Some(iden), &text[start_index..end_index]);
                    } else {
                        cb(start_index, None, &text[start_index..end_index]);
                    }
                } else if let Some((iden_start_index, iden)) = iden.take() {
                    cb(iden_start_index, Some(iden), "");
                }
                start_index = i + 1;
                end_index = start_index;
            }
            _ => {
                if start_index == end_index {
                    start_index = i;
                }
                end_index = i + c.len_utf8();
            }
        }
    }

    if start_index < end_index {
        if let Some((iden_start_index, iden)) = iden.take() {
            cb(iden_start_index, Some(iden), &text[start_index..end_index]);
        } else {
            cb(start_index, None, &text[start_index..end_index]);
        }
    } else if let Some((iden_start_index, iden)) = iden.take() {
        cb(iden_start_index, Some(iden), "");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<(usize, SearchQuery)> {
        let mut queries = Vec::new();

        parse_raw(text, |index, iden, value| {
            queries.push((index, SearchQuery::from_raw(iden, value)));
        });

        queries
    }

    #[test]
    fn display() {
        let queries = SearchQueries(VecDeque::from_iter([
            SearchQuery::IdenValue("iden1".to_string(), "value1".to_string()),
            SearchQuery::Standalone("standalone1".to_string()),
            SearchQuery::IdenValue("iden2".to_string(), " value2  ".to_string()),
        ]));

        assert_eq!(
            queries.to_string(),
            "iden1:value1 standalone1 iden2:\" value2  \" "
        );
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse(""), vec![]);
        assert_eq!(parse(" "), vec![]);
        assert_eq!(parse("\""), vec![]);
        assert_eq!(parse("\"\""), vec![]);
    }

    #[test]
    fn parse_simple() {
        assert_eq!(
            parse("standalone1"),
            vec![(0, SearchQuery::Standalone("standalone1".to_string()))]
        );

        assert_eq!(
            parse("iden1:value1"),
            vec![(
                0,
                SearchQuery::IdenValue("iden1".to_string(), "value1".to_string())
            )]
        );
    }

    #[test]
    fn parse_complex() {
        assert_eq!(
            parse("iden1:value1 iden2:\"value2\" iden3:\" value3\" iden4:\"value4 \" iden5:\" value5 \""),
           vec![
                (0, SearchQuery::IdenValue("iden1".to_string(), "value1".to_string())),
                (13, SearchQuery::IdenValue("iden2".to_string(), "value2".to_string())),
                (28, SearchQuery::IdenValue("iden3".to_string(), " value3".to_string())),
                (44, SearchQuery::IdenValue("iden4".to_string(), "value4 ".to_string())),
                (60, SearchQuery::IdenValue("iden5".to_string(), " value5 ".to_string()))
            ]
        );

        assert_eq!(
            parse(
                "standalone1   iden1:value1 iden2:\"  value 2   \"   standalone2  standalone3 iden3:\"value 3\"  standalone4"
            ),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (14, SearchQuery::IdenValue("iden1".to_string(), "value1".to_string())),
                (27, SearchQuery::IdenValue("iden2".to_string(), "  value 2   ".to_string())),
                (50, SearchQuery::Standalone("standalone2".to_string())),
                (63, SearchQuery::Standalone("standalone3".to_string())),
                (75, SearchQuery::IdenValue("iden3".to_string(), "value 3".to_string())),
                (92, SearchQuery::Standalone("standalone4".to_string()))
            ]
        );
    }

    #[test]
    fn parse_with_quotes() {
        assert_eq!(
            parse("standalone1 \"\" standalone2"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (15, SearchQuery::Standalone("standalone2".to_string()))
            ]
        );
        assert_eq!(
            parse("\"standalone1  \" standalone2"),
            vec![
                (1, SearchQuery::Standalone("standalone1  ".to_string())), // FIXMEThis should be 0
                (16, SearchQuery::Standalone("standalone2".to_string())),
            ]
        );
        assert_eq!(
            parse("standalone1 \"  standalone2 \"\"standalone3 "),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (13, SearchQuery::Standalone("  standalone2 ".to_string())), // FIXMEThis should be 12
                (29, SearchQuery::Standalone("standalone3 ".to_string())) // FIXMEThis should be 28
            ]
        );
        assert_eq!(
            parse("\"  standalone1 \" standalone1 "),
            vec![
                (1, SearchQuery::Standalone("  standalone1 ".to_string())), // FIXMEThis should be 0
                (17, SearchQuery::Standalone("standalone1".to_string()))
            ]
        );
        assert_eq!(
            parse("standalone1 \"iden1:value1"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (13, SearchQuery::Standalone("iden1:value1".to_string()))
            ]
        );
    }

    #[test]
    fn parse_empty_iden_or_value() {
        assert_eq!(
            parse("standalone1 : standalone2"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (12, SearchQuery::IdenValue("".to_string(), "".to_string())),
                (14, SearchQuery::Standalone("standalone2".to_string()))
            ]
        );
        assert_eq!(
            parse("standalone1 iden1: standalone2"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (
                    12,
                    SearchQuery::IdenValue("iden1".to_string(), "".to_string())
                ),
                (19, SearchQuery::Standalone("standalone2".to_string()))
            ]
        );
        assert_eq!(
            parse("standalone1 :value1 standalone2"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (
                    12,
                    SearchQuery::IdenValue("".to_string(), "value1".to_string())
                ),
                (20, SearchQuery::Standalone("standalone2".to_string()))
            ]
        );

        assert_eq!(
            parse("\"iden1\":value1"),
            vec![
                (1, SearchQuery::Standalone("iden1".to_string())), // FIXMEThis should be 0
                (
                    7,
                    SearchQuery::IdenValue("".to_string(), "value1".to_string())
                )
            ]
        );
    }
}
