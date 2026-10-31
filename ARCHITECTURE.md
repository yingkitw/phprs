# phprs Architecture

## Overview

phprs is a PHP interpreter implemented in Rust. The architecture separates **compilation** (PHP → op arrays), **execution** (VM dispatch), and **runtime services** (`src/php/`). PHP semantics are the target; full Zend parity is incremental.

## Module structure

```
src/
├── engine/           # Compiler, VM, types, memory, GC
│   ├── types.rs
│   ├── compile/      # Parser + AST → OpArray compilation (lexer is a sibling module)
│   ├── lexer/        # Tokenizer (PHP 7/8 tokens)
│   ├── vm/
│   │   ├── opcodes.rs
│   │   ├── dispatch_handlers.rs  # Opcode handlers + include path resolution
│   │   ├── builtins.rs           # Built-in PHP functions
│   │   ├── builtin_capability_tests.rs  # Broad builtin tests
│   │   ├── execute.rs            # Main loop, __FILE__/__DIR__
│   │   ├── exception_dispatch.rs # try/catch/finally + cross-frame propagation
│   │   ├── class_hierarchy.rs      # extends/implements member resolution
│   │   └── callable.rs           # invoke_user_function + call_user_func helpers
│   ├── jit.rs
│   ├── function_optimizer.rs
│   ├── opt.rs              # Post-compile passes: constant folding + DCE
│   ├── opcode_cache.rs
│   ├── operators.rs        # PHP operator semantics
│   ├── hash.rs             # Hash tables (DJBX33A lives in string.rs)
│   ├── alloc.rs / gc.rs / string.rs / string_intern.rs
│   └── benchmark.rs              # phprs-only benchmark export
└── php/              # Runtime (25 modules: regex, pdo, sqlite, streams, filesystem, …)
```

## Execution flow

```
PHP source
    → Lexer
    → Compiler (OpArray + optional FunctionTable)
    → opt::optimize_script (constant folding + dead code elimination)
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

- **77 opcodes** (arithmetic, control flow, calls, OOP, includes, exceptions, …). 76 have dispatch handlers; `AssignOp` (28) is a reserved slot with no handler and is never emitted.
- **Direct dispatch table** in `dispatch_handlers.rs`
- **Built-ins** delegated from `builtins.rs` to `src/php/*` modules
- **Cross-frame exception propagation**: every call site that runs a callee op-array restores caller state, then `propagate_after_call` (`src/engine/vm/exception_dispatch.rs`) re-dispatches the pending exception against the caller's try regions. `try_stack` entries carry `(TryCatchBegin idx, op_array filename)` so `dispatch_exception` only considers catches in the current op_array.

Authoritative builtin coverage: `src/engine/vm/builtin_capability_tests.rs`.

## CLI (`phprs`)

```bash
phprs run file.php
phprs serve [--port N]
phprs pkg init | install | update | build | run | publish
```

## Framework demos (`examples/`)

| Tree | Entry | Test |
|------|-------|---------|
| WordPress-shaped | `wordpress/index.php` | `example_wordpress_index_runs` |
| CodeIgniter-shaped | `codeigniter/public/index.php` | `example_codeigniter_public_index_runs` |
| Drupal-shaped | `drupal/index.php` | `example_drupal_index_runs` |

All **root** `examples/*.php` files: `examples_root_php_scripts_all_run`.

## Testing layout

| Location | Role |
|----------|------|
| `src/**/tests.rs`, `#[cfg(test)]` | Unit tests |
| `tests/examples_runtime.rs` | PHP example E2E |
| `tests/build_rust_examples.rs` | Rust `examples/rust/` compile |
| `tests/php_examples.rs` | PHP compile smoke |
| `tests/exception_propagation.rs` | Cross-frame try/catch dispatch (18 cases) |
| `tests/interface_implements.rs` | `interface`/`implements` + hierarchy resolution (20 cases) |
| `tests/string_interpolation.rs` | `"$arr[key]"` / `"{$expr}"` interpolation (16 cases) |
| `tests/php8x_features.rs` | PHP 8.x surface (`final const`, `__unset` etc., 93 cases) |
| `tests/call_arg_expressions.rs` | Full precedence in call/ctor/array arguments (4 cases) |
| `tests/loop_control.rs` | `break`/`continue` with explicit levels (16 cases) |
| `tests/switch_statement.rs` | `switch` + `do-while` semantics (15 cases) |
| `tests/opt_passes.rs` | Constant folding + DCE regression (11 cases) |

## Performance notes

JIT, opcode cache, and optimizer modules exist as **scaffolding**. Claims relative to stock PHP belong in [PERFORMANCE.md](PERFORMANCE.md) only with reproducible measurements.

## See also

- [SPEC.md](SPEC.md)
- [QUICKSTART.md](QUICKSTART.md)
- [README.md](README.md)
- [TODO.md](TODO.md)
- [PERFORMANCE.md](PERFORMANCE.md)
- [tests/README.md](tests/README.md)
