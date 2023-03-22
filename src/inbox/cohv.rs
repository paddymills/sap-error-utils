
use crate::api::{Order, OrderData};

#[derive(Debug)]
pub struct Header {
    _type: usize,

    order: usize,
    mark:  usize,
    qty:   usize,
    wbs:   usize,
    plant: usize,
}

impl Default for Header {
    fn default() -> Self {
        // we are going to assume no index will be at 255
        //  so then we can find an unmatched index
        Self {
            _type:  usize::MAX,

            order:  usize::MAX,
            mark:   usize::MAX,
            qty:    usize::MAX,
            wbs:    usize::MAX,
            plant:  usize::MAX,
        }
    }
}

impl Header {
    pub fn parse_row(&self, row: String) -> Order {
        let split_row: Vec<&str> = row.split("|").map(|c| c.trim()).collect();

        let data = OrderData {
            id:    split_row[self.order].parse().unwrap(),
            mark:  split_row[self.mark].into(),
            qty:   split_row[self.qty].parse().unwrap(),
            wbs:   split_row[self.wbs].try_into().unwrap(),
            plant: split_row[self.plant].into(),
        };

        match split_row[self._type] {
            "PP01" => Order::ProductionOrder(data),
            "PR"   => Order::PlannedOrder(data),
            _ => unreachable!()
        }
    }
}

impl TryFrom<String> for Header {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut head = Header::default();

        for (i, item) in value.split("|").enumerate() {
            match item.trim() {
                "Order Type"  => head._type = i,
                "Order"       => head.order = i,
                "Material"    => head.mark  = i,
                "Target qty"  => head.qty   = i,
                "WBS Element" => head.wbs   = i,
                "Plant"       => head.plant = i,

                _ => ()
            }
        }

        // validate that all columns matched
        let mut missing_columns = Vec::new();
        if head._type == usize::MAX { missing_columns.push("`Order Type`" ); }
        if head.order == usize::MAX { missing_columns.push("`Order`"      ); }
        if head.mark  == usize::MAX { missing_columns.push("`Material`"   ); }
        if head.qty   == usize::MAX { missing_columns.push("`Target Qty`" ); }
        if head.wbs   == usize::MAX { missing_columns.push("`WBS Element`"); }
        if head.plant == usize::MAX { missing_columns.push("`Plant`"      ); }
        if missing_columns.len() > 0 {
            return Err(format!("Failed to parse header: missing columns {:?}", missing_columns));
        }

        Ok(head)
    }
}
