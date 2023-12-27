
use csv::{ReaderBuilder, StringRecord, WriterBuilder};

use std::fs::DirEntry;
// use std::fs::DirEntry;
use std::{fs, io};
use std::path::PathBuf;

use crate::api::CnfFileRow;
use crate::paths;

const HEADERS: [&str; 13] = [
    "Mark", "Id", "PartWbs", "PartLoc", "PartQty", "PartUom", "Matl", "MatlWbs" , "MatlQty", "MatlUom", "MatlLoc", "Plant", "Program"
];
const DELIM: u8 = b'\t';

lazy_static! {
    // .ready file reader/writer factories
    // TODO: refactor into struct, with convenience (from_path) methods
    static ref READY_READER: ReaderBuilder = {
        let mut reader = ReaderBuilder::new();
        reader
            .delimiter(DELIM);

        reader
    };
    static ref READY_WRITER: WriterBuilder = {
        let mut writer = WriterBuilder::new();
        writer
            .has_headers(false)
            .delimiter(DELIM);

        writer
    };
}

pub fn get_last_n_files(n: usize) -> io::Result<Vec<DirEntry>> {
    let mut entries = fs::read_dir(paths::SAP_ARCHIVE.to_path_buf())?
        .filter_map(Result::ok)
        .filter(|entry| {
            paths::PROD_FILE_NAME.is_match(entry.file_name().to_str().unwrap_or(""))
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        b.metadata()
            .expect("Failed to get `b` Metadata")
            .modified()
            .expect("Failed to get `b` SystemTime")
            .cmp(
                &a.metadata()
                    .expect("Failed to get `a` Metadata")
                    .modified()
                    .expect("Failed to get `a` SystemTime")
            )
    });

    Ok(entries.into_iter().take(n).collect())
}

pub fn get_num_files() -> io::Result<usize> {
    let count = fs::read_dir(paths::SAP_ARCHIVE.to_path_buf())?
        .filter_map(Result::ok)
        .filter(|entry| {
            paths::PROD_FILE_NAME.is_match(entry.file_name().to_str().unwrap_or(""))
        })
        .count();

    Ok(count)
}

pub fn parse_file(filepath: PathBuf) -> io::Result<Vec<CnfFileRow>> {
    let mut records = Vec::new();

    let mut reader = READY_READER.from_path(filepath)?;
    reader.set_headers( StringRecord::from(HEADERS.to_vec()) );

    for result in reader.deserialize::<CnfFileRow>() {
        if let Ok(res) = result {
            records.push(res);
        }
    }

    Ok(records)
}

pub fn write_file<T>(records: Vec<T>, filepath: PathBuf) -> io::Result<()>
    where T: serde::Serialize
{
    let mut writer = READY_WRITER.from_path(filepath)?;

    for record in records {
        writer.serialize(record)?
    }

    Ok(())
}
