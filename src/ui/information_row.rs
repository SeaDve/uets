use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone, closure_local};

use std::cell::RefCell;

mod imp {
    use std::{cell::Cell, sync::OnceLock};

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::InformationRow)]
    #[template(resource = "/io/github/seadve/Uets/ui/information_row.ui")]
    pub struct InformationRow {
        /// Value of the information
        ///
        /// If this is empty, self will be hidden.
        #[property(get, set = Self::set_value, explicit_notify)]
        pub(super) value: RefCell<String>,
        #[property(get, set = Self::set_value_use_markup, explicit_notify)]
        pub(super) value_use_markup: Cell<bool>,

        #[template_child]
        pub(super) value_label: TemplateChild<gtk::Label>,
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

            self.value_label.connect_activate_link(clone!(
                #[weak]
                obj,
                #[upgrade_or_panic]
                move |_, uri| {
                    obj.emit_by_name::<bool>("activate-value-link", &[&uri])
                        .into()
                }
            ));

            obj.update_ui();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("activate-value-link")
                    .param_types([String::static_type()])
                    .return_type::<bool>()
                    .build()]
            })
        }
    }

    impl WidgetImpl for InformationRow {}
    impl ListBoxRowImpl for InformationRow {}
    impl PreferencesRowImpl for InformationRow {}
    impl ActionRowImpl for InformationRow {}

    impl InformationRow {
        fn set_value(&self, value: String) {
            let obj = self.obj();

            if value == obj.value() {
                return;
            }

            self.value.replace(value);
            obj.update_ui();
            obj.notify_value();
        }

        fn set_value_use_markup(&self, value_use_markup: bool) {
            let obj = self.obj();

            if value_use_markup == obj.value_use_markup() {
                return;
            }

            self.value_label.set_use_markup(value_use_markup);

            self.value_use_markup.set(value_use_markup);
            obj.notify_value_use_markup();
        }
    }
}

glib::wrapper! {
    pub struct InformationRow(ObjectSubclass<imp::InformationRow>)
        @extends gtk::Widget, adw::PreferencesRow;
}

impl InformationRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_activate_value_link<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &str) -> glib::Propagation + 'static,
    {
        self.connect_closure(
            "activate-value-link",
            false,
            closure_local!(|obj: &Self, uri: &str| f(obj, uri)),
        )
    }

    fn update_ui(&self) {
        let imp = self.imp();

        let value = self.value();
        self.set_visible(!value.trim().is_empty());
        imp.value_label.set_label(&value);
    }
}
