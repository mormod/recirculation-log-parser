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
pub fn to_timestamp(hour: u32, min: u32, second: u32, millis: u32) -> u32 {
    static mut HIGHEST_HOUR: u32 = 0;
    let mut modifier = 0;
    unsafe {
        modifier = if hour < HIGHEST_HOUR {
            24
        } else {
            HIGHEST_HOUR = hour;
            0
        };
    }
    return millis + 1000 * second + 1000 * 60 * min + 1000 * 60 * 60 * (hour + modifier);
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
        },
    }
}