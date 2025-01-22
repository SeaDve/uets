use std::{collections::HashMap, io::Cursor};

use anyhow::{Context, Result};
use calamine::{Data, DataType, Reader};

use crate::{
    date_time,
    date_time_range::DateTimeRange,
    entity_data::{EntityData, EntityDataField, EntityDataFieldTy},
    entity_id::EntityId,
    jpeg_image::JpegImage,
    sex::Sex,
    stock_data::StockData,
    stock_id::StockId,
    timeline::Timeline,
};

impl Timeline {
    pub fn register_data_from_workbook_bytes(&self, workbook_bytes: &[u8]) -> Result<()> {
        let mut book = calamine::open_workbook_auto_from_rs(Cursor::new(workbook_bytes))?;
        let range = book.worksheet_range_at(0).context("Empty sheets")??;

        let mut rows = range.rows();

        let col_title_row = rows.next().context("No column title")?;

        let entity_id_col_idx = find_position(col_title_row, |s| {
            matches!(s.to_lowercase().as_str(), "entity" | "entity id" | "id")
        });
        let col_idxs = EntityDataFieldTy::all()
            .iter()
            .filter_map(|field_ty| {
                let col_idx = match field_ty {
                    EntityDataFieldTy::StockId => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("stock")
                    }),
                    EntityDataFieldTy::Location => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("location")
                    }),
                    EntityDataFieldTy::ExpirationDt => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("expiration")
                    }),
                    EntityDataFieldTy::AllowedDtRange => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("date range")
                    }),
                    EntityDataFieldTy::Photo => find_position(col_title_row, |s| {
                        s.to_lowercase().as_str().contains("photo")
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
        let stock_id_col_idx = col_idxs.get(&EntityDataFieldTy::StockId).copied();

        let mut entity_data = HashMap::new();
        let mut stock_data = HashMap::new();
        for row in rows {
            if let Some(entity_id_col_idx) = entity_id_col_idx {
                let Some(entity_id) = row[entity_id_col_idx].as_string().map(EntityId::new) else {
                    continue;
                };

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
                        EntityDataFieldTy::ExpirationDt => row[idx]
                            .as_string()
                            .and_then(|s| {
                                date_time::parse(&s)
                                    .inspect_err(|err| {
                                        tracing::warn!("Failed to parse date time: {:?}", err)
                                    })
                                    .ok()
                            })
                            .map(EntityDataField::ExpirationDt),
                        EntityDataFieldTy::AllowedDtRange => row[idx]
                            .as_string()
                            .and_then(|s| {
                                s.parse::<DateTimeRange>()
                                    .inspect_err(|err| {
                                        tracing::warn!("Failed to parse date time range: {:?}", err)
                                    })
                                    .ok()
                            })
                            .map(EntityDataField::AllowedDtRange),
                        EntityDataFieldTy::Photo => row[idx]
                            .as_string()
                            .map(|s| JpegImage::from_base64(&s))
                            .map(EntityDataField::Photo),
                        EntityDataFieldTy::Name => row[idx].as_string().map(EntityDataField::Name),
                        EntityDataFieldTy::Sex => row[idx]
                            .as_string()
                            .and_then(|s| {
                                s.parse::<Sex>()
                                    .inspect_err(|err| {
                                        tracing::warn!("Failed to parse sex: {:?}", err)
                                    })
                                    .ok()
                            })
                            .map(EntityDataField::Sex),
                        EntityDataFieldTy::Email => {
                            row[idx].as_string().map(EntityDataField::Email)
                        }
                        EntityDataFieldTy::Program => {
                            row[idx].as_string().map(EntityDataField::Program)
                        }
                    });

                entity_data.insert(entity_id, EntityData::from_fields(fields));
            } else if let Some(stock_id_col_idx) = stock_id_col_idx {
                let Some(stock_id) = row[stock_id_col_idx].as_string().map(StockId::new) else {
                    continue;
                };

                stock_data.insert(stock_id, StockData {});
            }
        }

        self.register_entity_data(entity_data)?;
        self.register_stock_data(stock_data)?;

        Ok(())
    }
}

fn find_position(col_title_row: &[Data], predicate: impl Fn(&str) -> bool) -> Option<usize> {
    col_title_row
        .iter()
        .position(|cell| cell.as_string().is_some_and(|s| predicate(&s)))
}
