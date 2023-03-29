use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;

use nom::IResult;
use nom::bytes::complete::{take_while, tag};
use nom::character::complete::{multispace1, digit1};
use nom::character::is_newline;
use nom::error::ParseError;
use nom::multi::separated_list1;
use nom::sequence::tuple;
use nom_supreme::error::ErrorTree;
use nom_supreme::final_parser::final_parser;

use super::common::{handle_error, Span, bytes_to_string};

#[derive(Debug)]
pub struct TVComment {
    pub id: u32,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub content: String,
}

fn parse_id<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, u32, E> {
    let (r, id_raw) = digit1(input)?;
    let id: u32 = bytes_to_string::<ErrorTree<Span>>(id_raw).parse().unwrap();
    return Ok((r, id));
}

fn parse_date<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, (u32, u32, u32), E> {
    let (r, list) = separated_list1(tag("-"), digit1)(input)?;
    let month: u32 = bytes_to_string::<ErrorTree<Span>>(list[0]).parse().unwrap();
    let day: u32 = bytes_to_string::<ErrorTree<Span>>(list[1]).parse().unwrap();
    let year: u32 = bytes_to_string::<ErrorTree<Span>>(list[2]).parse().unwrap();
    Ok((r, (month, day, year)))
}

fn parse_time<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, (u32, u32, u32), E> {
    let (r, list) = separated_list1(tag(":"), digit1)(input)?;
    let hour: u32 = bytes_to_string::<ErrorTree<Span>>(list[0]).parse().unwrap();
    let min: u32 = bytes_to_string::<ErrorTree<Span>>(list[1]).parse().unwrap();
    let sec: u32 = bytes_to_string::<ErrorTree<Span>>(list[2]).parse().unwrap();
    Ok((r, (hour, min, sec)))
}

fn parse_content<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, String, E> {
    let (r, content_raw) = take_while(|c| !(is_newline(c)))(input)?;
    let content = bytes_to_string::<ErrorTree<Span>>(content_raw);
    Ok((r, content))
}


// 012	10-23-2014 09:21:58	New Offset on ID CAN_ID_PRESSURE_SIG2(0x10030001): 85,09 mmHg
fn parse_line<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Option<TVComment>, E> {
    let mut tvcomment = None;
    let mut rest = input;
    let parse_res = tuple((parse_id::<ErrorTree<Span>>, multispace1, parse_date, multispace1, parse_time, multispace1, parse_content))(rest);
    if parse_res.is_ok() {
        let (r, (id, _, (month, day, year), _, (hour, min, sec), _, content)) = parse_res.unwrap();
        tvcomment = Some(TVComment{
            id, year, month, day, hour, min, sec, content
        });
        rest = r;
    }
    else {
        (rest, _) = take_while(|c| !is_newline(c))(rest)?;
    }


    Ok((rest, tvcomment))
}

pub fn parse_comments<P: AsRef<Path>>(comment_file: &P) -> Vec<TVComment> {
    let mut can_ids = Vec::<TVComment>::new();
    match File::open(comment_file) {
        Ok(file) => {
            println!("lo");
            let reader = BufReader::new(file);
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        let parse = final_parser(parse_line::<ErrorTree<Span>>)(Span::new(line.as_bytes()));
                        match parse {
                            Ok(possible_can_id) => {
                                if let Some(can_id) = possible_can_id {
                                    can_ids.push(can_id);
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

    can_ids
}