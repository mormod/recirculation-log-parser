use clap::Parser;
use hdf5::types::VarLenUnicode;
use hdf5::Location;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::SystemTime,
};

mod parsers;
use parsers::{parse_canids, parse_comments, parse_messages, CanCmt, CanId, CanMsg};
use pretty_env_logger::env_logger::{Builder, Env};

#[derive(Debug)]
struct CanMsgCollection {
    can_id: CanId,
    collection: Vec<CanMsg>,
}

impl CanMsgCollection {
    fn new(can_id: CanId, can_msg: CanMsg) -> Self {
        Self {
            can_id,
            collection: vec![can_msg],
        }
    }
}

#[derive(Parser)]
#[command(author, version)]
#[command(
    about = "Uses a SmartECLA_IDs.h file (or similar) to parse a legacy CAN log to HDF5.
In the HDF5 file, every CAN ID is stored as a dataset in the 'CAN_IDs' group.
Every dataset has four attributes: hex_id (u32), unit (String), scale (f32) and description (String).
Every entry in every dataset is stored along with the time it was acquired.
When a comments file is supplied, comments will be stored in /COMMENTS."
)]
struct CanHdfCli {
    /// Path, where the resulting HDF file should be written to
    output_path: PathBuf,

    /// Path to SmartECLA_IDs.h (or similar)
    can_ids_path: PathBuf,

    /// Path to all CAN log files of the experiment.
    #[arg(required = true)]
    can_log_paths: Vec<PathBuf>,

    /// Indicate extended CAN logs (with hex data representations)
    #[arg(short)]
    extended_log: bool,

    /// Also parse comments file
    #[arg(short)]
    comments_path: Option<PathBuf>,
}

struct CanMeta<'a> {
    cli: &'a CanHdfCli,
    time_ms: u128,
    old_size_b: u64,
    least_trailing_zeros: u32,
}

fn create_str_attr(location: &Location, name: &str, value: &str) -> hdf5::Result<()> {
    let attr = location.new_attr::<VarLenUnicode>().create(name)?;
    let value_: VarLenUnicode = value.parse().unwrap();
    attr.write_scalar(&value_)
}

fn write_to_hdf5<'a, P: AsRef<Path>>(
    output_path: &P,
    collections: &Vec<CanMsgCollection>,
    can_cmts: &Vec<CanCmt>,
    meta: &'a CanMeta,
) -> hdf5::Result<()> {
    let root = hdf5::File::create(output_path)?;
    create_str_attr(&root, "created", chrono::Local::now().to_rfc3339().as_str())?;
    create_str_attr(
        &root,
        "CAN IDs file",
        meta.cli.can_ids_path.as_os_str().to_str().unwrap(),
    )?;
    root.new_attr::<usize>()
        .create("Processing time")?
        .write_scalar(&(meta.time_ms as usize))?;
    root.new_attr::<usize>()
        .create("Dataset count")?
        .write_scalar(&collections.len())?;
    root.new_attr::<u64>()
        .create("Previous size [B]")?
        .write_scalar(&meta.old_size_b)?;
    root.new_attr::<u32>()
        .create("TS least trailing zeros")?
        .write_scalar(&meta.least_trailing_zeros)?;

    let ds_group = root.create_group("CAN_IDs")?;

    for collection in collections {
        let str_id = match &collection.can_id.str_id {
            Some(str_id) => str_id.to_owned(),
            None => collection.can_id.hex_id.to_string(),
        };

        let dataset = ds_group
            .new_dataset_builder()
            .with_data(&collection.collection)
            .set_filters(&[hdf5::filters::Filter::Deflate(5)])
            .create(str_id.as_str())?;

        dataset
            .new_attr::<u32>()
            .create("hex_id")?
            .write_scalar(&collection.can_id.hex_id)?;

        let desc = match &collection.can_id.description {
            Some(desc) => desc.as_str(),
            None => "None",
        };
        create_str_attr(&dataset, "description", desc)?;

        let unit = match &collection.can_id.unit {
            Some(unit) => unit.as_str(),
            None => "None",
        };
        create_str_attr(&dataset, "unit", unit)?;

        let scale = match &collection.can_id.scale {
            Some(scale) => scale,
            None => &1.0,
        };
        dataset
            .new_attr::<f32>()
            .create("scale")?
            .write_scalar(scale)?;

        log::debug!("Written dataset {}", str_id);
    }

    if !can_cmts.is_empty() {
        root.new_dataset_builder()
            .with_data(&can_cmts)
            .set_filters(&[hdf5::filters::Filter::Deflate(5)])
            .create("COMMENTS")?;

        create_str_attr(
            &root,
            "Comments file path",
            meta.cli
                .comments_path
                .as_ref()
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap(),
        )?;

        for logfile in &meta.cli.can_log_paths {
            create_str_attr(&root, "Log file ", logfile.as_os_str().to_str().unwrap())?;
        }
    }

    log::debug!("Wrote comments to COMMENTS");

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

fn check_can_ids(can_msgs: &Vec<CanMsg>, can_ids: &mut HashMap<u32, CanId>) {
    let mut not_mappable_ids: HashSet<u32> = HashSet::new();

    for msg in can_msgs.iter() {
        if let None = can_ids.get(&msg.hex_id) {
            // We still might be able to match: IDs can have device information
            if let Some(id) = can_ids.get(&(msg.hex_id & !(0xF << 12))) {
                // There is a match!
                log::debug!("Found {} for unmatched {}", id, msg.hex_id);
                let mut new_str_id = id.str_id.to_owned().unwrap();
                new_str_id.push_str(format!("-DEV{}", (msg.hex_id >> 12) & 0xF).as_str());

                let new_can_id = CanId {
                    hex_id: msg.hex_id,
                    str_id: Some(new_str_id),
                    scale: id.scale,
                    description: id.description.clone(),
                    unit: id.unit.clone(),
                };

                log::debug!("\tCreated new CAN ID: {}", new_can_id);

                can_ids.insert(msg.hex_id, new_can_id);
            } else {
                // The information is missing completely
                let _ = not_mappable_ids.insert(msg.hex_id);
            }
        }
    }

    for nm in not_mappable_ids {
        log::warn!("Not mappable: {}", nm);
    }
}

fn create_collection(
    can_msgs: &Vec<CanMsg>,
    can_ids: &HashMap<u32, CanId>,
) -> Vec<CanMsgCollection> {
    let mut collection: Vec<CanMsgCollection> = Vec::new();
    let mut last_id: u32 = 0;
    for msg in can_msgs.iter() {
        if msg.hex_id > last_id {
            // This is a new CAN ID. We thus have to create a new collection first.
            last_id = msg.hex_id;

            // Construct the CAN ID to be inserted into the collection.
            let can_id = match can_ids.get(&msg.hex_id) {
                None => CanId::empty_with_id(msg.hex_id),
                Some(id) => id.to_owned(),
            };
            log::trace!("{}", can_id);
            collection.push(CanMsgCollection::new(can_id, *msg));
        } else {
            collection.last_mut().unwrap().collection.push(*msg);
        }
    }

    collection
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let start = SystemTime::now();

    let cli_input = CanHdfCli::parse();

    log::info!(
        "Collecting CAN IDs from {:#?}",
        cli_input.can_ids_path.as_os_str()
    );
    let mut can_ids = acquire_can_ids(&cli_input.can_ids_path);

    let mut total_size_b = 0;
    let mut can_msgs: Vec<CanMsg> = Vec::new();
    for (i, log_path) in cli_input.can_log_paths.iter().enumerate() {
        log::info!(
            "Parsing log file {:#?} ({}/{})...",
            log_path.as_os_str(),
            i + 1,
            cli_input.can_log_paths.len()
        );
        can_msgs.append(&mut parse_messages(&log_path, cli_input.extended_log));
        total_size_b += std::fs::metadata(&log_path).unwrap().len();
    }

    let mut trailing_zeros = 9;
    for can_msg in &can_msgs {
        let msg_trailing = can_msg.ts.trailing_zeros();
        if msg_trailing < trailing_zeros {
            trailing_zeros = msg_trailing;
        }
    }

    let mut can_cmts: Vec<CanCmt> = Vec::new();
    if let Some(comments_path) = &cli_input.comments_path {
        log::info!("Parsing comments from {:#?}...", comments_path.as_os_str());
        can_cmts.append(&mut parse_comments(&comments_path));
    }

    check_can_ids(&can_msgs, &mut can_ids);
    can_msgs.sort();

    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap().as_millis();
    log::info!("Took {} ms", duration);

    let meta = CanMeta {
        cli: &cli_input,
        time_ms: duration,
        old_size_b: total_size_b,
        least_trailing_zeros: trailing_zeros,
    };

    log::info!("Writing to {:#?}...", cli_input.output_path.as_os_str());
    let collection = create_collection(&can_msgs, &can_ids);
    let _ = write_to_hdf5(&cli_input.output_path, &collection, &can_cmts, &meta);

    log::debug!("Identified {} CAN IDs.", can_ids.len());

    if collection.len() == 0 {
        log::error!("Wrote 0 datasets! Maybe try using the parser for extended log files (-e)");
    } else {
        log::info!("Wrote {} datasets.", collection.len());
    }

    let new_size_b = std::fs::metadata(&cli_input.output_path).unwrap().len();
    log::info!(
        "{:?}: {} ({} B from {} B)",
        cli_input.output_path.as_os_str(),
        humansize::format_size(new_size_b, humansize::DECIMAL),
        new_size_b,
        total_size_b
    );
}
