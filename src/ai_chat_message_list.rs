use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::ai_chat_message::AiChatMessage;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct AiChatMessageList {
        pub(super) list: RefCell<Vec<AiChatMessage>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AiChatMessageList {
        const NAME: &'static str = "UetsAiChatMessageList";
        type Type = super::AiChatMessageList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for AiChatMessageList {}

    impl ListModelImpl for AiChatMessageList {
        fn item_type(&self) -> glib::Type {
            AiChatMessage::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get(position as usize)
                .map(|item| item.clone().upcast())
        }
    }
}

glib::wrapper! {
    pub struct AiChatMessageList(ObjectSubclass<imp::AiChatMessageList>)
        @implements gio::ListModel;
}

impl AiChatMessageList {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn iter(&self) -> impl Iterator<Item = AiChatMessage> + '_ {
        ListModelExtManual::iter(self).map(|item| item.unwrap())
    }

    pub fn last(&self) -> Option<AiChatMessage> {
        self.imp().list.borrow().last().cloned()
    }

    pub fn push(&self, item: AiChatMessage) {
        let imp = self.imp();

        let position = imp.list.borrow().len() as u32;
        imp.list.borrow_mut().push(item.clone());

        self.items_changed(position, 0, 1);
    }

    pub fn clear(&self) {
        let imp = self.imp();

        let prev_n_items = imp.list.borrow().len() as u32;

        if prev_n_items == 0 {
            return;
        }

        imp.list.borrow_mut().clear();

        self.items_changed(0, prev_n_items, 0);
    }
}

impl Default for AiChatMessageList {
    fn default() -> Self {
        Self::new()
    }
}
