
use std::{cmp::Ordering, hash::{Hash, Hasher}};

use regex::Regex;

use crate::api::{CnfFileRow, Wbs, Order, OrderData};

lazy_static! {
    static ref INBOX_TEXT: Regex = Regex::new(r"Planned order not found for (\d{7}[a-zA-Z]-[\w-]+), (D-\d{7}-\d{5}), ([\d,]+).000, Sigmanest Program:([\d-]+)")
        .expect("Failed to build INBOX_TEXT regex");
}

pub enum FailureMatchStatus {
    MatchComplete,
    NoConfirmationRow,
    NotEnoughOrdersApplied(u32),
}


#[derive(Debug)]
pub struct Failure {
    pub mark: String,
    pub wbs: Wbs,
    pub qty: u32,
    pub program: String,

    cnf_row: Option<CnfFileRow>,
    pub applied: Vec<OrderData>,
}

impl Failure {
    pub fn apply_order(&mut self, order: Order) -> Option<Order> {
        // TODO: return order if qty not applied

        match order {
            Order::PlannedOrder(order_data) => {
                self.apply_order_unchecked(order_data).map(|d| Order::PlannedOrder(d))
            },
            Order::ProductionOrder(_) => panic!("cannot apply a production order to a failure")
        }
    }

    /// Applies order to Failure without checking that it is a PlannedOrder
    pub fn apply_order_unchecked(&mut self, mut order_data: OrderData) -> Option<OrderData> {
        let failure_qty = self.qty();

        if failure_qty == 0 {
            return Some( order_data );
        }

        match order_data.qty {
            x if x <= failure_qty => {
                self.applied.push(order_data);

                None
            },
            _ => {
                let mut not_applied = order_data.clone();
                not_applied.qty -= failure_qty;

                order_data.qty = failure_qty;
                self.applied.push(order_data);

                Some( not_applied )
            }
        }
    }

    /// Returns the qty left to be applied
    pub fn qty(&self) -> u32 {
        let applied = self.applied
            .iter()
            .fold(0, |acc, elem| acc + elem.qty);

        self.qty - applied
    }

    pub fn set_confirmation_row_data(&mut self, row: CnfFileRow) {
        self.cnf_row = Some(row);
    }

    pub fn has_confirmation_row(&self) -> bool {
        self.cnf_row.is_some()
    }

    pub fn generate_output(&self) -> Result<Vec<CnfFileRow>, String> {
        match &self.cnf_row {
            Some(row) => {
                let mut result = Vec::new();
                
                for appl in &self.applied {
                    result.push(row.modify_with(appl));
                }
        
                Ok(result)
            },
            None => Err(format!("No CnfFileRow matched for {}", self.mark))
        }
    }

    pub fn status(&self) -> FailureMatchStatus {
        if let None = self.cnf_row {
            return FailureMatchStatus::NoConfirmationRow;
        }

        let diff = self.qty();
        if diff > 0 {
            return FailureMatchStatus::NotEnoughOrdersApplied(diff);
        }

        FailureMatchStatus::MatchComplete
    }

    pub fn new_inbox_text(&self) -> Option<String> {
        let qty = match self.status() {
            FailureMatchStatus::MatchComplete => 0,
            FailureMatchStatus::NoConfirmationRow => self.qty,
            FailureMatchStatus::NotEnoughOrdersApplied(qty) => qty,
        };

        if qty == 0 {
            return None;
        }

        Some(format!("Planned order not found for {}, {}, {}.000, Sigmanest Program:{}", self.mark, self.wbs, qty, self.program))
    }
}

impl PartialEq<CnfFileRow> for Failure {
    fn eq(&self, other: &CnfFileRow) -> bool {
        self.program == other.program && self.mark == other.mark && self.wbs == other.part_wbs
    }
}

impl PartialEq for Failure {
    fn eq(&self, other: &Self) -> bool {
        self.program == other.program && self.mark == other.mark && self.wbs == other.wbs
    }
}

impl PartialOrd for Failure {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }

        if self.mark < other.mark {
            return Some(Ordering::Less);
        }
        else if self.mark == other.mark {
            if self.program < other.program {
                return Some(Ordering::Less);
            }
            else if self.program == other.program {
                if self.wbs < other.wbs {
                    return Some(Ordering::Less);
                }
            }
        }

        Some(Ordering::Greater)
    }
}

impl Hash for Failure {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.mark.hash(state);
        self.wbs.hash(state);
        self.program.hash(state);
    }
}

/// Parses failure from inbox error string
/// 
/// looking to parse a line in the format of
/// `Planned order not found for {part name}, {wbs elements}, {qty}, Sigmanest Program: {program}`
/// 
/// will fail (return Err) if the input string does not match this pattern.
impl TryFrom<String> for Failure {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some(caps) = INBOX_TEXT.captures(&value) {
            return Ok(
                Self {
                    mark: caps.get(1).unwrap().as_str().into(),
                    wbs: Wbs::from( caps.get(2).unwrap() ),
                    qty: caps.get(3).unwrap().as_str().parse().unwrap(),
                    program: caps.get(4).unwrap().as_str().into(),

                    cnf_row: None::<CnfFileRow>,
                    applied: Vec::new(),
                }
            )
        }

        Err(format!("Failed to parse line `{}`", value))
    }
}
