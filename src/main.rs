mod parsers;

use std::{path::PathBuf, time::SystemTime, collections::{HashMap, HashSet}};

use parsers::{
    parse_canids, CanId, 
    parse_comments, TVComment, 
    parse_messages, CanMsg,
};

#[derive(Debug)]
struct CanMsgCollection {
    can_id: CanId,
    collection: Vec<CanMsg>
}

impl CanMsgCollection {
    fn new(can_id: CanId, can_msg: CanMsg) -> Self {
        Self {
            can_id,
            collection: vec![can_msg]
        }
    }
}

fn main() {
    let can_ids: Vec<CanId> = parse_canids(&PathBuf::from("src/ids.txt"));
    let mut all_can_ids = HashMap::new();
    for can_id in can_ids {
        all_can_ids.insert(can_id.hex_id, can_id);
    }

    let comments: Vec<TVComment> = parse_comments(&PathBuf::from("src/cmt.txt"));
    for comment in comments {
        println!("{:#?}", comment);
    }

    let start = SystemTime::now();
    let mut messages: Vec<CanMsg> = parse_messages(&PathBuf::from("src/log.txt"));
    let mut not_mappable_ids: HashSet<u32> = HashSet::new();

    for msg in messages.iter() {
        match all_can_ids.get(&msg.hex_id) {
            Some(_) => {},
            None => {
                let _ = not_mappable_ids.insert(msg.hex_id);
                all_can_ids.remove(&msg.hex_id);
            },
        };
    }

    messages.sort();

    let mut split_by_id: Vec<CanMsgCollection> = vec![];
    let mut last_id: u32= 0;
    for msg in messages.iter_mut() {
        if msg.hex_id > last_id {
            last_id = msg.hex_id;
            split_by_id.push(
                CanMsgCollection::new(
                    all_can_ids.get(&msg.hex_id).unwrap_or(&CanId::empty_with_id(msg.hex_id)).to_owned(), 
                    *msg,
                )
            );
        }
        else {
            split_by_id.last_mut().unwrap().collection.push(msg.to_owned());
        }
    }

    println!("Not mappable:");
    for nm in not_mappable_ids {
        println!("\t{}", nm);
    }

    let end = SystemTime::now();
    println!("Took {} ms", end.duration_since(start).unwrap().as_millis());

}   