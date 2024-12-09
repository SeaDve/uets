use regex::Regex;
use std::{
    collections::{HashSet, VecDeque},
    fmt::{self, Write},
    sync::LazyLock,
};

use gtk::pango;

struct EnquotedIfNeeded<'a>(&'a str);

impl fmt::Display for EnquotedIfNeeded<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Enquote if the value has whitespace or it looks like it is in quotes.
        if self.0.contains(char::is_whitespace) || is_in_quotes(self.0) {
            f.write_char('"')?;

            // Escape all quotes and backslashes.
            for s in self.0.chars() {
                match s {
                    '"' => f.write_str(r#"\""#)?,
                    '\\' => f.write_str(r#"\\"#)?,
                    c => f.write_char(c)?,
                }
            }

            f.write_char('"')
        } else {
            fmt::Display::fmt(self.0, f)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SQ {
    IdenValue {
        iden: String,
        value: String,
        start_index: usize,
        end_index: usize,
        value_start_index: usize,
        value_end_index: usize,
    },
    Value {
        value: String,
    },
}

impl fmt::Display for SQ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SQ::IdenValue { iden, value, .. } => {
                write!(f, "{}:{}", iden, EnquotedIfNeeded(value))
            }
            SQ::Value { value } => {
                write!(f, "{}", value)
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SearchQueries(VecDeque<SQ>);

impl fmt::Display for SearchQueries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();

        if let Some(first_query) = iter.next() {
            write!(f, "{}", first_query)?;

            for query in iter {
                write!(f, " {}", query)?;
            }
        }

        if let Some(SQ::IdenValue { .. }) = self.0.back() {
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
        static REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"(\S+:"(?:[^"\\]|\\.)*")|\S+|\S+:\S+"#).unwrap());

        let mut queries = VecDeque::new();

        for m in REGEX.find_iter(text) {
            if let Some((iden, raw_value)) = m.as_str().split_once(':') {
                let is_value_in_quotes = is_in_quotes(raw_value);
                let value = if is_value_in_quotes {
                    unquote(raw_value)
                } else {
                    raw_value.to_string()
                };
                queries.push_back(SQ::IdenValue {
                    iden: iden.to_string(),
                    value,
                    start_index: m.start(),
                    end_index: m.end(),
                    value_start_index: if is_value_in_quotes {
                        m.start() + iden.len() + 2
                    } else {
                        m.start() + iden.len() + 1
                    },
                    value_end_index: if is_value_in_quotes {
                        m.end() - 1
                    } else {
                        m.end()
                    },
                });
            } else {
                let value = m.as_str();
                queries.push_back(SQ::Value {
                    value: value.to_string(),
                });
            }
        }

        Self(queries)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the text attribute list for the queries.
    ///
    /// This must only be called right after parsing the queries. Modifying the queries will
    /// invalidate the attribute list.
    pub fn attr_list(&self) -> pango::AttrList {
        let attrs = pango::AttrList::new();

        for query in &self.0 {
            if let SQ::IdenValue {
                iden,
                start_index,
                end_index,
                value_start_index,
                value_end_index,
                ..
            } = query
            {
                // Entire query attr
                let mut attr =
                    pango::AttrInt::new_foreground_alpha((0.40 * u16::MAX as f32) as u16);
                attr.set_start_index(*start_index as u32);
                attr.set_end_index(*end_index as u32);
                attrs.insert(attr);

                // Iden attr
                let mut attr = pango::AttrInt::new_style(pango::Style::Italic);
                attr.set_start_index(*start_index as u32);
                attr.set_end_index((*start_index + iden.len()) as u32);
                attrs.insert(attr);

                // Val attr
                let mut attr = pango::AttrInt::new_weight(pango::Weight::Bold);
                attr.set_start_index(*value_start_index as u32);
                attr.set_end_index(*value_end_index as u32);
                attrs.insert(attr);
            }
        }

        attrs
    }

    /// Returns the last query that matches any of the given values.
    pub fn find_last_with_values(&self, iden: &str, values: &[&str]) -> Option<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));

        self.0.iter().rev().find_map(|query| match query {
            SQ::IdenValue {
                iden: i, value: v, ..
            } if i == iden && values.contains(&v.as_str()) => Some(v.as_str()),
            _ => None,
        })
    }

    /// Returns the last query that has the given iden.
    pub fn find_last(&self, iden: &str) -> Option<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));

        self.0.iter().rev().find_map(|query| match query {
            SQ::IdenValue {
                iden: i, value: v, ..
            } if i == iden => Some(v.as_str()),
            _ => None,
        })
    }

    /// Returns all unique standalone queries.
    pub fn all_standalones(&self) -> HashSet<&str> {
        self.0
            .iter()
            .filter_map(|query| match query {
                SQ::Value { value: v } => Some(v.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn remove_all_standalones(&mut self) {
        self.0.retain(|query| matches!(query, SQ::IdenValue { .. }));
    }

    /// Returns all unique values without for the given `iden`.
    pub fn all_values(&self, iden: &str) -> HashSet<&str> {
        debug_assert!(!iden.contains(char::is_whitespace));

        self.0
            .iter()
            .filter_map(|query| match query {
                SQ::IdenValue {
                    iden: i, value: v, ..
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

        let mut is_inserted = false;
        self.0.retain_mut(|query| match query {
            SQ::IdenValue {
                iden: i, value: v, ..
            } if i == iden && v == new_value => {
                let retain = !is_inserted;

                if !is_inserted {
                    is_inserted = true;
                }

                retain
            }
            SQ::IdenValue {
                iden: i, value: v, ..
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
            self.0.push_front(SQ::IdenValue {
                iden: iden.to_string(),
                value: new_value.to_string(),
                start_index: 0,
                end_index: 0,
                value_start_index: 0,
                value_end_index: 0,
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

        let mut is_inserted = false;
        self.0.retain_mut(|query| match query {
            SQ::IdenValue {
                iden: i, value: v, ..
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
            self.0.push_front(SQ::IdenValue {
                iden: iden.to_string(),
                value: new_value.to_string(),
                start_index: 0,
                end_index: 0,
                value_start_index: 0,
                value_end_index: 0,
            });
        }
    }

    /// Removes all queries with the given `iden` and `values`.
    pub fn remove_all(&mut self, iden: &str, values: &[&str]) {
        debug_assert!(!iden.contains(char::is_whitespace));

        self.0.retain(|query| {
            if let SQ::IdenValue {
                iden: i, value: v, ..
            } = query
            {
                i != iden || !values.contains(&v.as_str())
            } else {
                true
            }
        });
    }

    /// Removes all queries with the given `iden`.
    pub fn remove_all_iden(&mut self, iden: &str) {
        debug_assert!(!iden.contains(char::is_whitespace));

        self.0.retain(|query| {
            if let SQ::IdenValue { iden: i, .. } = query {
                i != iden
            } else {
                true
            }
        });
    }
}

fn is_quote(c: char) -> bool {
    c == '"'
}

fn is_in_quotes(value: &str) -> bool {
    value.chars().next().is_some_and(is_quote) && {
        let mut chars = value.chars().rev();
        let last = chars.next();
        let second_last = chars.next();
        last.is_some_and(is_quote)
            && (second_last.is_some_and(|c| c != '\\')
                || chars.take_while(|c| *c == '\\').count() % 2 != 0)
    }
}

fn unquote(s: &str) -> String {
    debug_assert!(is_in_quotes(s));

    let mut ret = String::new();

    // Skip the first and last quote, then unescape all quotes and backslashes.
    let mut chars = s.chars().skip(1).take(s.len() - 2);
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => ret.push('"'),
                Some('\\') => ret.push('\\'),
                Some(c) => {
                    ret.push('\\');
                    ret.push(c);
                }
                None => ret.push('\\'),
            }
        } else {
            ret.push(c);
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<SQ> {
        SearchQueries::parse(text).0.into_iter().collect()
    }

    fn value(value: &str) -> SQ {
        SQ::Value {
            value: value.to_string(),
        }
    }

    fn iden_value(
        iden: &str,
        value: &str,
        start_index: usize,
        end_index: usize,
        value_start_index: usize,
        value_end_index: usize,
    ) -> SQ {
        SQ::IdenValue {
            iden: iden.to_string(),
            value: value.to_string(),
            start_index,
            end_index,
            value_start_index,
            value_end_index,
        }
    }

    #[test]
    fn is_in_quotes_simple() {
        assert!(is_in_quotes("\"\""));
        assert!(is_in_quotes("\"a\""));

        assert!(!is_in_quotes("a"));
        assert!(!is_in_quotes("\""));
        assert!(!is_in_quotes("\"a"));
        assert!(!is_in_quotes("a\""));

        assert!(is_in_quotes(r#""a\\""#));
        assert!(is_in_quotes(r#""a\\\\""#));

        assert!(!is_in_quotes(r#""a\\\""#));
        assert!(!is_in_quotes(r#""a\""#));
    }

    #[test]
    fn display() {
        let queries = SearchQueries(VecDeque::from_iter([
            iden_value("iden1", "value1", 0, 12, 6, 12),
            value("standalone1"),
            iden_value("iden2", " value2  ", 25, 42, 32, 41),
        ]));
        let str = "iden1:value1 standalone1 iden2:\" value2  \" ";
        assert_eq!(queries.to_string(), str);
        assert_eq!(SearchQueries::parse(str), queries);

        let queries = SearchQueries(VecDeque::from_iter([
            value("s"),
            iden_value("iden1", r#"  va"lue1    "#, 2, 24, 9, 23),
            value("e"),
        ]));
        let str = r#"s iden1:"  va\"lue1    " e"#;
        assert_eq!(queries.to_string(), str);
        assert_eq!(SearchQueries::parse(str), queries);

        let queries = SearchQueries(VecDeque::from_iter([
            value("s"),
            iden_value("iden1", r#""va\"lue1""#, 2, 24, 9, 23),
            value("e"),
        ]));
        let str = r#"s iden1:"\"va\\\"lue1\"" e"#;
        assert_eq!(queries.to_string(), str);
        assert_eq!(SearchQueries::parse(str), queries);

        let queries = SearchQueries(VecDeque::from_iter([
            value("s"),
            iden_value("iden1", r#""value1"#, 2, 15, 8, 15),
            value("e"),
        ]));
        let str = r#"s iden1:"value1 e"#;
        assert_eq!(queries.to_string(), str);
        assert_eq!(SearchQueries::parse(str), queries);

        let queries = SearchQueries(VecDeque::from_iter([
            value("s"),
            iden_value("iden1", r#"value1""#, 2, 15, 8, 15),
            value("e"),
        ]));
        let str = r#"s iden1:value1" e"#;
        assert_eq!(queries.to_string(), str);
        assert_eq!(SearchQueries::parse(str), queries);

        let queries = SearchQueries(VecDeque::from_iter([
            value("s"),
            iden_value("iden1", r#"va\"lue1""#, 2, 17, 8, 17),
            value("e"),
        ]));
        let str = r#"s iden1:va\"lue1" e"#;
        assert_eq!(queries.to_string(), str);
        assert_eq!(SearchQueries::parse(str), queries);
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse(""), vec![]);
        assert_eq!(parse(" "), vec![]);
    }

    #[test]
    fn parse_simple() {
        assert_eq!(parse("standalone1"), vec![(value("standalone1"))]);

        assert_eq!(
            parse("iden1:value1"),
            vec![iden_value("iden1", "value1", 0, 12, 6, 12)]
        );
    }

    #[test]
    fn parse_complex() {
        assert_eq!(
            parse("iden1:value1 iden2:\"value2\" iden3:\" value3\" iden4:\"value4 \" iden5:\" value5 \""),
           vec![
                iden_value("iden1", "value1", 0, 12, 6, 12),
                iden_value("iden2", "value2", 13, 27, 20, 26),
                iden_value("iden3", " value3", 28, 43, 35, 42),
                iden_value("iden4", "value4 ", 44, 59, 51, 58),
                iden_value("iden5", " value5 ", 60, 76, 67, 75),
            ]
        );

        assert_eq!(
            parse(
                "standalone1   iden1:value1 iden2:\"  value 2   \"   standalone2  standalone3 iden3:\"value 3\"  standalone4"
            ),
            vec![
                value("standalone1"),
                iden_value("iden1", "value1", 14, 26, 20, 26),
                iden_value("iden2", "  value 2   ", 27, 47, 34, 46),
                value("standalone2"),
                value("standalone3"),
                iden_value("iden3", "value 3", 75, 90, 82, 89),
                value("standalone4"),
            ]
        );
    }

    #[test]
    fn parse_empty_iden_or_value() {
        assert_eq!(
            parse("standalone1 : standalone2"),
            vec![
                value("standalone1"),
                iden_value("", "", 12, 13, 13, 13),
                value("standalone2")
            ]
        );
        assert_eq!(
            parse("standalone1 iden1: standalone2"),
            vec![
                value("standalone1"),
                iden_value("iden1", "", 12, 18, 18, 18),
                value("standalone2")
            ]
        );
        assert_eq!(
            parse("standalone1 :value1 standalone2"),
            vec![
                value("standalone1"),
                iden_value("", "value1", 12, 19, 13, 19),
                value("standalone2")
            ]
        );
    }

    #[test]
    fn parse_with_quotes() {
        assert_eq!(
            parse(r#"s iden1:"va\lue1" e"#),
            vec![
                value("s"),
                iden_value("iden1", r#"va\lue1"#, 2, 17, 9, 16),
                value("e")
            ]
        );

        assert_eq!(
            parse(r#"s iden1:"va\"lue1" e"#),
            vec![
                value("s"),
                iden_value("iden1", r#"va"lue1"#, 2, 18, 9, 17),
                value("e")
            ]
        );

        assert_eq!(
            parse(r#"s iden1:"va\"lue1\"" e"#),
            vec![
                value("s"),
                iden_value("iden1", r#"va"lue1""#, 2, 20, 9, 19),
                value("e")
            ]
        );

        assert_eq!(
            parse(r#"s iden1:\"va\"lue1" e"#),
            vec![
                value("s"),
                iden_value("iden1", r#"\"va\"lue1""#, 2, 19, 8, 19),
                value("e")
            ]
        );

        assert_eq!(
            parse(r#"s iden1:"va\"lue1\" e"#),
            vec![
                value("s"),
                iden_value("iden1", r#""va\"lue1\""#, 2, 19, 8, 19),
                value("e")
            ]
        );

        assert_eq!(
            parse(r#"s iden1:"va\"lue1\\" e"#),
            vec![
                value("s"),
                iden_value("iden1", r#"va"lue1\"#, 2, 20, 9, 19),
                value("e")
            ]
        );
    }

    #[test]
    fn parse_random_quotes() {
        assert_eq!(
            parse("standalone1 \" standalone2"),
            vec![value("standalone1"), value("\""), value("standalone2")]
        );

        assert_eq!(
            parse("standalone1 \"\" standalone2"),
            vec![value("standalone1"), value("\"\""), value("standalone2")]
        );

        assert_eq!(
            parse("standalone1 \" \" standalone2"),
            vec![
                value("standalone1"),
                value("\""),
                value("\""),
                value("standalone2")
            ]
        );

        assert_eq!(
            parse("\"standalone1  \" standalone2"),
            vec![value("\"standalone1"), value("\""), value("standalone2"),]
        );

        assert_eq!(
            parse("standalone1 \"  standalone2 \"\"standalone3 "),
            vec![
                value("standalone1"),
                value("\""),
                value("standalone2"),
                value("\"\"standalone3")
            ]
        );

        assert_eq!(
            parse("\"  standalone1 \" standalone1 "),
            vec![
                value("\""),
                value("standalone1"),
                value("\""),
                value("standalone1")
            ]
        );

        assert_eq!(
            parse("standalone1 \"iden1:value1"),
            vec![
                value("standalone1"),
                iden_value("\"iden1", "value1", 12, 25, 19, 25)
            ]
        );

        assert_eq!(
            parse("iden1:value1\" standalone1"),
            vec![
                iden_value("iden1", "value1\"", 0, 13, 6, 13),
                value("standalone1")
            ]
        );

        assert_eq!(
            parse("standalone1 iden1:\"value1\"standalone2"),
            vec![
                value("standalone1"),
                iden_value("iden1", "value1", 12, 26, 19, 25),
                value("standalone2")
            ]
        );
    }
}
