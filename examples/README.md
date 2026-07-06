# phprs Examples

Curated demos for language features, standard-library subsets, and minimal framework bootstraps. **21** root-level PHP scripts plus **9** Rust library examples.

## Quick start

```bash
cargo run -p phprs-cli -- run examples/01_hello_world.php
```

Or the release binary:

```bash
./target/release/phprs run examples/01_hello_world.php
```

## Automated coverage

CI runs:

- **`cargo test --test examples_runtime`** — every `examples/*.php` at this directory’s root (not recursive) must compile and exit `PhpResult::Success`
- **`cargo test --test build_rust_examples`** — all `examples/rust/*.rs` examples build
- **`cargo test --test example_verification`** — selected Rust demos execute

See [TEST-GUIDE.md](TEST-GUIDE.md) and [../tests/README.md](../tests/README.md).

## Rust examples (`rust/`)

```bash
cargo run --example basic_types
cargo run --example string_operations
cargo run --example hash_table
cargo run --example operators
cargo run --example error_handling
cargo run --example filesystem
cargo run --example memory_management
cargo run --example runtime
cargo run --example performance_demo
```

## Root PHP examples

| File | Description |
|------|-------------|
| `01_hello_world.php` | Hello world |
| `basic_types.php` | Scalars and null |
| `variables.php` | Variables and `isset` / type checks |
| `operators.php` | Arithmetic, concat, builtins |
| `control_flow.php` | if/else, switch, for, while, foreach (value-only) |
| `string_operations.php` | Concat, `strlen`, comparison |
| `array_operations.php` | Arrays and foreach |
| `functions.php` | Function syntax (UDF execution still limited in CLI) |
| `classes.php` | Classes and methods (partial parity) |
| `error_handling.php` | Error handlers |
| `filesystem.php` | File/dir helpers |
| `match_expression.php` | `match` expressions |
| `attributes.php` | Attributes (partial) |
| `generators.php` | `yield` (partial) |
| `mbstring.php` | `mb_*` subset |
| `regex-examples.php` | `preg_*` demos (Rust regex; no look-ahead) |
| `http-stream-examples.php` | HTTP stream patterns |
| `session-examples.php` | **Simulated** `$_SESSION` (no `session_*` builtins) |
| `pdo-examples.php` | PDO stub API |
| `integration-test.php` | Combined PDO/regex/password/json smoke script |
| `test-streams-regex-pdo.php` | Streams/regex/PDO checklist |

## Framework-style trees

| Entry | Test |
|-------|------|
| `wordpress/index.php` | Manual only — `wp-includes/wp-db.php` uses `array()` syntax not yet supported by the compiler |
| `wordpress/test-theme-plugin.php` | Manual — same constraints |
| `codeigniter/public/index.php` | `example_codeigniter_public_index_runs` |
| `drupal/index.php` | `example_drupal_index_runs` |

### WordPress-shaped demo

```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

See [wordpress/README.md](wordpress/README.md).

### CodeIgniter 4-shaped demo

```bash
cargo run -p phprs-cli -- run examples/codeigniter/public/index.php
```

See [codeigniter/README.md](codeigniter/README.md).

### Drupal-shaped demo

```bash
cargo run -p phprs-cli -- run examples/drupal/index.php
```

See [drupal/README.md](drupal/README.md).

## More docs

- [STREAMS-REGEX-PDO-README.md](STREAMS-REGEX-PDO-README.md) — API notes and limitations
- [TEST-GUIDE.md](TEST-GUIDE.md) — how to run feature demos
- [TESTING-SUMMARY.md](TESTING-SUMMARY.md) — coverage summary
