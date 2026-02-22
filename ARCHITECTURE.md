# phprs Architecture

## Overview

phprs is a PHP interpreter implemented in Rust, maintaining the same architectural structure as the original C PHP implementation while leveraging Rust's safety guarantees.

## Module Structure

```
src/
├── engine/           # Core engine (compiler, VM, types, memory, GC, perf)
│   ├── types.rs      # Core PHP types (Val, PhpString, PhpArray, PhpObject)
│   ├── string.rs     # String handling with DJBX33A hashing
│   ├── hash.rs       # Hash tables (dynamic resizing, collision handling)
│   ├── alloc.rs      # Memory allocation (persistent/non-persistent)
│   ├── gc.rs         # Garbage collection (tri-color marking)
│   ├── operators.rs  # Type conversion and operators
│   ├── array_ops.rs  # Array operations and optimizations
│   ├── vm/           # Virtual machine
│   │   ├── opcodes.rs        # 63 opcode definitions
│   │   ├── execute_data.rs   # Execution context
│   │   ├── dispatch_handlers.rs # Dispatch table (computed goto style)
│   │   ├── handlers.rs       # Opcode handler implementations
│   │   ├── builtins.rs       # 40+ PHP functions
│   │   └── execute.rs        # Main execution loop
│   ├── compile/      # Compiler
│   │   ├── expression/       # Expression parsing
│   │   ├── statement/        # Statement parsing
│   │   ├── control_flow.rs   # Control structures
│   │   └── function.rs      # Function compilation
│   ├── jit.rs        # JIT compilation for hot functions
│   ├── function_optimizer.rs # Inlining and call optimizations
│   ├── opcode_cache.rs      # Opcode cache with optimization passes
│   ├── benchmark.rs  # Benchmark harness
│   ├── perf.rs       # Performance utilities
│   ├── perf_alloc.rs # Performance-oriented allocation
│   ├── facade/       # Factory helpers
│   ├── lexer/        # Tokenizer
│   ├── exception.rs  # Exception handling
│   └── errors.rs     # Error handling
└── php/              # PHP runtime
    ├── runtime.rs    # Main runtime
    ├── ini.rs        # INI configuration
    ├── variables.rs  # Variable handling
    ├── streams.rs    # Stream system
    ├── sapi.rs       # SAPI layer
    ├── output.rs     # Output buffering
    ├── globals.rs    # Global state
    ├── filesystem.rs # Filesystem operations
    └── extension.rs  # Extension framework
```

## Execution Flow

```
PHP Code
    ↓
Lexer (tokens)
    ↓
Compiler (opcodes)
    ↓
VM (execute_ex)
    ↓
Output
```

## Type System

| PHP Type | Rust Representation |
|-----------|-------------------|
| Null | `PhpValue::Null` |
| Boolean | `PhpValue::Bool(bool)` |
| Integer | `PhpValue::Long(i64)` |
| Float | `PhpValue::Double(f64)` |
| String | `PhpValue::String(Box<PhpString>)` |
| Array | `PhpValue::Array(Box<PhpArray>)` |
| Object | `PhpValue::Object(Box<PhpObject>)` |

## Memory Management

- **Ownership**: Rust's compile-time ownership prevents memory leaks
- **Reference Counting**: Automatic tracking for strings, arrays, objects
- **Garbage Collection**: Tri-color marking for cycle detection
- **Pools**: Persistent and non-persistent allocation pools

## Virtual Machine

### Opcodes (63 total)
- Arithmetic: Add, Sub, Mul, Div, Mod, Pow
- Comparison: IsEqual, IsNotEqual, IsSmaller, IsSmallerOrEqual, IsIdentical, IsNotIdentical
- Logical: BoolNot, BoolXor
- Bitwise: BwAnd, BwOr, BwXor, BwNot, Sl, Sr
- String: Concat
- Control: Jmp, JmpZ, JmpNZ, JmpNullZ, Return
- Variables: FetchVar, Assign, SendVal, InitFCall, IsSet, Empty, Unset
- Arrays: InitArray, AddArrayElement, FetchDim, Count, Keys, Values, ArrayDiff
- Type: TypeCheck
- Null coalescing: Coalesce
- OOP: NewObj, FetchObjProp, InitMethodCall, AssignObjProp, DoMethodCall
- Functions: DoFCall, Include
- Other: Nop, Echo, AssignDim, AssignObj, AssignStaticProp, AssignOp, InitArray,
          NewObj, Throw, TryCatchBegin, TryCatchEnd, CatchBegin, CatchEnd,
          FinallyBegin, FinallyEnd, TypeCheck, QmAssign (ternary assign)

### Built-in Functions (40+)
- **String**: strlen, strpos, substr, str_replace, strtolower, strtoupper, trim, explode, implode, sprintf
- **Array**: count, sizeof, in_array, array_key_exists, array_merge, array_push
- **Type**: isset, empty, is_int, is_integer, is_long, is_string, is_float, is_double, is_bool, is_null, is_array, intval, floatval, doubleval, strval
- **I/O**: echo, file_get_contents, file_exists
- **Debug**: var_dump, print_r
- **JSON**: json_encode, json_decode

## CLI (`phprs`)

Single binary with three subcommands:

```bash
phprs run file.php        # Execute a PHP file
phprs serve               # Start web playground on http://localhost:3080
phprs serve --port 8080   # Custom port
phprs pkg init            # Initialize project (composer.json)
phprs pkg install         # Install dependencies
phprs pkg build           # Build project
```

### Web Playground (`phprs serve`)
- Execute PHP via HTTP API
- Code editor with syntax highlighting
- Opcode viewer
- Dark/light theme
- Multi-language (EN/中文/日本語)

## Framework support (planned)

Roadmap items in [TODO.md](TODO.md):

- **CodeIgniter 4**: Bootstrap, autoloading, routing, controllers
- **Drupal**: Bootstrap (index.php → Drupal.php), kernel, module system
- **WordPress**: Bootstrap (index.php → wp-blog-header.php → wp-load.php → wp-settings.php), wp-config and wpdb, hooks/filters (do_action, apply_filters), theme and plugin loading

## Comparison with C PHP

| Aspect | C PHP | phprs |
|---------|-------|-------|
| Memory Safety | Manual | Ownership system |
| Type Safety | Runtime checks | Compile-time |
| Concurrency | TSRM | Mutex/OnceLock |
| Error Handling | Return codes | Result<T, E> |
| Memory Leaks | Possible | Prevented |
| Use-after-free | Possible | Prevented |

## See Also

- [spec.md](spec.md) - Project specification and scope
- [README.md](README.md) - Quick start guide
- [TODO.md](TODO.md) - Migration roadmap and statistics
- [PERFORMANCE.md](PERFORMANCE.md) - Optimizations and benchmarks vs PHP 8
