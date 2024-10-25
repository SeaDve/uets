use std::time::Instant;

use anyhow::{Context, Result};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::{map::Entry, IndexMap};

use crate::{
    date_time::DateTime,
    db::{self, EnvExt},
    entity::Entity,
    entity_id::EntityId,
};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use indexmap::IndexMap;

    use super::*;

    #[derive(Default)]
    pub struct EntityTracker {
        pub(super) entities: RefCell<IndexMap<EntityId, Entity>>,
        pub(super) db: OnceCell<(heed::Env, db::EntitiesDbType)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityTracker {
        const NAME: &'static str = "UetsEntityTracker";
        type Type = super::EntityTracker;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for EntityTracker {}

    impl ListModelImpl for EntityTracker {
        fn item_type(&self) -> glib::Type {
            Entity::static_type()
        }

        fn n_items(&self) -> u32 {
            self.entities.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.entities
                .borrow()
                .get_index(position as usize)
                .map(|(_, v)| v.upcast_ref::<glib::Object>())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub struct EntityTracker(ObjectSubclass<imp::EntityTracker>)
        @implements gio::ListModel;
}

impl EntityTracker {
    pub fn load_from_env(env: heed::Env) -> Result<Self> {
        let db_load_start_time = Instant::now();

        let (db, entities) = env.with_write_txn(|wtxn| {
            let db: db::EntitiesDbType = env
                .create_database(wtxn, Some(db::ENTITIES_DB_NAME))
                .context("Failed to create entities db")?;
            let entities = db
                .iter(wtxn)
                .context("Failed to iter entities from db")?
                .map(|res| {
                    res.map(|(id, raw)| {
                        let entity = Entity::from_db(&id, raw);
                        (id, entity)
                    })
                })
                .collect::<Result<IndexMap<_, _>, _>>()
                .context("Failed to collect entities from db")?;
            Ok((db, entities))
        })?;

        tracing::debug!(
            "Loaded {} entities in {:?}",
            entities.len(),
            db_load_start_time.elapsed()
        );

        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.entities.replace(entities);
        imp.db.set((env, db)).unwrap();

        Ok(this)
    }

    pub fn inside_entities(&self) -> Vec<EntityId> {
        let imp = self.imp();

        imp.entities
            .borrow()
            .iter()
            .filter(|(_, entity)| entity.is_inside())
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn handle_entity(&self, id: &EntityId) -> Result<()> {
        let imp = self.imp();

        let entity = imp
            .entities
            .borrow()
            .get(id)
            .cloned()
            .unwrap_or_else(|| Entity::new(id));

        let now = DateTime::now_utc();
        if entity.is_inside() {
            entity.add_exit_dt(now);
        } else {
            entity.add_entry_dt(now);
        }

        let (env, db) = self.db();
        env.with_write_txn(|wtxn| {
            db.put(wtxn, id, &entity.to_db())
                .context("Failed to put entity to db")?;
            Ok(())
        })?;

        let (index, removed, added) = match imp.entities.borrow_mut().entry(id.clone()) {
            Entry::Occupied(entry) => (entry.index(), 1, 1),
            Entry::Vacant(entry) => {
                let index = entry.index();
                entry.insert(entity.clone());
                (index, 0, 1)
            }
        };

        self.items_changed(index as u32, removed, added);

        Ok(())
    }

    pub fn reset(&self) -> Result<()> {
        let imp = self.imp();

        let (env, db) = self.db();
        env.with_write_txn(|wtxn| {
            db.clear(wtxn).context("Failed to clear entities db")?;
            Ok(())
        })?;

        let prev_len = imp.entities.borrow().len();

        if prev_len != 0 {
            imp.entities.borrow_mut().clear();
            self.items_changed(0, prev_len as u32, 0);
        }

        Ok(())
    }

    fn db(&self) -> &(heed::Env, db::EntitiesDbType) {
        self.imp().db.get().unwrap()
    }
}
