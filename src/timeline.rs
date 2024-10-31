use std::{collections::HashMap, time::Instant};

use anyhow::{bail, Context, Result};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::IndexMap;

use crate::{
    date_time::DateTime,
    db::{self, EnvExt},
    entity::Entity,
    entity_id::EntityId,
    entity_list::EntityList,
    stock::Stock,
    stock_id::StockId,
    stock_list::StockList,
    stock_timeline::StockTimeline,
    stock_timeline_item::StockTimelineItem,
    timeline_item::TimelineItem,
    timeline_item_kind::TimelineItemKind,
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
        pub(super) max_n_inside: Cell<u32>,
        #[property(get)]
        pub(super) n_entries: Cell<u32>,
        #[property(get)]
        pub(super) n_exits: Cell<u32>,
        #[property(get)]
        pub(super) last_entry_dt: Cell<Option<DateTime>>,
        #[property(get)]
        pub(super) last_exit_dt: Cell<Option<DateTime>>,

        pub(super) list: RefCell<IndexMap<DateTime, TimelineItem>>,
        pub(super) db: OnceCell<(
            heed::Env,
            db::TimelineDbType,
            db::EntitiesDbType,
            db::StocksDbType,
        )>,

        pub(super) entity_list: OnceCell<EntityList>,
        pub(super) stock_list: OnceCell<StockList>,
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
                .map_or(0, |(_, item)| item.n_inside())
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

        let (tdb, items, edb, entities, sdb, raw_stocks) = env.with_write_txn(|wtxn| {
            let tdb: db::TimelineDbType = env
                .create_database(wtxn, Some(db::TIMELINE_DB_NAME))
                .context("Failed to create timeline db")?;
            let items = tdb
                .iter(wtxn)
                .context("Failed to iter items from db")?
                .map(|res| {
                    res.map(|(dt, raw)| {
                        let item = TimelineItem::from_db(dt, raw);
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
                        let entity = Entity::from_db(id.clone(), raw);
                        (id, entity)
                    })
                })
                .collect::<Result<IndexMap<_, _>, _>>()
                .context("Failed to collect entities from db")?;

            let sdb: db::StocksDbType = env
                .create_database(wtxn, Some(db::STOCKS_DB_NAME))
                .context("Failed to create stocks db")?;
            let stocks = sdb
                .iter(wtxn)
                .context("Failed to iter stocks from db")?
                .collect::<Result<IndexMap<_, _>, _>>()
                .context("Failed to collect stocks from db")?;

            Ok((tdb, items, edb, entities, sdb, stocks))
        })?;
        debug_assert!(items.keys().is_sorted());

        tracing::debug!(
            "Loaded {} items and entities in {:?}",
            items.len(),
            db_load_start_time.elapsed()
        );

        let max_n_inside = items
            .values()
            .map(|item| item.n_inside())
            .max()
            .unwrap_or(0);
        let n_entries = items
            .values()
            .filter(|item| matches!(item.kind(), TimelineItemKind::Entry))
            .count();
        let n_exits = items
            .values()
            .filter(|item| matches!(item.kind(), TimelineItemKind::Exit { .. }))
            .count();

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

        let mut raw_stocks_data =
            HashMap::<StockId, (IndexMap<DateTime, StockTimelineItem>, u32)>::new();
        for (_, item) in &items {
            // Recover entry dts as it is not stored on db.
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

            // Recover stock timeline as it is not stored on db.
            if let Some(stock_id) = item.stock_id() {
                let (raw_stock_timeline, n_inside) =
                    raw_stocks_data.entry(stock_id.clone()).or_default();

                match item.kind() {
                    TimelineItemKind::Entry => {
                        *n_inside += 1;
                    }
                    TimelineItemKind::Exit { .. } => {
                        *n_inside -= 1;
                    }
                }

                raw_stock_timeline.insert(
                    item.dt(),
                    StockTimelineItem::new(
                        item.dt(),
                        item.kind(),
                        item.entity_id().clone(),
                        stock_id.clone(),
                        *n_inside,
                    ),
                );
            }
        }

        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.list.replace(items);
        imp.db.set((env, tdb, edb, sdb)).unwrap();
        imp.entity_list.set(EntityList::from_raw(entities)).unwrap();
        imp.stock_list
            .set(StockList::from_raw(
                raw_stocks
                    .into_iter()
                    .map(|(stock_id, raw_stock)| {
                        let stock_timeline = raw_stocks_data
                            .remove(&stock_id)
                            .map_or_else(StockTimeline::new, |(raw_stock_timeline, _)| {
                                StockTimeline::from_raw(raw_stock_timeline)
                            });
                        let stock = Stock::from_db(stock_id.clone(), stock_timeline, raw_stock);
                        (stock_id, stock)
                    })
                    .collect(),
            ))
            .unwrap();
        imp.max_n_inside.set(max_n_inside);
        imp.n_entries.set(n_entries as u32);
        imp.n_exits.set(n_exits as u32);
        imp.last_entry_dt.set(last_entry_dt);
        imp.last_exit_dt.set(last_exit_dt);

        Ok(this)
    }

    pub fn entity_list(&self) -> &EntityList {
        self.imp().entity_list.get().unwrap()
    }

    pub fn stock_list(&self) -> &StockList {
        self.imp().stock_list.get().unwrap()
    }

    pub fn handle_detected(
        &self,
        provided_entity_id: &EntityId,
        provided_stock_id: Option<&StockId>,
    ) -> Result<()> {
        let imp = self.imp();

        let entity = self
            .entity_list()
            .get(provided_entity_id)
            .unwrap_or_else(|| Entity::new(provided_entity_id.clone(), provided_stock_id.cloned()));

        // TODO Should this be allowed instead?
        //
        // When exiting, this should not be allowed as an entity cannot enter then exit
        // with different stock id. But if the same entity enters with a different stock id,
        // the id may have been reused on the a different item, I think this should be allowed,
        // or can it even happen?
        if provided_stock_id.is_some() && provided_stock_id != entity.stock_id() {
            bail!(
                "Entity `{}` already handled with different stock id",
                provided_entity_id
            );
        }

        let now_dt = DateTime::now();
        debug_assert!(imp
            .list
            .borrow()
            .last()
            .map_or(true, |(dt, _)| &now_dt > dt));

        let is_exit = entity.is_inside();

        if is_exit {
            entity.add_exit_dt(now_dt);
        } else {
            entity.add_entry_dt(now_dt);
        }

        let stock_id = provided_stock_id.or_else(|| entity.stock_id());

        let item_kind = if is_exit {
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
        let new_n_inside = if is_exit {
            self.n_inside() - 1
        } else {
            self.n_inside() + 1
        };
        let item = TimelineItem::new(
            now_dt,
            item_kind,
            provided_entity_id.clone(),
            stock_id.cloned(),
            new_n_inside,
        );

        // Use entity stock id from entity if no stock id is provided.
        let stock = if let Some(stock_id) = stock_id {
            let stock = self
                .stock_list()
                .get(stock_id)
                .unwrap_or_else(|| Stock::new(stock_id.clone()));
            let stock_timeline = stock.timeline();

            let stock_new_n_inside = if is_exit {
                stock_timeline.n_inside() - 1
            } else {
                stock_timeline.n_inside() + 1
            };
            stock_timeline.insert(StockTimelineItem::new(
                now_dt,
                item_kind,
                provided_entity_id.clone(),
                stock_id.clone(),
                stock_new_n_inside,
            ));

            Some(stock)
        } else {
            None
        };

        let (env, tdb, edb, sdb) = self.db();
        env.with_write_txn(|wtxn| {
            tdb.put(wtxn, &now_dt, &item.to_db())
                .context("Failed to put item to db")?;
            edb.put(wtxn, entity.id(), &entity.to_db())
                .context("Failed to put entity to db")?;
            if let Some(stock) = &stock {
                sdb.put(wtxn, stock.id(), &stock.to_db())
                    .context("Failed to put stock to db")?;
            }
            Ok(())
        })?;

        if is_exit {
            self.set_n_exits(self.n_exits() + 1);
            self.set_last_exit_dt(Some(now_dt));
        } else {
            self.set_n_entries(self.n_entries() + 1);
            self.set_last_entry_dt(Some(now_dt));
        }

        if new_n_inside > self.max_n_inside() {
            self.set_max_n_inside(new_n_inside);
        }

        let (index, prev_value) = imp.list.borrow_mut().insert_full(now_dt, item);
        debug_assert_eq!(prev_value, None);

        self.entity_list().insert(entity);
        if let Some(stock) = stock {
            self.stock_list().insert(stock);
        }

        self.notify_n_inside();
        self.items_changed(index as u32, 0, 1);

        debug_assert!(imp.list.borrow().keys().is_sorted());

        Ok(())
    }

    pub fn reset(&self) -> Result<()> {
        let imp = self.imp();

        let prev_len = imp.list.borrow().len();

        if prev_len == 0 {
            debug_assert_eq!(self.n_inside(), 0);
            debug_assert_eq!(self.max_n_inside(), 0);
            debug_assert_eq!(self.n_entries(), 0);
            debug_assert_eq!(self.n_exits(), 0);
            debug_assert_eq!(self.last_entry_dt(), None);
            debug_assert_eq!(self.last_exit_dt(), None);
            debug_assert_eq!(self.entity_list().len(), 0);
            debug_assert_eq!(self.stock_list().len(), 0);

            if cfg!(debug_assertions) {
                let (env, tdb, edb, sdb) = self.db();
                env.with_read_txn(|rtxn| {
                    let tdb_n_items = tdb.len(rtxn).context("Failed to get timeline db len")?;
                    debug_assert_eq!(tdb_n_items, 0);
                    let edb_n_items = edb.len(rtxn).context("Failed to get entities db len")?;
                    debug_assert_eq!(edb_n_items, 0);
                    let sdb_n_items = sdb.len(rtxn).context("Failed to get stocks db len")?;
                    debug_assert_eq!(sdb_n_items, 0);
                    Ok(())
                })?;
            }

            return Ok(());
        }

        let (env, tdb, edb, sdb) = self.db();
        env.with_write_txn(|wtxn| {
            tdb.clear(wtxn).context("Failed to clear timeline db")?;
            edb.clear(wtxn).context("Failed to clear entities db")?;
            sdb.clear(wtxn).context("Failed to clear stocks db")?;
            Ok(())
        })?;

        imp.list.borrow_mut().clear();

        self.set_max_n_inside(0);
        self.set_n_entries(0);
        self.set_n_exits(0);
        self.set_last_entry_dt(None);
        self.set_last_exit_dt(None);

        self.entity_list().clear();
        self.stock_list().clear();

        self.notify_n_inside();
        self.items_changed(0, prev_len as u32, 0);

        debug_assert!(imp.list.borrow().keys().is_sorted());

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.imp().list.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.imp().list.borrow().is_empty()
    }

    pub fn first(&self) -> Option<TimelineItem> {
        self.imp()
            .list
            .borrow()
            .first()
            .map(|(_, item)| item.clone())
    }

    pub fn last(&self) -> Option<TimelineItem> {
        self.imp()
            .list
            .borrow()
            .last()
            .map(|(_, item)| item.clone())
    }

    pub fn iter(&self) -> impl Iterator<Item = TimelineItem> + '_ {
        ListModelExtManual::iter(self).map(|item| item.unwrap())
    }

    fn set_max_n_inside(&self, max_n_inside: u32) {
        let imp = self.imp();

        if max_n_inside == self.max_n_inside() {
            return;
        }

        imp.max_n_inside.set(max_n_inside);
        self.notify_max_n_inside();
    }

    fn set_n_entries(&self, n_entries: u32) {
        let imp = self.imp();

        if n_entries == self.n_entries() {
            return;
        }

        imp.n_entries.set(n_entries);
        self.notify_n_entries();
    }

    fn set_n_exits(&self, n_exits: u32) {
        let imp = self.imp();

        if n_exits == self.n_exits() {
            return;
        }

        imp.n_exits.set(n_exits);
        self.notify_n_exits();
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

    fn db(
        &self,
    ) -> &(
        heed::Env,
        db::TimelineDbType,
        db::EntitiesDbType,
        db::StocksDbType,
    ) {
        self.imp().db.get().unwrap()
    }
}
