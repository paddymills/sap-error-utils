
use std::{
    collections::HashMap,
    path::PathBuf,
    hash::Hash,
};

use calamine::{Reader, open_workbook, Xlsx, DataType};

// TODO: use serde for this.

#[derive(Debug, Default)]
pub struct XlsxTableReader<H: HeaderColumn> {
    header: HashMap<H, usize>
}

impl<H> XlsxTableReader<H>
    where
        H: HeaderColumn + Eq + Hash
{
    pub fn new() -> Self {
        Self {
            header: HashMap::new(),
        }
    }

    fn is_header_matched(&self) -> bool {
        let mut cols = H::columns_to_match();

        let mut not_matched = Vec::new();
        while let Some(col) = cols.pop() {
            if !self.header.contains_key(&col) {
                not_matched.push(col)
            }
        }

        // TODO: return list of columns not matched
        not_matched.len() == 0
    }

    pub fn parse_header(&mut self, row: &[DataType]) {
        // TODO: multi-line header
        for (i, col) in row.iter().enumerate() {
            if let Some(key) = H::match_header_column(col.get_string().unwrap()) {
                self.header.insert(key, i);
            }

            // TODO: break if all columns matched
        }
    }

    pub fn read_file(&mut self, path: PathBuf) -> Result<Vec<H::Row>, &'static str> {
        let mut wb: Xlsx<_> = match open_workbook(path) {
            Ok(wb) => wb,
            Err(_) => return Err("failed ot open file")
        };
        
        let rng = &wb.worksheets()[0].1;
        let mut rows = rng.rows().into_iter();

        self.parse_header(rows.next().unwrap());

        // validate header matched 
        if !self.is_header_matched() {
            // TODO: specify which header columns not matched
            return Err("Not all header columns matched!");
        }

        let mut results = Vec::new();
        for row in rows {
            results.push(H::parse_row(&self.header, row));
        }

        return Ok(results);

        // Err(String::from("Failed to open first worksheet"))
    }
}

pub trait HeaderColumn {
    type Row;

    fn column_name(&self) -> String;
    fn match_header_column(column_text: &str) -> Option<Self> where Self: Sized;
    fn columns_to_match() -> Vec<Self> where Self: Sized;
    fn parse_row(header: &HashMap<Self, usize>, row: &[DataType]) -> Self::Row where Self: Sized;
}