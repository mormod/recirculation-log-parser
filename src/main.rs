
use std::{path::{PathBuf, Path}, time::SystemTime, collections::{HashMap, HashSet}};
use clap::Parser;
use hdf5::{Location};
use hdf5::types::VarLenUnicode;

mod parsers;
use parsers::{
    parse_canids, CanId, 
    parse_messages, CanMsg,
};

#[derive(Debug)]
struct CanMsgCollection {
    can_id: CanId,
    collection: Vec<CanMsg>,
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
#[command(author, version, about)]
struct CanHdfCli {
    /// Path to SmartECLA_IDs.h (or similar)
    can_ids_path: PathBuf,
    
    /// Path to the CAN log file
    can_log_paths: Vec<PathBuf>,
    
    /// Path, where the resulting HDF file should be written to
    output_path: PathBuf,

    /// Indicate extended CAN logs (with hex data representations)
    #[arg(short)]
    extended_log: bool
}

fn create_str_attr(location: &Location, name: &str, value: &str) -> hdf5::Result<()>
{
    let attr = location.new_attr::<VarLenUnicode>().create(name)?;
    let value_: VarLenUnicode = value.parse().unwrap();
    attr.write_scalar(&value_)
}

fn write_to_hdf5<P: AsRef<Path>>(output_path: &P, collections: &Vec<CanMsgCollection>) -> hdf5::Result<()> {
    let root = hdf5::File::create(output_path)?;
    create_str_attr(&root, "created", chrono::Local::now().to_rfc3339().as_str())?;

    for collection in collections {
        let str_id = match &collection.can_id.str_id {
            Some(str_id) => str_id.to_owned(),
            None => collection.can_id.hex_id.to_string()
        }; 

        let dataset = root.new_dataset_builder();
        let dataset = dataset.with_data(&collection.collection);
        let dataset = dataset.set_filters(&[hdf5::filters::Filter::Deflate(5)]);
        let dataset = dataset.create(format!("{:#?}", str_id).as_str())?;

        dataset.new_attr::<u32>()
            .create("hex_id")?
            .write_scalar(&collection.can_id.hex_id)?;
        
        let desc = match &collection.can_id.description {
            Some(desc) => desc.as_str(),
            None => "None"
        };
        create_str_attr(&dataset, "description", desc)?;

        let unit = match &collection.can_id.unit {
            Some(unit) => unit.as_str(),
            None => "None"
        };
        create_str_attr(&dataset, "unit", unit)?;

        let scale = match &collection.can_id.scale {
            Some(scale) => scale,
            None => &1.0
        };
        dataset.new_attr::<f32>()
            .create("scale")?
            .write_scalar(scale)?;

        
        log::trace!("HDF5 Filters: {:?}", dataset.filters());
        log::debug!("Written subgroup {}", str_id);

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

    for nm in not_mappable_ids {
        log::warn!("Not mappable: {}", nm);
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

fn main() {
    pretty_env_logger::init();

    let start = SystemTime::now();
    
    let cli_input = CanHdfCli::parse();

    log::info!("Collecting CAN IDs from {:#?}", cli_input.can_ids_path.as_os_str());
    let can_ids = acquire_can_ids(&cli_input.can_ids_path);

    let mut can_msgs: Vec<CanMsg> = Vec::new();
    for log_path in cli_input.can_log_paths {
        log::info!("Collecting CAN log from {:#?}", log_path.as_os_str());
        can_msgs.append(&mut parse_messages(&log_path, cli_input.extended_log));
    }
    
    check_can_ids(&can_msgs, &can_ids);
    can_msgs.sort();

    log::debug!("Writing to {:#?}...", cli_input.output_path.as_os_str());
    let collection = create_collection(&can_msgs, &can_ids);
    let _ = write_to_hdf5(&cli_input.output_path, &collection);

    let end = SystemTime::now();
    log::debug!("Took {} ms", 
        end.duration_since(start)
            .unwrap()
            .as_millis()
    );
    log::debug!("Identified {} CAN IDs.", can_ids.len());
    log::debug!("Wrote {} datasets.", collection.len());

    log::info!("{}", humansize::format_size(
        std::fs::metadata(cli_input.output_path).unwrap().len(), 
        humansize::DECIMAL)
    );
}   