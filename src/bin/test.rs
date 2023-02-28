
use std::path::PathBuf;

use sap_error_utils::inbox::parse_cohv;

// main for testing
fn main() {
    let file = PathBuf::from("cohv.txt");

    let _ = parse_cohv(file).unwrap();
}