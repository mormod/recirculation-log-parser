use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while, take_until},
    character::{complete::{hex_digit1, multispace0, line_ending, multispace1}, is_space, is_newline},
    sequence::{preceded, tuple, delimited},
    number::complete::float,
    multi::{separated_list0, separated_list1},
    error::ParseError,
    IResult, combinator::{rest, eof},
};

use core::str::from_utf8;
use nom_locate::LocatedSpan;
use miette::GraphicalReportHandler;
use nom_supreme::{
    error::{BaseErrorKind, ErrorTree, GenericErrorTree}, final_parser::final_parser,
};

pub type Span<'a> = LocatedSpan<&'a [u8]>;

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
#[error("Bad Input")]
struct BadInput {
    #[source_code]
    src: &'static str,

    #[label("{kind}")]
    bad_bit: miette::SourceSpan,

    kind: BaseErrorKind<&'static str, Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Debug)]
pub struct CanId {
    hex_id: u32,
    str_id: String,
    description: Option<String>,
    scale: Option<f32>,
    unit: Option<String>,
}

impl Default for CanId {
    fn default() -> Self {
        CanId {
            hex_id: 0,
            str_id: "".to_string(),
            description: None,
            scale: None,
            unit: None
        }
    }
}

fn parse_equal<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    delimited(multispace1, tag("="), multispace1)(input)
}

fn parse_str_id<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    take_till(is_space)(input)
}

fn parse_hex<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    preceded(alt((tag("0X"), tag("0x"))), hex_digit1)(input)
}

fn parse_to_newline<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    take_while(|c| !is_newline(c))(input)
}

fn parse_to_comment_sep<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E>{
    take_while(|c| !(c == b'|' || is_newline(c)))(input)
}

fn parse_comment<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Vec<Span<'a>>, E> {
    let (r, _) = tuple((tag(","), multispace0, tag("//")))(input)?;
    separated_list0(tag("|"), parse_to_comment_sep)(r)
}   

fn bytes_to_string<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> String {
    from_utf8(input.as_ref()).unwrap().trim().to_string()
}

fn check_if_relevant<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    alt((
        tag("*/"),
        tag("//"),
        tag("/*"),
        tag("*"),
        tag("#"),
        tag("enum"),
        tag("}"),
        tag("error"),
        line_ending,
    ))(input)
}

fn parse_line<'a, E: ParseError<Span<'a>>>(line: Span<'a>) -> IResult<Span<'a>, Option<CanId>, E> {
    let mut can_id = CanId::default();
    // Skip any spaces at the start of the line
    if line.len() == 0 {
        return Ok((line, None));
    }

    let (r, _) = take_while(is_space)(line)?;
    // Check, if the line is irrelevant
    let is_unrelevant_line = check_if_relevant::<ErrorTree<Span>>(r).is_ok();

    if is_unrelevant_line {
        let (r, _) = take_while(|c| !is_newline(c))(r)?;
        return Ok((r, None));
    }

    // Consume every commented line
    // From here, just consume everything until the first space is met
    let (r, str_id_raw) = parse_str_id(r)?;

    // Match any number of spaces followed by an equal sign, followed by another arbitrary number of spaces
    let (r, _) = parse_equal(r)?;

    // Consume a "0x", then everything in {0-9, a-f, A-F}
    let (r, hex_id_raw) = parse_hex(r)?;

    // Match the trailing comma after the ID, followed by any number of spaces. 
    // Split the comment into a list, separated by "|"s. 
    let (mut r, comment_list) = parse_comment(r)?;


    can_id.str_id = bytes_to_string::<ErrorTree<Span>>(str_id_raw);
    can_id.hex_id = u32::from_str_radix(&bytes_to_string::<ErrorTree<Span>>(hex_id_raw), 16).unwrap();

    let comment_list_len = comment_list.len();
    if comment_list_len >= 1 {
        can_id.description = Some(bytes_to_string::<ErrorTree<Span>>(comment_list[0]));
    }

    if comment_list_len >= 2 {
        if comment_list[1].len() != 0 {
            let is_full_comment = tuple((multispace0::<Span<'a>, ErrorTree<Span>>, float, multispace0, parse_to_comment_sep))(comment_list[1]);
            if is_full_comment.is_err() {
                let (ri, (_, unit_raw)) = tuple((multispace0, parse_to_newline))(comment_list[1])?;
                can_id.unit = Some(bytes_to_string::<ErrorTree<Span>>(unit_raw));
                r = ri;
            }
            else {
                let (ri, (_, scale, _, unit_raw)) = is_full_comment.unwrap();
                can_id.unit = Some(bytes_to_string::<ErrorTree<Span>>(unit_raw));
                can_id.scale = Some(scale);
                r = ri;
            }
        }
    }

    println!("{can_id:#?}");
    Ok((r, Some(can_id)))
}

fn handle_error<'a>(src: &'static str, e: ErrorTree<Span<'a>>) {
    match e {
        GenericErrorTree::Base { location, kind } => {
            let offset = location.location_offset().into();
            let err = BadInput {
                src,
                bad_bit: miette::SourceSpan::new(offset, 0.into()),
                kind,
            };
            let mut s = String::new();
            GraphicalReportHandler::new()
                .render_report(&mut s, &err)
                .unwrap();
            println!("{s}");
        }
        GenericErrorTree::Stack { .. } => todo!("stack"),
        GenericErrorTree::Alt(s) => {
            for i in s {
                handle_error(src, i);
            }
        },
    }
}

fn main() {
    let input_static = include_str!("SmartECLA_IDs.h");
    let ipt = Span::new(input_static.as_bytes());
    let parse_res= final_parser(separated_list1(alt((line_ending, eof)), parse_line::<ErrorTree<Span>>))(ipt);
    
    let _can_ids = match parse_res {
        Ok(can_id_vec) => {
            let mut vec: Vec<CanId> = vec![];
            for entry in can_id_vec {
                if let Some(can_id) = entry {
                    vec.push(can_id);
                }
            }
            vec
        },
        Err(e) => {
            handle_error(&input_static, e);
            return;
        }
    };

}
