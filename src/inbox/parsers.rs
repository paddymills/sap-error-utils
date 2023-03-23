
use calamine::DataType;
use regex::Regex;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use crate::api::{Order, OrderData};
use crate::excel::{XlsxTableReader, HeaderColumn};
use super::{Failure, cohv::Header};

pub fn parse_failures(path: PathBuf) -> io::Result<Vec<Failure>> {
    let mut results = Vec::new();

    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        match Failure::try_from(line?) {
            Ok(f) => results.push(f),
            Err(e) => eprintln!("{}", e)
        }
    }

    Ok(results)
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

    fn parse_row(header: &HashMap<Self, usize>, row: &[DataType]) -> Self::Row
        where Self: Sized
    {
        // TODO: handle parsing errors (get_string/get_int)
        let order = row[*header.get(&Self::Order).unwrap()].get_string().unwrap().parse().unwrap();
        let matl  = row[*header.get(&Self::Matl).unwrap() ].get_string().unwrap().into();
        let qty   = row[*header.get(&Self::Qty).unwrap()  ].get_float().unwrap() as u32;
        let wbs   = row[*header.get(&Self::Wbs).unwrap()  ].get_string().unwrap().try_into().unwrap();
        let _type = row[*header.get(&Self::Type).unwrap() ].get_string().unwrap();
        let plant = row[*header.get(&Self::Plant).unwrap()].get_string().unwrap().into();

        let data = OrderData { id: order, mark: matl, qty, wbs, plant };

        Order::new(_type, data)
    }
}

pub fn parse_cohv_xl(cohv_file: PathBuf) -> anyhow::Result<Vec<Order>> {
    
    let mut reader = XlsxTableReader::<CohvHeader>::new();
    reader.read_file(cohv_file).map_err(anyhow::Error::msg)
}
