use std::error;
use std::fmt;

/// Th error for parsing a JSON record.
#[derive(Debug, PartialEq)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Error {
        Error { kind: kind }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.kind.as_str())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.kind.as_str()
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

/// The error kind of error for parsing a JSON record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorKind {
    InvalidQuery,
    InvalidRecord,
}

impl ErrorKind {
    #[inline]
    fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::InvalidQuery => "invalid query",
            ErrorKind::InvalidRecord => "invalid record",
        }
    }
}
