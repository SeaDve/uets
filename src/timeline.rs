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
    pub struct Timeline(ObjectSubclass<imp::Timeline>)
        @implements gio::ListModel;
}

impl Timeline {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn push(&self, item: TimelineItem) {
        let imp = self.imp();

        let dt = item.dt().clone();
        match item.kind() {
            TimelineItemKind::Entry => {
                if self
                    .last_entry_dt()
                    .map_or(true, |last_entry_dt| dt > last_entry_dt)
                {
                    self.set_last_entry_dt(Some(dt));
                }
            }
            TimelineItemKind::Exit { .. } => {
                if self
                    .last_exit_dt()
                    .map_or(true, |last_exit_dt| dt > last_exit_dt)
                {
                    self.set_last_exit_dt(Some(dt));
                }
            }
        }

        let index = imp.list.borrow().len();
        imp.list.borrow_mut().push(item);
        self.items_changed(index as u32, 0, 1);
    }

    pub fn clear(&self) {
        let imp = self.imp();

        let n_items = imp.list.borrow().len();
        imp.list.borrow_mut().clear();
        self.items_changed(0, n_items as u32, 0);
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
        items.sort_by(|a, b| a.dt().cmp(b.dt()));

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

        this
    }
}
