
use std::fmt::{Display, Debug};
use regex::Regex;
use serde::{Deserializer, de::Error, Serialize};

lazy_static! {
    static ref COST_CENTER_WBS: Regex = Regex::new(r"S-.*-2-(2\d{3})").expect("Failed to build COST_CENTER_WBS regex");
    static ref HD_WBS: Regex = Regex::new(r"D-(\d{7})-(\d{5})").expect("Failed to build HD_WBS regex");
    static ref LEGACY_WBS: Regex = Regex::new(r"S-(\d{7})-2-(\d{2})").expect("Failed to build LEGACY_WBS regex");
}

#[derive(Clone, Hash, PartialEq, PartialOrd, Deserialize)]
pub enum Wbs {
    None,
    CostCenter { cc: u32 },
    Hd { job: String, id: u32 },
    Legacy { job: String, shipment: u32 },
}

impl Wbs {
    pub fn set_id(mut self, new_id: u32) {
        match self {
            Self::Hd { job: _, ref mut id } => *id = new_id,

            Self::CostCenter { .. } => panic!("Cannot assign an Id to a CostCenter Wbs"),
            Self::Legacy { .. } => panic!("Cannot assign an Id to a Legacy Wbs"),
            Self::None => panic!("Cannot assign Id to no Wbs")
        }
    }

    pub fn into_hd_wbs(self, id: u32) -> Self {
        match self {
            Self::Hd { .. } => self,
            Self::Legacy { job, shipment: _ } => Self::Hd { job, id },

            Self::CostCenter { .. } => panic!("Cannot convert CostCenter Wbs to an HD Wbs"),
            Self::None => Self::None
        }
        
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Wbs, D::Error>
        where D: Deserializer<'de>
    {
        let s: &str = serde::de::Deserialize::deserialize(deserializer)?;
        Wbs::try_from(s).map_err(D::Error::custom)
    }
}

impl Serialize for Wbs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        serializer.collect_str(self)
    }
}

// impl From<&str> for Wbs {
//     fn from(value: &str) -> Self {
//         if value == "" {
//             return Self::None;
//         }

//         if let Some(caps) = COST_CENTER_WBS.captures(value) {
//             Self::CostCenter {
//                 cc: caps.get(1).unwrap().as_str().parse().unwrap()
//             }
//         }

//         else if let Some(caps) = HD_WBS.captures(value) {
//             Self::Hd {
//                 job: caps.get(1).unwrap().as_str().into(),
//                 id: caps.get(2).unwrap().as_str().parse().unwrap()
//             }
//         }
        
//         else if let Some(caps) = LEGACY_WBS.captures(value) {
//             Self::Legacy {
//                 job: caps.get(1).unwrap().as_str().into(),
//                 shipment: caps.get(2).unwrap().as_str().parse().unwrap()
//             }
//         }

//         else {
//             panic!("Failed to parse WBS <{}>", value);
//         }
//     }
// }

impl TryFrom<&str> for Wbs {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value == "" {
            return Ok(Self::None);
        }

        if let Some(caps) = COST_CENTER_WBS.captures(value) {
            Ok(Self::CostCenter {
                // unwraps should not panic here, if regex worked
                cc: caps.get(1).unwrap().as_str().parse().unwrap()
            })
        }

        else if let Some(caps) = HD_WBS.captures(value) {
            Ok(Self::Hd {
                // unwraps should not panic here, if regex worked
                job: caps.get(1).unwrap().as_str().into(),
                id: caps.get(2).unwrap().as_str().parse().unwrap()
            })
        }
        
        else if let Some(caps) = LEGACY_WBS.captures(value) {
            Ok(Self::Legacy {
                // unwraps should not panic here, if regex worked
                job: caps.get(1).unwrap().as_str().into(),
                shipment: caps.get(2).unwrap().as_str().parse().unwrap()
            })
        }

        else {
            Err( anyhow!("Failed to parse WBS <{}>", value) )
        }
    }
}

impl From<String> for Wbs {
    fn from(value: String) -> Self {
        Self::try_from( value.as_str() ).unwrap()
    }
}

impl From<regex::Match<'_>> for Wbs {
    fn from(value: regex::Match) -> Self {
        Self::try_from( value.as_str() ).unwrap()
    }
}

impl Display for Wbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CostCenter { cc            } => write!(f, "{}", cc),
            Self::Hd         { job, id       } => write!(f, "D-{}-{}", job, id),
            Self::Legacy     { job, shipment } => write!(f, "S-{}-{}", job, shipment),
            Self::None                         => write!(f, ""),
        }
    }
}

impl Debug for Wbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CostCenter { .. } => write!(f, "CostCenter <{}>", self),
            Self::Hd         { .. } => write!(f, "Hd <{}>", self),
            Self::Legacy     { .. } => write!(f, "Legacy <{}>", self),
            Self::None              => write!(f, "<No Wbs>"),
        }
    }
}
