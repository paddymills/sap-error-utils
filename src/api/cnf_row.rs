
use std::ops::{Add, AddAssign};

use super::{Plant, OrderData, Wbs};

use super::cnf_serde::three_digit_f64;

/// Confirmation file row (SAP Confirmation Files)
/// 
/// tab delimited row in the format
/// ```tsv
/// {mark}	S-{job}	{part wbs}	{part location: PROD}	{part qty}	{part UoM: EA}	{material master}	{material wbs}	{material qty}	{material UoM: IN2}	{material location}	{plant}	{program}	
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all="PascalCase")]
pub struct CnfFileRow {
    /// Part mark (piecemark)
    pub mark:     String,
    /// Job number (without structure) in the format `S-{job}`
    pub job:      String,
    /// WBS element for part
    pub part_wbs: Wbs,
    /// Location for part (PROD)
    pub part_loc: String,
    /// Part quantity
    pub part_qty: u64,
    /// Part unit of measure (EA)
    pub part_uom: String,

    /// Material master
    pub matl:     String,
    /// Material WBS Element
    pub matl_wbs: Option<Wbs>,
    /// Material quantity
    ///
    /// This is the amount consumed for all parts.
    /// 
    /// `{qty per part} * {part_qty} = {matl_qty}`
    #[serde(serialize_with="three_digit_f64")]
    pub matl_qty: f64,
    /// Material unit of measure (IN2, usually)
    pub matl_uom: String,
    /// Material storage location
    pub matl_loc: Option<String>,

    /// Plant for Part and Material
    /// 
    /// If the part is consuming in 1 plant and the material from another,
    /// this should be the plant of the part.
    /// 
    /// Reason being that the part confirmation will fail for the wrong
    /// plant but the material consumption will cause a COGI error,
    /// which can be easily fixed in COGI.
    pub plant:    Plant,
    /// Program number
    pub program:  String
}

impl CnfFileRow {

    pub fn area_per_ea(&self) -> f64 {
        self.matl_qty / self.part_qty as f64
    }

    pub fn modify_with(&self, order: OrderData) -> Self {
        let mut result = self.clone();

        result.part_wbs = order.wbs;
        result.part_qty = order.qty as u64;
        result.matl_qty = self.area_per_ea() * order.qty as f64;
        result.plant = order.plant;

        result
    }
}

impl Add<CnfFileRow> for CnfFileRow {
    type Output = Self;

    fn add(self, rhs: CnfFileRow) -> Self::Output {
        let mut result = self.clone();

        result.part_qty += rhs.part_qty;
        result.matl_qty += rhs.matl_qty;

        result
    }
}

impl AddAssign<CnfFileRow> for CnfFileRow {
    fn add_assign(&mut self, rhs: CnfFileRow) {
        self.part_qty += rhs.part_qty;
        self.matl_qty += rhs.matl_qty;
    }
}
