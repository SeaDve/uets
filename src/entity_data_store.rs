use std::{cell::RefCell, collections::HashMap, io::Cursor};

use anyhow::{Context, Result};
use calamine::{DataType, Reader};

use crate::{entity_data::EntityData, entity_id::EntityId, stock_id::StockId};

#[derive(Default)]
pub struct EntityDataStore {
    map: RefCell<HashMap<EntityId, EntityData>>,
}

impl EntityDataStore {
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

        let mut n_added = 0;
        for row in rows {
            let raw_entity_id = row[entity_id_col_idx]
                .as_string()
                .context("Entity id doesn't contain string")?;
            let entity_id = EntityId::new(raw_entity_id);
            let stock_id = stock_id_col_idx
                .and_then(|idx| row[idx].as_string())
                .map(StockId::new);

            self.map
                .borrow_mut()
                .insert(entity_id, EntityData { stock_id });
            n_added += 1;
        }

        tracing::debug!("Registered `{}` new entity data", n_added);

        Ok(())
    }

    pub fn retrieve(&self, entity_id: &EntityId) -> Option<EntityData> {
        self.map.borrow().get(entity_id).cloned()
    }
}
