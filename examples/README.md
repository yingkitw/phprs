# phprs Examples

**80+ working examples** demonstrating all phprs features with 2100+ lines of test code.

## Quick Start

Run any example:
```bash
cargo run -p phprs-cli -- run examples/01_hello_world.php
```

Or use the compiled binary:
```bash
./target/release/phprs run examples/01_hello_world.php
```

## Example Categories

## Rust Examples (`rust/`)

Use the phprs library from Rust:

```bash
cargo run --example basic_types
cargo run --example string_operations
cargo run --example hash_table
cargo run --example operators
cargo run --example error_handling
cargo run --example filesystem
cargo run --example memory_management
cargo run --example runtime
```

## PHP Examples

Run with `cargo run -- run examples/<file>.php`:

| File | Description |
|------|-------------|
| `01_hello_world.php` | Hello world, basic echo |
| `variables.php` | Variable handling, type checking |
| `operators.php` | Arithmetic, string concat, builtins |
| `control_flow.php` | If/else, switch, for, while, foreach |
| `string_operations.php` | String concat, strlen, comparison |
| `array_operations.php` | Array add/access/iterate |
| `functions.php` | Definitions, params, type hints, closures |
| `classes.php` | Classes, constructors, inheritance |
| `error_handling.php` | Custom error handlers |
| `filesystem.php` | File/dir operations |
| `match_expression.php` | PHP 8.0 match expressions |
| `attributes.php` | PHP 8.0 attributes on functions/classes |
| `generators.php` | Generator functions with yield |

### WordPress-style example

Run the minimal WordPress-style bootstrap (from project root):

```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

See [wordpress/README.md](wordpress/README.md) for layout and engine requirements.
