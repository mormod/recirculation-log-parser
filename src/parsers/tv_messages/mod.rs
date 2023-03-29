use core::fmt;
use std::{path::Path, fs::File, io::{BufReader, BufRead}};
use nom::{error::ParseError, IResult, multi::{separated_list1}, character::{complete::{space1, digit1}, is_space, is_hex_digit}, bytes::complete::{take_while, tag}, number::complete::float, branch::alt, sequence::{tuple}};
use nom_supreme::{final_parser::final_parser, error::ErrorTree};
use std::str::from_utf8;

use super::common::{handle_error, Span, bytes_to_string};

#[derive(Debug)]
pub struct CanTs {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub tsec: u32
}

impl fmt::Display for CanTs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}.{:04}", self.hour, self.min, self.sec, self.tsec)
    }
}

#[derive(Debug)]
pub struct CanMsg {
    pub hex_id: u32,
    pub value: f32,
    pub ts: CanTs
}

impl fmt::Display for CanMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {} = {}", self.hex_id, self.ts, self.value)
    }
}

fn parse_spaces<'a, E: ParseError<Span<'a>>>(line: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    space1(line)
}

fn parse_ts<'a, E: ParseError<Span<'a>>>(raw_ts: Span<'a>) -> IResult<Span<'a>, CanTs, E> {
    let (r, list) = separated_list1(alt((tag(":"), tag("."))), digit1)(raw_ts)?;
    let hour: u32 = bytes_to_string::<ErrorTree<Span>>(list[0]).parse().unwrap();
    let min: u32 = bytes_to_string::<ErrorTree<Span>>(list[1]).parse().unwrap();
    let sec: u32 = bytes_to_string::<ErrorTree<Span>>(list[2]).parse().unwrap();
    let tsec: u32 = bytes_to_string::<ErrorTree<Span>>(list[3]).parse().unwrap();
    Ok((r, CanTs {
        hour, min, sec, tsec
    }))
}

//100C0000h	8	2E 00 00 00 01 00 00 00 	0,44921875 L/min	1	 average Blood Flow 		08:44:04.97
fn parse_extended<'a, E: ParseError<Span<'a>>>(line: Span<'a>) -> IResult<Span<'a>, Option<CanMsg>, E> {
    let mut can_msg: Option<CanMsg> = None;

    if line.len() > 0 {
        let parse_res = separated_list1(parse_spaces::<ErrorTree<Span>>, take_while(|c| !is_space(c)))(line);
        if parse_res.is_err() {
            unreachable!("This should never happen.");
        }
    
        let (r, list) = parse_res.unwrap();
    
        // CanId is always the first item. If its not, this is not a valid line
        let (_, hex_id_raw) = take_while(is_hex_digit)(list[0])?;
    
        let hex_id_res = u32::from_str_radix(from_utf8(hex_id_raw.as_ref()).unwrap().trim(), 16);
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
fn parse_simple<'a, E: ParseError<Span<'a>>>(line: Span<'a>) -> IResult<Span<'a>, Option<CanMsg>, E> {
    let mut can_msg: Option<CanMsg> = None;

    if line.len() > 0 {
        let parse_res = separated_list1(parse_spaces::<ErrorTree<Span>>, take_while(|c| !is_space(c)))(line);
        if parse_res.is_err() {
            unreachable!("This should never happen.");
        }
    
        let (_, list) = parse_res.unwrap();
        
        // CanId is always the first item. If its not, this is not a valid line
        let hex_id_res = tuple((tag::<_, _, ErrorTree<Span>>("0x"), take_while(is_hex_digit)))(list[0]);
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
pub fn parse_messages<P: AsRef<Path>>(log_file: &P) -> Vec<CanMsg> {
    let mut can_msgs = Vec::<CanMsg>::new();
    match File::open(log_file) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        let parse = final_parser(parse_simple::<ErrorTree<Span>>)(Span::new(line.as_bytes()));
                        match parse {
                            Ok(possible_can_msg) => {
                                if let Some(can_msg) = possible_can_msg {
                                    can_msgs.push(can_msg);
                                }
                            },
                            Err(e) => {
                                handle_error(line.to_owned(), e);
                            }
                        }
                    },
                    Err(_) => {}
                }  
            }    
        }
        Err(e) => println!("{e:#?}"),
    };

    can_msgs
}