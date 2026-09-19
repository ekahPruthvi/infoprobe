#![forbid(unsafe_code)]

mod error;
mod model;
mod parse;
mod write;

pub use error::{InvalidData, ParseError, ParseErrorKind};
pub use model::{Document, Entry, Section, Str, Value};
pub use parse::parse;
