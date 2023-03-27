
use calamine::DataType;
use regex::Regex;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use crate::api::{Order, OrderData};
use crate::excel::{XlsxTableReader, HeaderColumn};
use super::{Failure, cohv::Header};

pub fn parse_failures(failures: impl Iterator<Item = impl ToString>) -> Vec<anyhow::Result<Failure>> {
    failures
        .map(|f| Failure::try_from(f.to_string()))
        .collect()
}

enum ParsingMode {
    Header,
    Row(Header)
}

pub fn parse_cohv_txt(cohv_file: PathBuf) -> io::Result<Vec<Order>> {
    let data_row = Regex::new(r"^(?:\|?[^\|]+)*\|$")
        .expect("Failed to build DATA_ROW regex");

    let mut results = Vec::new();

    let file = File::open(cohv_file)?;
    let reader = io::BufReader::new(file);

    let mut mode = ParsingMode::Header;

    for line in reader.lines() {
        if let Ok(l) = line {
            if data_row.is_match(&l) {
                match mode {
                    ParsingMode::Header => mode = ParsingMode::Row(Header::try_from(l).unwrap()),
                    ParsingMode::Row(ref header) => results.push(header.parse_row(l)),
                }
            }
        }
    }

    Ok(results)
}


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum CohvHeader {
    Order,
    Matl,
    Qty,
    Wbs,
    Type,
    Plant,
}

impl HeaderColumn for CohvHeader {
    type Row = Order;

    fn column_name(&self) -> String {
        use CohvHeader::*;
    
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
            CohvHeader::Order,
            CohvHeader::Matl,
            CohvHeader::Qty,
            CohvHeader::Wbs,
            CohvHeader::Type,
            CohvHeader::Plant,
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
            "Order Type"             => Some( Self::Type  ),
            "Plant"                  => Some( Self::Plant ),
            _                        => None
        }
    }

    fn parse_row(header: &HashMap<Self, usize>, row: &[DataType]) -> anyhow::Result<Self::Row>
        where Self: Sized
    {
        // TODO: handle parsing errors (get_string/get_int)
        let order = row[*header.get(&Self::Order).unwrap()].get_string().ok_or( anyhow!("Failed to coerce order to String") )?.parse()?;

        let matl  = row[*header.get(&Self::Matl).unwrap() ].get_string().ok_or( anyhow!("Failed to read Material as String") )?.into();
        let qty   = row[*header.get(&Self::Qty).unwrap()  ].get_float() .ok_or( anyhow!("Failed to read qty as Float") )? as u32;
        let wbs   = row[*header.get(&Self::Wbs).unwrap()  ].get_string().ok_or( anyhow!("Failed to read Wbs Element") )?.try_into()?;
        let _type = row[*header.get(&Self::Type).unwrap() ].get_string().ok_or( anyhow!("Failed to read Order Type") )?;
        let plant = row[*header.get(&Self::Plant).unwrap()].get_string().ok_or( anyhow!("Failed to read Plant") )?.into();

        let data = OrderData { id: order, mark: matl, qty, wbs, plant };

        Ok( Order::new(_type, data) )
    }
}

pub fn parse_cohv_xl(cohv_file: PathBuf) -> anyhow::Result<Vec<Order>> {
    
    let mut reader = XlsxTableReader::<CohvHeader>::new();
    let vals = reader.read_file(cohv_file)?
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    Ok(vals)
}
