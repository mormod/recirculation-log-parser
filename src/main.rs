mod parsers;

use std::{path::PathBuf, time::SystemTime, collections::HashMap};

use parsers::{
    parse_canids, CanId, 
    parse_comments, TVComment, 
    parse_messages, CanMsg, CanTs,
};

fn main() {
    let can_ids: Vec<CanId> = parse_canids(&PathBuf::from("src/ids.txt"));
    let mut can_ids_hash = HashMap::new();
    for can_id in can_ids {
        can_ids_hash.insert(can_id.hex_id, can_id);
    }


    // let comments: Vec<TVComment> = parse_comments(&PathBuf::from("src/comments.txt"));
    // for comment in comments {
    //     println!("{:#?}", comment);
    // }

    let start = SystemTime::now();
    let messages: Vec<CanMsg> = parse_messages(&PathBuf::from("src/log.txt"));
    
    let mut not_mappable = Vec::<u32>::new();
    for msg in &messages {
        match can_ids_hash.get(&msg.hex_id) {
            Some(id) => {
                println!("Message {} corresponding to {}", msg, id)
            },
            None => not_mappable.push(msg.hex_id)
        };
    }

    for nm in not_mappable {
        println!("Not mappable: {}", nm);
    }

    let end = SystemTime::now();
    println!("Took {} ms", end.duration_since(start).unwrap().as_millis());

}   