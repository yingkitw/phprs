# phprs Examples

This directory contains example programs demonstrating how to use the phprs library.

## Rust Examples

Rust examples demonstrate how to use the phprs library from Rust code:

### `basic_types.rs`
Demonstrates creating and manipulating basic PHP types (long, double, string, boolean, null).

```bash
cargo run --example basic_types
```

### `string_operations.rs`
Shows PHP string handling, including initialization, concatenation, and hashing.

```bash
cargo run --example string_operations
```

### `hash_table.rs`
Demonstrates PHP array/hash table operations, including adding, finding, and iterating elements.

```bash
cargo run --example hash_table
```

### `operators.rs`
Shows PHP operators and type conversions, including arithmetic operations and comparisons.

```bash
cargo run --example operators
```

### `error_handling.rs`
Demonstrates PHP error handling system, including custom error handlers and error reporting.

```bash
cargo run --example error_handling
```

### `filesystem.rs`
Shows PHP filesystem functions, including directory scanning, file existence checks, and file operations.

```bash
cargo run --example filesystem
```

### `memory_management.rs`
Demonstrates PHP memory allocation and statistics tracking.

```bash
cargo run --example memory_management
```

### `runtime.rs`
Shows PHP runtime functions and initialization, including version information and module lifecycle.

```bash
cargo run --example runtime
```

## PHP Examples

PHP example files are test cases that can be executed once the phprs interpreter is fully implemented:

### `basic_types.php`
Demonstrates basic PHP types: integers, floats, strings, booleans, null, and arrays.

### `string_operations.php`
Shows PHP string operations: concatenation, length, and comparison.

### `array_operations.php`
Demonstrates PHP array operations: adding elements, accessing by key, and iteration.

### `operators.php`
Shows PHP operators: arithmetic, comparison, and type conversion.

### `error_handling.php`
Demonstrates PHP error handling: custom error handlers and error reporting.

### `filesystem.php`
Shows PHP filesystem functions: file existence, directory operations, and file reading.

### `variables.php`
Demonstrates PHP variable handling: variable variables and type checking.

### `control_flow.php`
Shows PHP control flow: if-else, switch, for, while, and foreach loops.

### `functions.php`
Demonstrates PHP functions: definitions, parameters, type hints, recursion, and anonymous functions.

### `classes.php`
Shows PHP classes: definitions, constructors, methods, inheritance, and polymorphism.

## Building All Examples

To build all Rust examples:

```bash
cargo build --examples
```

The compiled binaries will be in `target/debug/examples/` or `target/release/examples/` (for release builds).

## Running PHP Examples

Once the phprs interpreter is implemented, PHP examples can be executed with:

```bash
phprs examples/basic_types.php
phprs examples/string_operations.php
# ... etc
```

These PHP files serve as test cases to verify that the phprs implementation correctly handles various PHP language features.
