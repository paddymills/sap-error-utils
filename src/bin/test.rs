
use std::{path::PathBuf, collections::HashMap};

use calamine::DataType;
use sap_error_utils::api::{Wbs, Plant};
pub use sap_error_utils::excel::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum ExampleHeader {
    Order,
    Matl,
    Qty,
    Wbs,
    Type,
    Plant,
}

impl HeaderColumn for ExampleHeader {
    type Row = ExampleRow;

    fn column_name(&self) -> String {
        use ExampleHeader::*;
    
        match self {
            Order => "Order",
            Matl => "Material",
            Qty => "Qty",
            Wbs => "WBS Element",
            Type => "Order Type",
            Plant => "Plant"
        }.into()
    }

    fn columns_to_match() -> Vec<Self> where Self: Sized {
        vec![
            ExampleHeader::Order,
            ExampleHeader::Matl,
            ExampleHeader::Qty,
            ExampleHeader::Wbs,
            ExampleHeader::Type,
            ExampleHeader::Plant,
        ]
    }

    fn match_header_column(column_text: &str) -> Option<Self>
        where Self: Sized
    {
        match column_text {
            "Order"                  => Some( Self::Order ),
            "Material Number"        => Some( Self::Matl  ),
            "Order quantity (GMEIN)" => Some( Self::Qty   ),
            "WBS Element"            => Some( Self::Wbs   ),
            _                        => None
        }
    }

    fn parse_row(header: &HashMap<Self, usize>, row: &[DataType]) -> anyhow::Result<Self::Row>
        where Self: Sized
    {
        // TODO: handle parsing errors (get_string/get_int)
        let order = row[*header.get(&Self::Order).unwrap()].get_string().unwrap().into();
        let matl  = row[*header.get(&Self::Matl).unwrap() ].get_string().unwrap().into();
        let qty   = row[*header.get(&Self::Qty).unwrap()  ].get_float().unwrap() as u32;
        let wbs   = row[*header.get(&Self::Wbs).unwrap()  ].get_string().unwrap().try_into().unwrap();
        let _type = row[*header.get(&Self::Type).unwrap() ].get_string().unwrap().into();
        let plant = row[*header.get(&Self::Plant).unwrap()].get_string().unwrap().into();

        Ok( ExampleRow { order, matl, qty, wbs, _type, plant } )
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
            Type => "Order Type",
            Plant => "Plant",
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
    wbs: Wbs,
    _type: String,
    plant: Plant
}

fn main() -> std::io::Result<()> {

    let userprofile = match std::env::var_os("USERPROFILE") {
        Some(path) => path,
        None => panic!("Could not locate env variable `USERPROFILE`")
    };

    let path = PathBuf::from(format!("{}/Documents/SAP/SAP GUI/cohv.xlsx", userprofile.to_str().unwrap()));

    let mut reader = XlsxTableReader::<ExampleHeader>::new();
    for row in reader.read_file(path).unwrap() {
        println!("{:?}", row);
    }

    Ok(())
}