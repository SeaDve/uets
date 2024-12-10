use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use std::cell::RefCell;

mod imp {
    use std::cell::Cell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::InformationRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/information_row.ui")]
    pub struct InformationRow {
        #[property(get, set = Self::set_label, explicit_notify)]
        pub(super) label: RefCell<String>,

        pub(super) label_use_markup: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InformationRow {
        const NAME: &'static str = "UetsInformationRow";
        type Type = super::InformationRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for InformationRow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.update_visibility_and_subtitle();
        }
    }

    impl WidgetImpl for InformationRow {}
    impl ListBoxRowImpl for InformationRow {}
    impl PreferencesRowImpl for InformationRow {}
    impl ActionRowImpl for InformationRow {}

    impl InformationRow {
        fn set_label(&self, label: String) {
            let obj = self.obj();

            if label == obj.label() {
                return;
            }

            self.label.replace(label);
            obj.update_visibility_and_subtitle();
            obj.notify_label();
        }
    }
}

glib::wrapper! {
    pub struct InformationRow(ObjectSubclass<imp::InformationRow>)
        @extends gtk::Widget, adw::PreferencesRow, adw::ActionRow;
}

impl InformationRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_text(&self, label: impl Into<String>) {
        let imp = self.imp();

        imp.label_use_markup.set(false);
        self.set_label(label.into());
    }

    pub fn set_markup(&self, label: impl Into<String>) {
        let imp = self.imp();

        imp.label_use_markup.set(true);
        self.set_label(label.into());
    }

    fn update_visibility_and_subtitle(&self) {
        let imp = self.imp();

        let _guard = self.freeze_notify();

        let label = self.label();
        self.set_visible(!label.trim().is_empty());

        if imp.label_use_markup.get() {
            self.set_subtitle(&label);
        } else {
            self.set_subtitle(&glib::markup_escape_text(&label));
        }
    }
}
