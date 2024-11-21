use crate::stock_id::StockId;

#[derive(Debug, Default)]
pub struct EntityData {
    pub stock_id: Option<StockId>,
}
