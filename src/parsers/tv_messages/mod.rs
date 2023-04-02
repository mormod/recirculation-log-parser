use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::{
        complete::{digit1, space1, space0},
        is_hex_digit, is_space,
    },
    error::ParseError,
    multi::separated_list1,
    number::complete::float,
    sequence::tuple,
    IResult,
};
use nom_supreme::{error::ErrorTree, final_parser::final_parser};
use std::str::from_utf8;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use super::common::{bytes_to_string, handle_error, to_timestamp, Span};

pub mod can_msg;
pub use can_msg::CanMsg;

pub type FnCanMsgParser<'a> = fn(Span<'a>) -> IResult<Span<'a>, Option<CanMsg>, ErrorTree<Span<'a>>>;

fn parse_spaces<'a>(line: Span<'a>) -> IResult<Span<'a>, Span<'a>, ErrorTree<Span<'a>>> {
    space1(line)
}

fn parse_ts<'a, E: ParseError<Span<'a>>>(raw_ts: Span<'a>) -> IResult<Span<'a>, u32, E> {
    let (r, list) = separated_list1(alt((tag(":"), tag("."))), digit1)(raw_ts)?;
    let hour: u32 = bytes_to_string(list[0]).parse().unwrap();
    let min: u32 = bytes_to_string(list[1]).parse().unwrap();
    let sec: u32 = bytes_to_string(list[2]).parse().unwrap();

    let millis = if list.len() > 3 {
        bytes_to_string(list[3]).parse::<u32>().unwrap()
    } else {
        0
    };

    Ok((r, to_timestamp(hour, min, sec, millis)))
}

//100C0000h	8	2E 00 00 00 01 00 00 00 	0,44921875 L/min	1	 average Blood Flow 		08:44:04.97
fn parse_extended<'a, E: ParseError<Span<'a>>>(
    line: Span<'a>,
) -> IResult<Span<'a>, Option<CanMsg>, E> {
    let mut can_msg: Option<CanMsg> = None;

    if line.len() > 0 {
        let parse_res = separated_list1(
            parse_spaces,
            take_while(|c| !is_space(c)),
        )(line);
        if parse_res.is_err() {
            unreachable!("This should never happen.");
        }

        let (_, mut list) = parse_res.unwrap();

        for i in 0..(list.len()-1) {
            if bytes_to_string(*list.get(i).unwrap()) == String::from("") {
                list.remove(i);
            }
        }

        // CanId is always the first item. If its not, this is not a valid line
        let (_, hex_id_raw) = take_while(is_hex_digit)(list[0])?;

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
        let parse_res = separated_list1(
            parse_spaces,
            take_while(|c| !is_space(c)),
        )(line);

        if parse_res.is_err() {
            unreachable!("This should never happen.");
        }

        let (_, list) = parse_res.unwrap();

        // CanId is always the first item. If its not, this is not a valid line
        let hex_id_res =
            tuple((tag::<_, _, ErrorTree<Span>>("0x"), take_while(is_hex_digit)))(list[0]);
        if hex_id_res.is_err() {
            return Ok((Span::new("".as_bytes()), can_msg));
        }
        let (_, (_, hex_id_raw)) = hex_id_res.unwrap();
        let hex_id_res = u32::from_str_radix(from_utf8(hex_id_raw.as_ref()).unwrap().trim(), 16);

        let hex_id = hex_id_res.unwrap();
        let (_, value) = float(list[1])?;
        let (_, ts) = parse_ts(list[list.len() - 1])?;

        can_msg = Some(CanMsg { hex_id, value, ts });
    }

    Ok((Span::new("".as_bytes()), can_msg))
}

pub fn parse_messages<'a, P: AsRef<Path>>(log_file: &P, is_extended: bool) -> Vec<CanMsg> {
    if is_extended {
        log::debug!("Using extended parser!");
    }

    let mut can_msgs = Vec::<CanMsg>::new();
    match File::open(log_file) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        let parser: FnCanMsgParser = if is_extended {
                            parse_extended
                        }
                        else {
                            parse_simple
                        };
                        let parse = final_parser(parser)(Span::new(
                            line.as_bytes(),
                        ));
                        match parse {
                            Ok(possible_can_msg) => {
                                if let Some(can_msg) = possible_can_msg {
                                    can_msgs.push(can_msg);
                                }
                            }
                            Err(e) => {
                                handle_error(line.to_owned(), e);
                            }
                        }
                    }
                    Err(_) => {log::error!("Invalid line");}
                }
            }
        }
        Err(e) => println!("{e:#?}"),
    };

    can_msgs
}
