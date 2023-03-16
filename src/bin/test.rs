use std::path::PathBuf;

use calamine::{Reader, open_workbook, Xlsx, DataType};

fn main() -> std::io::Result<()> {

    let userprofile = match std::env::var_os("USERPROFILE") {
        Some(path) => path,
        None => panic!("Could not locate env variable `USERPROFILE`")
    };

    let path = PathBuf::from(format!("{}/Documents/SAP/SAP GUI/export.xlsx", userprofile.to_str().unwrap()));
    let mut wb: Xlsx<_> = open_workbook(path).expect("Cannot open file");
    if let Some(Ok(rng)) = wb.worksheet_range("Sheet1") {
        for row in rng.rows() {
            println!("{:?}", row);
        }
    }

    Ok(())
}