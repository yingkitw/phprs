# phprs

A PHP interpreter written in Rust, migrated from the PHP C implementation.

## Features

- **Memory Safe**: Rust's ownership system prevents memory leaks and dangling pointers
- **Type Safe**: Compile-time type checking with Rust enums
- **Thread Safe**: Safe concurrent access with Mutex and OnceLock
- **Fast**: Zero-cost abstractions, similar performance to C PHP

## Quick Start

```bash
# Build
cargo build --release

# Run tests
cargo test

# Compile and run PHP files
cargo run --bin php -- examples/basic_types.php

# Start web playground
cargo run --bin php-server
# Open http://localhost:3080
```

## Project Structure

```
phprs/
├── src/
│   ├── engine/       # Core engine: compiler, VM, types, memory
│   └── php/          # PHP runtime: streams, filesystem, SAPI
├── bin/
│   ├── php           # CLI interpreter
│   ├── php-server    # Web playground
│   └── php-pkg       # Package manager (in development)
├── examples/       # PHP and Rust examples
└── tests/          # Comprehensive test suite
```

## API Usage

```rust
use phprs::engine::compile;
use phprs::engine::vm;

// Compile PHP code
let op_array = compile_string("<?php echo 'Hello'; ?>", "inline.php")?;

// Execute
let mut exec_data = ExecuteData::new();
execute_ex(&mut exec_data, &op_array);
```

## Status

- ✅ Core engine (types, strings, hash tables, memory, GC)
- ✅ PHP runtime (streams, filesystem, output)
- ✅ Compiler (expressions, statements, functions, classes)
- ✅ VM (52 opcodes, 40+ built-in functions)
- ✅ SAPI (CLI, built-in web server)
- 🚧 Package manager (in development)

## Tests

```bash
cargo test                    # All tests (248 passing)
cargo test --lib             # Library only
cargo test --test php_examples # PHP examples
```

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - Detailed architecture
- [TODO.md](TODO.md) - Migration roadmap
- [examples/README.md](examples/README.md) - Example guide
