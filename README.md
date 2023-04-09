# Running without installing
Use `cargo run --release -- <OUT_PATH> <IDS_PATH> <LOG_PATH>... [-c <COMMENTS_PATH>]` to run.
Use `cargo run --release -- -h` for further help.

# Installing
Use `cargo install --path .` in this directory. The `can-parser` executable will be stored in `~/.cargo/bin/`. 
Make sure to add this to your path, if you want to run this comfortably.

# Enabling debug messages
This package uses `env_logging`. It defaults to a minimal message level of `info`.
To enable more verbose debug messages, run this with `env RUST_LOG=level` with `level` in the set `{trace, debug, info , warn, error}`.

# Help
```
Uses a SmartECLA_IDs.h file (or similar) to parse a legacy CAN log to HDF5.
In the HDF5 file, every CAN ID is stored as a dataset in the 'CAN_IDs' group.
Every dataset has four attributes: hex_id (u32), unit (String), scale (f32) and description (String).
Every entry in every dataset is stored along with the time it was acquired.
When a comments file is supplied, comments will be stored in /COMMENTS.

Usage: can-parser [OPTIONS] <OUTPUT_PATH> <CAN_IDS_PATH> <CAN_LOG_PATHS>...

Arguments:
  <OUTPUT_PATH>       Path, where the resulting HDF file should be written to
  <CAN_IDS_PATH>      Path to SmartECLA_IDs.h (or similar)
  <CAN_LOG_PATHS>...  Path to all CAN log files of the experiment

Options:
  -e                      Indicate extended CAN logs (with hex data representations)
  -c <COMMENTS_PATH>      Also parse comments file
  -h, --help              Print help
  -V, --version           Print version
```