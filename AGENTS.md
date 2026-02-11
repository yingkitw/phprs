# phprs Development Guidelines

Guidelines for contributing to phprs codebase.

## Commands

### Build & Check
```bash
cargo build              # Build project
cargo build --release    # Build optimized release
cargo check             # Quick syntax/type checking
cargo clippy            # Run linter with all checks
cargo fmt                # Format code
```

### Testing
```bash
cargo test                           # Run all tests
cargo test <test_name>              # Run specific test
cargo test --test <test_file>       # Run integration test
cargo test -- --nocapture            # Show test output
```

### Examples
```bash
cargo run --example <name>           # Run specific example
```

### Workspace
```bash
cargo build -p php                    # Build PHP binary
cargo build -p php-pkg                # Build package manager
cargo build -p php-server            # Build server binary
```

## Code Style

### Structure
- `src/engine/` - Core Engine (types, vm, string, hash, compile, lexer)
- `src/php/` - PHP Runtime (runtime, streams, variables, filesystem, sapi)
- `tests/` - Integration tests
- `examples/` - Example code

### Import Order
```rust
// std → third-party → local
use std::sync::atomic::{AtomicU32, Ordering};
use anyhow::Result;
use crate::engine::types::PhpType;
```

- Avoid glob imports (`use *;`)
- Group related imports

### Naming Conventions
- Types: `PascalCase` (e.g., `PhpString`, `Val`, `CompileContext`)
- Functions: `snake_case` (e.g., `string_init`, `compile_expression`)
- Constants: `SCREAMING_SNAKE_CASE`
- Files: `snake_case.rs`
- Enum variants: `PascalCase`

### Rust Patterns
- Use `#[repr(u8)]` for enums mapping to C values
- Use `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` for simple enums
- Implement `From` traits for type conversions
- Use `Result<T, Box<dyn std::error::Error>>` for complex errors
- Use `Result<T, String>` for simple errors
- Prefer `Box::new()` for heap allocation
- Use `AtomicU32` for reference counting
- Prefer immutable references (`&`) over mutable (`&mut`)
- Use `?` operator for error propagation

### Documentation
- Use module-level comments (`//!`)
- Use function comments (`///`) for public functions
- Include examples when appropriate
- Document unsafe blocks with `# Safety` comments

### Code Organization
- **KISS**: Keep functions simple and focused
- **DRY**: Extract common functionality; avoid duplication
- **SoC**: Separate concerns logically
- Keep functions under 50 lines when possible
- Extract complex logic into separate functions

### Testing
- Unit tests in `tests.rs` modules within source files
- Integration tests in `tests/` directory
- Use `#[test]` attribute
- Use `#[cfg(test)]` for test-only code
- Test names: `test_function_name_behavior`
- Include positive and negative test cases

## Architecture Notes

- Single crate architecture with all code in `src/`
- Workspace with binary crates in `bin/` directory
- Migration from C to Rust maintaining PHP compatibility
- Memory safety handled by Rust's ownership system
- Optimized release profile in Cargo.toml

## Key Dependencies

- `anyhow`/`thiserror` - Error handling
- `serde` - Serialization
- `log`/`env_logger` - Logging

## Workflow

1. Focus on one module at a time
2. Reference original PHP/C implementation when needed
3. Always run `cargo check` and `cargo test` before committing
4. Use `cargo fmt` and `cargo clippy` to ensure quality
5. Consider performance and memory safety
6. Write maintainable, reusable code
7. Update both implementation and tests for new features
