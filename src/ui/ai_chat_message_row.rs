use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use crate::ai_chat_message::{AiChatMessage, AiChatMessageState, AiChatMessageTy};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::AiChatMessageRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/ai_chat_message_row.ui")]
    pub struct AiChatMessageRow {
        #[property(get, set = Self::set_message, construct_only)]
        pub(super) message: OnceCell<AiChatMessage>,

        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) user_page: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) user_message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) ai_page: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) ai_message_label: TemplateChild<gtk::Label>,

        pub(super) message_state_notify_id: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AiChatMessageRow {
        const NAME: &'static str = "UetsAiChatMessageRow";
        type Type = super::AiChatMessageRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AiChatMessageRow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            match obj.message().ty() {
                AiChatMessageTy::User => {
                    self.stack.set_visible_child(&*self.user_page);
                }
                AiChatMessageTy::Ai => {
                    self.stack.set_visible_child(&*self.ai_page);
                }
            }

            obj.update_labels();
        }

        fn dispose(&self) {
            let obj = self.obj();

            if let Some(handler_id) = self.message_state_notify_id.take() {
                obj.message().disconnect(handler_id);
            }
        }
    }

    impl WidgetImpl for AiChatMessageRow {}
    impl ListBoxRowImpl for AiChatMessageRow {}

    impl AiChatMessageRow {
        fn set_message(&self, message: AiChatMessage) {
            let obj = self.obj();

            let handler_id = message.connect_state_notify(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_labels();
                }
            ));
            self.message_state_notify_id.replace(Some(handler_id));

            self.message.set(message).unwrap();
        }
    }
}

glib::wrapper! {
    pub struct AiChatMessageRow(ObjectSubclass<imp::AiChatMessageRow>)
        @extends gtk::Widget;
}

impl AiChatMessageRow {
    pub fn new(message: &AiChatMessage) -> Self {
        glib::Object::builder().property("message", message).build()
    }

    fn update_labels(&self) {
        let imp = self.imp();

        match self.message().state() {
            AiChatMessageState::Idle => {
                imp.user_message_label.set_text("");
                imp.ai_message_label.set_text("");
            }
            AiChatMessageState::Loading => {
                imp.user_message_label.set_text("…");
                imp.ai_message_label.set_text("…");
            }
            AiChatMessageState::Loaded(message) => {
                imp.user_message_label.set_text(&message);
                imp.ai_message_label.set_text(&message);
            }
        }
    }
}
