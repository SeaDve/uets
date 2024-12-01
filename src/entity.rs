use chrono::{DateTime, Utc};
use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{
    date_time_range::DateTimeRange,
    entity_data::{EntityData, EntityDataField, EntityDataFieldTy},
    entity_id::EntityId,
    log::Log,
    stock_id::StockId,
};

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        marker::PhantomData,
    };

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Entity)]
    pub struct Entity {
        #[property(get = Self::is_inside)]
        pub(super) is_inside: PhantomData<bool>,

        pub(super) id: OnceCell<EntityId>,
        pub(super) data: OnceCell<EntityData>,

        pub(super) is_inside_log: RefCell<Log<bool>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Entity {
        const NAME: &'static str = "UetsEntity";
        type Type = super::Entity;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Entity {}

    impl Entity {
        fn is_inside(&self) -> bool {
            self.is_inside_log
                .borrow()
                .latest()
                .copied()
                .unwrap_or(false)
        }
    }
}

glib::wrapper! {
    pub struct Entity(ObjectSubclass<imp::Entity>);
}

impl Entity {
    pub fn new(id: EntityId, data: EntityData) -> Self {
        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.id.set(id).unwrap();
        imp.data.set(data).unwrap();

        this
    }

    pub fn with_data(&self, data: EntityData) -> Self {
        let imp = self.imp();

        // FIXME add ability to change stock id
        let fields = data
            .into_fields()
            .filter(|f| f.ty() != EntityDataFieldTy::StockId)
            .chain(self.stock_id().cloned().map(EntityDataField::StockId));

        let new = Self::new(self.id().clone(), EntityData::from_fields(fields));

        let new_imp = new.imp();
        new_imp
            .is_inside_log
            .replace(imp.is_inside_log.borrow().clone());

        new
    }

    pub fn id(&self) -> &EntityId {
        self.imp().id.get().unwrap()
    }

    pub fn data(&self) -> &EntityData {
        self.imp().data.get().unwrap()
    }

    pub fn stock_id(&self) -> Option<&StockId> {
        self.data().stock_id()
    }

    pub fn is_inside_for_dt(&self, dt: DateTime<Utc>) -> bool {
        self.imp()
            .is_inside_log
            .borrow()
            .for_dt(dt)
            .copied()
            .unwrap_or(false)
    }

    pub fn is_inside_for_dt_range(&self, dt_range: &DateTimeRange) -> bool {
        if let Some(end) = dt_range.end {
            self.is_inside_for_dt(end)
        } else {
            self.is_inside()
        }
    }

    pub fn last_action_dt(&self) -> Option<DateTime<Utc>> {
        self.imp().is_inside_log.borrow().latest_dt()
    }

    pub fn with_is_inside_log_mut(&self, f: impl FnOnce(&mut Log<bool>)) {
        let prev_is_inside = self.is_inside();

        f(&mut self.imp().is_inside_log.borrow_mut());

        if prev_is_inside != self.is_inside() {
            self.notify_is_inside();
        }
    }
}
