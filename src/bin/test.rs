
use std::path::PathBuf;

use sap_error_utils::inbox::parse_failures;

// main for testing
fn main() {
    let file = PathBuf::from("inbox.txt");

    let inbox = parse_failures(file).unwrap();
    for f in inbox {
        println!("{:?}", f);
    }
}