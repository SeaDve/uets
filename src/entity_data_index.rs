use std::{cell::RefCell, collections::HashMap, io::Cursor, time::Instant};

use anyhow::{Context, Result};
use calamine::{DataType, Reader};

use crate::{
    db::{self, EnvExt},
    entity_data::EntityData,
    entity_id::EntityId,
    stock_id::StockId,
};

#[derive(Debug)]
pub struct EntityDataIndex {
    env: heed::Env,
    db: db::EntityDataIndexDbType,
    map: RefCell<HashMap<EntityId, EntityData>>,
}

impl EntityDataIndex {
    pub fn load_from_env(env: heed::Env) -> Result<Self> {
        let start_time = Instant::now();

        let (db, data) = env.with_write_txn(|wtxn| {
            let db: db::EntityDataIndexDbType =
                env.create_database(wtxn, Some(db::ENTITY_DATA_INDEX_DB_NAME))?;
            let data = db.iter(wtxn)?.collect::<Result<HashMap<_, _>, _>>()?;

            Ok((db, data))
        })?;

        tracing::debug!("Loaded entity data index in {:?}", start_time.elapsed());

        Ok(Self {
            env: env.clone(),
            db,
            map: RefCell::new(data),
        })
    }

    pub fn register(&self, bytes: &[u8]) -> Result<()> {
        let mut book = calamine::open_workbook_auto_from_rs(Cursor::new(bytes))?;
        let range = book.worksheet_range_at(0).context("Empty sheets")??;

        let mut rows = range.rows();

        let col_title_row = rows.next().context("No column title")?;
        let entity_id_col_idx = col_title_row
            .iter()
            .position(|cell| {
                cell.as_string()
                    .is_some_and(|s| matches!(s.to_lowercase().as_str(), "entity" | "entity id"))
            })
            .context("Missing entity id col")?;
        let stock_id_col_idx = col_title_row.iter().position(|cell| {
            cell.as_string().is_some_and(|s| {
                matches!(
                    s.to_lowercase().as_str(),
                    "stock" | "stock id" | "stock name"
                )
            })
        });

        let mut entity_data = Vec::new();
        for row in rows {
            let raw_entity_id = row[entity_id_col_idx]
                .as_string()
                .context("Entity id doesn't contain string")?;
            let entity_id = EntityId::new(raw_entity_id);
            let stock_id = stock_id_col_idx
                .and_then(|idx| row[idx].as_string())
                .map(StockId::new);
            entity_data.push((entity_id, EntityData { stock_id }));
        }

        self.env.with_write_txn(|wtxn| {
            for (entity_id, entity_data) in &entity_data {
                self.db.put(wtxn, entity_id, entity_data)?;
            }
            Ok(())
        })?;

        let n_added = entity_data.len();

        for (entity_id, entity_data) in entity_data {
            self.map.borrow_mut().insert(entity_id, entity_data);
        }

        tracing::debug!("Registered `{}` entity data", n_added);

        Ok(())
    }

    pub fn retrieve(&self, entity_id: &EntityId) -> Option<EntityData> {
        self.map.borrow().get(entity_id).cloned()
    }

    pub fn retrieve_stock_ids(&self) -> Vec<StockId> {
        self.map
            .borrow()
            .values()
            .filter_map(|data| data.stock_id.clone())
            .collect()
    }
}
