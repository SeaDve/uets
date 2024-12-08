use std::cell::RefCell;

use gtk::{glib, prelude::*};

pub struct SignalHandlerIdGroup(RefCell<Vec<glib::SignalHandlerId>>);

impl SignalHandlerIdGroup {
    pub fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    pub fn add(&self, id: glib::SignalHandlerId) {
        self.0.borrow_mut().push(id);
    }
}

pub trait SignalHandlerIdGroupObjectExt {
    fn disconnect_group(&self, id: SignalHandlerIdGroup);
}

impl<T: ObjectType> SignalHandlerIdGroupObjectExt for T {
    fn disconnect_group(&self, group: SignalHandlerIdGroup) {
        for id in group.0.into_inner().into_iter() {
            self.disconnect(id);
        }
    }
}
