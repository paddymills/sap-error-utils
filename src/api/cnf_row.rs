
use prodctrl::Plant;

use super::cnf_serde::three_digit_f64;

/// Confirmation file row (SAP Confirmation Files)
/// 
/// tab delimited row in the format
/// ```tsv
/// {mark}	S-{job}	{part wbs}	{part location: PROD}	{part qty}	{part UoM: EA}	{material master}	{material wbs}	{material qty}	{material UoM: IN2}	{material location}	{plant}	{program}	
/// ```
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all="PascalCase")]
pub struct CnfFileRow {
    /// Part mark (piecemark)
    pub mark:     String,
    /// Job number (without structure) in the format `S-{job}`
    pub job:      String,
    /// WBS element for part
    pub part_wbs: String,
    /// Location for part (PROD)
    pub part_loc: String,
    /// Part quantity
    pub part_qty: u64,
    /// Part unit of measure (EA)
    pub part_uom: String,

    /// Material master
    pub matl:     String,
    /// Material WBS Element
    pub matl_wbs: Option<String>,
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
