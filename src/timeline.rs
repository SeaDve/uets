use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::{
    date_time::DateTime,
    timeline_item::{TimelineItem, TimelineItemKind},
};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Timeline)]
    pub struct Timeline {
        #[property(get)]
        pub(super) last_entry_dt: RefCell<Option<DateTime>>,
        #[property(get)]
        pub(super) last_exit_dt: RefCell<Option<DateTime>>,

        pub(super) list: RefCell<Vec<TimelineItem>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Timeline {
        const NAME: &'static str = "UetsTimeline";
        type Type = super::Timeline;
        type Interfaces = (gio::ListModel,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for Timeline {}

    impl ListModelImpl for Timeline {
        fn item_type(&self) -> glib::Type {
            TimelineItem::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get(position as usize)
                .map(|o| o.upcast_ref::<glib::Object>().clone())
        }
    }
}

glib::wrapper! {
    /// A timeline with sorted items by date-time.
    pub struct Timeline(ObjectSubclass<imp::Timeline>)
        @implements gio::ListModel;
}

impl Timeline {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn insert(&self, item: TimelineItem) {
        let imp = self.imp();

        let last_index = imp.list.borrow().len() - 1;

        let insert_at = match imp
            .list
            .borrow()
            .binary_search_by_key(item.dt(), TimelineItem::sort_key)
        {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        // Check if it is the latest item.
        if insert_at == last_index + 1 {
            match item.kind() {
                TimelineItemKind::Entry => {
                    debug_assert!(self
                        .last_entry_dt()
                        .map_or(true, |last_entry_dt| item.dt() > &last_entry_dt));

                    self.set_last_entry_dt(Some(item.dt().clone()));
                }
                TimelineItemKind::Exit { .. } => {
                    debug_assert!(self
                        .last_exit_dt()
                        .map_or(true, |last_exit_dt| item.dt() > &last_exit_dt));

                    self.set_last_exit_dt(Some(item.dt().clone()));
                }
            }
        }

        imp.list.borrow_mut().insert(insert_at, item);
        self.items_changed(insert_at as u32, 0, 1);

        debug_assert!(imp.list.borrow().is_sorted_by_key(TimelineItem::sort_key));
    }

    pub fn clear(&self) {
        let imp = self.imp();

        let n_items = imp.list.borrow().len();
        imp.list.borrow_mut().clear();
        self.items_changed(0, n_items as u32, 0);

        debug_assert!(imp.list.borrow().is_sorted_by_key(TimelineItem::sort_key));
    }

    pub fn len(&self) -> usize {
        self.imp().list.borrow().len()
    }

    fn set_last_entry_dt(&self, dt: Option<DateTime>) {
        let imp = self.imp();

        if dt == self.last_entry_dt() {
            return;
        }

        imp.last_entry_dt.replace(dt);
        self.notify_last_entry_dt();
    }

    fn set_last_exit_dt(&self, dt: Option<DateTime>) {
        let imp = self.imp();

        if dt == self.last_exit_dt() {
            return;
        }

        imp.last_exit_dt.replace(dt);
        self.notify_last_exit_dt();
    }
}

impl FromIterator<TimelineItem> for Timeline {
    fn from_iter<T: IntoIterator<Item = TimelineItem>>(iter: T) -> Self {
        let this = Self::new();

        let mut items = iter.into_iter().collect::<Vec<_>>();
        items.sort_by_key(TimelineItem::sort_key);

        let last_entry_dt = {
            let mut last_entry_dt = None;
            for item in items.iter().rev() {
                if item.kind() == TimelineItemKind::Entry {
                    last_entry_dt = Some(item.dt().clone());
                    break;
                }
            }
            last_entry_dt
        };
        let last_exit_dt = {
            let mut last_exit_dt = None;
            for item in items.iter().rev() {
                if let TimelineItemKind::Exit { .. } = item.kind() {
                    last_exit_dt = Some(item.dt().clone());
                    break;
                }
            }
            last_exit_dt
        };

        let imp = this.imp();
        imp.list.replace(items);
        imp.last_entry_dt.replace(last_entry_dt);
        imp.last_exit_dt.replace(last_exit_dt);

        debug_assert!(imp.list.borrow().is_sorted_by_key(TimelineItem::sort_key));

        this
    }
}
