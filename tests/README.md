# Test Suite Documentation

This directory contains integration and workspace-level tests for phprs. Unit tests live next to engine and `php/` modules under `src/`.

## Layout

| File / directory | Role |
|------------------|------|
| `array_key_normalization.rs` | Numeric-string array key normalization (PHP `"1"` ↔ `1` semantics) |
| `bitwise_operators.rs` | Bitwise operator compilation and execution |
| `build_rust_examples.rs` | `cargo build --examples` for all `examples/rust/*.rs` |
| `compound_assignment.rs` | Compound assignment operators (`+=`, `.=`, `??=`, etc.) |
| `comprehensive_tests.rs` | Lexer, stream, and factory integration tests |
| `edge_cases.rs` | Boundary and stress cases |
| `error_handling.rs` | Error handlers and reporting |
| `example_verification.rs` | Rust `examples/rust/` smoke tests |
| `examples_runtime.rs` | End-to-end: compile + VM + stdout for **every root** `examples/*.php` (`examples_root_php_scripts_all_run`) plus curated framework entrypoints (CodeIgniter, Drupal) |
| `integer_arithmetic.rs` | Integer arithmetic edge cases (overflow, division, modulo) |
| `integration_tests.rs` | Cross-module integration |
| `php8x_features.rs` | PHP 8.x language feature tests (match, enums, readonly, etc.) |
| `php_examples.rs` | Compile checks and existence tests for tutorial PHP files |

## Engine unit tests (highlights)

- `src/engine/vm/builtin_capability_tests.rs` — broad `execute_builtin_function` coverage (regex, hash, math, PDO-shaped stubs, output, etc.)
- `src/engine/vm/tests.rs` — compile/execute paths for language features
- `src/php/*/tests.rs` — runtime module tests (regex, PDO, streams, filesystem, …)

## Running tests

```bash
# Full workspace (recommended)
cargo test --workspace

# Library unit tests only
cargo test --lib

# All root PHP examples (matrix)
cargo test --test examples_runtime examples_root_php_scripts_all_run

# Curated example assertions
cargo test --test examples_runtime

# Rust example binaries compile
cargo test --test build_rust_examples

# PHP compile-only suite
cargo test --test php_examples
```

Show stdout from tests:

```bash
cargo test --test examples_runtime -- --nocapture
```

## Adding coverage

1. **New root `examples/foo.php`** — no extra test file needed; `examples_root_php_scripts_all_run` picks it up automatically. Script must finish with `PhpResult::Success` and non-empty output.
2. **New builtin** — add cases to `builtin_capability_tests.rs` and/or module unit tests.
3. **New nested framework file** — covered indirectly via its entrypoint test, or add a dedicated `#[test]` in `examples_runtime.rs` if it is a public demo.
4. **New Rust example** — add `[[example]]` in `Cargo.toml` and ensure `build_rust_examples` still passes.

## Naming

- Unit: `test_<function>_<scenario>`
- Integration: `test_<feature>_<scenario>`
- Example runtime: `example_<name>_runs` or the auto matrix `examples_root_php_scripts_all_run`
