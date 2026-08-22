use std::io;
use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("unexpected end of input")]
pub struct EofError;

pub type Result<T> = std::result::Result<T, EofError>;

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    #[error("unexpected end of input")]
    Eof,
    #[error("invalid bytecode or data")]
    Invalid,
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

impl From<io::Error> for EofError {
    fn from(_: io::Error) -> EofError {
        EofError
    }
}

impl From<io::Error> for ParseError {
    fn from(_: io::Error) -> ParseError {
        ParseError::Eof
    }
}

impl From<EofError> for ParseError {
    fn from(_: EofError) -> ParseError {
        ParseError::Eof
    }
}
