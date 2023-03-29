mod parsers;

use std::path::PathBuf;

use parsers::{
    parse_canids, CanId, 
    parse_comments, TVComment
};

fn main() {
    let can_ids: Vec<CanId> = parse_canids(&PathBuf::from("src/SmartECLA_IDs.h"));
    for can_id in can_ids {
        println!("{:?}", can_id);
    }

    let comments: Vec<TVComment> = parse_comments(&PathBuf::from("src/comments.txt"));
    for comment in comments {
        println!("{:#?}", comment);
    }
}   