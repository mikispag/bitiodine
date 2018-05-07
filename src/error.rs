use std::{io, result};

#[derive(Debug)]
pub struct EofError;

pub type Result<T> = result::Result<T, EofError>;

#[derive(Debug)]
pub enum ParseError {
    Eof,
    Invalid,
}

pub type ParseResult<T> = result::Result<T, ParseError>;

impl From<io::Error> for EofError {
    fn from(val: io::Error) -> EofError {
        assert_eq!(val.kind(), io::ErrorKind::UnexpectedEof);
        EofError
    }
}

impl From<io::Error> for ParseError {
    fn from(val: io::Error) -> ParseError {
        assert_eq!(val.kind(), io::ErrorKind::UnexpectedEof);
        ParseError::Eof
    }
}

impl From<EofError> for ParseError {
    fn from(_: EofError) -> ParseError {
        ParseError::Eof
    }
}
