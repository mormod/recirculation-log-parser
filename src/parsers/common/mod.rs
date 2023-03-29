use miette::GraphicalReportHandler;
use nom::error::ParseError;
use nom_locate::LocatedSpan;
use nom_supreme::error::{BaseErrorKind, ErrorTree, GenericErrorTree};
use std::str::from_utf8;


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

pub fn bytes_to_string<'a, E: ParseError<Span<'a>>>(input: Span<'a>) -> String {
    from_utf8(input.as_ref()).unwrap().trim().to_string()
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
            println!("{s}");
        }
        GenericErrorTree::Stack { .. } => todo!("stack"),
        GenericErrorTree::Alt(s) => {
            for i in s {
                handle_error(src.clone(), i);
            }
        },
    }
}