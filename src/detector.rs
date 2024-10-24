use gtk::{
    glib::{self, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::id::Id;

mod imp {
    use std::sync::OnceLock;

    use gtk::glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct Detector {}

    #[glib::object_subclass]
    impl ObjectSubclass for Detector {
        const NAME: &'static str = "UetsDetector";
        type Type = super::Detector;
    }

    impl ObjectImpl for Detector {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("detected")
                    .param_types([Id::static_type()])
                    .build()]
            })
        }
    }
}

glib::wrapper! {
    pub struct Detector(ObjectSubclass<imp::Detector>);
}

impl Detector {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_detected<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Id) + 'static,
    {
        self.connect_closure(
            "detected",
            false,
            closure_local!(|obj: &Self, id: &Id| f(obj, id)),
        )
    }

    pub fn simulate_detected(&self, id: &Id) {
        self.emit_detected(id);
    }

    fn emit_detected(&self, id: &Id) {
        self.emit_by_name("detected", &[id])
    }
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}
