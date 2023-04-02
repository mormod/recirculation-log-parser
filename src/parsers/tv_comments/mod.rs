use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;

use nom::IResult;
use nom::bytes::complete::{take_while, tag, is_a};
use nom::character::complete::{multispace1, digit1};
use nom::character::is_newline;
use nom::error::ParseError;
use nom::multi::{separated_list1, many0};
use nom::sequence::tuple;
use nom_supreme::error::ErrorTree;
use nom_supreme::final_parser::final_parser;

use super::common::{handle_error, Span, bytes_to_string, to_timestamp};

pub mod tv_comment;
pub use tv_comment::TVComment;

#[allow(dead_code)]
fn parse_id<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, u32, E> {
    let (r, id_raw) = digit1(input)?;
    let id: u32 = bytes_to_string::<ErrorTree<Span>>(id_raw).parse().unwrap();
    return Ok((r, id));
}

#[allow(dead_code)]
fn parse_time<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, (u32, u32, u32), E> {
    let (r, list) = separated_list1(tag(":"), digit1)(input)?;
    let hour: u32 = bytes_to_string::<ErrorTree<Span>>(list[0]).parse().unwrap();
    let min: u32 = bytes_to_string::<ErrorTree<Span>>(list[1]).parse().unwrap();
    let sec: u32 = bytes_to_string::<ErrorTree<Span>>(list[2]).parse().unwrap();
    Ok((r, (hour, min, sec)))
}

#[allow(dead_code)]
fn parse_content<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, String, E> {
    let (r, content_raw) = take_while(|c| !(is_newline(c)))(input)?;
    let content = bytes_to_string::<ErrorTree<Span>>(content_raw);
    Ok((r, content))
}



#[allow(dead_code)] // 012	10-23-2014 09:21:58	New Offset on ID CAN_ID_PRESSURE_SIG2(0x10030001): 85,09 mmHg
fn parse_line<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Option<TVComment>, E> {
    let mut tvcomment = None;
    let mut rest = input;
    let parse_res = tuple((parse_id::<ErrorTree<Span>>, multispace1, many0(is_a("0123456789:.-")), multispace1, parse_time, multispace1, parse_content))(rest);
    if parse_res.is_ok() {
        let (r, (id, _, _, _, (hour, min, sec), _, content)) = parse_res.unwrap();
        let ts = to_timestamp(hour, min, sec, 0);
            tvcomment = Some(TVComment{
            id, ts, content
        });
        rest = r;
    }
    else {
        (rest, _) = take_while(|c| !is_newline(c))(rest)?;
    }


    Ok((rest, tvcomment))
}

#[allow(dead_code)]
pub fn parse_comments<P: AsRef<Path>>(comment_file: &P) -> Vec<TVComment> {
    let mut can_ids = Vec::<TVComment>::new();
    match File::open(comment_file) {
        Ok(file) => {
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