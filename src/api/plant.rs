
#[derive(Debug)]
pub enum Plant {
    Lancaster,
    Williamsport
}

impl From<String> for Plant {
    fn from(value: String) -> Self {
        match value.as_str() {
            "HS01" => Self::Lancaster,
            "HS02" => Self::Williamsport,
            _ => panic!("Unexpected plant string <{}>", value)
        }
    }
}