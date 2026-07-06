# phprs Specification

## Purpose

phprs is a **PHP interpreter implemented in Rust**. It aims to execute PHP code with:

- **Memory safety** for the **interpreter implementation** (Rust ownership and bounds checking in safe code)
- **Thread-safe host code** where shared engine state uses Rust concurrency primitives
- **Growing PHP compatibility** — semantics and extensions are implemented incrementally; parity with Zend PHP is not claimed globally
- **Measured performance** — optimization work is welcome; published speedups vs stock PHP require reproducible benchmarks (see [PERFORMANCE.md](PERFORMANCE.md))

## Why Rust?

### Security (implementation)
- Fewer classic memory-safety bug classes in **safe Rust** than in a typical C extension/runtime
- Explicit `unsafe` blocks should be rare and documented
- Application security still depends on scripts, deployment, and extensions you enable

### Performance (hypothesis, not a product claim)
- LLVM backend for the host binary
- Room for opcode dispatch, caching, and JIT-oriented hooks
- **No guaranteed multiplier vs PHP 8.x** without workload-specific measurements

### Development
- `cargo test`, clippy, rustfmt, and a large crates.io ecosystem
- Type-driven refactors for engine code

## Scope

### In Scope (Implemented to varying degrees)

- **Core engine**: Types, strings, hash tables, allocation, GC, operators, arrays.
- **Compiler**: … legacy `array()` constructor (`array()`, `array('k' => v)`, indexed elements) plus short `[]` literals
- **VM**: Opcode dispatch table, built-in functions (see `src/engine/vm/builtins.rs` and `builtin_capability_tests.rs`), exceptions, includes, static members, magic-method fallbacks.
- **Runtime**: INI, variables, streams, SAPI (CLI), output buffering, filesystem helpers, extension framework.
- **Standard library (partial)**: Math, hash, datetime, mbstring subset, URL helpers, regex (`preg_*` via Rust `regex`), HTTP `file_get_contents`, PDO **stub**, JSON, introspection helpers.
- **Tooling**: CLI (`run`, `serve`, `pkg`), Composer-style package manager (partial).
- **Performance scaffolding**: JIT hooks, function optimizer, opcode cache, phprs-only benchmark export (no bundled PHP baseline).
- **Framework demos**: Minimal trees under `examples/` for WordPress-shaped, CodeIgniter 4, and Drupal bootstraps (not full framework parity).
- **Tests**: 376+ workspace tests including `examples_runtime` (all root `examples/*.php`), `build_rust_examples`, and broad builtin capability tests.

### Explicitly Limited or Not Implemented

- **PHP session extension** (`session_start`, `session_id`, …) — not in the engine; `examples/session-examples.php` simulates `$_SESSION` as a plain array
- **User-defined function calls from CLI** — incomplete in some paths (see `examples/functions.php`); engine tests cover more than the demo scripts
- **Full PCRE** — Rust regex: no look-ahead/look-behind; some PHP patterns need rewriting
- **Real PDO drivers** — in-memory/stub behavior only
- **Production WordPress / Laravel / Symfony** — demo stubs only

### Rust-Specific Features

- Thread-safe globals where needed (`Arc`, `OnceLock`, `RwLock`)
- Tokio for async HTTP in the host
- Enum-based opcodes with exhaustive dispatch setup

## Compliance

- **PHP semantics**: Aligned with PHP 7/8 **where implemented**; gaps are bugs or backlog items.
- **Rust**: Edition 2024; `cargo build --workspace` and `cargo test --workspace` must succeed.
- **Documentation**: Must not claim features or benchmarks without tests or measured methodology.
- **Examples**: Every root-level `examples/*.php` must compile and run to `PhpResult::Success` under `tests/examples_runtime.rs`.

## References

- [README.md](README.md) - Quick start and overview
- [ARCHITECTURE.md](ARCHITECTURE.md) - Module layout and execution flow
- [TODO.md](TODO.md) - Roadmap and statistics
- [PERFORMANCE.md](PERFORMANCE.md) - Evidence policy and optimization notes
