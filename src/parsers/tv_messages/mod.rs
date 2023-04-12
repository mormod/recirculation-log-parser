use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::{
        complete::{digit1, space1},
        is_hex_digit, is_space,
    },
    combinator::opt,
    error::ParseError,
    multi::separated_list1,
    number::complete::float,
    sequence::{preceded, tuple},
    IResult,
};
use nom_supreme::{error::ErrorTree, final_parser::final_parser};
use std::str::from_utf8;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use super::common::bytes_to_number;

use super::common::{bytes_to_string, handle_error, Span};

pub mod can_msg;
pub use can_msg::CanMsg;

pub type FnCanMsgParser<'a> =
    fn(Span<'a>) -> IResult<Span<'a>, Option<CanMsg>, ErrorTree<Span<'a>>>;

fn to_timestamp(
    hour: u64,
    min: u64,
    second: u64,
    subsec: u64,
    digits: u32,
    did_surpass_midnight: bool,
) -> u64 {
    // Measure all in nanoseconds
    static SUBSEC_UNIT: u64 = 1_000_000_000;
    // Hour modifier, if we surpassed midnight.
    let modifier: u64 = if did_surpass_midnight { 24 } else { 0 };
    // Mostly, subsecond units are not given with the righ amount of
    // digits and are therefore parsed wrong. Correct for that here.
    let millis_scalar: u64 = SUBSEC_UNIT / 10_u64.pow(digits);

    return (millis_scalar * subsec)
        + (SUBSEC_UNIT * second)
        + (SUBSEC_UNIT * 60_u64 * min)
        + (SUBSEC_UNIT * 60_u64 * 60_u64 * (hour + modifier));
}

fn parse_ts<'a, E: ParseError<Span<'a>>>(raw_ts: Span<'a>) -> IResult<Span<'a>, u64, E> {
    static mut LAST_HOUR: u64 = 12;
    static mut DID_SURPASS_MIDNIGHT: bool = false;

    // Timestamps are fucked.
    // Due to the misfortunate format of timestamps throughout all log files, we have to do something
    // less straight forward than parsing from "normal" time.
    // Logs might surpass midnight, which may or may not be inidcated by a preceeding "1." before the
    // current time. Sometimes, a leading "0." indicates that we did not surpass midnight yet.
    // If midnight is surpassed, "0." may or may not change to "1.".
    //
    // [0.|1.]HH:MM:SS.[0-9]{1..4}
    let (r, (day_indicator, hour_raw, _, min_raw, _, sec_raw, millis)) = tuple((
        opt(tuple((digit1, tag(".")))),
        digit1,
        tag(":"),
        digit1,
        tag(":"),
        digit1,
        opt(tuple((tag("."), digit1))),
    ))(raw_ts)?;

    let (hour, _) = bytes_to_number(hour_raw);
    let (min, _) = bytes_to_number(min_raw);
    let (sec, _) = bytes_to_number(sec_raw);

    let (subsec, digits): (u64, u32) = if let Some((_, subsec_raw)) = millis {
        bytes_to_number(subsec_raw)
    } else {
        (0, 0)
    };

    #[allow(unused_assignments)]
    let mut ts = 0;
    unsafe {
        let midnight_indicator = match day_indicator {
            // Parse "digit" from a byte to a string and check whether it is greater 0.
            // The digit is encoded in UTF-8, which enumerates digits starting with "30" for 0.
            Some((digit, _)) => {
                let (digit, _) = bytes_to_number(digit);
                digit > 0
            }
            None => false,
        };
        let hour_diff = (LAST_HOUR - hour) == 23;
        DID_SURPASS_MIDNIGHT = midnight_indicator || hour_diff;
        if DID_SURPASS_MIDNIGHT {
            log::debug!("Timestamps wrapped over at midnight!");
            log::trace!("{}", bytes_to_string(raw_ts));
            log::trace!(
                "\tParsed to: {}.{hour}:{min}:{sec}.{subsec} > {ts}",
                bytes_to_string(
                    day_indicator
                        .unwrap_or((Span::new("0".as_bytes()), Span::new(".".as_bytes())))
                        .0
                )
            );
            log::trace!(
                "\tLAST_HOUR: {LAST_HOUR}, hour_diff: {hour_diff} => {DID_SURPASS_MIDNIGHT}"
            );
        }
        LAST_HOUR = hour;
        ts = to_timestamp(hour, min, sec, subsec, digits, DID_SURPASS_MIDNIGHT);
    }

    Ok((r, ts))
}

fn parse_spaces<'a>(line: Span<'a>) -> IResult<Span<'a>, Span<'a>, ErrorTree<Span<'a>>> {
    space1(line)
}

//100C0000h	8	2E 00 00 00 01 00 00 00 	0,44921875 L/min	1	 average Blood Flow 		08:44:04.97
fn parse_extended<'a, E: ParseError<Span<'a>>>(
    line: Span<'a>,
) -> IResult<Span<'a>, Option<CanMsg>, E> {
    let mut can_msg: Option<CanMsg> = None;

    if line.len() > 0 {
        let parse_res = separated_list1(parse_spaces, take_while(|c| !is_space(c)))(line);
        if parse_res.is_err() {
            unreachable!("This should never happen.");
        }

        let (_, mut list) = parse_res.unwrap();

        if list.len() <= 10 {
            // This is not a "real" CAN log message. It might be a comment. Just ignore those
            return Ok((Span::new("".as_bytes()), can_msg));
        }

        for i in 0..(list.len() - 1) {
            if bytes_to_string(*list.get(i).unwrap()) == String::from("") {
                list.remove(i);
            }
        }

        // CanId is always the first item. If its not, this is not a valid line
        let (_, hex_id_raw) =
            preceded(alt((tag("0x"), tag(""))), take_while(is_hex_digit))(list[0])?;

        let hex_id_res = u32::from_str_radix(bytes_to_string(hex_id_raw).as_str(), 16);

        if hex_id_res.is_err() {
            return Ok((Span::new("".as_bytes()), can_msg));
        }
        let hex_id = hex_id_res.unwrap();
        let (_, value) = float(list[10])?;

        let (_, ts) = parse_ts(list[list.len() - 1])?;

        can_msg = Some(CanMsg { hex_id, value, ts });
    }

    Ok((Span::new("".as_bytes()), can_msg))
}

// 0x10FE0102	0 Prozent	08:52:25.19
fn parse_simple<'a, E: ParseError<Span<'a>>>(
    line: Span<'a>,
) -> IResult<Span<'a>, Option<CanMsg>, E> {
    let mut can_msg: Option<CanMsg> = None;

    if line.len() > 0 {
        let parse_res = separated_list1(parse_spaces, take_while(|c| !is_space(c)))(line);

        if parse_res.is_err() {
            unreachable!("This should never happen.");
        }

        let (_, list) = parse_res.unwrap();

        if list.len() <= 2 {
            // This is an incomplete log entry
            return Ok((Span::new("".as_bytes()), None));
        }

        // CanId is always the first item. If its not, this is not a valid line
        let hex_id_res =
            tuple((tag::<_, _, ErrorTree<Span>>("0x"), take_while(is_hex_digit)))(list[0]);
        if hex_id_res.is_err() {
            return Ok((Span::new("".as_bytes()), can_msg));
        }
        let (_, (_, hex_id_raw)) = hex_id_res.unwrap();
        let hex_id_res = u32::from_str_radix(from_utf8(hex_id_raw.as_ref()).unwrap().trim(), 16);

        let hex_id = hex_id_res.unwrap();
        let value: f32 = bytes_to_string(list[1])
            .trim()
            .replace(',', ".")
            .parse()
            .unwrap();
        let (_, ts) = parse_ts(list[list.len() - 1])?;

        can_msg = Some(CanMsg { hex_id, value, ts });
    }

    Ok((Span::new("".as_bytes()), can_msg))
}

pub fn parse_messages<'a, P: AsRef<Path>>(log_file: &P, is_extended: bool) -> Vec<CanMsg> {
    if is_extended {
        log::debug!("Using extended parser!");
    }
    let mut lnr = 1;
    let mut can_msgs = Vec::<CanMsg>::new();
    let mut line_buf = vec![];
    match File::open(log_file) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            while let Ok(_) = reader.read_until(b'\n', &mut line_buf) {
                if line_buf.is_empty() {
                    break;
                }
                let parser: FnCanMsgParser = if is_extended {
                    parse_extended
                } else {
                    parse_simple
                };
                let parse = final_parser(parser)(Span::new(&line_buf[..]));
                match parse {
                    Ok(possible_can_msg) => {
                        if let Some(can_msg) = possible_can_msg {
                            can_msgs.push(can_msg);
                        }
                    }
                    Err(e) => {
                        handle_error(String::from_utf8_lossy(&line_buf).to_string(), e);
                    }
                }
                line_buf.clear();

                if lnr % 5000000 == 0 {
                    log::trace!("Still at it...")
                }
                lnr += 1
            }
        }
        Err(e) => println!("{e:#?}"),
    };

    can_msgs
}
