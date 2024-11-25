use serde::{Deserialize, Serialize};

use crate::stock_id::StockId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub stock_id: Option<StockId>,
}
