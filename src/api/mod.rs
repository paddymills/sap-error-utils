
// TODO: migrate use of part/matl data to separate structs and flatten with serde
//  current cannot do with csv crate: https://github.com/BurntSushi/rust-csv/issues/98

// TODO: refactor use of String to Box<str> in fixed length strings (wbs, plant, etc)
//  saves 8 bytes of memory since length and capacity do not need to be tracked
//  see: https://mahdi.blog/rust-box-str-vs-string/

mod cnf_row;
mod issue_row;
mod order;
mod plant;
mod wbs;

pub use cnf_row::CnfFileRow;
pub use issue_row::IssueFileRow;
pub use order::{Order, OrderData};
pub use plant::Plant;
pub use wbs::Wbs;

mod cnf_serde {
    use serde::{self, Serializer};

    pub fn three_digit_f64<S>(val: &f64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:.3}", val))
    }
}

