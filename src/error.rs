use std::io;
use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("unexpected end of input")]
pub struct EofError;

pub type Result<T> = std::result::Result<T, EofError>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unexpected end of input")]
    Eof,
    #[error("invalid bytecode or data")]
    Invalid,
    #[error("{0}")]
    Chain(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

impl From<io::Error> for EofError {
    fn from(_: io::Error) -> EofError {
        EofError
    }
}

impl From<EofError> for ParseError {
    fn from(_: EofError) -> ParseError {
        ParseError::Eof
    }
}
