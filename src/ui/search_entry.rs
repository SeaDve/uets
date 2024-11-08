use std::time::Duration;

use gtk::{
    glib::{self, clone, closure_local},
    pango,
    prelude::*,
    subclass::prelude::*,
};

use crate::search_query::{SearchQueries, SearchQuery};

const DEFAULT_SEARCH_DELAY_MS: u32 = 200;

const SPACING: i32 = 6;

mod imp {
    use std::{
        cell::{Cell, OnceCell, RefCell},
        marker::PhantomData,
        sync::OnceLock,
    };

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::SearchEntry)]
    pub struct SearchEntry {
        #[property(get = Self::placeholder_text, set = Self::set_placeholder_text, explicit_notify)]
        pub(super) placeholder_text: PhantomData<Option<String>>,
        #[property(get, set = Self::set_search_delay, explicit_notify)]
        pub(super) search_delay: Cell<u32>,

        pub(super) entry: gtk::Text,
        pub(super) search_icon: gtk::Image,
        pub(super) clear_icon: gtk::Image,

        pub(super) search_changed_timeout_id: RefCell<Option<glib::SourceId>>,
        pub(super) entry_changed_id: OnceCell<glib::SignalHandlerId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchEntry {
        const NAME: &'static str = "UetsSearchEntry";
        type Type = super::SearchEntry;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("entry");
        }
    }

    impl ObjectImpl for SearchEntry {
        fn constructed(&self) {
            self.parent_constructed();

            self.search_delay.set(DEFAULT_SEARCH_DELAY_MS);

            let obj = self.obj();

            self.search_icon
                .set_icon_name(Some("system-search-symbolic"));
            self.search_icon.set_parent(&*obj);

            self.entry.set_parent(&*obj);
            self.entry.set_hexpand(true);
            obj.init_delegate();

            let entry_changed_id = self.entry.connect_changed(clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();

                    if imp.entry.text().is_empty() {
                        imp.clear_icon.set_child_visible(false);
                        obj.queue_allocate();

                        if let Some(source_id) = imp.search_changed_timeout_id.take() {
                            source_id.remove();
                        }

                        obj.emit_by_name::<()>("search-changed", &[]);
                        obj.update_entry_attributes();
                    } else {
                        imp.clear_icon.set_child_visible(true);
                        obj.queue_allocate();

                        obj.restart_search_changed_timeout();
                    }
                },
            ));
            self.entry_changed_id.set(entry_changed_id).unwrap();

            self.clear_icon.set_icon_name(Some("edit-clear-symbolic"));
            self.clear_icon.set_parent(&*obj);
            self.clear_icon.set_child_visible(false);

            let press = gtk::GestureClick::new();
            press.connect_released(|gesture, _, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
            });
            press.connect_released(clone!(
                #[weak]
                obj,
                move |_, _, _, _| {
                    let imp = obj.imp();
                    imp.entry.set_text("");
                },
            ));
            self.clear_icon.add_controller(press);

            obj.add_css_class("search");
        }

        fn dispose(&self) {
            let obj = self.obj();

            obj.finish_delegate();

            self.search_icon.unparent();
            self.entry.unparent();
            self.clear_icon.unparent();

            if let Some(source_id) = self.search_changed_timeout_id.take() {
                source_id.remove();
            }
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            if !self.delegate_set_property(id, value, pspec) {
                self.derived_set_property(id, value, pspec)
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            if let Some(value) = self.delegate_get_property(id, pspec) {
                value
            } else {
                self.derived_property(id, pspec)
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| vec![Signal::builder("search-changed").build()])
        }
    }

    impl WidgetImpl for SearchEntry {
        fn grab_focus(&self) -> bool {
            self.entry.grab_focus_without_selecting()
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let (text_min, text_nat, min_baseline, nat_baseline) =
                self.entry.measure(orientation, for_size);

            let mut min = text_min;
            let mut nat = text_nat;
            let mut min_baseline = min_baseline;
            let mut nat_baseline = nat_baseline;

            let (search_icon_min, search_icon_nat, _, _) =
                self.search_icon.measure(gtk::Orientation::Horizontal, -1);

            if orientation == gtk::Orientation::Horizontal {
                min += search_icon_min + SPACING;
                nat += search_icon_nat + SPACING;
            } else {
                min = min.max(search_icon_min);
                nat = nat.max(search_icon_nat);
            }

            let (clear_icon_min, clear_icon_nat, _, _) =
                self.clear_icon.measure(gtk::Orientation::Horizontal, -1);

            if orientation == gtk::Orientation::Horizontal {
                min += clear_icon_min + SPACING;
                nat += clear_icon_nat + SPACING;
            } else {
                min = min.max(clear_icon_min);
                nat = nat.max(clear_icon_nat);

                if min_baseline >= 0 {
                    min_baseline += (min - text_min) / 2;
                }
                if nat_baseline >= 0 {
                    nat_baseline += (nat - text_nat) / 2;
                }
            }

            (min, nat, min_baseline, nat_baseline)
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let obj = self.obj();

            let is_rtl = obj.direction() == gtk::TextDirection::Rtl;

            let mut text_alloc = gtk::Allocation::new(0, 0, width, height);

            let baseline = if obj.valign() == gtk::Align::Baseline {
                baseline
            } else {
                -1
            };

            let (_, search_icon_width, _, _) =
                self.search_icon.measure(gtk::Orientation::Horizontal, -1);

            let search_icon_alloc = gtk::Allocation::new(
                if is_rtl { width - search_icon_width } else { 0 },
                0,
                search_icon_width,
                height,
            );
            self.search_icon.size_allocate(&search_icon_alloc, baseline);

            text_alloc.set_width(text_alloc.width() - (search_icon_width + SPACING));
            text_alloc.set_x(
                text_alloc.x()
                    + if is_rtl {
                        0
                    } else {
                        search_icon_width + SPACING
                    },
            );

            if self.clear_icon.is_child_visible() {
                let (_, clear_icon_width, _, _) =
                    self.clear_icon.measure(gtk::Orientation::Horizontal, -1);

                let clear_icon_alloc = gtk::Allocation::new(
                    if is_rtl { 0 } else { width - clear_icon_width },
                    0,
                    clear_icon_width,
                    height,
                );

                self.clear_icon.size_allocate(&clear_icon_alloc, baseline);

                text_alloc.set_width(text_alloc.width() - (clear_icon_width + SPACING));
                text_alloc.set_x(
                    text_alloc.x()
                        + if is_rtl {
                            clear_icon_width + SPACING
                        } else {
                            0
                        },
                );
            }

            self.entry.size_allocate(&text_alloc, baseline);
        }
    }

    impl EditableImpl for SearchEntry {
        fn delegate(&self) -> Option<gtk::Editable> {
            Some(self.entry.clone().upcast())
        }
    }

    impl SearchEntry {
        fn placeholder_text(&self) -> Option<String> {
            self.entry.placeholder_text().map(|t| t.into())
        }

        fn set_placeholder_text(&self, placeholder_text: Option<&str>) {
            let obj = self.obj();

            if placeholder_text == obj.placeholder_text().as_deref() {
                return;
            }

            self.entry.set_placeholder_text(placeholder_text);
            obj.notify_placeholder_text();
        }

        fn set_search_delay(&self, search_delay: u32) {
            let obj = self.obj();

            if search_delay == obj.search_delay() {
                return;
            }

            self.search_delay.set(search_delay);
            obj.restart_search_changed_timeout();
            obj.notify_search_delay();
        }
    }
}

glib::wrapper! {
    pub struct SearchEntry(ObjectSubclass<imp::SearchEntry>)
        @extends gtk::Widget,
        @implements gtk::Editable;
}

impl SearchEntry {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_search_changed<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure("search-changed", false, closure_local!(|obj: &Self| f(obj)))
    }

    pub fn set_queries(&self, queries: &SearchQueries) {
        self.set_text_instant(&queries.to_string());
    }

    pub fn queries(&self) -> SearchQueries {
        SearchQueries::from_raw(
            parse_queries(&self.imp().entry.text())
                .into_iter()
                .map(|(_, q)| q)
                .collect(),
        )
    }

    /// Sets the text without delaying the search-changed signal.
    fn set_text_instant(&self, text: &str) {
        let imp = self.imp();

        let entry_changed_id = imp.entry_changed_id.get().unwrap();
        imp.entry.block_signal(entry_changed_id);
        imp.entry.set_text(text);
        imp.entry.unblock_signal(entry_changed_id);

        imp.clear_icon
            .set_child_visible(!imp.entry.text().is_empty());
        self.queue_allocate();

        if let Some(source_id) = imp.search_changed_timeout_id.take() {
            source_id.remove();
        }

        self.emit_by_name::<()>("search-changed", &[]);
        self.update_entry_attributes();
    }

    fn restart_search_changed_timeout(&self) {
        let imp = self.imp();

        if let Some(source_id) = imp.search_changed_timeout_id.take() {
            source_id.remove();
        }

        let search_delay = self.search_delay();
        let source_id = glib::timeout_add_local_once(
            Duration::from_millis(search_delay as u64),
            clone!(
                #[weak(rename_to = obj)]
                self,
                move || {
                    let imp = obj.imp();
                    imp.search_changed_timeout_id.replace(None);

                    obj.emit_by_name::<()>("search-changed", &[]);
                    obj.update_entry_attributes();
                },
            ),
        );
        imp.search_changed_timeout_id.replace(Some(source_id));
    }

    fn update_entry_attributes(&self) {
        let imp = self.imp();

        let text = imp.entry.text();

        let attrs = pango::AttrList::new();

        parse_queries_inner(&text, |index, iden, value| {
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

                let mut attr =
                    pango::AttrInt::new_foreground_alpha((0.40 * u16::MAX as f32) as u16);
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attrs.insert(attr);
            }
        });

        imp.entry.set_attributes(Some(&attrs))
    }
}

fn parse_queries(text: &str) -> Vec<(usize, SearchQuery)> {
    let mut queries = Vec::new();

    parse_queries_inner(text, |start_index, iden, value| {
        let value = if is_value_in_quotes(value) {
            &value[1..value.len() - 1]
        } else {
            value
        };
        if let Some(iden) = iden {
            queries.push((
                start_index,
                SearchQuery::IdenValue(iden.to_string(), value.to_string()),
            ));
        } else {
            queries.push((start_index, SearchQuery::Standalone(value.to_string())));
        }
    });

    queries
}

fn parse_queries_inner(text: &str, mut cb: impl FnMut(usize, Option<&str>, &str)) {
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

fn is_value_in_quotes(value: &str) -> bool {
    value.starts_with('"') && value.ends_with('"') && value.len() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_queries_empty() {
        assert_eq!(parse_queries(""), vec![]);
        assert_eq!(parse_queries(" "), vec![]);
        assert_eq!(parse_queries("\""), vec![]);
        assert_eq!(parse_queries("\"\""), vec![]);
    }

    #[test]
    fn parse_queries_simple() {
        assert_eq!(
            parse_queries("standalone1"),
            vec![(0, SearchQuery::Standalone("standalone1".to_string()))]
        );

        assert_eq!(
            parse_queries("iden1:value1"),
            vec![(
                0,
                SearchQuery::IdenValue("iden1".to_string(), "value1".to_string())
            )]
        );
    }

    #[test]
    fn parse_queries_quotes() {
        assert_eq!(
            parse_queries("standalone1 \"\" standalone2"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (15, SearchQuery::Standalone("standalone2".to_string()))
            ]
        );
        assert_eq!(
            parse_queries("\"standalone1  \" standalone2"),
            vec![
                (1, SearchQuery::Standalone("standalone1  ".to_string())), // FIXMEThis should be 0
                (16, SearchQuery::Standalone("standalone2".to_string())),
            ]
        );
        assert_eq!(
            parse_queries("standalone1 \"  standalone2 \"\"standalone3 "),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (13, SearchQuery::Standalone("  standalone2 ".to_string())), // FIXMEThis should be 12
                (29, SearchQuery::Standalone("standalone3 ".to_string())) // FIXMEThis should be 28
            ]
        );
        assert_eq!(
            parse_queries("\"  standalone1 \" standalone1 "),
            vec![
                (1, SearchQuery::Standalone("  standalone1 ".to_string())), // FIXMEThis should be 0
                (17, SearchQuery::Standalone("standalone1".to_string()))
            ]
        );
        assert_eq!(
            parse_queries("standalone1 \"iden1:value1"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (13, SearchQuery::Standalone("iden1:value1".to_string()))
            ]
        );
    }

    #[test]
    fn parse_queries_empty_iden_or_value() {
        assert_eq!(
            parse_queries("standalone1 : standalone2"),
            vec![
                (0, SearchQuery::Standalone("standalone1".to_string())),
                (12, SearchQuery::IdenValue("".to_string(), "".to_string())),
                (14, SearchQuery::Standalone("standalone2".to_string()))
            ]
        );
        assert_eq!(
            parse_queries("standalone1 iden1: standalone2"),
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
            parse_queries("standalone1 :value1 standalone2"),
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
            parse_queries("\"iden1\":value1"),
            vec![
                (1, SearchQuery::Standalone("iden1".to_string())), // FIXMEThis should be 0
                (
                    7,
                    SearchQuery::IdenValue("".to_string(), "value1".to_string())
                )
            ]
        );
    }

    #[test]
    fn parse_queries_complex() {
        assert_eq!(
            parse_queries("iden1:value1 iden2:\"value2\" iden3:\" value3\" iden4:\"value4 \" iden5:\" value5 \""),
           vec![
                (0, SearchQuery::IdenValue("iden1".to_string(), "value1".to_string())),
                (13, SearchQuery::IdenValue("iden2".to_string(), "value2".to_string())),
                (28, SearchQuery::IdenValue("iden3".to_string(), " value3".to_string())),
                (44, SearchQuery::IdenValue("iden4".to_string(), "value4 ".to_string())),
                (60, SearchQuery::IdenValue("iden5".to_string(), " value5 ".to_string()))
            ]
        );

        assert_eq!(
            parse_queries(
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
}
