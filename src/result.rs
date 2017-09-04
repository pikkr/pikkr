use super::error;
use std::result;

/// A specialized Result type for parsing a JSON record.
pub type Result<T> = result::Result<T, error::Error>;
