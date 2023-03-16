
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Plant {
    /// Lancaster (HS01)
    #[serde(rename = "HS01")]
    Lancaster,
    /// Williamsport (HS02)
    #[serde(rename = "HS02")]
    Williamsport
}

impl From<String> for Plant {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&str> for Plant {
    fn from(value: &str) -> Self {
        match value {
            "HS01" => Self::Lancaster,
            "HS02" => Self::Williamsport,
            _ => panic!("Unexpected plant string <{}>", value)
        }
    }
}