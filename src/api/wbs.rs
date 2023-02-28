
use std::fmt::{Display, Debug};
use regex::Regex;

lazy_static! {
    static ref HD_WBS: Regex = Regex::new(r"D-(\d{7})-(\d{5})").expect("Failed to build HD_WBS regex");
    static ref LEGACY_WBS: Regex = Regex::new(r"S-(\d{7})-2-(\d{2})").expect("Failed to build LEGACY_WBS regex");
}

#[derive(Clone)]
pub enum Wbs {
    Legacy { job: String, shipment: u32 },
    Hd { job: String, id: u32 },
}

impl Wbs {
    pub fn set_id(mut self, new_id: u32) {
        match self {
            Self::Legacy { .. } => panic!("Cannot assign an Id to a Legacy Wbs"),
            Self::Hd { job: _, ref mut id } => *id = new_id
        }
    }

    pub fn into_hd_wbs(self, id: u32) -> Self {
        match self {
            Self::Legacy { job, shipment: _ } => Self::Hd { job, id },
            Self::Hd { .. } => self
        }
        
    }
}

impl From<&str> for Wbs {
    fn from(value: &str) -> Self {
        if let Some(caps) = HD_WBS.captures(value) {
            Self::Hd {
                job: caps.get(1).unwrap().as_str().into(),
                id: caps.get(2).unwrap().as_str().parse().unwrap()
            }
        }
        
        else if let Some(caps) = LEGACY_WBS.captures(value) {
            Self::Legacy {
                job: caps.get(1).unwrap().as_str().into(),
                shipment: caps.get(2).unwrap().as_str().parse().unwrap()
            }
        }

        else {
            panic!("Failed to parse WBS <{}>", value);
        }
    }
}

impl From<String> for Wbs {
    fn from(value: String) -> Self {
        Self::from( value.as_str() )
    }
}

impl From<regex::Match<'_>> for Wbs {
    fn from(value: regex::Match) -> Self {
        Self::from( value.as_str() )
    }
}

impl Display for Wbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Legacy { job, shipment } => write!(f, "S-{}-{}", job, shipment),
            Self::Hd     { job, id       } => write!(f, "D-{}-{}", job, id)
        }
    }
}

impl Debug for Wbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Legacy { .. } => write!(f, "Legacy<{}>", self),
            Self::Hd     { .. } => write!(f, "Hd <{}>", self)
        }
    }
}
