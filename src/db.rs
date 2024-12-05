use std::{
    fs,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use gtk::glib;
use heed::types::SerdeJson;
use serde::{Deserialize, Serialize};

use crate::{
    entity_data::EntityData, entity_id::EntityId, jpeg_image::JpegImage, stock_data::StockData,
    stock_id::StockId, APP_ID,
};

const N_NAMED_DBS: u32 = 4;

pub type TimelineDbType = heed::Database<SerdeJson<DateTime<Utc>>, SerdeJson<RawTimelineItem>>;
pub const TIMELINE_DB_NAME: &str = "timeline";

pub type EntitiesDbType = heed::Database<SerdeJson<EntityId>, SerdeJson<EntityData>>;
pub const ENTITIES_DB_NAME: &str = "entities";

pub type StocksDbType = heed::Database<SerdeJson<StockId>, SerdeJson<StockData>>;
pub const STOCKS_DB_NAME: &str = "stocks";

pub type DetectedWoIdDbType =
    heed::Database<SerdeJson<DateTime<Utc>>, SerdeJson<RawDetectedWoIdItem>>;
pub const DETECTED_WO_ID_DB_NAME: &str = "detected_wo_id";

#[derive(Debug, Serialize, Deserialize)]
pub struct RawTimelineItem {
    pub is_entry: bool,
    pub entity_id: EntityId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawDetectedWoIdItem {
    pub image: Option<JpegImage>,
}

pub fn new_env() -> Result<heed::Env> {
    let path = glib::user_data_dir().join(format!("{}/db", APP_ID));
    fs::create_dir_all(&path)
        .with_context(|| format!("Failed to create db dir at {}", path.display()))?;

    let env = unsafe {
        heed::EnvOpenOptions::new()
            .max_dbs(N_NAMED_DBS)
            .open(&path)
            .with_context(|| format!("Failed to open heed env at {}", path.display()))?
    };

    tracing::debug!(
        ?path,
        info = ?env.info(),
        real_disk_size = ?env.real_disk_size(),
        non_free_pages_size = ?env.non_free_pages_size(),
        "Opened db env"
    );

    Ok(env)
}

pub trait EnvExt {
    /// Run a func with a write txn and commit it.
    fn with_write_txn<T>(&self, func: impl FnOnce(&mut heed::RwTxn<'_>) -> Result<T>) -> Result<T>;

    /// Run a func with a read txn.
    fn with_read_txn<T>(&self, func: impl FnOnce(&heed::RoTxn<'_>) -> Result<T>) -> Result<T>;
}

impl EnvExt for heed::Env {
    fn with_write_txn<T>(&self, func: impl FnOnce(&mut heed::RwTxn<'_>) -> Result<T>) -> Result<T> {
        let start_time = Instant::now();

        let mut wtxn = self.write_txn().context("Failed to create write txn")?;
        let ret = func(&mut wtxn)?;
        wtxn.commit().context("Failed to commit write txn")?;

        // There are 16.67 ms in a 60 Hz frame, so warn if the write txn
        // takes longer than that.
        if start_time.elapsed() > Duration::from_millis(15) {
            tracing::warn!("Database write txn took {:?}", start_time.elapsed());
        } else {
            tracing::trace!("Database write txn took {:?}", start_time.elapsed());
        }

        Ok(ret)
    }

    fn with_read_txn<T>(&self, func: impl FnOnce(&heed::RoTxn<'_>) -> Result<T>) -> Result<T> {
        let start_time = Instant::now();

        let rtxn = self.read_txn().context("Failed to create read txn")?;
        let ret = func(&rtxn)?;
        drop(rtxn);

        // There are 16.67 ms in a 60 Hz frame, so warn if the read txn
        // takes longer than that.
        if start_time.elapsed() > Duration::from_millis(15) {
            tracing::warn!("Database read txn took {:?}", start_time.elapsed());
        } else {
            tracing::trace!("Database read txn took {:?}", start_time.elapsed());
        }

        Ok(ret)
    }
}
