//! Facade traits for improved maintainability
//!
//! This module provides trait-based facades that simplify common operations
//! across the engine implementation.

mod factory;
mod parser;
mod stream;

pub use factory::*;
pub use parser::*;
pub use stream::*;
