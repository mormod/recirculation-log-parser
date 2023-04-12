use miette::GraphicalReportHandler;
use nom_locate::LocatedSpan;
use nom_supreme::error::{BaseErrorKind, ErrorTree, GenericErrorTree};

pub type Span<'a> = LocatedSpan<&'a [u8]>;

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
#[error("Bad Input")]
pub struct BadInput {
    #[source_code]
    pub src: String,

    #[label("{kind}")]
    pub bad_bit: miette::SourceSpan,

    pub kind: BaseErrorKind<&'static str, Box<dyn std::error::Error + Send + Sync>>,
}

pub fn bytes_to_string<'a>(input: Span<'a>) -> String {
    String::from_utf8_lossy(input.as_ref()).trim().to_string()
}

pub fn bytes_to_number<'a>(input: Span<'a>) -> (u64, u32) {
    let mut number: u64 = 0;
    let mut digits: u32 = 0;
    // Reverse the contents of the span, to allow for parsing
    for (i, &digit) in input.fragment().iter().rev().enumerate() {
        // UTF-8 to digit: As 0 is encoded as 0x30 through to 9 being 0x39,
        // we can simply substract 0x30 from the digit.
        number += ((digit - 0x30) as u64) * 10_u64.pow(i.try_into().unwrap());
        digits += 1;
    }
    (number, digits)
}

pub fn handle_error<'a>(src: String, e: ErrorTree<Span<'a>>) {
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
            log::warn!("{s}");
        }
        GenericErrorTree::Stack { .. } => todo!("stack"),
        GenericErrorTree::Alt(s) => {
            for i in s {
                handle_error(src.clone(), i);
            }
        }
    }
}
