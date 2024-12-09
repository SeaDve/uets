use std::{collections::HashSet, time::Duration};

use chrono::{TimeDelta, Utc};
use gtk::{
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};

use crate::{application::Application, entity_id::EntityId, settings::Settings};

#[derive(Clone, glib::Boxed)]
#[boxed_type(name = "UetsOverstayed")]
pub struct EntityIdSet(pub HashSet<EntityId>);

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::Signal;

    use super::*;

    #[derive(Default)]
    pub struct EntityEntryTracker {
        pub(crate) inside_entities: RefCell<HashSet<EntityId>>,
        pub(crate) overstayed_entities: RefCell<HashSet<EntityId>>,

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
                vec![
                    Signal::builder("overstayed")
                        .param_types([EntityIdSet::static_type()])
                        .build(),
                    Signal::builder("overstayed-changed")
                        .param_types([EntityIdSet::static_type()])
                        .build(),
                ]
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
        F: Fn(&Self, &EntityIdSet) + 'static,
    {
        self.connect_closure(
            "overstayed",
            false,
            closure_local!(|obj: &Self, id: &EntityIdSet| f(obj, id)),
        )
    }

    pub fn connect_overstayed_changed<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &EntityIdSet) + 'static,
    {
        self.connect_closure(
            "overstayed-changed",
            false,
            closure_local!(|obj: &Self, id: &EntityIdSet| f(obj, id)),
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

    pub fn is_overstayed(&self, entity_id: &EntityId) -> bool {
        self.imp().emitted_overstayed.borrow().contains(entity_id)
    }

    fn check_overstayed(&self) {
        let imp = self.imp();

        let app = Application::get();
        let settings = app.settings();

        let entity_list = app.timeline().entity_list();
        let dt_now = Utc::now();

        let mut overstayed_entities = HashSet::new();
        for entity_id in imp.inside_entities.borrow().iter() {
            let entity = entity_list.get(entity_id).expect("entity must be known");

            let last_action_dt = entity
                .last_action_dt()
                .expect("entity must have last action dt");

            if settings.compute_overstayed(dt_now - last_action_dt) {
                overstayed_entities.insert(entity_id.clone());
            }
        }

        if overstayed_entities == *imp.overstayed_entities.borrow() {
            return;
        }

        imp.emitted_overstayed
            .borrow_mut()
            .retain(|id| overstayed_entities.contains(id));

        let unemitted_overstayed = overstayed_entities
            .iter()
            .filter(|id| !imp.emitted_overstayed.borrow().contains(id))
            .cloned()
            .collect::<HashSet<_>>();
        self.emit_by_name::<()>("overstayed", &[&EntityIdSet(unemitted_overstayed.clone())]);
        imp.emitted_overstayed
            .borrow_mut()
            .extend(unemitted_overstayed);

        let prev_overstayed_entities = imp.overstayed_entities.replace(overstayed_entities.clone());
        self.emit_by_name::<()>(
            "overstayed-changed",
            &[&EntityIdSet(
                overstayed_entities
                    .symmetric_difference(&prev_overstayed_entities)
                    .cloned()
                    .collect::<HashSet<_>>(),
            )],
        );
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

pub trait EntityEntryTrackerSettingsExt {
    fn compute_overstayed(&self, duration: TimeDelta) -> bool;
}

impl EntityEntryTrackerSettingsExt for Settings {
    fn compute_overstayed(&self, duration: TimeDelta) -> bool {
        duration.num_seconds().unsigned_abs() > self.max_entry_to_exit_duration_secs() as u64
    }
}
