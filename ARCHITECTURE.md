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

### Opcodes (52 total)
- Arithmetic: Add, Sub, Mul, Div, Mod, Pow
- Comparison: IsEqual, IsSmaller, IsIdentical
- Logical: BoolAnd, BoolOr, BoolXor, BoolNot
- Bitwise: BwAnd, BwOr, BwXor, BwNot, Shift
- String: Concat, StrLen, StrPos, StrReplace
- Control: Jmp, JmpZ, JmpNZ, Return
- Variables: FetchVar, Assign, SendVal, InitFCall
- OOP: NewObj, FetchObjProp, InitMethodCall

### Built-in Functions (40+)
- **String**: strlen, strpos, substr, str_replace, strtolower, strtoupper, trim
- **Array**: explode, implode, array_merge, count, in_array
- **Type**: isset, empty, is_int, is_string, is_array, var_dump
- **I/O**: file_get_contents, file_exists, echo, print
- **JSON**: json_encode, json_decode

## Binary Tools

### php (CLI)
```bash
php file.php              # Compile and display info
```

### php-server (Web Playground)
```bash
php-server                  # Start on http://localhost:3080
```

Features:
- Execute PHP via HTTP API
- Code editor with syntax highlighting
- Opcode viewer
- Dark/light theme
- Multi-language (EN/中文/日本語)

### php-pkg (Package Manager)
```bash
php-pkg init              # Initialize project
php-pkg install           # Install dependencies
```

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
