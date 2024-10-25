use std::{
    fs,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use gtk::glib;
use heed::types::SerdeJson;
use serde::{Deserialize, Serialize};

use crate::{date_time::DateTime, entity_id::EntityIdCodec, APP_ID};

const N_NAMED_DBS: u32 = 1;

pub type EntitiesDbType = heed::Database<EntityIdCodec, SerdeJson<RawEntity>>;
pub const ENTITIES_DB_NAME: &str = "entities";

#[derive(Debug, Serialize, Deserialize)]
pub struct RawEntity {
    pub entry_dts: Vec<DateTime>,
    pub exit_dts: Vec<DateTime>,
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
}
