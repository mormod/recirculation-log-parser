

use std::{path::PathBuf, time::SystemTime, collections::{HashMap, HashSet}, ops::Deref};

mod parsers;
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

fn create_str_attr<T>(location: &T, name: &str, value: &str) -> hdf5::Result<()>
where
    T: Deref<Target = hdf5::Location>,
{
    let attr = location.new_attr::<hdf5::types::VarLenUnicode>().create(name)?;
    let value_: hdf5::types::VarLenUnicode = value.parse().unwrap();
    attr.write_scalar(&value_)
}

fn write_to_h5(collections: &Vec<CanMsgCollection>) -> hdf5::Result<()> {

    #[cfg(feature = "blosc")]
    hdf5::filters::blosc_set_nthreads(4);

    let root = hdf5::File::create("canlog.h5")?;
    create_str_attr::<hdf5::Group>(&root, "created", chrono::Local::now().to_rfc3339().as_str())?;

    for collection in collections {
        let str_id = match &collection.can_id.str_id {
            Some(str_id) => str_id.to_owned(),
            None => collection.can_id.hex_id.to_string()
        };
        
        let subgroup = root.create_group(str_id.as_str())?;
        
        subgroup.new_attr::<u32>().create("hex_id")?.write_scalar(&collection.can_id.hex_id)?;
        
        let desc = match &collection.can_id.description {
            Some(desc) => desc.as_str(),
            None => "None"
        };
        create_str_attr::<hdf5::Group>(&subgroup, "description", desc)?;

        let unit = match &collection.can_id.unit {
            Some(unit) => unit.as_str(),
            None => "None"
        };
        create_str_attr::<hdf5::Group>(&subgroup, "unit", unit)?;

        let scale = match &collection.can_id.scale {
            Some(scale) => scale,
            None => &1.0
        };
        subgroup.new_attr::<f32>().create("scale")?.write_scalar(scale)?;

        let dataset = subgroup.new_dataset_builder();
        
        #[cfg(feature = "blosc")]
        let dataset = dataset.blosc_zstd(9, true); // zstd + shuffle

        let dataset = dataset.with_data(&collection.collection);
        dataset.create(format!("{:#?}", str_id).as_str())?;
    }

    Ok(())
}

fn main() {
    let can_ids: Vec<CanId> = parse_canids(&PathBuf::from("src/ids.txt"));
    let mut all_can_ids = HashMap::new();
    for can_id in can_ids {
        all_can_ids.insert(can_id.hex_id, can_id);
    }

    // let comments: Vec<TVComment> = parse_comments(&PathBuf::from("src/cmt.txt"));
    // for comment in comments {
    //     println!("{:#?}", comment);
    // }

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

    let _ = write_to_h5(&split_by_id);

    println!("Not mappable:");
    for nm in not_mappable_ids {
        println!("\t{}", nm);
    }

    let end = SystemTime::now();
    println!("Took {} ms", end.duration_since(start).unwrap().as_millis());

}   