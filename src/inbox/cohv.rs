
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use regex::Regex;

use crate::api::Order;

lazy_static! {
    static ref DATA_ROW: Regex = Regex::new(r"(|[\w ]+)+|")
        .expect("Failed to build DATA_ROW regex");

    static ref DELIMIT_ROW: Regex = Regex::new(r"-+")
        .expect("Failed to build DELIMIT_ROW regex");
}

#[derive(Debug, Default)]
struct Header {
    _type: u8,

    order: u8,
    mark: u8,
    qty: u8,
    wbs: u8,
    plant: u8,
}

pub fn parse_cohv(cohv_file: PathBuf) -> io::Result<Vec<Order>> {
    let results = Vec::new();

    let file = File::open(cohv_file)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        if let Ok(l) = line {
            if DATA_ROW.is_match(&l) {
                println!("{}", l);
            }
        }
    }

    Ok(results)
}

fn parse_header(_row: Vec<String>) -> Header {
    Header::default()
}
