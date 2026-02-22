# phprs Specification

## Purpose

phprs is a **PHP interpreter implemented in Rust**, migrated from the PHP C implementation. It aims to execute PHP code with memory safety, type safety, and competitive performance while preserving PHP semantics.

## Scope

### In Scope

- **Core engine**: Type system (Val, strings, arrays, objects), string handling (DJBX33A), hash tables, memory allocation (persistent/non-persistent), garbage collection (tri-color marking), operators and type conversion.
- **Compiler**: Lexer (PHP tokens including `?`, `??`, `?->`, `::`), expression and statement parsing, control flow, functions, classes, traits, namespaces, closures, type declarations, PHP 8.0 features (match, attributes, generators).
- **VM**: 63 opcodes, dispatch-table execution, 40+ built-in functions, exceptions (try/catch/finally/throw).
- **Runtime**: INI config, variables, streams, SAPI (CLI), output buffering, filesystem, extension framework.
- **Tooling**: Unified CLI (`run`, `serve`, `pkg`), web playground, package manager (Composer-style, Packagist, PSR-4 autoload).
- **Performance**: JIT for hot functions, function optimizer, opcode cache, optimized VM dispatch, memory and array optimizations.

### Out of Scope (Planned / Future)

- Stream wrappers (HTTP, FTP), regex (preg_*), sessions, PDO/database.
- Full framework support (CodeIgniter 4, Drupal, WordPress bootstrap and wpdb).

## Compliance

- **PHP semantics**: Expression and statement behavior aligned with PHP 7/8 where implemented.
- **Rust**: Edition 2024, `cargo build` and `cargo test` must succeed.

## References

- [README.md](README.md) - Quick start and overview
- [ARCHITECTURE.md](ARCHITECTURE.md) - Module layout and execution flow
- [TODO.md](TODO.md) - Roadmap and statistics
- [PERFORMANCE.md](PERFORMANCE.md) - Optimizations and benchmarks
