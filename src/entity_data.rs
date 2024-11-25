use crate::stock_id::StockId;

#[derive(Debug, Clone)]
pub struct EntityData {
    pub stock_id: Option<StockId>,
}
