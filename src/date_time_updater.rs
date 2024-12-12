use std::time::Duration;

use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct DateTimeUpdater {
        pub(super) timeout_id: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateTimeUpdater {
        const NAME: &'static str = "UetsDateTimeUpdater";
        type Type = super::DateTimeUpdater;
    }

    impl ObjectImpl for DateTimeUpdater {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let timeout_id = glib::timeout_add_local_full(
                UPDATE_INTERVAL,
                glib::Priority::LOW,
                clone!(
                    #[weak]
                    obj,
                    #[upgrade_or_panic]
                    move || {
                        obj.emit_by_name::<()>("update", &[]);
                        glib::ControlFlow::Continue
                    }
                ),
            );
            self.timeout_id.replace(Some(timeout_id));
        }

        fn dispose(&self) {
            if let Some(timeout_id) = self.timeout_id.take() {
                timeout_id.remove();
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| vec![Signal::builder("update").build()])
        }
    }
}

glib::wrapper! {
    pub struct DateTimeUpdater(ObjectSubclass<imp::DateTimeUpdater>);
}

impl DateTimeUpdater {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_update<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure("update", false, closure_local!(|obj: &Self| f(obj)))
    }
}

impl Default for DateTimeUpdater {
    fn default() -> Self {
        Self::new()
    }
}
