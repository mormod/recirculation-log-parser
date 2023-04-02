
use std::{path::{PathBuf, Path}, time::SystemTime, collections::{HashMap, HashSet}, ops::Deref};

use clap::Parser;

mod parsers;
use parsers::{
    parse_canids, CanId, 
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CanHdfCli {
    can_ids_path: PathBuf,
    can_log_path: PathBuf,
    output_path: PathBuf
}

fn create_str_attr<T>(location: &T, name: &str, value: &str) -> hdf5::Result<()>
where
    T: Deref<Target = hdf5::Location>,
{
    let attr = location.new_attr::<hdf5::types::VarLenUnicode>().create(name)?;
    let value_: hdf5::types::VarLenUnicode = value.parse().unwrap();
    attr.write_scalar(&value_)
}

fn write_to_hdf5<P: AsRef<Path>>(output_path: &P, collections: &Vec<CanMsgCollection>) -> hdf5::Result<()> {
    let root = hdf5::File::create(output_path)?;
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
        let dataset = dataset.with_data(&collection.collection);
        dataset.create(format!("{:#?}", str_id).as_str())?;
    }

    Ok(())
}

fn acquire_can_ids<P: AsRef<Path>>(path: &P) -> HashMap<u32, CanId> {
    let can_ids: Vec<CanId> = parse_canids(path);
    let mut all_can_ids = HashMap::new();
    for can_id in can_ids {
        all_can_ids.insert(can_id.hex_id, can_id);
    }
    all_can_ids
}

fn check_can_ids(can_msgs: &Vec<CanMsg>, can_ids: &HashMap<u32, CanId>) {
    let mut not_mappable_ids: HashSet<u32> = HashSet::new();

    for msg in can_msgs.iter() {
        match can_ids.get(&msg.hex_id) {
            Some(_) => {},
            None => {
                let _ = not_mappable_ids.insert(msg.hex_id);
            },
        };
    }

    println!("Not mappable:");
    for nm in not_mappable_ids {
        println!("\t{}", nm);
    }
}

fn create_collection(can_msgs: &Vec<CanMsg>, can_ids: &HashMap<u32, CanId>) -> Vec<CanMsgCollection> {
    let mut split_by_id: Vec<CanMsgCollection> = vec![];
    let mut last_id: u32= 0;
    for msg in can_msgs.iter() {
        if msg.hex_id > last_id {
            last_id = msg.hex_id;
            let can_id = match can_ids.get(&msg.hex_id) {
                None => CanId::empty_with_id(msg.hex_id),
                Some(id) => id.to_owned(),
            };
            split_by_id.push(
                CanMsgCollection::new(
                    can_id, 
                    *msg,
                )
            );
        }
        else {
            split_by_id.last_mut().unwrap().collection.push(msg.to_owned());
        }
    }
    split_by_id
}

fn acquire_and_combine<P: AsRef<Path>>(can_ids_path: &P, can_log_path: &P) -> Vec<CanMsgCollection>{

    let start = SystemTime::now();
    
    let can_ids = acquire_can_ids(can_ids_path);
    let mut can_msgs: Vec<CanMsg> = parse_messages(can_log_path);

    check_can_ids(&can_msgs, &can_ids);

    can_msgs.sort();
    let collection = create_collection(&can_msgs, &can_ids);

    let end = SystemTime::now();
    println!("Took {} ms", end.duration_since(start).unwrap().as_millis());

    collection
}

fn main() {
    let cli_input = CanHdfCli::parse();
    println!("Collecting CAN IDs from {:#?}", cli_input.can_ids_path.as_os_str());
    println!("Collecting CAN log from {:#?}", cli_input.can_log_path.as_os_str());
    let collection = acquire_and_combine(&cli_input.can_ids_path, &cli_input.can_log_path);
    println!("Writing to {:#?}", cli_input.output_path.as_os_str());
    let _ = write_to_hdf5(&cli_input.output_path, &collection);
}   