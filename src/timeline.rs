use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::timeline_item::TimelineItem;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct Timeline {
        pub(super) list: RefCell<Vec<TimelineItem>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Timeline {
        const NAME: &'static str = "UetsTimeline";
        type Type = super::Timeline;
        type Interfaces = (gio::ListModel,);
    }

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
}

impl FromIterator<TimelineItem> for Timeline {
    fn from_iter<T: IntoIterator<Item = TimelineItem>>(iter: T) -> Self {
        let this = Self::new();

        let imp = this.imp();
        imp.list.borrow_mut().extend(iter);

        this
    }
}
