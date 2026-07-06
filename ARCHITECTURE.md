# phprs Architecture

## Overview

phprs is a PHP interpreter implemented in Rust. The architecture separates **compilation** (PHP → op arrays), **execution** (VM dispatch), and **runtime services** (`src/php/`). PHP semantics are the target; full Zend parity is incremental.

## Module structure

```
src/
├── engine/           # Compiler, VM, types, memory, GC
│   ├── types.rs
│   ├── compile/      # Lexer, parser, statements, expressions
│   ├── vm/
│   │   ├── opcodes.rs
│   │   ├── dispatch_handlers.rs  # Opcode handlers + include path resolution
│   │   ├── builtins.rs           # Built-in PHP functions
│   │   ├── builtin_capability_tests.rs  # Broad builtin tests
│   │   └── execute.rs            # Main loop, __FILE__/__DIR__
│   ├── jit.rs
│   ├── function_optimizer.rs
│   ├── opcode_cache.rs
│   └── benchmark.rs              # phprs-only benchmark export
└── php/              # Runtime (regex, pdo stub, streams, filesystem, …)
```

## Execution flow

```
PHP source
    → Lexer
    → Compiler (OpArray + optional FunctionTable)
    → execute_ex (VM)
    → stdout / return value
```

### Include / require resolution

For relative paths, `execute_include` in `dispatch_handlers.rs` uses `resolve_include_path`:

1. If the path is absolute, use as-is.
2. Else if `cwd.join(path)` exists as a file, use that (PHP cwd-first behavior).
3. Else resolve relative to `current_script_dir` (directory of the script being executed).

Magic constants `__FILE__` and `__DIR__` are set per script in `execute.rs` from the op array filename.

## Type system

| PHP | Rust (`PhpValue`) |
|-----|-------------------|
| null | `Null` |
| bool | `Bool` |
| int | `Long` |
| float | `Double` |
| string | `String` |
| array | `Array` |
| object | `Object` |

## Virtual machine

- **67 opcodes** (arithmetic, control flow, calls, OOP, includes, exceptions, …)
- **Direct dispatch table** in `dispatch_handlers.rs`
- **Built-ins** delegated from `builtins.rs` to `src/php/*` modules

Authoritative builtin coverage: `src/engine/vm/builtin_capability_tests.rs`.

## CLI (`phprs`)

```bash
phprs run file.php
phprs serve [--port N]
phprs pkg init | install | require | build
```

## Framework demos (`examples/`)

| Tree | Entry | CI test |
|------|-------|---------|
| CodeIgniter-shaped | `codeigniter/public/index.php` | `example_codeigniter_public_index_runs` |
| Drupal-shaped | `drupal/index.php` | `example_drupal_index_runs` |
| WordPress-shaped | `wordpress/index.php` | Manual (compiler gaps in nested stubs) |

All **root** `examples/*.php` files: `examples_root_php_scripts_all_run`.

## Testing layout

| Location | Role |
|----------|------|
| `src/**/tests.rs`, `#[cfg(test)]` | Unit tests |
| `tests/examples_runtime.rs` | PHP example E2E |
| `tests/build_rust_examples.rs` | Rust `examples/rust/` compile |
| `tests/php_examples.rs` | PHP compile smoke |

## Performance notes

JIT, opcode cache, and optimizer modules exist as **scaffolding**. Claims relative to stock PHP belong in [PERFORMANCE.md](PERFORMANCE.md) only with reproducible measurements.

## See also

- [SPEC.md](SPEC.md)
- [README.md](README.md)
- [TODO.md](TODO.md)
- [PERFORMANCE.md](PERFORMANCE.md)
- [tests/README.md](tests/README.md)
