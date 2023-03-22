
use super::{Wbs, Plant};

#[derive(Debug)]
pub enum Order {
    PlannedOrder(OrderData),
    ProductionOrder(OrderData)
}

impl Order {
    pub fn new(order_type: &str, data: OrderData) -> Self {
        match order_type {
            "PP01" => Order::ProductionOrder(data),
            "PR"   => Order::PlannedOrder(data),
            _ => unreachable!()
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderData {
    pub id: u32,
    pub mark: String,
    pub qty: u32,
    pub wbs: Wbs,
    pub plant: Plant
}

impl OrderData {
    pub fn apply_qty(&mut self, qty: u32) {
        if self.qty < qty {
            panic!("Cannot apply qty({}) greater than PlannedOrder({})", qty, self.qty);
        }

        self.qty -= qty;
    }
}
