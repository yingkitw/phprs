//! PHP Engine
//!
//! Core engine implementation

pub mod alloc;
pub mod array_ops;
pub mod ast;
pub mod benchmark;
pub mod compile;
pub mod errors;
pub mod exception;
pub mod facade;
pub mod function_optimizer;
pub mod gc;
pub mod hash;
pub mod jit;
pub mod lexer;
pub mod opcode_cache;
pub mod operators;
pub mod perf;
pub mod perf_alloc;
pub mod string;
pub mod types;
pub mod vm;

pub use types::*;

// Re-export facade for convenient access
pub use facade::{ValFactory, StdValFactory};
