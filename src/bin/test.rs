
use std::{path::PathBuf, collections::HashMap};

use calamine::DataType;
pub use sap_error_utils::excel::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum ExampleHeader {
    Order,
    Matl,
    Qty,
    Wbs
}

impl HeaderColumn for ExampleHeader {
    fn column_name(&self) -> String {
        use ExampleHeader::*;
    
        match self {
            Order => "Order",
            Matl => "Material",
            Qty => "Qty",
            Wbs => "WBS Element",
        }.into()
    }
    
    fn column_text (&self) -> String {
        self.to_string()
    }
}

impl ExampleHeader {
    fn all() -> Vec<Self> {
        vec![
            Self::Order,
            Self::Matl,
            Self::Qty,
            Self::Wbs
        ]
    }
}

impl ToString for ExampleHeader {
    fn to_string(&self) -> String {
        use ExampleHeader::*;

        match self {
            Order => "Order",
            Matl => "Material Number",
            Qty => "Order quantity (GMEIN)",
            Wbs => "WBS Element",
        }.into()
    }
}

impl PartialEq<String> for ExampleHeader {
    fn eq(&self, other: &String) -> bool {
        &self.to_string() == other
    }
}

#[derive(Debug)]
struct ExampleRow {
    order: String,
    matl: String,
    qty: u32,
    wbs: String
}

impl XlsxRow<ExampleHeader> for ExampleRow {
    fn parse_row(row: HashMap<&ExampleHeader, &DataType>) -> Self
    {
        let order = row.get(&ExampleHeader::Order).unwrap().get_string().unwrap().into();
        // let order = match row.get(&ExampleHeader::Order) {
        //     Some(DataType::String(s)) => s.into(),
        //     _ => String::new()
        // };
        let matl = match row.get(&ExampleHeader::Matl) {
            Some(DataType::String(s)) => s.into(),
            _ => String::new()
        };
        let qty = match row.get(&ExampleHeader::Qty) {
            Some(DataType::Int(i)) => *i as u32,
            _ => 0u32
        };
        let wbs = match row.get(&ExampleHeader::Wbs) {
            Some(DataType::String(s)) => s.into(),
            _ => String::new()
        };


        Self { order, matl, qty, wbs }
    }
}

fn main() -> std::io::Result<()> {

    let userprofile = match std::env::var_os("USERPROFILE") {
        Some(path) => path,
        None => panic!("Could not locate env variable `USERPROFILE`")
    };

    let path = PathBuf::from(format!("{}/Documents/SAP/SAP GUI/cohv.xlsx", userprofile.to_str().unwrap()));

    let mut reader = XlsxTableReader::new();
    reader.set_header(ExampleHeader::all());
    for row in reader.read_file::<ExampleRow>(path) {
        println!("{:?}", row);
    }

    Ok(())
}