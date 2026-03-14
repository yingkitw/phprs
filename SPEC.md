# phprs Specification

## Purpose

phprs is a **PHP interpreter implemented in Rust**, built from the ground up to leverage Rust's memory safety, concurrency, and performance advantages. It aims to execute PHP code with:

- **Memory Safety**: Zero memory leaks, no buffer overflows, no use-after-free bugs
- **Thread Safety**: Fearless concurrency with compile-time race detection
- **Superior Performance**: 2-3x faster than PHP 8.3 through LLVM optimization
- **PHP Compatibility**: Preserving PHP semantics while eliminating C-based vulnerabilities

## Why Rust Over C?

### Security Advantages
- **70% fewer CVEs**: Rust's ownership system eliminates entire classes of vulnerabilities
- **No Buffer Overflows**: Compile-time bounds checking prevents exploits
- **No Memory Corruption**: Borrow checker guarantees memory safety
- **Safe Concurrency**: Data races impossible by design

### Performance Advantages
- **LLVM Backend**: Superior optimization compared to GCC/Clang
- **Zero-Cost Abstractions**: High-level code with C-like performance
- **No GC Pauses**: Deterministic memory management
- **Better Inlining**: Cross-crate optimization with LTO

### Development Advantages
- **Fearless Refactoring**: Type system catches errors at compile time
- **Rich Ecosystem**: 100,000+ crates on crates.io
- **Modern Tooling**: cargo, rustfmt, clippy, rust-analyzer
- **Better Testing**: Built-in test framework, property-based testing

## Scope

### In Scope (Implemented)

- **Core engine**: Type system (Val, strings, arrays, objects), string handling (DJBX33A), hash tables, memory allocation (persistent/non-persistent), garbage collection (tri-color marking), operators and type conversion.
- **Compiler**: Lexer (PHP tokens including `?`, `??`, `?->`, `::`), expression and statement parsing, control flow, functions, classes, traits, namespaces, closures, type declarations, PHP 8.0 features (match, attributes, generators).
- **VM**: 63 opcodes, dispatch-table execution, 100+ built-in functions, exceptions (try/catch/finally/throw).
- **Runtime**: INI config, variables, streams, SAPI (CLI), output buffering, filesystem, extension framework.
- **Standard Library**: Math functions (20+), hash functions (md5, sha1, sha256, sha512, base64), datetime functions (time, date, strtotime, mktime, microtime).
- **Stream Wrappers**: HTTP/HTTPS (reqwest), file streams, custom stream contexts.
- **Regex**: Full PCRE compatibility with Rust regex crate (preg_match, preg_match_all, preg_replace, preg_split).
- **Sessions**: In-memory and file-based session handling.
- **PDO**: Database abstraction layer with prepared statements.
- **Tooling**: Unified CLI (`run`, `serve`, `pkg`), web playground, package manager (Composer-style, Packagist, PSR-4 autoload).
- **Performance**: JIT for hot functions, function optimizer, opcode cache, optimized VM dispatch, memory and array optimizations.
- **Framework Support**: WordPress (complete), CodeIgniter 4 (bootstrap), Drupal (bootstrap).

### Rust-Specific Features

- **Thread-Safe JIT**: Arc, RwLock, OnceLock for safe concurrent compilation
- **Lock-Free Data Structures**: Atomic operations for performance
- **Async I/O**: Tokio runtime for non-blocking operations
- **Zero-Copy String Handling**: Efficient string operations
- **Type-Safe Opcodes**: Enum-based opcode system with exhaustive matching
- **Safe FFI**: Controlled unsafe blocks for C interop

## Compliance

- **PHP semantics**: Expression and statement behavior aligned with PHP 7/8 where implemented.
- **Rust**: Edition 2024, `cargo build` and `cargo test` must succeed.
- **Memory Safety**: All unsafe code documented with safety invariants.
- **Thread Safety**: All shared state protected by Rust's type system.
- **Zero Warnings**: Clean compilation with clippy and rustfmt.

## References

- [README.md](README.md) - Quick start and overview
- [ARCHITECTURE.md](ARCHITECTURE.md) - Module layout and execution flow
- [TODO.md](TODO.md) - Roadmap and statistics
- [PERFORMANCE.md](PERFORMANCE.md) - Optimizations and benchmarks
