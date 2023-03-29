mod parsers;

use std::{path::PathBuf, time::SystemTime, collections::{HashMap, HashSet}};

use parsers::{
    parse_canids, CanId, 
    // parse_comments, TVComment, 
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
    let mut messages: Vec<CanMsg> = parse_messages(&PathBuf::from("src/log.txt"));
    let mut not_mappable: HashSet<u32> = HashSet::new();

    for msg in messages.iter() {
        match can_ids_hash.get(&msg.hex_id) {
            Some(_) => {},
            None => {
                let _ = not_mappable.insert(msg.hex_id);
            },
        };
    }

    messages.sort();

    let mut split_by_id: Vec<Vec<CanMsg>> = vec![];
    let mut last_id: u32= 0;
    for msg in messages.iter_mut() {
        if msg.hex_id > last_id {
            last_id = msg.hex_id;
            split_by_id.push(vec![msg.to_owned()]);
        }
        else {
            split_by_id.last_mut().unwrap().push(msg.to_owned());
        }
    }

    println!("{:#?}", split_by_id[6]);

    println!("Not mappable:");
    for nm in not_mappable {
        println!("\t{}", nm);
    }

    let end = SystemTime::now();
    println!("Took {} ms", end.duration_since(start).unwrap().as_millis());

}   