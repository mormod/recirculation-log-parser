use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::exit;

use nom::bytes::complete::{is_a, tag, take_while};
use nom::character::complete::{digit1, multispace1};
use nom::character::is_newline;
use nom::error::ParseError;
use nom::multi::{many0, separated_list1};
use nom::sequence::tuple;
use nom::IResult;
use nom_supreme::error::ErrorTree;
use nom_supreme::final_parser::final_parser;

use super::common::{bytes_to_string, handle_error, Span};

pub mod tv_comment;
pub use tv_comment::CanCmt;

fn parse_id<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, u32, E> {
    let (r, id_raw) = digit1(input)?;
    let id: u32 = bytes_to_string(id_raw).parse().unwrap();
    return Ok((r, id));
}

fn parse_time<'a, E: ParseError<Span<'a>>>(
    input: Span<'a>,
) -> IResult<Span<'a>, (u32, u32, u32), E> {
    let (r, list) = separated_list1(tag(":"), digit1)(input)?;
    let hour: u32 = bytes_to_string(list[0]).parse().unwrap();
    let min: u32 = bytes_to_string(list[1]).parse().unwrap();
    let sec: u32 = bytes_to_string(list[2]).parse().unwrap();
    Ok((r, (hour, min, sec)))
}

fn parse_content<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, String, E> {
    let (r, content_raw) = take_while(|c| !(is_newline(c)))(input)?;
    let content = bytes_to_string(content_raw);
    Ok((r, content))
}

fn parse_comment<'a>(
    input: Span<'a>,
) -> IResult<
    Span<'a>,
    (
        u32,
        Span<'a>,
        Vec<Span<'a>>,
        Span<'a>,
        (u32, u32, u32),
        Span<'a>,
        String,
    ),
> {
    tuple((
        parse_id,
        multispace1,
        many0(is_a("0123456789:.-")),
        multispace1,
        parse_time,
        multispace1,
        parse_content,
    ))(input)
}

fn to_timestamp(hour: u32, min: u32, second: u32, millis: u32) -> u32 {
    static mut HIGHEST_HOUR: u32 = 0;
    let modifier = unsafe {
        if hour < HIGHEST_HOUR {
            24
        } else {
            HIGHEST_HOUR = hour;
            0
        }
    };
    return millis + 1000 * second + 1000 * 60 * min + 1000 * 60 * 24 * (hour + modifier);
}

#[allow(dead_code)] // 012	10-23-2014 09:21:58	New Offset on ID CAN_ID_PRESSURE_SIG2(0x10030001): 85,09 mmHg
fn parse_line<'a>(input: Span<'a>) -> IResult<Span<'a>, Option<CanCmt>, ErrorTree<Span<'a>>> {
    let mut tvcomment = None;
    let parse_res = parse_comment(input);
    if parse_res.is_ok() {
        let (_, (id, _, _, _, (hour, min, sec), _, content)) = parse_res.unwrap();
        let ts = to_timestamp(hour, min, sec, 0);
        tvcomment = Some(CanCmt {
            id,
            ts,
            value: content.parse().unwrap(),
        });
    }

    Ok((Span::new("".as_bytes()), tvcomment))
}

#[allow(dead_code)]
pub fn parse_comments<P: AsRef<Path>>(comment_file: &P) -> Vec<CanCmt> {
    let mut cmts = Vec::<CanCmt>::new();
    match File::open(comment_file) {
        Ok(file) => {
            let mut line_buf = vec![];
            let mut reader = BufReader::new(file);
            while let Ok(_) = reader.read_until(b'\n', &mut line_buf) {
                if line_buf.is_empty() {
                    break;
                }
                let parse = final_parser(parse_line)(Span::new(&line_buf[..]));
                match parse {
                    Ok(possible_cmt) => {
                        if let Some(can_msg) = possible_cmt {
                            cmts.push(can_msg);
                        }
                    }
                    Err(e) => {
                        handle_error(String::from_utf8_lossy(&line_buf).to_string(), e);
                    }
                }
                line_buf.clear();
            }
        }
        Err(e) => {
            log::error!("{e:#?}");
            exit(1);
        }
    };

    log::debug!("Parsed {} comments.", cmts.len());
    cmts
}
