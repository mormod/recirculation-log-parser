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

fn parse_to_comment_sep<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E>{
    take_while(|c| !(c == b'|' || is_newline(c)))(input)
}

fn parse_comment<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, (Option<String>, Option<f32>, Option<String>), E> {
    let mut description: Option<String> = None;
    let mut scale: Option<f32> = None;
    let mut unit:Option<String> = None;

    // Consume everything to an d including "//"
    let (r, _) = tuple((take_until("//"), tag("//")))(input)?;
    // Parse content between "|" until the end of the line to a Vec
    let (r, vec) = separated_list0(tag("|"), parse_to_comment_sep)(r)?;

    if vec.len() >= 1 {
        // Consome all spaces, we might have an empty description but still some spaces.
        let (r, _) = consume_spaces(vec[0])?;
        if !r.fragment().is_empty() {
            // The fragment is not empty. We have content. 
            description = Some(bytes_to_string::<ErrorTree<Span>>(r));
        }

    }
    if vec.len() >= 2 {
        // Check if there is something to consume
        let (mut r, _) = consume_spaces(vec[1])?;
        if !r.fragment().is_empty() {
            // If there is, we definitly have a unit, but do we have a scale as well?
            let is_float_res = float::<Span<'a>, ErrorTree<Span>>(r);
            if let Some((remains, scale_raw)) = is_float_res.ok() {
                // We do! Extract the scale!
                scale = Some(scale_raw);
                // Set the remainder to what has not been parsed by "float"
                r = remains;
            }
            // At this point, we may have a scale of not. If we have, the scale bytes have been consumed. 
            // The remaining bytes may or may not be empty. If there is content, its the unit.
            // Check, if we have a unit. Consume every space, then check if we have content left.
            let (r, _) = consume_spaces(r)?;
            if !r.fragment().is_empty() {
                // r then contains the unit encoded in utf-8
                unit = Some(bytes_to_string::<ErrorTree<Span>>(r).trim().to_string());
            }
        }
    }
    
    Ok((r, (description, scale, unit)))
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

fn consume_spaces<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    take_while(is_space)(input)    
}

fn parse_line<'a, E: ParseError<Span<'a>>>(line: Span<'a>) -> IResult<Span<'a>, Option<CanId>, E> {
    let mut can_id = CanId::default();
    // Skip empty lines
    if line.len() == 0 {
        return Ok((line, None));
    }

    // Skip any number of spaces at the beginning of the line
    let (r, _) = consume_spaces(line)?;
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
    let (r, (description, scale, unit)) = parse_comment(r)?;


    can_id.str_id = bytes_to_string::<ErrorTree<Span>>(str_id_raw);
    can_id.hex_id = u32::from_str_radix(&bytes_to_string::<ErrorTree<Span>>(hex_id_raw), 16).unwrap();
    can_id.description = description;
    can_id.scale = scale;
    can_id.unit = unit;
    
    // Get rid of any trailing spaces
    let (r, _) = consume_spaces(r)?;
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
    // let input_static = "	CAN_ID_PERFORMANCE_COUNTER = 0x00000010, // Performance Messungen| 2.3\n    	CAN_ID_MODEL_PUMP_ALARM_CANNULA = 0x02B00001, //critical flow | 0.001 l/min \nCAN_ID_PGA_X_PUMP_STATUS = 0x1006000D, // Pump Status | | 100 100 1 1\n	CAN_ID_N560_X_BPM = 0x10070001, //pulse rate 	| per minute    \n";
    let ipt = Span::new(input_static.as_bytes());
    let parse_res= final_parser(separated_list1(line_ending, parse_line::<ErrorTree<Span>>))(ipt);
    
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
