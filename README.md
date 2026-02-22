# phprs

A PHP interpreter written in Rust, migrated from the PHP C implementation.

## Features

- **Memory Safe**: Rust's ownership system prevents memory leaks and dangling pointers
- **Type Safe**: Compile-time type checking with Rust enums
- **Thread Safe**: Safe concurrent access with RwLock and OnceLock (Rust 2024 compliant)
- **Fast**: Zero-cost abstractions, JIT compilation, and optimized dispatch table
- **JIT Compiler**: Just-in-time compilation for hot code paths
- **Function Optimizer**: Advanced inlining and call optimizations
- **Opcode Cache**: Cached execution with optimization passes

## Quick Start

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run a PHP file
cargo run -- run examples/01_hello_world.php

# Start web playground
cargo run -- serve
# Open http://localhost:3080

# Package manager
cargo run -- pkg init
```

## Project Structure

```
phprs/
├── src/
│   ├── engine/       # Core engine: types, string, hash, alloc, gc, operators,
│   │                 # array_ops, lexer, compile, vm, jit, function_optimizer,
│   │                 # opcode_cache, benchmark, perf, perf_alloc, facade
│   └── php/          # PHP runtime: streams, filesystem, SAPI, globals
├── bin/
│   └── phprs         # Unified CLI (run, serve, pkg)
├── examples/        # PHP and Rust examples (including performance_demo)
└── tests/           # Test suite
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
- ✅ Compiler (expressions, statements, functions, classes, traits, namespaces)
- ✅ VM (63 opcodes, dispatch table, 40+ built-in functions)
- ✅ PHP 8.0 features (match expressions, attributes, generators)
- ✅ SAPI (CLI, built-in web server)
- ✅ Package manager (composer.json, Packagist, dependency resolution)
- 📋 Planned: framework support (CodeIgniter 4, Drupal, WordPress — see [TODO.md](TODO.md))

## Tests

```bash
cargo test                    # All tests
cargo test --lib             # Library only
cargo test --test php_examples # PHP examples
```

## Documentation

- [spec.md](spec.md) - Project specification and scope
- [ARCHITECTURE.md](ARCHITECTURE.md) - Module structure and execution flow
- [TODO.md](TODO.md) - Migration roadmap and statistics
- [PERFORMANCE.md](PERFORMANCE.md) - Optimizations and benchmarks vs PHP 8
- [examples/README.md](examples/README.md) - Example guide
