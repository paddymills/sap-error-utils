
use regex::{Regex, RegexSetBuilder, RegexSet};

use super::Plant;
use super::CnfFileRow;
use super::cnf_serde::three_digit_f64;

lazy_static! {
    // old, non-hd, wbs element
    static ref OLD_WBS: Regex = Regex::new(r"S-(\d{7})-2-(\d{2})").expect("Failed to build OLD_WBS Regex");
    // HD wbs element
    static ref HD_WBS: Regex = Regex::new(r"D-(\d{7})-\d{5}").expect("Failed to build HD_WBS Regex");
    // old, non-hd, wbs element
    static ref CC_WBS: Regex = Regex::new(r"S-.*-2-(2\d{3})").expect("Failed to build CC_WBS Regex");

    // Production job number match
    static ref PROD_JOB: Regex = Regex::new(r"S-\d{7}").expect("Failed to build JOB_PART Regex");

    // machine name pattern matching
    static ref MACHINES: RegexSet = {
        let names = vec!["gemini", "titan" , "mg", "farley", "ficep"];

        // each name will must be begin and end with '-', '_', or string start/end
        RegexSetBuilder::new(
            names
                .iter()
                .map(|n| format!("(^|[_-]){}($|[_-])", n))
        )
            .case_insensitive(true)
            .build()
            .expect("failed to build machine name patterns")
    };
}

/// Issue file row (SAP Confirmation Files)
/// 
/// ### Text format
/// tab delimited row in the format:
/// ```tsv
/// {code}	{user1}	{user2}	{material master}	{material wbs}	{material qty}	{material UoM: IN2}	{material location}	{plant}	{program}	
/// ```
/// 
/// ### Transaction Codes
/// 
/// | code | SAP transactions | description |
/// |---|---|---|
/// | PR01 | MIGO 221Q | Comsumption for project from project |
/// | PR02 | MIGO 221 | Consumption for project from warehouse |
/// | PR03 | MIGO 311Q + MIGO 221Q | Transfer from project to project And consumption from latter project |
/// | CC01 | MIGO 201 | Consumption for cost center from warehouse |
/// | CC02 | MIGO [transfer from WBS] & 201 | Consumption for cost center from project |
/// 
/// ### User Columns
/// 
/// User columns are to fill in where the material is being charged,
/// depending on what type of [transaction code](#transaction-codes) is used.
/// 
/// | code | user1 | user2 |
/// |---|---|---|
/// | PR* | `D-{job}` | Shipment |
/// | CC* | Cost Center | [G/L Account](#gl-accounts) |
///
/// ### G/L Accounts
/// 
/// G/L accounts should be a `634xxx` code
/// 
/// | Usage | G/L Account |
/// |---|---|
/// | Machine Parts (i.e. CNC table parts) | `634124` |
/// | Shop Supplies (default) | `637118` |
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all="PascalCase")]
pub struct IssueFileRow {
    /// [Transaction code](#transaction-codes)
    pub code: IssueCode,
    /// Project or Cost Center ([User1 Column](#user-columns))
    pub user1: String,
    /// Shipment/GL Account ([User2 Column](#user-columns))
    pub user2: String,

    /// Material master
    pub matl:     String,
    /// Material WBS Element
    pub matl_wbs: Option<String>,
    /// Material quantity
    #[serde(serialize_with="three_digit_f64")]
    pub matl_qty: f64,
    /// Material unit of measure
    pub matl_uom: String,
    /// Material storage location
    pub matl_loc: Option<String>,

    /// Material plant
    pub plant:    Plant,
    /// Program number
    pub program:  String
}

/// Issue codes
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum IssueCode {
    /// Issue material to the same project
    #[serde(rename = "PR01")]
    ProjectFromProject,
    /// Issue material from stock (no WBS element) to a project
    #[serde(rename = "PR02")]
    ProjectFromStock,
    /// Issue material to a project from a different project
    #[serde(rename = "PR03")]
    ProjectFromOtherProject,
    /// Issue material from stock to a cost center
    #[serde(rename = "CC01")]
    CostCenterFromStock,
    /// Issue material from a project to a cost center
    #[serde(rename = "CC02")]
    CostCenterFromProject,
}

impl Into<IssueFileRow> for CnfFileRow {
    /// Convert a [`CnfFileRow`] into an [`IssueFileRow`]
    fn into(self) -> IssueFileRow {
        let (code, user1, user2) = infer_codes(&self);

        IssueFileRow {
            code, user1, user2,

            matl:     self.matl,
            matl_wbs: self.matl_wbs,
            matl_qty: self.matl_qty,
            matl_uom: self.matl_uom,
            matl_loc: self.matl_loc,
            plant:    self.plant,
            program:  self.program
        }
    }
}

fn infer_codes(row: &CnfFileRow) -> (IssueCode, String, String) {
    // part WBS ends in cost center -> cost center issuing
    if let Some(caps) = CC_WBS.captures(&row.part_wbs) {
        let code = match &row.matl_wbs {
            Some(_) => IssueCode::CostCenterFromProject,
            None => IssueCode::CostCenterFromStock
        };
    
        // cost center
        let user1 = caps
            .get(1)
            .map_or("20xx", |m| m.as_str());
    
        // infer G/L account
        let user2 = infer_gl_acct(&row.mark);

        return (code, user1.into(), user2)
    }

    // project stock issuing
    if PROD_JOB.is_match(&row.job) {
        // infer job and shipment from part WBS element
        let (user1, user2) = match OLD_WBS.captures(&row.part_wbs) {
            Some(caps) => (
                format!("D-{}", caps.get(1).unwrap().as_str()),
                caps.get(2).unwrap().as_str().into()
            ),
            None => {
                if let Some(caps) = HD_WBS.captures(&row.part_wbs) {
                    // TODO: fetch shipment from database
                    (
                        format!("D-{}", caps.get(1).unwrap().as_str()),
                        "01".into()
                    )
                } else {
                    error!("failed to parse Part WBS Element: {}", &row.part_wbs);
                    panic!("failed to parse Part WBS Element")
                }
            }
        };

        let code = match &row.matl_wbs {
            // project stock material
            Some(wbs) => {
                // part and material have the same project
                if wbs.starts_with(&user1) { IssueCode::ProjectFromProject }

                // part and material have different projects
                else { IssueCode::ProjectFromOtherProject }
            },

            // plant stock material
            None => IssueCode::ProjectFromStock
        };

        return (code, user1, user2)
    }

    // unmatched data
    error!("unmatched data when converting to issuing row");
    panic!("cnf -> issue conversion failed");
    // TODO: default result
}

fn infer_gl_acct(mark: &String) -> String {
    match MACHINES.is_match(mark) {
        true  => "634124".into(),   // machine parts
        false => "637118".into()    // all others
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_row() -> CnfFileRow {
        CnfFileRow {
            mark: "1210123A-X1A".into(),
            job: "S-1210123".into(),
            part_wbs: "S-1210123-2-10".into(),
            part_loc: "PROD".into(),
            part_qty: 5u64,
            part_uom: "EA".into(),
            
            matl: "50W-0008".into(),
            matl_wbs: None,
            matl_qty: 1_001.569f64,
            matl_uom: "IN2".into(),
            matl_loc: Some("K2".into()),

            plant: Plant::Lancaster,
            program: "54091".into()
        }
    }

    #[test]
    fn machines_regex() {
        assert!(MACHINES.is_match("GEMINI_TABLE-A"));
        assert_eq!(false, MACHINES.is_match("geminitest"));
        assert!(MACHINES.is_match("for_titan"));
        assert_eq!(false, MACHINES.is_match("an_img"));
        assert!(MACHINES.is_match("mg-test"));
        assert!(MACHINES.is_match("for_mg"));
        assert!(MACHINES.is_match("farley-a"));
    }

    #[test]
    fn infer_job_shipment() {
        let row = get_test_row();
        let (_, u1, u2) = infer_codes(&row);

        assert_eq!(&u1, "D-1210123");
        assert_eq!(&u2, "10");
    }

    #[test]
    fn infer_project_from_stock() {
        let row = get_test_row();
        let (c, ..) = infer_codes(&row);

        assert_eq!(c, IssueCode::ProjectFromStock);
    }

    #[test]
    fn infer_project_from_project() {
        let mut row = get_test_row();
        row.matl_wbs = Some("D-1210123-10004".into());

        let (c, ..) = infer_codes(&row);
        assert_eq!(c, IssueCode::ProjectFromProject);
    }

    #[test]
    fn infer_project_from_other_project() {
        let mut row = get_test_row();
        row.matl_wbs = Some("D-1200248-10004".into());

        let (c, ..) = infer_codes(&row);

        assert_eq!(c, IssueCode::ProjectFromOtherProject);
    }

    #[test]
    fn infer_cost_center_stock() {
        let mut row = get_test_row();
        // row.job = "D-HSU".into();
        row.part_wbs = "S-HSU-2-2062".into();

        let (c, ..) = infer_codes(&row);

        assert_eq!(c, IssueCode::CostCenterFromStock);
    }

    #[test]
    fn infer_cost_center_project() {
        let mut row = get_test_row();
        // row.job = "D-HSU".into();
        row.part_wbs = "S-HSU-2-2062".into();
        row.matl_wbs = Some("D-1200248-10004".into());

        let (c, ..) = infer_codes(&row);

        assert_eq!(c, IssueCode::CostCenterFromProject);
    }

    #[test]
    #[should_panic]
    fn infer_fallout() {
        let mut row = get_test_row();
        row.job = "D-HSU".into();

        let _ = infer_codes(&row);
    }
}

