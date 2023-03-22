
use std::{
    collections::HashMap,
    path::PathBuf,
    hash::Hash,
};

use calamine::{Reader, open_workbook, Xlsx, DataType};

// TODO: use serde for this.

#[derive(Debug, Default)]
pub struct XlsxTableReader<H: HeaderColumn> {
    header: HashMap<H, Option<usize>>
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

    pub fn set_header(&mut self, header: Vec<H>) {
        self.header.extend(
            header
                .into_iter()
                .map(|h| (h, None))
        );
    }

    fn is_header_matched(&self) -> bool {
        match self.header
            .iter()
            .filter(|(_, v)| v.is_none())
            .count() {
                0 => true,
                _ => false
            }
    }

    pub fn get_row_value(&self, index: &H, row: &[DataType]) -> DataType {
        match self.header.get(index) {
            Some(Some(key)) => row[*key].clone(),
            _ => panic!("index {:?} not in header", index.column_name())
        }
    }

    pub fn read_file<R>(&mut self, path: PathBuf) -> Vec<R>
        where
            R: XlsxRow<H>
    {
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
            if !self.is_header_matched() {
                    // TODO: specify which header columns not matched
                    panic!("Not all header columns matched!")
                }

            let mut results: Vec<R> = Vec::new();
            for row in rows {
                let vals = self.header.iter()
                    .map( |(k,v)| (k.clone(), &row[v.unwrap()]) )
                    .collect();
                results.push(R::parse_row(vals));
            }

            return results;
        }

        // TODO: handle failure
        Vec::new()
    }
}

pub trait RowParser {
    // two part tool to parse a table
    //  - Header column matching enum
    //  - Row serializing struct

    // header impls
    //  - header column match (maybe TryFrom)
    //  - all columns matched
    type Header;
    type XlRow;

    fn parse_header(&self, row: &[DataType]) -> Result<(), String>;
    fn match_header_column(column_text: &String) -> Option<Self::Header>;
    fn parse_row(&self, row: &[DataType]) -> Self::XlRow;
}

pub trait HeaderColumn {
    fn column_name(&self) -> String;
    fn column_text(&self) -> String;

    fn matches_column_name(&self, name: &String) -> bool {
        &self.column_text() == name
    }
}

pub trait XlsxRow<H: HeaderColumn> {
    fn parse_row(row: HashMap<&H, &DataType>) -> Self;
}