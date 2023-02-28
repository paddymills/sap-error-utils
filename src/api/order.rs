
use super::Wbs;

#[derive(Debug)]
pub enum Order {
    PlannedOrder(OrderData),
    ProductionOrder(OrderData)
}

#[derive(Debug, Clone)]
pub struct OrderData {
    pub id: u32,
    pub mark: String,
    pub qty: u32,
    pub wbs: Wbs,
}

impl OrderData {
    pub fn apply_qty(&mut self, qty: u32) {
        if self.qty < qty {
            panic!("Cannot apply qty({}) greater than PlannedOrder({})", qty, self.qty);
        }

        self.qty -= qty;
    }
}
