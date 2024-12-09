use gtk::{glib, prelude::*, subclass::prelude::*};

#[derive(Default, Clone, Copy, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "UetsAiChatMessageTy")]
pub enum AiChatMessageTy {
    #[default]
    User,
    Ai,
}

#[derive(Default, Clone, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "UetsAiChatMessageState")]
pub enum AiChatMessageState {
    #[default]
    Idle,
    Loading,
    Loaded(String),
}

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::AiChatMessage)]
    pub struct AiChatMessage {
        #[property(get, set, construct_only, builder(AiChatMessageTy::default()))]
        pub(super) ty: Cell<AiChatMessageTy>,
        #[property(get, set = Self::set_state, explicit_notify)]
        pub(super) state: RefCell<AiChatMessageState>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AiChatMessage {
        const NAME: &'static str = "UetsAiChatMessage";
        type Type = super::AiChatMessage;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AiChatMessage {}

    impl AiChatMessage {
        fn set_state(&self, state: AiChatMessageState) {
            let obj = self.obj();

            if state == obj.state() {
                return;
            }

            self.state.replace(state);
            obj.notify_state();
        }
    }
}

glib::wrapper! {
    pub struct AiChatMessage(ObjectSubclass<imp::AiChatMessage>);
}

impl AiChatMessage {
    pub fn new(ty: AiChatMessageTy) -> Self {
        glib::Object::builder().property("ty", ty).build()
    }

    pub fn set_loading(&self) {
        self.set_state(AiChatMessageState::Loading);
    }

    pub fn set_loaded(&self, text: impl Into<String>) {
        self.set_state(AiChatMessageState::Loaded(text.into()));
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self.state(), AiChatMessageState::Loaded(_))
    }

    pub fn text(&self) -> Option<String> {
        match self.state() {
            AiChatMessageState::Loaded(text) => Some(text),
            _ => None,
        }
    }
}
