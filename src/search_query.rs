use std::{
    collections::{HashSet, VecDeque},
    fmt,
};

use gtk::pango;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchQuery {
    index: usize,
    iden: Option<String>,
    value: String,
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(iden) = &self.iden {
            if iden.contains(char::is_whitespace) {
                write!(f, "{}:\"{}\"", iden, self.value)
            } else {
                write!(f, "{}:{}", iden, self.value)
            }
        } else {
            write!(f, "{}", self.value)
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SearchQueries(VecDeque<SearchQuery>);

impl fmt::Display for SearchQueries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();

        if let Some(first_query) = iter.next() {
            write!(f, "{}", first_query)?;

            for query in iter {
                write!(f, " {}", query)?;
            }
        }

        if let Some(SearchQuery { iden: Some(_), .. }) = self.0.back() {
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

        parse_raw(text, |index, iden, value| {
            queries.push_back(SearchQuery {
                index,
                iden: iden.map(|i| i.to_string()),
                value: value.to_string(),
            });
        });

        Self(queries)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn attr_list(&self) -> pango::AttrList {
        let attrs = pango::AttrList::new();

        for query in &self.0 {
            if let Some(iden) = &query.iden {
                let start_index = query.index as u32;
                let end_index = (query.index + iden.len() + 1 + query.value.len()) as u32;

                let is_value_in_quotes = is_value_in_quotes(&query.value);
                let value_start_index = if is_value_in_quotes {
                    (query.index + iden.len() + 2) as u32
                } else {
                    (query.index + iden.len() + 1) as u32
                };
                let value_end_index = if is_value_in_quotes {
                    end_index - 1
                } else {
                    end_index
                };

                let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
                attr.set_start_index(start_index);
                attr.set_end_index((query.index + iden.len()) as u32);
                attrs.insert(attr);

                let mut attr = pango::AttrInt::new_weight(pango::Weight::Bold);
                attr.set_start_index(value_start_index);
                attr.set_end_index(value_end_index);
                attrs.insert(attr);

                let mut attr =
                    pango::AttrInt::new_foreground_alpha((0.40 * u16::MAX as f32) as u16);
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attrs.insert(attr);
            }
        }

        attrs
    }

    /// Returns the last query that matches any of the given values.
    pub fn find_last_match(&self, iden: &str, values: &[&str]) -> Option<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(values.iter().all(|v| !v.contains(is_quote)));

        self.0.iter().rev().find_map(|query| match query {
            SearchQuery {
                iden: Some(i),
                value: v,
                ..
            } if i == iden && values.contains(&v.as_str()) => Some(v.as_str()),
            _ => None,
        })
    }

    /// Returns all unique standalone queries.
    pub fn all_standalones(&self) -> HashSet<&str> {
        self.0
            .iter()
            .filter_map(|query| match query {
                SearchQuery {
                    iden: None,
                    value: v,
                    ..
                } => Some(v.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn remove_all_standalones(&mut self) {
        self.0
            .retain(|query| matches!(query, SearchQuery { iden: Some(_), .. }));
    }

    /// Returns all unique values without for the given `iden`.
    pub fn all_values(&self, iden: &str) -> HashSet<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));

        self.0
            .iter()
            .filter_map(|query| match query {
                SearchQuery {
                    iden: Some(i),
                    value: v,
                    ..
                } if i == iden => Some(v.as_str()),
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
            SearchQuery {
                iden: Some(i),
                value: v,
                ..
            } if i == iden && v == new_value => {
                let retain = !is_inserted;

                if !is_inserted {
                    is_inserted = true;
                }

                retain
            }
            SearchQuery {
                iden: Some(i),
                value: v,
                ..
            } if i == iden && old_values.contains(&v.as_str()) => {
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
            self.0.push_front(SearchQuery {
                index: 0,
                iden: Some(iden.to_string()),
                value: new_value.to_string(),
            });
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
            SearchQuery {
                iden: Some(i),
                value: v,
                ..
            } if i == iden => {
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
            self.0.push_front(SearchQuery {
                index: 0,
                iden: Some(iden.to_string()),
                value: new_value.to_string(),
            });
        }
    }

    /// Removes all queries with the given `iden` and `value`.
    pub fn remove_all(&mut self, iden: &str, value: &str) {
        debug_assert!(!iden.contains(char::is_whitespace));
        debug_assert!(!value.contains(is_quote));

        self.0.retain(|query| {
            if let SearchQuery {
                iden: Some(i),
                value: v,
                ..
            } = query
            {
                i != iden || v != value
            } else {
                true
            }
        });
    }
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

    fn parse(text: &str) -> Vec<SearchQuery> {
        SearchQueries::parse(text).0.into_iter().collect()
    }

    fn standalone(index: usize, value: &str) -> SearchQuery {
        SearchQuery {
            index,
            iden: None,
            value: value.to_string(),
        }
    }

    fn iden_value(index: usize, iden: &str, value: &str) -> SearchQuery {
        SearchQuery {
            index,
            iden: Some(iden.to_string()),
            value: value.to_string(),
        }
    }

    #[test]
    fn display() {
        let queries = SearchQueries(VecDeque::from_iter([
            iden_value(0, "iden1", "value1"),
            standalone(0, "standalone1"),
            iden_value(0, "iden2", " value2  "),
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
        assert_eq!(parse("standalone1"), vec![(standalone(0, "standalone1"))]);

        assert_eq!(
            parse("iden1:value1"),
            vec![iden_value(0, "iden1", "value1")]
        );
    }

    #[test]
    fn parse_complex() {
        assert_eq!(
            parse("iden1:value1 iden2:\"value2\" iden3:\" value3\" iden4:\"value4 \" iden5:\" value5 \""),
           vec![
                iden_value (0, "iden1", "value1"),
                iden_value(13, "iden2", "value2"),
                iden_value(28, "iden3", " value3"),
                iden_value(44, "iden4", "value4 "),
                iden_value(60, "iden5", " value5 ")
            ]
        );

        assert_eq!(
            parse(
                "standalone1   iden1:value1 iden2:\"  value 2   \"   standalone2  standalone3 iden3:\"value 3\"  standalone4"
            ),
            vec![
                standalone(0, "standalone1"),
                iden_value(14, "iden1", "value1"),
                iden_value(27, "iden2", "  value 2   "),
                standalone(50, "standalone2"),
                standalone(63, "standalone3"),
                iden_value(75, "iden3", "value 3"),
                standalone(92, "standalone4")
            ]
        );
    }

    #[test]
    fn parse_with_quotes() {
        assert_eq!(
            parse("standalone1 \"\" standalone2"),
            vec![standalone(0, "standalone1"), standalone(15, "standalone2")]
        );
        assert_eq!(
            parse("\"standalone1  \" standalone2"),
            vec![
                standalone(1, "standalone1  "), // FIXMEThis should be 0
                standalone(16, "standalone2"),
            ]
        );
        assert_eq!(
            parse("standalone1 \"  standalone2 \"\"standalone3 "),
            vec![
                standalone(0, "standalone1"),
                standalone(13, "  standalone2 "), // FIXMEThis should be 12
                standalone(29, "standalone3 ")    // FIXMEThis should be 28
            ]
        );
        assert_eq!(
            parse("\"  standalone1 \" standalone1 "),
            vec![
                standalone(1, "  standalone1 "), // FIXMEThis should be 0
                standalone(17, "standalone1")
            ]
        );
        assert_eq!(
            parse("standalone1 \"iden1:value1"),
            vec![standalone(0, "standalone1"), standalone(13, "iden1:value1")]
        );
        assert_eq!(
            parse("iden1:value1\" standalone1"),
            vec![
                iden_value(0, "iden1", "value1"),
                standalone(14, "standalone1")
            ]
        );
    }

    #[test]
    fn parse_empty_iden_or_value() {
        assert_eq!(
            parse("standalone1 : standalone2"),
            vec![
                standalone(0, "standalone1"),
                iden_value(12, "", ""),
                standalone(14, "standalone2")
            ]
        );
        assert_eq!(
            parse("standalone1 iden1: standalone2"),
            vec![
                standalone(0, "standalone1"),
                iden_value(12, "iden1", ""),
                standalone(19, "standalone2")
            ]
        );
        assert_eq!(
            parse("standalone1 :value1 standalone2"),
            vec![
                standalone(0, "standalone1"),
                iden_value(12, "", "value1"),
                standalone(20, "standalone2")
            ]
        );

        assert_eq!(
            parse("\"iden1\":value1"),
            vec![
                standalone(1, "iden1"), // FIXMEThis should be 0
                iden_value(7, "", "value1")
            ]
        );
    }
}
