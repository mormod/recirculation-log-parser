use std::path::{PathBuf};

use parse_smartecla_header::{CanId, gather_canid_infos};

mod parse_smartecla_header;

fn main() {
    let can_ids:Vec<CanId> = gather_canid_infos(&PathBuf::from("src/SmartECLA_IDs.h"));
    for can_id in can_ids {
        println!("{:?}", can_id);
    }
}