use std::time::Duration;

use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

const DEFAULT_SEARCH_DELAY_MS: u32 = 150;

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

    /// Sets the text without delaying the search-changed signal.
    pub fn set_text_instant(&self, text: &str) {
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
                },
            ),
        );
        imp.search_changed_timeout_id.replace(Some(source_id));
    }
}
