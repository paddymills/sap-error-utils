
mod failure;
pub use failure::Failure;

mod cohv;
pub use cohv::parse_cohv;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

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
