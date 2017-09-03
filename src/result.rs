/// The error type represents all possible errors that occurs when parsing JSON string
#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedEof,
}

/// The type alias for `Result`, with the error type `ParseError`
pub type ParseResult<T> = Result<T, ParseError>;
