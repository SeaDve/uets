use std::{collections::HashMap, io::Cursor};

use anyhow::{Context, Result};
use calamine::{Data, DataType, Reader};

use crate::{
    entity::Entity,
    entity_data::{EntityData, EntityDataField, EntityDataFieldTy},
    entity_id::EntityId,
    stock_id::StockId,
    timeline::Timeline,
};

impl Timeline {
    pub fn insert_entities_from_workbook_bytes(&self, workbook_bytes: &[u8]) -> Result<()> {
        let mut book = calamine::open_workbook_auto_from_rs(Cursor::new(workbook_bytes))?;
        let range = book.worksheet_range_at(0).context("Empty sheets")??;

        let mut rows = range.rows();

        let col_title_row = rows.next().context("No column title")?;
        let entity_id_col_idx = find_position(col_title_row, |s| {
            matches!(s.to_lowercase().as_str(), "entity" | "entity id")
        })
        .context("Missing entity id col")?;

        let col_idxs = EntityDataFieldTy::all()
            .iter()
            .filter_map(|field_ty| {
                let col_idx = match field_ty {
                    EntityDataFieldTy::StockId => find_position(col_title_row, |s| {
                        matches!(
                            s.to_lowercase().as_str(),
                            "stock" | "stock id" | "stock name"
                        )
                    }),
                    EntityDataFieldTy::Location => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("location")
                    }),
                    EntityDataFieldTy::ExpirationDt => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("expiration")
                    }),
                    EntityDataFieldTy::Name => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("name")
                    }),
                    EntityDataFieldTy::Sex => {
                        find_position(col_title_row, |s| s.to_lowercase().as_str().contains("sex"))
                    }
                    EntityDataFieldTy::Email => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("email")
                    }),
                    EntityDataFieldTy::Program => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("program")
                    }),
                };

                col_idx.map(|col_idx| (field_ty, col_idx))
            })
            .collect::<HashMap<_, _>>();

        let mut entities = Vec::new();
        for row in rows {
            let raw_entity_id = row[entity_id_col_idx]
                .as_string()
                .context("Entity id doesn't contain string")?;
            let entity_id = EntityId::new(raw_entity_id);

            let fields = col_idxs
                .iter()
                .filter_map(|(field_ty, &idx)| match field_ty {
                    EntityDataFieldTy::StockId => row[idx]
                        .as_string()
                        .map(StockId::new)
                        .map(EntityDataField::StockId),
                    EntityDataFieldTy::Location => {
                        row[idx].as_string().map(EntityDataField::Location)
                    }
                    EntityDataFieldTy::ExpirationDt => {
                        row[idx].as_string().map(EntityDataField::ExpirationDt)
                    }
                    EntityDataFieldTy::Name => row[idx].as_string().map(EntityDataField::Name),
                    EntityDataFieldTy::Sex => row[idx].as_string().map(EntityDataField::Sex),
                    EntityDataFieldTy::Email => row[idx].as_string().map(EntityDataField::Email),
                    EntityDataFieldTy::Program => {
                        row[idx].as_string().map(EntityDataField::Program)
                    }
                });

            entities.push(Entity::new(entity_id, EntityData::from_fields(fields)));
        }

        self.insert_entities(entities)?;

        Ok(())
    }
}

fn find_position(col_title_row: &[Data], predicate: impl Fn(&str) -> bool) -> Option<usize> {
    col_title_row
        .iter()
        .position(|cell| cell.as_string().is_some_and(|s| predicate(&s)))
}
