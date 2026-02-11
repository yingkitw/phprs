//! PHP Engine
//!
//! Core engine implementation

pub mod alloc;
pub mod ast;
pub mod compile;
pub mod errors;
pub mod exception;
pub mod facade;
pub mod gc;
pub mod hash;
pub mod lexer;
pub mod operators;
pub mod perf;
pub mod string;
pub mod types;
pub mod vm;

pub use types::*;

// Re-export facade for convenient access
pub use facade::{ValFactory, StdValFactory};
