
use regex::Regex;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use crate::api::Order;
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

pub fn parse_cohv(cohv_file: PathBuf) -> io::Result<Vec<Order>> {
    let data_row = Regex::new(r"(?:\|([\w ]+))+?\|")
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
