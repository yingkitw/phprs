# phprs Architecture

## Overview

phprs is a PHP interpreter implemented in Rust, maintaining the same architectural structure as the original C PHP implementation while leveraging Rust's safety guarantees.

## Module Structure

```
src/
├── engine/           # Core engine (compiler, VM, types, memory, GC)
│   ├── types.rs      # Core PHP types (Val, PhpString, PhpArray, PhpObject)
│   ├── string.rs     # String handling with DJBX33A hashing
│   ├── hash.rs       # Hash tables (dynamic resizing, collision handling)
│   ├── alloc.rs      # Memory allocation (persistent/non-persistent)
│   ├── gc.rs         # Garbage collection (tri-color marking)
│   ├── operators.rs   # Type conversion and operators
│   ├── vm/           # Virtual machine
│   │   ├── opcodes.rs      # 52 opcode definitions
│   │   ├── execute_data.rs # Execution context
│   │   ├── handlers.rs     # Opcode dispatch
│   │   ├── builtins.rs     # 40+ PHP functions
│   │   └── execute.rs      # Main execution loop
│   ├── compile/      # Compiler
│   │   ├── expression/     # Expression parsing
│   │   ├── statement/      # Statement parsing
│   │   ├── control_flow.rs # Control structures
│   │   └── function.rs     # Function compilation
│   ├── facade/       # Factory helpers
│   ├── lexer/        # Tokenizer
│   ├── exception.rs   # Exception handling
│   └── errors.rs      # Error handling
└── php/              # PHP runtime
    ├── runtime.rs      # Main runtime
    ├── ini.rs          # INI configuration
    ├── variables.rs    # Variable handling
    ├── streams.rs      # Stream system
    ├── sapi.rs         # SAPI layer
    ├── output.rs       # Output buffering
    ├── globals.rs      # Global state
    ├── filesystem.rs   # Filesystem operations
    └── extension.rs   # Extension framework
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

### Opcodes (64 total)
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
          FinallyBegin, FinallyEnd, TypeCheck

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

- [README.md](README.md) - Quick start guide
- [TODO.md](TODO.md) - Migration roadmap
