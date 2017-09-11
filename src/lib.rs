//! JSON parser which picks up values directly without performing tokenization
extern crate fnv;
#[cfg(feature = "avx-accel")]
extern crate x86intrin;

#[cfg(feature = "avx-accel")]
mod avx;
#[cfg(not(feature = "avx-accel"))]
mod emulated;
mod bit;
mod error;
mod index_builder;
mod parser;
mod pikkr;
mod query;
mod result;
mod utf8;

#[cfg(not(feature = "avx-accel"))]
pub use emulated::avx;

pub use error::{Error, ErrorKind};
pub use pikkr::Pikkr;
pub use result::Result;
