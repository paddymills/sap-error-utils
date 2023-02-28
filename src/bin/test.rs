
use std::path::PathBuf;

use sap_error_utils::inbox::parsers::parse_cohv;

// main for testing
fn main() {
    let file = PathBuf::from("cohv.txt");

    let orders = parse_cohv(file).unwrap();
    for order in orders {
        println!("{:?}", order);
    }
}