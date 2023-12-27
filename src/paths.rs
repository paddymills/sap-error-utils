
//! Path tools for confirmation files
// TODO: refactor into paths module

use regex::Regex;
use std::io::Error;
use std::path::{Path, PathBuf};

lazy_static! {
    /// Base confirmation files folder
    pub static ref CNF_FILES: &'static Path = Path::new(r"\\hssieng\SNData\SimTrans\SAP Data Files");
    /// Confirmation files outbox (to be picked up by workflow)
    pub static ref CNF_OUTBOX: &'static Path = Path::new(r"\\hssieng\SNData\SimTrans\Outbox");

    /// Confirmation file outbound (where SAP workflow picks them up from)
    pub static ref SAP_OUTBOUND: &'static Path = Path::new(r"\\hiifileserv1\sigmanestprd\Outbound");
    
    /// SAP archive for confirmation, issue, stock, etc. files
    pub static ref SAP_ARCHIVE: &'static Path = Path::new(r"\\hiifileserv1\sigmanestprd\Archive");

    /// Production file pattern
    pub static ref PROD_FILE_NAME: Regex = Regex::new(r"Production_(\d{14}).(?:ready|outbound\.archive)").expect("failed to build regex");
}

/// Create a filename with a naturally sortable timestamp
/// 
/// returns a formatted string `{prefix}_{year}{month}{day}{hour}{minute}{seconds}.{ext}`
pub fn timestamped_file(prefix: &str, ext: &str) -> String {
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();

    format!("{}_{}.{}", prefix, timestamp, ext)
}

/// Get all confirmation files to be processed
pub fn get_ready_files() -> Result<Vec<PathBuf>, Error> {
    let files = std::fs::read_dir(&*CNF_FILES)?
        .filter_map(|f| f.ok())
        .filter(|f| PROD_FILE_NAME.is_match(f.file_name().to_str().unwrap_or("skip file")))
        .map(|f| f.path().to_path_buf())
        .collect::<Vec<PathBuf>>();

    Ok(files)
}

/// Confirmation file path functions to extend to [`std::path::PathBuf`]
pub trait CnfFilePaths {
    /// Create a new production file name from current timestamp
    fn new_prod_file() -> Self;
    /// Create a new issue file name from current timestamp
    fn new_issue_file() -> Self;
    /// Create an archive file name from an existing file name
    fn archive_file(self: &Self) -> Self;
    /// Create an backup file name from an existing file name
    fn backup_file(self: &Self) -> Self;
    /// Create an issue file name from an existing file name
    fn production_file(self: &Self) -> Self;
    /// Create an issue file name from an existing file name
    fn issue_file(self: &Self) -> Self;
}

impl CnfFilePaths for PathBuf {
    fn new_prod_file() -> Self {
        // CNF_FILES.join( chrono::Local::now().format("Production_%Y%m%d%H%M%S.ready").to_string() )
        CNF_FILES.join( timestamped_file("Production", "ready") )
    }

    fn new_issue_file() -> Self {
        // CNF_FILES.join( chrono::Local::now().format("Issue_%Y%m%d%H%M%S.ready").to_string() )
        CNF_FILES.join( timestamped_file("Issue", "ready") )
    }
    
    fn archive_file(self: &Self) -> Self {
        let mut path = CNF_FILES.join( "processed" );

        // safe to unwrap Option<&OsStr> here
        //  because we will assume whoever consumes this api
        //  is not dumb enough to call it on a folder or path ending in '..'
        path.push(self.file_name().unwrap());
    
        path
    }
    
    fn backup_file(self: &Self) -> Self {
        let mut path = CNF_FILES.join( "original" );

        // safe to unwrap Option<&OsStr> here
        //  because we will assume whoever consumes this api
        //  is not dumb enough to call it on a folder or path ending in '..'
        path.push(self.file_name().unwrap());
    
        path
    }

    fn production_file(self: &Self) -> Self {
        let mut path = CNF_OUTBOX.to_path_buf();

        // safe to unwrap Option<&OsStr> and Option<&str> here
        //  because we already checked that it is a file
        path.push( self.file_name().unwrap().to_str().unwrap() );
    
        path
    }
    
    fn issue_file(self: &Self) -> Self {
        let mut path = CNF_OUTBOX.to_path_buf();

        // safe to unwrap Option<&OsStr> and Option<&str> here
        //  because we already checked that it is a file
        path.push( self.file_name().unwrap().to_str().unwrap().replace("Production", "Issue") );
    
        path
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_paths() {
        let test_file = PathBuf::from(r"\\hssieng\SNData\SimTrans\SAP Data Files\Production_20220105083000.ready");
        assert_eq!(test_file.archive_file(), PathBuf::from(r"\\hssieng\SNData\SimTrans\SAP Data Files\processed\Production_20220105083000.ready"));
        assert_eq!(test_file.backup_file(), PathBuf::from(r"\\hssieng\SNData\SimTrans\SAP Data Files\original\Production_20220105083000.ready"));
        assert_eq!(test_file.production_file(), PathBuf::from(r"\\hssieng\SNData\SimTrans\Outbox\Production_20220105083000.ready"));
        assert_eq!(test_file.issue_file(), PathBuf::from(r"\\hssieng\SNData\SimTrans\Outbox\Issue_20220105083000.ready"));
    }
}
