//! Lexer (Token Scanner)
//!
//! PHP language scanner
//!
//! This module implements the PHP tokenizer/lexer, now modularized for better maintainability.

mod core;
mod keywords;
mod readers;
mod tokens;

#[cfg(test)]
mod tests;

pub use core::*;
pub use keywords::*;
pub use tokens::*;
