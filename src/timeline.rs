use std::time::Instant;

use anyhow::{Context, Result};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::IndexMap;

use crate::{
    date_time::DateTime,
    db::{self, EnvExt},
    entity::Entity,
    entity_id::EntityId,
    entity_list::EntityList,
    timeline_item::{TimelineItem, TimelineItemKind},
};

mod imp {
    use std::{
        cell::{Cell, OnceCell, RefCell},
        marker::PhantomData,
    };

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Timeline)]
    pub struct Timeline {
        #[property(get = Self::n_inside)]
        pub(super) n_inside: PhantomData<u32>,
        #[property(get)]
        pub(super) last_entry_dt: Cell<Option<DateTime>>,
        #[property(get)]
        pub(super) last_exit_dt: Cell<Option<DateTime>>,

        pub(super) list: RefCell<IndexMap<DateTime, TimelineItem>>,
        pub(super) db: OnceCell<(heed::Env, db::TimelineDbType, db::EntitiesDbType)>,

        pub(super) entity_list: OnceCell<EntityList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Timeline {
        const NAME: &'static str = "UetsTimeline";
        type Type = super::Timeline;
        type Interfaces = (gio::ListModel,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for Timeline {}

    impl ListModelImpl for Timeline {
        fn item_type(&self) -> glib::Type {
            TimelineItem::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, v)| v.upcast_ref::<glib::Object>().clone())
        }
    }

    impl Timeline {
        fn n_inside(&self) -> u32 {
            self.list
                .borrow()
                .last()
                .map(|(_, item)| item.n_inside())
                .unwrap_or(0)
        }
    }
}

glib::wrapper! {
    /// A timeline with sorted items by date-time.
    pub struct Timeline(ObjectSubclass<imp::Timeline>)
        @implements gio::ListModel;
}

impl Timeline {
    pub fn load_from_env(env: heed::Env) -> Result<Self> {
        let db_load_start_time = Instant::now();

        let (tdb, items, edb, entities) = env.with_write_txn(|wtxn| {
            let tdb: db::TimelineDbType = env
                .create_database(wtxn, Some(db::TIMELINE_DB_NAME))
                .context("Failed to create timeline db")?;
            let items = tdb
                .iter(wtxn)
                .context("Failed to iter items from db")?
                .map(|res| {
                    res.map(|(dt, raw)| {
                        let item = TimelineItem::from_db(dt, &raw);
                        (dt, item)
                    })
                })
                .collect::<Result<IndexMap<_, _>, _>>()
                .context("Failed to collect items from db")?;

            let edb: db::EntitiesDbType = env
                .create_database(wtxn, Some(db::ENTITIES_DB_NAME))
                .context("Failed to create entities db")?;
            let entities = edb
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

            Ok((tdb, items, edb, entities))
        })?;
        debug_assert!(items.keys().is_sorted());

        tracing::debug!(
            "Loaded {} items and entities in {:?}",
            items.len(),
            db_load_start_time.elapsed()
        );

        let last_entry_dt = {
            let mut last_entry_dt = None;
            for (_, item) in items.iter().rev() {
                if item.kind() == TimelineItemKind::Entry {
                    last_entry_dt = Some(item.dt());
                    break;
                }
            }
            last_entry_dt
        };
        let last_exit_dt = {
            let mut last_exit_dt = None;
            for (_, item) in items.iter().rev() {
                if let TimelineItemKind::Exit { .. } = item.kind() {
                    last_exit_dt = Some(item.dt());
                    break;
                }
            }
            last_exit_dt
        };

        // We don't store entry dts on entities db
        for (_, item) in &items {
            let entity = entities
                .get(item.entity_id())
                .expect("timeline must match with entities db");
            match item.kind() {
                TimelineItemKind::Entry => {
                    entity.add_entry_dt(item.dt());
                }
                TimelineItemKind::Exit { .. } => {
                    entity.add_exit_dt(item.dt());
                }
            }
        }

        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.list.replace(items);
        imp.db.set((env, tdb, edb)).unwrap();
        imp.entity_list.set(EntityList::from_raw(entities)).unwrap();
        imp.last_entry_dt.set(last_entry_dt);
        imp.last_exit_dt.set(last_exit_dt);

        Ok(this)
    }

    pub fn entity_list(&self) -> &EntityList {
        self.imp().entity_list.get().unwrap()
    }

    pub fn handle_detected(&self, entity_id: &EntityId) -> Result<()> {
        let imp = self.imp();

        let entity = self
            .entity_list()
            .get(entity_id)
            .unwrap_or_else(|| Entity::new(entity_id));

        let now_dt = DateTime::now();
        debug_assert!(imp
            .list
            .borrow()
            .last()
            .map_or(true, |(dt, _)| &now_dt > dt));

        let was_inside = entity.is_inside();

        if was_inside {
            entity.add_exit_dt(now_dt);
        } else {
            entity.add_entry_dt(now_dt);
        }

        let item_kind = if was_inside {
            TimelineItemKind::Exit {
                inside_duration: entity
                    .last_dt_pair()
                    .expect("added exit dt and thus a dt pair")
                    .inside_duration()
                    .expect("a complete dt pair"),
            }
        } else {
            TimelineItemKind::Entry
        };
        let new_n_inside = if was_inside {
            self.n_inside() - 1
        } else {
            self.n_inside() + 1
        };
        let item = TimelineItem::new(now_dt, item_kind, entity_id.clone(), new_n_inside);

        let (env, tdb, edb) = self.db();
        env.with_write_txn(|wtxn| {
            tdb.put(wtxn, &now_dt, &item.to_db())
                .context("Failed to put item to db")?;
            edb.put(wtxn, entity_id, &entity.to_db())
                .context("Failed to put entity to db")?;
            Ok(())
        })?;

        if was_inside {
            self.set_last_exit_dt(Some(now_dt));
        } else {
            self.set_last_entry_dt(Some(now_dt));
        }

        let (index, prev_value) = imp.list.borrow_mut().insert_full(now_dt, item);
        debug_assert_eq!(prev_value, None);

        self.entity_list().insert(entity);

        self.notify_n_inside();
        self.items_changed(index as u32, 0, 1);

        debug_assert!(imp.list.borrow().keys().is_sorted());

        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let imp = self.imp();

        let prev_len = imp.list.borrow().len();

        if prev_len == 0 {
            debug_assert_eq!(self.n_inside(), 0);
            debug_assert_eq!(self.last_entry_dt(), None);
            debug_assert_eq!(self.last_exit_dt(), None);
            debug_assert_eq!(self.entity_list().len(), 0);

            if cfg!(debug_assertions) {
                let (env, tdb, edb) = self.db();
                env.with_read_txn(|rtxn| {
                    let tdb_n_items = tdb.len(rtxn).context("Failed to get timeline db len")?;
                    debug_assert_eq!(tdb_n_items, 0);
                    let edb_n_items = edb.len(rtxn).context("Failed to get entities db len")?;
                    debug_assert_eq!(edb_n_items, 0);
                    Ok(())
                })?;
            }

            return Ok(());
        }

        let (env, tdb, edb) = self.db();
        env.with_write_txn(|wtxn| {
            tdb.clear(wtxn).context("Failed to clear timeline db")?;
            edb.clear(wtxn).context("Failed to clear entities db")?;
            Ok(())
        })?;

        imp.list.borrow_mut().clear();

        self.set_last_entry_dt(None);
        self.set_last_exit_dt(None);

        self.entity_list().clear();

        self.notify_n_inside();
        self.items_changed(0, prev_len as u32, 0);

        debug_assert!(imp.list.borrow().keys().is_sorted());

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.imp().list.borrow().len()
    }

    fn set_last_entry_dt(&self, dt: Option<DateTime>) {
        let imp = self.imp();

        if dt == self.last_entry_dt() {
            return;
        }

        imp.last_entry_dt.replace(dt);
        self.notify_last_entry_dt();
    }

    fn set_last_exit_dt(&self, dt: Option<DateTime>) {
        let imp = self.imp();

        if dt == self.last_exit_dt() {
            return;
        }

        imp.last_exit_dt.replace(dt);
        self.notify_last_exit_dt();
    }

    fn db(&self) -> &(heed::Env, db::TimelineDbType, db::EntitiesDbType) {
        self.imp().db.get().unwrap()
    }
}
