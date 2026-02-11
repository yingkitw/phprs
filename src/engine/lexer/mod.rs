//! Lexer (Token Scanner)
//!
//! PHP language scanner
//!
//! This module implements the PHP tokenizer/lexer, now modularized for better maintainability.

mod tokens;
mod keywords;
mod readers;
mod core;

#[cfg(test)]
mod tests;

pub use tokens::*;
pub use keywords::*;
pub use core::*;
