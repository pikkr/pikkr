//! JSON parser which picks up values directly without performing tokenization
extern crate fnv;
extern crate x86intrin;

mod avx;
mod bit;
mod index_builder;
mod parser;
mod pikkr;
mod query;
mod stat;
mod utf8;

pub use pikkr::Pikkr;
