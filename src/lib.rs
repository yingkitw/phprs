//! PHP-RS - PHP Interpreter in Rust
//!
//! A complete migration of the PHP interpreter from C to Rust

pub mod engine;
pub mod php;

pub use engine::types::*;

// Re-export commonly used modules for examples
pub use php::filesystem;
pub use php::globals;
pub use php::ini;
pub use php::output;
pub use php::runtime;
pub use php::sapi;
pub use php::streams;
pub use php::variables;
pub use engine::alloc;
pub use engine::compile;
pub use engine::errors;
pub use engine::hash;
pub use engine::operators;
pub use engine::string;
pub use engine::vm;
