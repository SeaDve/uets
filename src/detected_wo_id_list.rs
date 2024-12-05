use std::time::Instant;

use anyhow::Result;
use chrono::{DateTime, Utc};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use indexmap::{map::Entry, IndexMap};

use crate::{
    db::{self, EnvExt},
    detected_wo_id_item::DetectedWoIdItem,
};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct DetectedWoIdList {
        pub(super) list: RefCell<IndexMap<DateTime<Utc>, DetectedWoIdItem>>,

        pub(super) db: OnceCell<(heed::Env, db::DetectedWoIdDbType)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetectedWoIdList {
        const NAME: &'static str = "UetsDetectedWoIdList";
        type Type = super::DetectedWoIdList;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for DetectedWoIdList {}

    impl ListModelImpl for DetectedWoIdList {
        fn item_type(&self) -> glib::Type {
            DetectedWoIdItem::static_type()
        }

        fn n_items(&self) -> u32 {
            self.list.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.list
                .borrow()
                .get_index(position as usize)
                .map(|(_, v)| v.upcast_ref::<glib::Object>())
                .cloned()
        }
    }
}

glib::wrapper! {
    pub struct DetectedWoIdList(ObjectSubclass<imp::DetectedWoIdList>)
        @implements gio::ListModel;
}

impl DetectedWoIdList {
    pub fn load_from_env(env: heed::Env) -> Result<Self> {
        let start_time = Instant::now();

        let (db, items) = env.with_write_txn(|wtxn| {
            let db: db::DetectedWoIdDbType =
                env.create_database(wtxn, Some(db::DETECTED_WO_ID_DB_NAME))?;
            let items = db
                .iter(wtxn)?
                .map(|res| res.map(|(dt, raw)| (dt, DetectedWoIdItem::from_db(dt, raw))))
                .collect::<Result<IndexMap<_, _>, _>>()?;
            Ok((db, items))
        })?;

        tracing::debug!("Loaded {} items in {:?}", items.len(), start_time.elapsed());

        let this = glib::Object::new::<Self>();

        let imp = this.imp();
        imp.list.replace(items);
        imp.db.set((env, db)).unwrap();

        Ok(this)
    }

    pub fn insert(&self, item: DetectedWoIdItem) -> Result<()> {
        let imp = self.imp();

        let (env, db) = self.db();
        env.with_write_txn(|wtxn| {
            db.put(wtxn, &item.dt(), &item.to_db())?;
            Ok(())
        })?;

        let (index, removed, added) = match imp.list.borrow_mut().entry(item.dt()) {
            Entry::Occupied(entry) => (entry.index(), 1, 1),
            Entry::Vacant(entry) => {
                let index = entry.index();
                entry.insert(item);
                (index, 0, 1)
            }
        };

        self.items_changed(index as u32, removed, added);

        Ok(())
    }

    pub fn remove(&self, dt: &DateTime<Utc>) -> Result<()> {
        let imp = self.imp();

        let (env, db) = self.db();
        env.with_write_txn(|wtxn| {
            db.delete(wtxn, dt)?;
            Ok(())
        })?;

        let entry = imp.list.borrow_mut().shift_remove_full(dt);
        if let Some((index, _, _)) = entry {
            self.items_changed(index as u32, 1, 0);
        }

        Ok(())
    }

    fn db(&self) -> &(heed::Env, db::DetectedWoIdDbType) {
        self.imp().db.get().unwrap()
    }
}
