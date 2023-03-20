
use std::{
    collections::HashMap,
    path::PathBuf,
    hash::Hash,
};

use calamine::{Reader, open_workbook, Xlsx, DataType};

// TODO: use serde for this.

#[derive(Debug, Default)]
pub struct XlsxTableReader<T: HeaderColumn> {
    header: HashMap<T, Option<usize>>
}

impl<T> XlsxTableReader<T>
    where
        T: HeaderColumn + Eq + Hash
{
    pub fn new() -> Self {
        Self {
            header: HashMap::new(),
        }
    }

    pub fn set_header(&mut self, header: Vec<T>) {
        self.header.extend(
            header
                .into_iter()
                .map(|h| (h, None))
        );
    }

    pub fn read_file(&mut self, path: PathBuf) {
        let mut wb: Xlsx<_> = open_workbook(path).expect("Cannot open file");
        
        
        if let Some(Ok(rng)) = wb.worksheet_range("Sheet1") {
            let mut rows = rng.rows();
            
            // TODO: multi-line header
            for (i, col) in rows.next().expect("Cannot read an empty sheet").iter().enumerate() {
                if let DataType::String(s) = col {
                    for (k, v) in self.header.iter_mut() {
                        if let None = v {
                            if k.matches_column_name(s) {
                                *v = Some(i);
                                break;
                            }
                        }
                    }
                }
            }

            // validate header matched 
            if self.header
                .iter()
                .filter(|(_, v)| v.is_none())
                .count() > 0 {
                    // TODO: specify which header columns not matched
                    panic!("Not all header columns matched!")
                }

            for row in rows {
                for (k, v) in &self.header {
                    print!("| {}: {} ", k.column_name(), row[v.unwrap()])
                }
                println!("|")
                // println!("{:?}", row);
            }
        }
    }
}

pub trait HeaderColumn {
    fn column_name(&self) -> String;
    fn column_text (&self) -> String;

    fn matches_column_name(&self, name: &String) -> bool {
        &self.column_text() == name
    }
}
