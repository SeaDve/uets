use std::{collections::HashSet, time::Duration};

use chrono::Utc;
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{application::Application, entity_id::EntityId};

#[derive(Clone, glib::Boxed)]
#[boxed_type(name = "UetsOverstayed")]
pub struct Overstayed(pub HashSet<EntityId>);

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct EntityEntryTracker {
        pub(crate) inside_entities: RefCell<HashSet<EntityId>>,
        pub(crate) emitted_overstayed: RefCell<HashSet<EntityId>>,

        pub(crate) check_overstayed_timeout_id: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityEntryTracker {
        const NAME: &'static str = "UetsEntityEntryTracker";
        type Type = super::EntityEntryTracker;
    }

    impl ObjectImpl for EntityEntryTracker {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            Application::get()
                .settings()
                .connect_max_entry_to_exit_duration_secs_changed(clone!(
                    #[weak(rename_to = obj)]
                    obj,
                    move |_| {
                        obj.check_overstayed();
                        obj.update_check_overstayed_timeout();
                    }
                ));

            obj.update_check_overstayed_timeout();
        }

        fn dispose(&self) {
            if let Some(source_id) = self.check_overstayed_timeout_id.take() {
                source_id.remove();
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("overstayed")
                    .param_types([Overstayed::static_type()])
                    .build()]
            })
        }
    }
}

glib::wrapper! {
    pub struct EntityEntryTracker(ObjectSubclass<imp::EntityEntryTracker>);
}

impl EntityEntryTracker {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_overstayed<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Overstayed) + 'static,
    {
        self.connect_closure(
            "overstayed",
            false,
            closure_local!(|obj: &Self, id: &Overstayed| f(obj, id)),
        )
    }

    pub fn handle_entry(&self, entity_id: &EntityId) {
        let imp = self.imp();

        imp.inside_entities.borrow_mut().insert(entity_id.clone());
        imp.emitted_overstayed.borrow_mut().remove(entity_id);
    }

    pub fn handle_exit(&self, entity_id: &EntityId) {
        let imp = self.imp();

        imp.inside_entities.borrow_mut().remove(entity_id);
        imp.emitted_overstayed.borrow_mut().remove(entity_id);
    }

    fn check_overstayed(&self) {
        let imp = self.imp();

        let app = Application::get();

        let max_entry_to_exit_duration_secs = app.settings().max_entry_to_exit_duration_secs();
        let entity_list = app.timeline().entity_list();
        let dt_now = Utc::now();

        let mut overstayed = Overstayed(HashSet::new());
        for entity_id in imp.inside_entities.borrow().iter() {
            let entity = entity_list.get(entity_id).expect("entity must be known");

            let last_action_dt = entity
                .last_action_dt()
                .expect("entity must have last action dt");

            if (dt_now - last_action_dt).num_seconds().unsigned_abs()
                > max_entry_to_exit_duration_secs as u64
                && !imp.emitted_overstayed.borrow().contains(entity_id)
            {
                overstayed.0.insert(entity_id.clone());
            }
        }

        self.emit_by_name::<()>("overstayed", &[&overstayed]);
        imp.emitted_overstayed.borrow_mut().extend(overstayed.0);
    }

    fn update_check_overstayed_timeout(&self) {
        let imp = self.imp();

        if let Some(prev_source_id) = imp.check_overstayed_timeout_id.borrow_mut().take() {
            prev_source_id.remove();
        }

        let max_entry_to_exit_duration_secs = Application::get()
            .settings()
            .max_entry_to_exit_duration_secs() as f64;
        let source_id = glib::timeout_add_local_full(
            Duration::from_secs_f64(max_entry_to_exit_duration_secs / 4.0),
            glib::Priority::LOW,
            clone!(
                #[weak(rename_to = obj)]
                self,
                #[upgrade_or_panic]
                move || {
                    obj.check_overstayed();

                    glib::ControlFlow::Continue
                }
            ),
        );

        imp.check_overstayed_timeout_id.replace(Some(source_id));
    }
}

impl Default for EntityEntryTracker {
    fn default() -> Self {
        Self::new()
    }
}
