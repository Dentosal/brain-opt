#![allow(dead_code)]
#![forbid(mutable_borrow_reservation_conflict)]
#![forbid(bare_trait_objects)]
#![warn(clippy::pedantic)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_possible_truncation)]

mod codegen;
mod compiler;
pub mod error;
mod parser;
pub mod target_abi;

pub use target_abi::ABI;

pub use compiler::compile_tokens;
pub use parser::parse;
