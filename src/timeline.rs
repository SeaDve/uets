use std::{collections::HashMap, time::Instant};

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::IndexMap;

use crate::{
    date_time_boxed::DateTimeBoxed,
    date_time_range::DateTimeRange,
    db::{self, EnvExt},
    entity::Entity,
    entity_data::EntityData,
    entity_id::EntityId,
    entity_list::EntityList,
    log::Log,
    stock::{Stock, StockLogs},
    stock_id::StockId,
    stock_list::StockList,
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
        /// Number of entities inside at the current time.
        #[property(get = Self::n_inside)]
        pub(super) n_inside: PhantomData<u32>,
        /// Maximum number of entities inside at all time.
        #[property(get = Self::max_n_inside)]
        pub(super) max_n_inside: PhantomData<u32>,
        /// Number of entries at all time.
        #[property(get = Self::n_entries)]
        pub(super) n_entries: PhantomData<u32>,
        /// Number of exits at all time.
        #[property(get = Self::n_exits)]
        pub(super) n_exits: PhantomData<u32>,
        /// Last entry time.
        #[property(get)]
        pub(super) last_entry_dt: Cell<Option<DateTimeBoxed>>,
        /// Last exit time.
        #[property(get)]
        pub(super) last_exit_dt: Cell<Option<DateTimeBoxed>>,

        pub(super) list: RefCell<IndexMap<DateTime<Utc>, TimelineItem>>,
        pub(super) db: OnceCell<(
            heed::Env,
            db::TimelineDbType,
            db::EntitiesDbType,
            db::StocksDbType,
        )>,

        pub(super) entity_list: OnceCell<EntityList>,
        pub(super) stock_list: OnceCell<StockList>,

        pub(super) n_inside_log: RefCell<Log<u32>>,
        pub(super) max_n_inside_log: RefCell<Log<u32>>,
        pub(super) n_entries_log: RefCell<Log<u32>>,
        pub(super) n_exits_log: RefCell<Log<u32>>,
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
            self.n_inside_log.borrow().latest().copied().unwrap_or(0)
        }

        fn max_n_inside(&self) -> u32 {
            self.max_n_inside_log
                .borrow()
                .latest()
                .copied()
                .unwrap_or(0)
        }

        fn n_entries(&self) -> u32 {
            self.n_entries_log.borrow().latest().copied().unwrap_or(0)
        }

        fn n_exits(&self) -> u32 {
            self.n_exits_log.borrow().latest().copied().unwrap_or(0)
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
        let start_time = Instant::now();

        let (tdb, items, edb, entities, sdb, stocks) = env.with_write_txn(|wtxn| {
            let tdb: db::TimelineDbType = env.create_database(wtxn, Some(db::TIMELINE_DB_NAME))?;
            let items = tdb
                .iter(wtxn)?
                .map(|res| res.map(|(dt, raw)| (dt, TimelineItem::from_db(dt, raw))))
                .collect::<Result<IndexMap<_, _>, _>>()?;

            let edb: db::EntitiesDbType = env.create_database(wtxn, Some(db::ENTITIES_DB_NAME))?;
            let entities = edb
                .iter(wtxn)?
                .map(|res| {
                    res.map(|(id, raw)| {
                        let entity = Entity::from_db(id.clone(), raw);
                        (id, entity)
                    })
                })
                .collect::<Result<IndexMap<_, _>, _>>()?;

            let sdb: db::StocksDbType = env.create_database(wtxn, Some(db::STOCKS_DB_NAME))?;
            let stocks = sdb
                .iter(wtxn)?
                .map(|res| {
                    res.map(|(id, raw)| {
                        let stock = Stock::from_db(id.clone(), raw);
                        (id, stock)
                    })
                })
                .collect::<Result<IndexMap<_, _>, _>>()?;

            Ok((tdb, items, edb, entities, sdb, stocks))
        })?;

        tracing::debug!(
            "Loaded {} items, entities, and stocks dbs in {:?}",
            items.len(),
            start_time.elapsed()
        );

        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.list.replace(items);
        imp.db.set((env, tdb, edb, sdb)).unwrap();
        imp.entity_list.set(EntityList::from_raw(entities)).unwrap();
        imp.stock_list.set(StockList::from_raw(stocks)).unwrap();

        this.setup_data();

        tracing::debug!("Loaded timeline in {:?}", start_time.elapsed());

        debug_assert!(imp.list.borrow().keys().is_sorted());

        Ok(this)
    }

    pub fn entity_list(&self) -> &EntityList {
        self.imp().entity_list.get().unwrap()
    }

    pub fn stock_list(&self) -> &StockList {
        self.imp().stock_list.get().unwrap()
    }

    pub fn get(&self, dt: &DateTime<Utc>) -> Option<TimelineItem> {
        self.imp().list.borrow().get(dt).cloned()
    }

    pub fn iter<'a>(
        &'a self,
        dt_range: &'a DateTimeRange,
    ) -> impl DoubleEndedIterator<Item = TimelineItem> + '_ {
        ListModelExtManual::iter::<TimelineItem>(self)
            .map(|item| item.unwrap())
            .filter(|item| dt_range.contains(item.dt()))
    }

    pub fn iter_stock<'a>(
        &'a self,
        dt_range: &'a DateTimeRange,
        stock_id: &'a StockId,
    ) -> impl DoubleEndedIterator<Item = TimelineItem> + '_ {
        self.iter(dt_range).filter(|item| {
            let entity = self
                .entity_list()
                .get(item.entity_id())
                .expect("entity must be known");
            entity.stock_id() == Some(stock_id)
        })
    }

    pub fn n_inside_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .n_inside_log
            .borrow()
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn n_inside_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.n_inside_for_dt(end)
        } else {
            self.n_inside()
        }
    }

    pub fn max_n_inside_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .max_n_inside_log
            .borrow()
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn max_n_inside_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.max_n_inside_for_dt(end)
        } else {
            self.max_n_inside()
        }
    }

    pub fn n_entries_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .n_entries_log
            .borrow()
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn n_entries_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.n_entries_for_dt(end)
        } else {
            self.n_entries()
        }
    }

    pub fn n_exits_for_dt(&self, dt: DateTime<Utc>) -> u32 {
        self.imp()
            .n_exits_log
            .borrow()
            .for_dt(dt)
            .copied()
            .unwrap_or(0)
    }

    pub fn n_exits_for_dt_range(&self, dt_range: &DateTimeRange) -> u32 {
        if let Some(end) = dt_range.end {
            self.n_exits_for_dt(end)
        } else {
            self.n_exits()
        }
    }

    pub fn handle_detected(
        &self,
        provided_entity_id: &EntityId,
        entity_data: Option<EntityData>,
    ) -> Result<()> {
        let imp = self.imp();

        let provided_stock_id = entity_data.as_ref().and_then(|d| d.stock_id.as_ref());

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

        let now_dt = Utc::now();
        debug_assert!(imp
            .list
            .borrow()
            .last()
            .map_or(true, |(dt, _)| &now_dt > dt));

        let is_exit = entity.is_inside();

        let item_kind = if is_exit {
            TimelineItemKind::Exit
        } else {
            TimelineItemKind::Entry
        };
        let item = TimelineItem::new(now_dt, item_kind, provided_entity_id.clone());

        // Use entity stock id from entity if no stock id is provided.
        let stock = provided_stock_id
            .or_else(|| entity.stock_id())
            .map(|stock_id| {
                self.stock_list()
                    .get(stock_id)
                    .unwrap_or_else(|| Stock::new(stock_id.clone()))
            });

        let (env, tdb, edb, sdb) = self.db();
        env.with_write_txn(|wtxn| {
            tdb.put(wtxn, &now_dt, &item.to_db())?;
            edb.put(wtxn, entity.id(), &entity.to_db())?;
            if let Some(stock) = &stock {
                sdb.put(wtxn, stock.id(), &stock.to_db())?;
            }
            Ok(())
        })?;

        let prev_n_inside = self.n_inside();
        let new_n_inside = if is_exit {
            prev_n_inside - 1
        } else {
            prev_n_inside + 1
        };
        imp.n_inside_log.borrow_mut().insert(now_dt, new_n_inside);
        self.notify_n_inside();

        if new_n_inside > self.max_n_inside() {
            imp.max_n_inside_log
                .borrow_mut()
                .insert(now_dt, new_n_inside);
            self.notify_max_n_inside();
        }

        if is_exit {
            let new_n_exits = self.n_exits() + 1;
            imp.n_exits_log.borrow_mut().insert(now_dt, new_n_exits);
            self.notify_n_exits();

            self.set_last_exit_dt(Some(DateTimeBoxed(now_dt)));

            let last_entry_dt = entity
                .last_action_dt()
                .expect("entity must already have an entry");
            let entry_item = self.get(&last_entry_dt).expect("entry item must be known");
            entry_item.set_pair(&item);
            item.set_pair(&entry_item);
        } else {
            let new_n_entries = self.n_entries() + 1;
            imp.n_entries_log.borrow_mut().insert(now_dt, new_n_entries);
            self.notify_n_entries();

            self.set_last_entry_dt(Some(DateTimeBoxed(now_dt)));
        }

        entity.with_is_inside_log_mut(|map| {
            map.insert(now_dt, !is_exit);
        });

        if let Some(stock) = &stock {
            let prev_n_inisde = stock.n_inside();
            let new_n_inside = if is_exit {
                prev_n_inisde - 1
            } else {
                prev_n_inisde + 1
            };

            stock.with_logs_mut(|logs| {
                logs.n_inside.insert(now_dt, new_n_inside);

                let prev_max_n_inside = logs.max_n_inside.latest().copied().unwrap_or(0);
                if new_n_inside > prev_max_n_inside {
                    logs.max_n_inside.insert(now_dt, new_n_inside);
                }

                if is_exit {
                    let prev_n_exits = logs.n_exits.latest().copied().unwrap_or(0);
                    logs.n_exits.insert(now_dt, prev_n_exits + 1);
                    logs.last_exit_dt.insert(now_dt, now_dt);
                } else {
                    let prev_n_entries = logs.n_entries.latest().copied().unwrap_or(0);
                    logs.n_entries.insert(now_dt, prev_n_entries + 1);
                    logs.last_entry_dt.insert(now_dt, now_dt);
                }
            });
        }

        let (index, prev_value) = imp.list.borrow_mut().insert_full(now_dt, item);
        debug_assert_eq!(prev_value, None);

        self.entity_list().insert(entity);
        if let Some(stock) = stock {
            self.stock_list().insert(stock);
        }

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
                    let tdb_n_items = tdb.len(rtxn)?;
                    debug_assert_eq!(tdb_n_items, 0);
                    let edb_n_items = edb.len(rtxn)?;
                    debug_assert_eq!(edb_n_items, 0);
                    let sdb_n_items = sdb.len(rtxn)?;
                    debug_assert_eq!(sdb_n_items, 0);
                    Ok(())
                })?;
            }

            return Ok(());
        }

        let (env, tdb, edb, sdb) = self.db();
        env.with_write_txn(|wtxn| {
            tdb.clear(wtxn)?;
            edb.clear(wtxn)?;
            sdb.clear(wtxn)?;
            Ok(())
        })?;

        imp.list.borrow_mut().clear();

        imp.n_inside_log.borrow_mut().clear();
        imp.max_n_inside_log.borrow_mut().clear();
        imp.n_entries_log.borrow_mut().clear();
        imp.n_exits_log.borrow_mut().clear();

        self.set_last_entry_dt(None);
        self.set_last_exit_dt(None);

        self.entity_list().clear();
        self.stock_list().clear();

        self.notify_n_inside();
        self.notify_max_n_inside();
        self.notify_n_entries();
        self.notify_n_exits();
        self.items_changed(0, prev_len as u32, 0);

        debug_assert!(imp.list.borrow().keys().is_sorted());

        Ok(())
    }

    fn set_last_entry_dt(&self, dt: Option<DateTimeBoxed>) {
        let imp = self.imp();

        if dt == self.last_entry_dt() {
            return;
        }

        imp.last_entry_dt.replace(dt);
        self.notify_last_entry_dt();
    }

    fn set_last_exit_dt(&self, dt: Option<DateTimeBoxed>) {
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

    fn setup_data(&self) {
        let imp = self.imp();

        let mut n_inside = 0;
        let mut max_n_inside = 0;
        let mut n_entries = 0;
        let mut n_exits = 0;

        let mut n_inside_log = Log::<u32>::new();
        let mut max_n_inside_log = Log::<u32>::new();
        let mut n_entries_log = Log::<u32>::new();
        let mut n_exits_log = Log::<u32>::new();

        let mut entity_is_inside_logs = HashMap::<EntityId, Log<bool>>::new();
        let mut stock_logs: HashMap<StockId, StockLogs> = HashMap::new();

        for item in imp.list.borrow().values() {
            let entity = self
                .entity_list()
                .get(item.entity_id())
                .expect("entity must be known");

            if item.kind().is_exit() {
                n_inside -= 1;
                n_exits += 1;

                n_exits_log.insert(item.dt(), n_exits);

                let last_entry_dt = entity_is_inside_logs
                    .get(item.entity_id())
                    .expect("entity must be known")
                    .latest_dt()
                    .expect("entity must already have an entry");
                let entry_item = self.get(&last_entry_dt).expect("entry item must be known");
                entry_item.set_pair(item);
                item.set_pair(&entry_item);
            } else {
                n_inside += 1;
                n_entries += 1;

                n_entries_log.insert(item.dt(), n_entries);
            }

            n_inside_log.insert(item.dt(), n_inside);

            if n_inside > max_n_inside {
                max_n_inside = n_inside;
                max_n_inside_log.insert(item.dt(), max_n_inside);
            }

            entity_is_inside_logs
                .entry(item.entity_id().clone())
                .or_default()
                .insert(item.dt(), item.kind().is_entry());

            if let Some(stock_id) = entity.stock_id() {
                let logs = stock_logs.entry(stock_id.clone()).or_default();

                let prev_n_inside = logs.n_inside.latest().copied().unwrap_or(0);
                let new_n_inside = if item.kind().is_exit() {
                    prev_n_inside - 1
                } else {
                    prev_n_inside + 1
                };
                logs.n_inside.insert(item.dt(), new_n_inside);

                let prev_max_n_inside = logs.max_n_inside.latest().copied().unwrap_or(0);
                if new_n_inside > prev_max_n_inside {
                    logs.max_n_inside.insert(item.dt(), new_n_inside);
                }

                if item.kind().is_exit() {
                    let prev_n_exits = logs.n_exits.latest().copied().unwrap_or(0);
                    logs.n_exits.insert(item.dt(), prev_n_exits + 1);
                    logs.last_exit_dt.insert(item.dt(), item.dt());
                } else {
                    let prev_n_entries = logs.n_entries.latest().copied().unwrap_or(0);
                    logs.n_entries.insert(item.dt(), prev_n_entries + 1);
                    logs.last_entry_dt.insert(item.dt(), item.dt());
                }
            }
        }

        let mut last_entry_dt = None;
        for item in imp.list.borrow().values().rev() {
            if item.kind().is_entry() {
                last_entry_dt = Some(item.dt());
                break;
            }
        }

        let mut last_exit_dt = None;
        for item in imp.list.borrow().values().rev() {
            if item.kind().is_exit() {
                last_exit_dt = Some(item.dt());
                break;
            }
        }

        imp.n_inside_log.replace(n_inside_log);
        imp.max_n_inside_log.replace(max_n_inside_log);
        imp.n_entries_log.replace(n_entries_log);
        imp.n_exits_log.replace(n_exits_log);
        self.notify_n_inside();
        self.notify_max_n_inside();
        self.notify_n_entries();
        self.notify_n_exits();

        self.set_last_entry_dt(last_entry_dt.map(DateTimeBoxed));
        self.set_last_exit_dt(last_exit_dt.map(DateTimeBoxed));

        for (entity_id, log) in entity_is_inside_logs {
            let entity = self.entity_list().get(&entity_id).unwrap();
            entity.with_is_inside_log_mut(|l| {
                *l = log;
            });
        }

        for (stock_id, logs) in stock_logs {
            let stock = self.stock_list().get(&stock_id).unwrap();
            stock.with_logs_mut(|l| {
                *l = logs;
            });
        }

        debug_assert_eq!(
            self.n_entries(),
            imp.list
                .borrow()
                .values()
                .filter(|item| item.kind().is_entry())
                .count() as u32
        );
        debug_assert_eq!(
            self.n_exits(),
            imp.list
                .borrow()
                .values()
                .filter(|item| item.kind().is_exit())
                .count() as u32
        );
    }
}
